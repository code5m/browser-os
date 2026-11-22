//! Idempotent application shutdown with one absolute deadline.
//!
//! Resource owners start independently unless an explicit dependency is declared.
//! The coordinator retains the scheduler thread; that scheduler retains all task
//! JoinHandles until they finish. A deadline returns Pending, never a successful
//! cleanup claim. Process exit still ends these owners: recoverable native state
//! therefore has its own durable, identity-checked recovery journal.
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const APPLICATION_SHUTDOWN_BUDGET: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupOutcome {
    Ok,
    Failed(String),
    Panicked(String),
    /// Work has not completed by the application's shared deadline.
    Pending(String),
}

#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub name: String,
    pub outcome: CleanupOutcome,
}
impl CleanupReport {
    pub fn is_failure(&self) -> bool {
        !matches!(self.outcome, CleanupOutcome::Ok)
    }
}

#[derive(Debug, Clone)]
pub struct ShutdownReport {
    pub reports: Vec<CleanupReport>,
    pub already_shutdown: bool,
    /// True only after the coordinator has joined all started task owners.
    /// Individual outcomes still determine whether resource cleanup succeeded.
    pub owners_complete: bool,
}
impl ShutdownReport {
    pub fn ok(&self) -> bool {
        !self.already_shutdown && self.owners_complete && self.failures().is_empty()
    }
    pub fn failures(&self) -> Vec<&CleanupReport> {
        self.reports.iter().filter(|r| r.is_failure()).collect()
    }
    pub fn pending(&self) -> usize {
        self.reports.iter().filter(|r| matches!(r.outcome, CleanupOutcome::Pending(_))).count()
    }
    /// Legacy accessor: number of task reports, including Pending. This is
    /// never evidence that pending cleanup actually completed.
    pub fn executed(&self) -> usize {
        self.reports.len()
    }
}

