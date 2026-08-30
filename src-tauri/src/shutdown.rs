//! M0-2.b：幂等关闭协调器（`ShutdownCoordinator`）——统一生命周期核心的最小实现。
//!
//! 本检查点**只提供核心与测试**，不迁移任何调用方（M0-2.c 负责把主窗关闭、菜单退出、
//! 系统退出以及 grid/PTY/tab/线程调用方切到本核心）。因此当前阶段核心尚未被业务调用，
//! 尚未接入 UI 的诊断访问器继续局部标注 `#[allow(dead_code)]`，避免新增编译器警告。
//!
//! 契约（对应 M0-2 验收项）：
//!   1. 幂等：连续调用两次不会产生重复副作用，也不会 panic 或死锁。
//!   2. 失败隔离：单个清理任务返回 `Err` 或 panic，都不阻止其余资源回收。
//!   3. 可观测：每次执行返回结构化报告，记录每个任务的成功/失败/panic。
//!
//! 并发语义（明确约定，避免误用）：
//!   - 首次调用者执行全部任务并缓存报告；已完成的后续调用直接返回同一份报告。
//!   - 若清理**尚未结束**时再次调用（其他线程并发，或清理任务内部重入），
//!     立即返回「进行中」报告且**不阻塞**：清理任务常持有业务锁（如 `AppState` 的
//!     `terminals`/`child_layouts`），阻塞等待极易与这些锁形成死锁。
//!     需要「关闭已完成」语义的调用方应在 M0-2.c 迁移时配合退出流程等待，而非在此加锁。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

/// 单个清理任务的最终结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupOutcome {
    /// 清理成功。
    Ok,
    /// 任务返回 `Err(msg)`：已隔离，其余任务继续执行。
    Failed(String),
    /// 任务 panic：已隔离（转为结果），其余任务继续执行，panic 不向上传播。
    Panicked(String),
}

/// 单个清理任务的报告。
#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub name: String,
    pub outcome: CleanupOutcome,
}

impl CleanupReport {
    /// 是否属于需要归因的失败（含 panic）。
    pub fn is_failure(&self) -> bool {
        !matches!(self.outcome, CleanupOutcome::Ok)
    }
}

/// 一次关闭执行的整体报告。
#[derive(Debug, Clone)]
pub struct ShutdownReport {
    /// 各任务结果，顺序与注册顺序一致。
    pub reports: Vec<CleanupReport>,
    /// 为 true 表示本次调用**未**执行清理：此前已关闭，或清理正在由其他调用者执行（含重入）。
    pub already_shutdown: bool,
}

impl ShutdownReport {
    /// 全部任务成功（无失败、无 panic）且本次确实执行了清理。
    pub fn ok(&self) -> bool {
        !self.already_shutdown && self.failures().is_empty()
    }

    /// 失败与 panic 的任务列表，供调用方归因与日志。
    pub fn failures(&self) -> Vec<&CleanupReport> {
        self.reports.iter().filter(|r| r.is_failure()).collect()
    }

    /// 已执行的任务数量（用于测试与观测）。
    pub fn executed(&self) -> usize {
        self.reports.len()
    }
}

type CleanupTask = Box<dyn FnOnce() -> Result<(), String> + Send>;

struct Inner {
    tasks: Vec<(String, CleanupTask)>,
    /// 关闭开始后禁止再注册，避免「注册到一半的清理」造成资源遗漏。
    closed: bool,
    /// 当前正在执行清理的线程（用于并发/重入判定，避免阻塞等待导致死锁）。
    runner: Option<ThreadId>,
}

/// 幂等关闭协调器：`Send + Sync`，可交由 Tauri `manage` 作为全局状态。
pub struct ShutdownCoordinator {
    inner: Mutex<Inner>,
    /// 首次执行结果的缓存；有值即代表清理已完成。
    report: Mutex<Option<Arc<ShutdownReport>>>,
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownCoordinator {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                tasks: Vec::new(),
                closed: false,
                runner: None,
            }),
            report: Mutex::new(None),
        }
    }

    /// 注册一个清理任务。清理开始后注册会被拒绝（返回 `Err`，不 panic）。
    pub fn register<F>(&self, name: impl Into<String>, task: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        let name = name.into();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.closed {
            return Err(format!("关闭已开始，拒绝注册清理任务：{name}"));
        }
        inner.tasks.push((name, Box::new(task)));
        Ok(())
    }

    /// 是否已执行过清理（M0-2.c 迁移退出路径后使用）。
    #[allow(dead_code)]
    pub fn is_shutdown(&self) -> bool {
        self.report
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// 上一次执行的完整报告（重复调用只能拿到空报告，诊断取历史时用本方法）。
    #[allow(dead_code)]
    pub fn last_report(&self) -> Option<Arc<ShutdownReport>> {
        self.report
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 已注册但尚未执行的任务数量（M0-2.c 迁移与诊断使用）。
    #[allow(dead_code)]
    pub fn pending_tasks(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .tasks
            .len()
    }

    /// 执行关闭：幂等、失败隔离、不因重入/并发而死锁。
    pub fn shutdown(&self) -> Arc<ShutdownReport> {
        // 已完成：本次不再执行任何任务。返回空报告并标记 already_shutdown，
        // 保持「reports 描述本次调用」的语义；历史结果可用 last_report() 查询。
        if self
            .report
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            return Arc::new(ShutdownReport {
                reports: Vec::new(),
                already_shutdown: true,
            });
        }

        let me = std::thread::current().id();
        let tasks = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            if inner.closed {
                // 清理进行中（并发或重入）：不阻塞、不重复执行。
                return Arc::new(ShutdownReport {
                    reports: Vec::new(),
                    already_shutdown: true,
                });
            }
            let _ = me;
            inner.closed = true;
            inner.runner = Some(me);
            std::mem::take(&mut inner.tasks)
        };

        let mut reports = Vec::with_capacity(tasks.len());
        for (name, task) in tasks {
            let outcome = match catch_unwind(AssertUnwindSafe(task)) {
                Ok(Ok(())) => CleanupOutcome::Ok,
                Ok(Err(msg)) => CleanupOutcome::Failed(msg),
                Err(payload) => CleanupOutcome::Panicked(panic_message(payload)),
            };
            reports.push(CleanupReport { name, outcome });
        }

        let report = Arc::new(ShutdownReport {
            reports,
            already_shutdown: false,
        });
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.runner = None;
        }
        *self.report.lock().unwrap_or_else(|e| e.into_inner()) = Some(report.clone());
        report
    }
}

/// 从 panic payload 提取可读信息（panic 不应导致关闭流程二次崩溃）。
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "未知 panic（无可读 payload）".to_string()
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
    fn empty_coordinator_shutdown_is_ok() {
        let coordinator = ShutdownCoordinator::new();
        let report = coordinator.shutdown();
        assert!(report.ok(), "无任务时关闭应成功");
        assert_eq!(report.executed(), 0);
        assert!(coordinator.shutdown().already_shutdown);
    }
}