type CleanupTask = Box<dyn FnOnce(Instant) -> Result<(), String> + Send>;
struct RegisteredTask {
    name: String,
    after: Vec<String>,
    run: CleanupTask,
}
struct Progress {
    reports: Vec<CleanupReport>,
    finished: bool,
}
struct Execution {
    progress: Mutex<Progress>,
    changed: Condvar,
    // Retain captures when spawning the scheduler/task thread itself fails.
    // A failed Builder::spawn otherwise drops its closure before it can run.
    scheduled: Mutex<Option<Vec<RegisteredTask>>>,
    retained_tasks: Mutex<Vec<Arc<Mutex<Option<CleanupTask>>>>>,
}
struct Inner {
    tasks: Vec<RegisteredTask>,
    closed: bool,
    execution: Option<Arc<Execution>>,
    /// Own the scheduler even after the foreground deadline has expired.
    worker: Option<JoinHandle<()>>,
}
pub struct ShutdownCoordinator {
    inner: Mutex<Inner>,
}
impl Default for ShutdownCoordinator {
    fn default() -> Self { Self::new() }
}
impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self { inner: Mutex::new(Inner {
            tasks: Vec::new(), closed: false, execution: None, worker: None,
        }) }
    }

    /// Preserve the historical registration order for ordinary tasks.
    pub fn register<F>(&self, name: impl Into<String>, task: F) -> Result<(), String>
    where F: FnOnce() -> Result<(), String> + Send + 'static {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let after = inner.tasks.last().map(|t| vec![t.name.clone()]).unwrap_or_default();
        Self::insert(&mut inner, name.into(), after, Box::new(move |_| task()))
    }

    /// Independent resource owners share the SAME deadline. Dependencies only
    /// impose completion order; an error in one task does not suppress cleanup.
    pub fn register_with_deadline<F>(
        &self, name: impl Into<String>, after: &[&str], task: F,
    ) -> Result<(), String>
    where F: FnOnce(Instant) -> Result<(), String> + Send + 'static {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        Self::insert(&mut inner, name.into(),
            after.iter().map(|name| (*name).to_owned()).collect(), Box::new(task))
    }

    fn insert(inner: &mut Inner, name: String, after: Vec<String>, run: CleanupTask) -> Result<(), String> {
        if inner.closed { return Err(format!("关闭已开始，拒绝注册清理任务：{name}")); }
        if inner.tasks.iter().any(|t| t.name == name) {
            return Err(format!("SHUTDOWN_DUPLICATE_TASK: {name}"));
        }
        // Depend only on already-registered tasks: cycles cannot be constructed.
        if after.iter().any(|dependency| !inner.tasks.iter().any(|t| &t.name == dependency)) {
            return Err(format!("SHUTDOWN_UNKNOWN_DEPENDENCY: {name}"));
        }
        inner.tasks.push(RegisteredTask { name, after, run });
        Ok(())
    }

    #[allow(dead_code)]
    pub fn is_shutdown(&self) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.execution.as_ref().map(|execution| {
            let progress = execution.progress.lock().unwrap_or_else(|e| e.into_inner());
            progress.finished && !progress.reports.iter()
                .any(|report| matches!(report.outcome, CleanupOutcome::Pending(_)))
        }).unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn last_report(&self) -> Option<Arc<ShutdownReport>> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.execution.as_ref().map(|execution| {
            let progress = execution.progress.lock().unwrap_or_else(|e| e.into_inner());
            Arc::new(ShutdownReport {
                reports: progress.reports.clone(), already_shutdown: false,
                owners_complete: progress.finished,
            })
        })
    }

    #[allow(dead_code)]
    pub fn pending_tasks(&self) -> usize {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(execution) = &inner.execution {
            execution.progress.lock().unwrap_or_else(|e| e.into_inner()).reports.iter()
                .filter(|r| matches!(r.outcome, CleanupOutcome::Pending(_))).count()
        } else { inner.tasks.len() }
    }

    pub fn shutdown(&self) -> Arc<ShutdownReport> {
        self.shutdown_until(Instant::now() + APPLICATION_SHUTDOWN_BUDGET)
    }

    /// The caller never adds a fresh timeout for each task or terminal.
    /// Reentrant/concurrent requests return immediately without waiting on locks
    /// held by the first caller's cleanup workers.
    pub fn shutdown_until(&self, deadline: Instant) -> Arc<ShutdownReport> {
        let execution = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if inner.closed {
                return Arc::new(ShutdownReport {
                    reports: Vec::new(), already_shutdown: true, owners_complete: false,
                });
            }
            inner.closed = true;
            let tasks = std::mem::take(&mut inner.tasks);
            let execution = Arc::new(Execution {
                progress: Mutex::new(Progress {
                    reports: tasks.iter().map(|t| CleanupReport {
                        name: t.name.clone(),
                        outcome: CleanupOutcome::Pending("SHUTDOWN_WAITING_FOR_DEPENDENCY".into()),
                    }).collect(),
                    finished: tasks.is_empty(),
                }),
                changed: Condvar::new(),
                scheduled: Mutex::new(Some(tasks)),
                retained_tasks: Mutex::new(Vec::new()),
            });
            inner.execution = Some(execution.clone());
            let has_tasks = !execution.progress.lock().unwrap_or_else(|e| e.into_inner()).reports.is_empty();
            if has_tasks {
                let progress = execution.clone();
                match std::thread::Builder::new().name("browseros-cleanup-owner".into())
                    .spawn(move || run_tasks(deadline, progress)) {
                    Ok(worker) => inner.worker = Some(worker),
                    Err(error) => {
                        let mut state = execution.progress.lock().unwrap_or_else(|e| e.into_inner());
                        for report in &mut state.reports {
                            report.outcome = CleanupOutcome::Pending(format!("SHUTDOWN_WORKER_START_FAILED: {error}"));
                        }
                        state.finished = true;
                    }
                }
            }
            execution
        };
        let mut state = execution.progress.lock().unwrap_or_else(|e| e.into_inner());
        while !state.finished {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() { break; }
            state = execution.changed.wait_timeout(state, remaining)
                .unwrap_or_else(|e| e.into_inner()).0;
        }
        let report = Arc::new(ShutdownReport {
            reports: state.reports.clone(), already_shutdown: false,
            owners_complete: state.finished,
        });
        drop(state);
        // Join only when the owned scheduler has actually ended. A pending
        // thread remains owned, and is never mistaken for completed cleanup.
        let worker = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if inner.worker.as_ref().is_some_and(|worker| worker.is_finished()) {
                inner.worker.take()
            } else { None }
        };
        if let Some(worker) = worker { let _ = worker.join(); }
        report
    }
}

fn run_tasks(deadline: Instant, execution: Arc<Execution>) {
    let tasks = execution.scheduled.lock().unwrap_or_else(|e| e.into_inner())
        .take().expect("scheduler owns the registered task list once");
    let names: Vec<String> = tasks.iter().map(|task| task.name.clone()).collect();
    let mut waiting: Vec<Option<RegisteredTask>> = tasks.into_iter().map(Some).collect();
    let mut finished = vec![false; waiting.len()];
    let mut running = vec![false; waiting.len()];
    let mut workers = Vec::new();
    let (sender, receiver) = std::sync::mpsc::channel::<(usize, CleanupOutcome)>();
    let mut completed = 0;
    while completed < waiting.len() {
        for index in 0..waiting.len() {
            let eligible = waiting[index].as_ref().is_some_and(|task|
                task.after.iter().all(|dependency|
                    names.iter().position(|name| name == dependency).is_some_and(|i| finished[i])
                )
            );
            if !eligible { continue; }
            let task = waiting[index].take().expect("eligible task");
            {
                let mut state = execution.progress.lock().unwrap_or_else(|e| e.into_inner());
                state.reports[index].outcome = CleanupOutcome::Pending("SHUTDOWN_CLEANUP_IN_PROGRESS".into());
            }
            let sender = sender.clone();
            let name = task.name;
            let retained = Arc::new(Mutex::new(Some(task.run)));
            execution.retained_tasks.lock().unwrap_or_else(|e| e.into_inner()).push(retained.clone());
            match std::thread::Builder::new().name(format!("cleanup-{name}"))
                .spawn(move || {
                    let task = retained.lock().unwrap_or_else(|e| e.into_inner())
                        .take().expect("cleanup task is owned once");
                    let outcome = match catch_unwind(AssertUnwindSafe(|| task(deadline))) {
                        Ok(Ok(())) => CleanupOutcome::Ok,
                        Ok(Err(message)) => CleanupOutcome::Failed(message),
                        Err(payload) => CleanupOutcome::Panicked(panic_message(payload)),
                    };
                    let _ = sender.send((index, outcome));
                }) {
                Ok(worker) => {
                    running[index] = true;
                    workers.push(worker);
                }
                Err(error) => {
                    // The task never ran. Keep dependent resources pending;
                    // recording a spawn failure does not complete a dependency.
                    completed += 1;
                    execution.progress.lock().unwrap_or_else(|e| e.into_inner()).reports[index].outcome =
                        CleanupOutcome::Pending(format!("SHUTDOWN_TASK_START_FAILED: {error}"));
                    execution.changed.notify_all();
                }
            }
        }
        if completed == waiting.len() { break; }
        if !running.iter().any(|active| *active) {
            // Only dependencies of tasks that could not start remain. Retain
            // their captures too; never run them out of order, spin or wait on
            // a result that no worker can send.
            for (index, task) in waiting.iter_mut().enumerate() {
                if let Some(task) = task.take() {
                    execution.retained_tasks.lock().unwrap_or_else(|e| e.into_inner())
                        .push(Arc::new(Mutex::new(Some(task.run))));
                    execution.progress.lock().unwrap_or_else(|e| e.into_inner()).reports[index].outcome =
                        CleanupOutcome::Pending("SHUTDOWN_DEPENDENCY_NOT_EXECUTED".into());
                }
            }
            break;
        }
        if let Ok((index, outcome)) = receiver.recv() {
            running[index] = false;
            finished[index] = true;
            completed += 1;
            execution.progress.lock().unwrap_or_else(|e| e.into_inner()).reports[index].outcome = outcome;
            execution.changed.notify_all();
        }
    }
    // The application deadline is enforced by the caller, not by detaching
    // these owners. A blocked task and its resources stay in this owner thread.
    for worker in workers { let _ = worker.join(); }
    execution.progress.lock().unwrap_or_else(|e| e.into_inner()).finished = true;
    execution.changed.notify_all();
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() { return (*message).to_owned(); }
    if let Some(message) = payload.downcast_ref::<String>() { return message.clone(); }
    "未知 panic（无可读 payload）".to_owned()
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn counter_task(counter: Arc<AtomicUsize>) -> impl FnOnce() -> Result<(), String> {
        move || {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_shutdown_reaps_real_child_pid_and_joins_owned_worker() {
        use std::{
            process::{Child, Command},
            sync::atomic::AtomicBool,
        };
        struct KillOnDrop(Child);
        impl Drop for KillOnDrop {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        // A disposable owned child is the only process touched by this fixture.
        let child = Command::new("/usr/bin/sleep")
            .arg("30")
            .spawn()
            .expect("sleep test fixture");
        let pid = child.id();
        let owned = Arc::new(Mutex::new(KillOnDrop(child)));
        assert!(std::path::Path::new(&format!("/proc/{pid}")).exists());
        let stop = Arc::new(AtomicBool::new(false));
        let running = stop.clone();
        let worker = std::thread::spawn(move || {
            while !running.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        let coordinator = ShutdownCoordinator::new();
        let own_child = owned.clone();
        coordinator
            .register("fixture-child", move || {
                let mut guard = own_child.lock().map_err(|_| "CHILD_LOCK".to_owned())?;
                guard.0.kill().map_err(|error| error.to_string())?;
                guard.0.wait().map_err(|error| error.to_string())?;
                Ok(())
            })
            .unwrap();
        coordinator
            .register("fixture-worker", move || {
                stop.store(true, Ordering::SeqCst);
                worker.join().map_err(|_| "WORKER_JOIN_PANIC".to_owned())
            })
            .unwrap();
        let report = coordinator.shutdown();
        assert!(
            report.ok(),
            "synthetic OS child/worker cleanup: {:?}",
            report.failures()
        );
        assert_eq!(report.executed(), 2);
        assert!(owned.lock().unwrap().0.try_wait().unwrap().is_some());
        assert!(coordinator.shutdown().already_shutdown);
        // This asserts ONLY the fixture child's reaping, not BrowserOS GUI leaks.
    }

    #[test]
    fn repeated_shutdown_runs_each_task_exactly_once() {
        let coordinator = ShutdownCoordinator::new();
        let a = Arc::new(AtomicUsize::new(0));
        let b = Arc::new(AtomicUsize::new(0));
        coordinator
            .register("a", counter_task(a.clone()))
            .expect("注册 a");
        coordinator
            .register("b", counter_task(b.clone()))
            .expect("注册 b");

        let first = coordinator.shutdown();
        let second = coordinator.shutdown();
        let third = coordinator.shutdown();

        assert_eq!(first.executed(), 2, "首次调用应执行全部任务");
        assert!(first.ok(), "全部成功应报告 ok");
        assert_eq!(a.load(Ordering::SeqCst), 1, "任务 a 只能执行一次");
        assert_eq!(b.load(Ordering::SeqCst), 1, "任务 b 只能执行一次");
        assert!(second.already_shutdown, "第二次调用应标记为已关闭");
        assert!(second.reports.is_empty(), "重复调用不得产生新的清理副作用");
        assert!(third.already_shutdown);
        assert!(coordinator.is_shutdown());
        // 重复调用只返回空报告，历史结果仍可查询，避免诊断信息丢失。
        assert_eq!(
            coordinator.last_report().map(|r| r.executed()),
            Some(2),
            "last_report 应保留首次执行的完整结果"
        );
    }

    #[test]
    fn concurrent_shutdown_runs_tasks_only_once() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let hits = Arc::new(AtomicUsize::new(0));
        coordinator
            .register("once", counter_task(hits.clone()))
            .expect("注册");

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let c = coordinator.clone();
                std::thread::spawn(move || c.shutdown())
            })
            .collect();
        let reports: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        assert_eq!(hits.load(Ordering::SeqCst), 1, "并发调用只执行一次清理");
        assert_eq!(
            reports.iter().filter(|r| !r.already_shutdown).count(),
            1,
            "有且只有一个调用者真正执行清理"
        );
    }

    #[test]
    fn failing_and_panicking_tasks_do_not_block_others() {
        let coordinator = ShutdownCoordinator::new();
        let after_failure = Arc::new(AtomicUsize::new(0));
        let after_panic = Arc::new(AtomicUsize::new(0));

        coordinator
            .register("boom", || Err("模拟清理失败".to_string()))
            .expect("注册失败任务");
        coordinator
            .register("panic", || panic!("模拟清理 panic"))
            .expect("注册 panic 任务");
        coordinator
            .register("after-failure", counter_task(after_failure.clone()))
            .expect("注册");
        coordinator
            .register("after-panic", counter_task(after_panic.clone()))
            .expect("注册");

        let report = coordinator.shutdown();

        assert_eq!(report.executed(), 4, "失败/panic 不得中断后续任务");
        assert_eq!(after_failure.load(Ordering::SeqCst), 1);
        assert_eq!(after_panic.load(Ordering::SeqCst), 1);
        let failures = report.failures();
        assert_eq!(failures.len(), 2, "两个失败任务均应入报告");
        assert!(matches!(
            failures[0].outcome,
            CleanupOutcome::Failed(ref m) if m == "模拟清理失败"
        ));
        assert!(
            matches!(failures[1].outcome, CleanupOutcome::Panicked(_)),
            "panic 必须被捕获为 Panicked，而不是传播"
        );
        assert!(!report.ok());
    }

    #[test]
    fn register_after_shutdown_is_rejected() {
        let coordinator = ShutdownCoordinator::new();
        coordinator.shutdown();

        let ran = Arc::new(AtomicUsize::new(0));
        let err = coordinator
            .register("late", counter_task(ran.clone()))
            .expect_err("关闭后注册必须被拒绝");

        assert!(err.contains("late"));
        assert_eq!(ran.load(Ordering::SeqCst), 0, "被拒绝的任务不得执行");
    }

    #[test]
    fn reentrant_shutdown_inside_cleanup_does_not_deadlock() {
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let inner_ran = Arc::new(AtomicUsize::new(0));
        let nested = Arc::new(AtomicUsize::new(0));

        let c_for_task = coordinator.clone();
        let nested_counter = nested.clone();
        coordinator
            .register("reentrant", move || {
                inner_ran.fetch_add(1, Ordering::SeqCst);
                // 清理任务内部再次调用 shutdown：必须立即返回，不得加锁等待自身。
                let report = c_for_task.shutdown();
                assert!(
                    report.already_shutdown,
                    "重入调用应标记为进行中/已关闭，不得重复执行"
                );
                nested_counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .expect("注册重入任务");

        // 用带超时的线程守护：死锁时测试在 5 秒后失败，而不是悬挂。
        let (tx, rx) = std::sync::mpsc::channel();
        let c = coordinator.clone();
        std::thread::spawn(move || {
            let report = c.shutdown();
            let _ = tx.send(report);
        });

        let report = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("重入调用不得死锁（5 秒超时）");
        assert_eq!(nested.load(Ordering::SeqCst), 1, "重入后任务应正常收尾");
        assert_eq!(report.executed(), 1);
        assert!(report.ok());
    }

    #[test]
    fn shared_deadline_reports_pending_without_blocking_independent_cleanup() {
        let coordinator = ShutdownCoordinator::new();
        let (release, blocked) = std::sync::mpsc::channel();
        let independent = Arc::new(AtomicUsize::new(0));
        coordinator.register("blocked-profile-owner", move || {
            blocked.recv().map_err(|_| "FIXTURE_RELEASE_FAILED".to_owned())?;
            Ok(())
        }).unwrap();
        let hits = independent.clone();
        coordinator.register_with_deadline("independent-job-owner", &[], move |_| {
            hits.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }).unwrap();
        let deadline = Instant::now() + Duration::from_millis(100);
        let report = coordinator.shutdown_until(deadline);
        assert!(!report.ok(), "unreleased owner must not become a PASS");
        assert!(report.pending() >= 1);
        // Allow the retained owner to complete even when a slow host has not
        // scheduled the independent worker before the foreground deadline.
        release.send(()).unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        while !coordinator.is_shutdown() && Instant::now() < until {
            std::thread::yield_now();
        }
        assert!(coordinator.is_shutdown(), "timed-out owners must remain managed");
        assert_eq!(independent.load(Ordering::SeqCst), 1);
        assert!(coordinator.last_report().unwrap().ok());
        assert!(coordinator.shutdown().already_shutdown);
    }

    #[test]
    fn independent_owners_receive_the_same_absolute_deadline() {
        let coordinator = ShutdownCoordinator::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        for index in 0..32 {
            let seen = seen.clone();
            coordinator.register_with_deadline(format!("resource-{index}"), &[], move |deadline| {
                seen.lock().unwrap().push(deadline);
                Ok(())
            }).unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        assert!(coordinator.shutdown_until(deadline).ok());
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 32);
        assert!(seen.iter().all(|actual| *actual == deadline));
    }

    #[test]
    fn cleanup_dependency_finishes_before_dependent_resource_closes() {
        let coordinator = ShutdownCoordinator::new();
        let flushed = Arc::new(AtomicUsize::new(0));
        let marker = flushed.clone();
        coordinator.register_with_deadline("flush", &[], move |_| {
            marker.store(1, Ordering::SeqCst);
            Err("FIXTURE_PERSIST_FAILURE".into())
        }).unwrap();
        coordinator.register_with_deadline("close", &["flush"], move |_| {
            assert_eq!(flushed.load(Ordering::SeqCst), 1);
            Ok(())
        }).unwrap();
        let report = coordinator.shutdown();
        assert_eq!(report.failures().len(), 1);
        assert!(matches!(report.reports[1].outcome, CleanupOutcome::Ok));
    }

    #[test]
    fn empty_coordinator_shutdown_is_ok() {
        let coordinator = ShutdownCoordinator::new();
        let report = coordinator.shutdown();
        assert!(report.ok(), "无任务时关闭应成功");
        assert_eq!(report.executed(), 0);
        assert!(coordinator.shutdown().already_shutdown);
    }
}

// ===========================================================================
// M0-2.d：重复退出 / 半初始化退出 / 失败降级 回归矩阵
//
// 任务名与顺序刻意对齐 `bridge::register_shutdown_tasks` 的真实注册顺序
// （stop-background-workers → flush-sessions → close-tabs → kill-terminals →
// shutdown-grid；M1-9 在 close-tabs 前插入 flush-sessions——先确定会话数据的
// 落盘/释放，再销毁子 webview），这样失败归因可以直接对应到生产路径，
// 而不是只为测试造的假名。
// ===========================================================================
#[cfg(test)]
mod m0_2d_lifecycle_tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

    /// 记录任务执行次数与顺序的探针，行为贴近真实清理任务（可被外部注入失败/panic）。
    struct Probe {
        calls: Arc<Mutex<HashMap<String, usize>>>,
        order: Arc<Mutex<Vec<String>>>,
    }

    impl Probe {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(HashMap::new())),
                order: Arc::new(Mutex::new(Vec::new())),
            }
        }

        /// 生成一个注册到协调器的清理任务；`fail`/`panic` 用于注入降级场景。
        fn task(
            &self,
            name: &'static str,
            fail: bool,
            should_panic: bool,
        ) -> impl FnOnce() -> Result<(), String> {
            let calls = self.calls.clone();
            let order = self.order.clone();
            move || {
                *calls.lock().unwrap().entry(name.to_string()).or_insert(0) += 1;
                order.lock().unwrap().push(name.to_string());
                if should_panic {
                    panic!("{name} 模拟 panic");
                }
                if fail {
                    return Err(format!("{name} 模拟失败"));
                }
                Ok(())
            }
        }

        fn count(&self, name: &str) -> usize {
            self.calls.lock().unwrap().get(name).copied().unwrap_or(0)
        }

        fn order(&self) -> Vec<String> {
            self.order.lock().unwrap().clone()
        }
    }

    /// 注册 M0-2.c 的四类真实任务，`failing` 指定哪些任务需要降级模拟。
    fn register_realistic_tasks(
        coordinator: &ShutdownCoordinator,
        probe: &Probe,
        failing: &[&'static str],
        panicking: &[&'static str],
    ) {
        for name in [
            "stop-background-workers",
            "close-tabs",
            "kill-terminals",
            "shutdown-grid",
        ] {
            coordinator
                .register(
                    name,
                    probe.task(name, failing.contains(&name), panicking.contains(&name)),
                )
                .expect("注册真实清理任务");
        }
    }

    // ---------- 1. 重复退出 ----------

    #[test]
    fn dual_exit_paths_run_cleanup_only_once() {
        // 真实场景：窗口 CloseRequested 与 RunEvent::ExitRequested 是两条独立入口，
        // 用户点关闭后系统再发退出事件，清理只能执行一次。
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        register_realistic_tasks(&coordinator, &probe, &[], &[]);

        let window_close = coordinator.shutdown();
        let system_exit = coordinator.shutdown();

        assert_eq!(window_close.executed(), 4, "窗口关闭入口执行全部任务");
        assert!(window_close.ok());
        assert!(
            system_exit.already_shutdown && system_exit.reports.is_empty(),
            "系统退出入口不得重复执行清理"
        );
        for name in [
            "stop-background-workers",
            "close-tabs",
            "kill-terminals",
            "shutdown-grid",
        ] {
            assert_eq!(probe.count(name), 1, "{name} 只能执行一次");
        }
    }

    #[test]
    fn concurrent_exit_paths_run_cleanup_only_once() {
        // 两条退出入口同时到达（例如关闭窗口的同时系统关机）：仍只允许一个执行者。
        let coordinator = Arc::new(ShutdownCoordinator::new());
        let probe = Probe::new();
        register_realistic_tasks(&coordinator, &probe, &[], &[]);

        let (tx, rx) = std::sync::mpsc::channel();
        for label in ["window-close", "system-exit", "menu-quit"] {
            let c = coordinator.clone();
            let tx = tx.clone();
            std::thread::spawn(move || {
                let report = c.shutdown();
                let _ = tx.send((label, report));
            });
        }
        drop(tx);

        let mut executed = 0;
        let mut seen = 0;
        while let Ok((_, report)) = rx.recv_timeout(Duration::from_secs(5)) {
            seen += 1;
            if !report.already_shutdown {
                executed += 1;
            }
        }
        assert_eq!(seen, 3, "三条退出路径都应拿到报告（无死锁、无丢报告）");
        assert_eq!(executed, 1, "三条并发退出路径只能有一个真正执行清理");
        assert_eq!(probe.count("shutdown-grid"), 1, "grid 关闭不得重复执行");
    }

    #[test]
    fn repeated_exit_after_failure_does_not_retry() {
        // 失败降级 + 重复退出：清理失败后再次触发退出，不得重跑失败任务
        // （否则会出现「关不掉就反复 kill」的副作用放大）。
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        register_realistic_tasks(&coordinator, &probe, &["shutdown-grid"], &[]);

        let first = coordinator.shutdown();
        let second = coordinator.shutdown();

        assert_eq!(first.failures().len(), 1, "grid 失败应入报告");
        assert_eq!(probe.count("shutdown-grid"), 1, "失败任务不得被重试");
        assert!(second.already_shutdown && second.reports.is_empty());
        assert!(coordinator.is_shutdown(), "失败也须标记为已关闭");
    }

    // ---------- 2. 半初始化退出 ----------

    #[test]
    fn shutdown_before_any_task_registered_is_safe() {
        // 半初始化：setup 尚未注册任何清理任务就退出（启动早期崩溃/用户秒关）。
        let coordinator = ShutdownCoordinator::new();
        let report = coordinator.shutdown();

        assert!(report.ok(), "无任务时关闭应成功，不得因半初始化而失败");
        assert_eq!(report.executed(), 0);
        assert!(coordinator.is_shutdown());
    }

    #[test]
    fn shutdown_during_partial_registration_runs_only_registered_tasks() {
        // 半初始化：注册到一半就退出（只注册了 stop-background-workers）。
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        coordinator
            .register(
                "stop-background-workers",
                probe.task("stop-background-workers", false, false),
            )
            .expect("注册第一批任务");

        let report = coordinator.shutdown();

        assert_eq!(report.executed(), 1, "只执行已注册的任务");
        assert!(report.ok());
        assert_eq!(probe.count("stop-background-workers"), 1);
        assert_eq!(probe.count("close-tabs"), 0, "未注册的任务不得凭空出现");
        // 退出后 setup 继续注册也会被拒绝，避免「注册到一半的清理」。
        assert!(coordinator
            .register("close-tabs", probe.task("close-tabs", false, false))
            .is_err());
        assert_eq!(probe.count("close-tabs"), 0);
    }

    // ---------- 3. 失败降级 ----------

    #[test]
    fn grid_cleanup_failure_does_not_block_terminals_and_tabs() {
        // 注册顺序中 grid 排在最后：用「前面的 kill-terminals 失败」验证后续仍执行，
        // 再用「shutdown-grid 失败」验证失败被记录而不影响已成功的任务。
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        register_realistic_tasks(&coordinator, &probe, &["kill-terminals"], &[]);

        let report = coordinator.shutdown();

        assert_eq!(report.executed(), 4, "失败不得中断后续任务");
        assert_eq!(report.failures().len(), 1);
        assert_eq!(report.failures()[0].name, "kill-terminals");
        assert_eq!(probe.count("close-tabs"), 1, "失败之前的任务已执行");
        assert_eq!(probe.count("shutdown-grid"), 1, "失败之后的任务仍须执行");
        assert!(!report.ok());
    }

    #[test]
    fn panicking_cleanup_does_not_block_remaining_tasks() {
        // 真实降级场景：锁中毒或插件异常导致清理任务 panic，而不是返回 Err。
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        register_realistic_tasks(&coordinator, &probe, &[], &["close-tabs"]);

        let report = coordinator.shutdown();

        assert_eq!(report.executed(), 4, "panic 不得中断后续任务");
        assert_eq!(report.failures().len(), 1);
        assert!(
            matches!(report.failures()[0].outcome, CleanupOutcome::Panicked(_)),
            "panic 必须转为 Panicked 结果"
        );
        assert_eq!(probe.count("kill-terminals"), 1);
        assert_eq!(probe.count("shutdown-grid"), 1);
    }

    #[test]
    fn all_cleanups_failing_still_reports_every_task() {
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        register_realistic_tasks(
            &coordinator,
            &probe,
            &[
                "stop-background-workers",
                "close-tabs",
                "kill-terminals",
                "shutdown-grid",
            ],
            &[],
        );

        let report = coordinator.shutdown();

        assert_eq!(report.executed(), 4, "全部失败也要跑完全部任务");
        assert_eq!(report.failures().len(), 4);
        assert!(!report.ok());
        assert!(coordinator.is_shutdown(), "全失败仍须标记为已关闭");
    }

    #[test]
    fn report_order_matches_registration_order_for_triage() {
        // 归因依赖顺序：报告必须与注册顺序一致，否则日志里无法判断谁先失败。
        let coordinator = ShutdownCoordinator::new();
        let probe = Probe::new();
        register_realistic_tasks(&coordinator, &probe, &["close-tabs"], &[]);

        let report = coordinator.shutdown();

        let names: Vec<&str> = report.reports.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "stop-background-workers",
                "close-tabs",
                "kill-terminals",
                "shutdown-grid"
            ],
            "报告顺序必须与注册顺序一致"
        );
        assert_eq!(probe.order(), names, "实际执行顺序应与报告一致");
    }
}
