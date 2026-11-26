//! Grid-only ownership registry. The application deadline limits waiting;
//! it never detaches an IPC worker or drops a process whose tree is still live.
use super::{ChildComms, GridChildHandle, GridRetirement};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const MAX_GRID_WORKERS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VisibilityMode {
    Show,
    Blur,
    Minimized,
}

#[derive(Clone)]
pub(super) struct VisibilityIntent {
    pub(super) revision: u64,
    pub(super) mode: VisibilityMode,
    pub(super) targets: Vec<crate::grid_ipc::RendererBinding>,
    ready_at: Instant,
}
#[derive(Default)]
struct VisibilityState {
    revision: u64,
    pending: Option<VisibilityIntent>,
    running: bool,
}
/// One pending desired state and one sender, regardless of native focus rate.
#[derive(Default)]
pub(super) struct LatestVisibility {
    state: Mutex<VisibilityState>,
    wake: Condvar,
}
impl LatestVisibility {
    pub(super) fn request(&self, mode: VisibilityMode, delay: Duration,
        targets: Vec<crate::grid_ipc::RendererBinding>,
    ) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.revision = state.revision.wrapping_add(1);
        state.pending = Some(VisibilityIntent {
            revision: state.revision,
            mode,
            targets,
            ready_at: Instant::now() + delay,
        });
        let start = !state.running;
        state.running = true;
        self.wake.notify_one();
        start
    }
    pub(super) fn next(&self, lifecycle: &GridLifecycle) -> Option<VisibilityIntent> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if lifecycle.is_stopping() {
                state.pending = None;
                state.running = false;
                return None;
            }
            let Some(intent) = state.pending.as_ref() else {
                state.running = false;
                return None;
            };
            let remaining = intent.ready_at.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Some(intent.clone());
            }
            // The existing 150 ms focus policy is interruptible by a newer
            // intent. A short bound also observes application shutdown.
            state = self
                .wake
                .wait_timeout(state, remaining.min(Duration::from_millis(20)))
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
    }
    pub(super) fn is_latest(&self, revision: u64) -> bool {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).pending.as_ref()
            .is_some_and(|intent| intent.revision == revision)
    }
    pub(super) fn finish(&self, revision: u64) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state
            .pending
            .as_ref()
            .is_some_and(|intent| intent.revision != revision)
        {
            return true;
        }
        state.pending = None;
        state.running = false;
        false
    }
    pub(super) fn spawn_failed(&self) {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).running = false;
    }
    pub(super) fn reserve_pending(&self) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.running || state.pending.is_none() {
            return false;
        }
        state.running = true;
        true
    }
}

type Children = Arc<Mutex<HashMap<u32, GridChildHandle>>>;
type Listeners = Arc<Mutex<HashMap<u32, Arc<ChildComms>>>>;

#[derive(Default)]
pub(super) struct GridLifecycle {
    stopping: AtomicBool,
    operations: AtomicUsize,
    workers: Mutex<Vec<JoinHandle<()>>>,
    retired: Mutex<VecDeque<Arc<Mutex<GridRetirement>>>>,
    retiring: Mutex<HashMap<u32, usize>>,
    retiring_total: AtomicUsize,
    // The existing cleanup worker is the source of completion, even when
    // a foreground recovery waiter has already returned Pending.
    retirement_app: Mutex<Option<AppHandle>>,
    cleanup_worker: Mutex<Option<JoinHandle<()>>>,
    cleanup_done: AtomicBool,
    wake_lock: Mutex<()>,
    wake: Condvar,
    last_error: Mutex<Option<String>>,
    worker_failed: AtomicBool,
}

pub(super) struct GridOperation(Arc<GridLifecycle>);
impl Drop for GridOperation {
    fn drop(&mut self) {
        self.0.operations.fetch_sub(1, Ordering::SeqCst);
        self.0.wake.notify_all();
    }
}

impl GridLifecycle {
    pub(super) fn set_retirement_app(&self, app: AppHandle) {
        *self.retirement_app.lock().unwrap() = Some(app);
    }

    pub(super) fn check(&self) -> Result<(), String> {
        if self.is_stopping() {
            Err("GRID_SHUTDOWN_IN_PROGRESS".into())
        } else {
            Ok(())
        }
    }

    pub(super) fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    pub(super) fn enter(self: &Arc<Self>) -> Result<GridOperation, String> {
        self.check()?;
        self.operations.fetch_add(1, Ordering::SeqCst);
        let permit = GridOperation(Arc::clone(self));
        // The increment and second check close admission versus publication.
        // Shutdown cannot complete while a successful permit is still alive.
        self.check()?;
        Ok(permit)
    }

    pub(super) fn request_shutdown(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.wake.notify_all();
    }

    pub(super) fn wait_or_stop(&self, duration: Duration) -> bool {
        let deadline = Instant::now() + duration;
        let mut lock = self.wake_lock.lock().unwrap_or_else(|p| p.into_inner());
        while !self.is_stopping() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            lock = self
                .wake
                .wait_timeout(lock, remaining.min(Duration::from_millis(20)))
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
        self.is_stopping()
    }

    pub(super) fn spawn(
        self: &Arc<Self>,
        name: &str,
        work: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        let _permit = self.enter()?;
        self.reap_workers();
        let mut workers = self.workers.lock().unwrap_or_else(|p| p.into_inner());
        if workers.len() >= MAX_GRID_WORKERS {
            return Err("GRID_WORKER_CAPACITY_BUSY".into());
        }
        let handle = std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(work)
            .map_err(|error| format!("GRID_WORKER_START_FAILED:{name}:{error}"))?;
        // Even if shutdown won after enter(), retain the actual thread. The
        // worker observes the same stop gate; its handle is never detached.
        workers.push(handle);
        drop(workers);
        self.wake.notify_all();
        Ok(())
    }

    pub(super) fn retire(&self, index: u32, child: GridChildHandle) {
        child.comms.revoke_owner(child.child.id());
        *self.retiring.lock().unwrap().entry(index).or_insert(0) += 1;
        self.retiring_total.fetch_add(1, Ordering::SeqCst);
        self.retired
            .lock()
            .unwrap()
            .push_back(Arc::new(Mutex::new(GridRetirement::new(index, child))));
        self.wake.notify_all();
    }

    pub(super) fn is_retiring(&self, index: u32) -> bool {
        self.retiring.lock().unwrap().contains_key(&index)
    }

    fn remember_error(&self, error: String) {
        // Diagnostics must not grow with repeated native polling failures.
        *self.last_error.lock().unwrap() = Some(error.chars().take(512).collect());
    }

    fn reap_workers(&self) {
        let finished = {
            let mut workers = self.workers.lock().unwrap();
            let mut finished = Vec::new();
            let mut index = 0;
            while index < workers.len() {
                if workers[index].is_finished() {
                    finished.push(workers.swap_remove(index));
                } else {
                    index += 1;
                }
            }
            finished
        };
        for worker in finished {
            if worker.join().is_err() {
                self.worker_failed.store(true, Ordering::SeqCst);
                self.remember_error("GRID_WORKER_PANICKED".into());
            }
        }
    }

    fn poll_retirements(&self) {
        // Keep an owning Arc in the shared registry while native polling is
        // in progress. A cleanup worker panic cannot drop its entire batch.
        let batch = self
            .retired
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        for slot in batch {
            let mut owner = slot.lock().unwrap_or_else(|p| p.into_inner());
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| owner.poll()));
            match outcome {
                Ok(Ok(true)) => {
                    let index = owner.index();
                    let download_retired = owner.download_retired.clone();
                    drop(owner);
                    let removed = {
                        let mut queue = self.retired.lock().unwrap();
                        let position = queue.iter().position(|item| Arc::ptr_eq(item, &slot));
                        position.and_then(|position| queue.remove(position))
                    };
                    if removed.is_some() {
                        drop(removed);
                        let route_complete = {
                            let mut retiring = self.retiring.lock().unwrap();
                            if let Some(count) = retiring.get_mut(&index) {
                                *count -= 1;
                                if *count == 0 {
                                    retiring.remove(&index);
                                    true
                                } else { false }
                            } else { false }
                        };
                        self.retiring_total.fetch_sub(1, Ordering::SeqCst);
                        self.wake.notify_all();
                        if !self.is_stopping() {
                            if let Some(payload) = download_retired {
                                if let Some(app) = self.retirement_app.lock().unwrap().clone() {
                                    // Only after the exact native retirement is confirmed.
                                    // Unknown is not a failure or file-deletion receipt.
                                    let _ = app.emit_to("main", "isolated-renderer-download", payload);
                                }
                            }
                        }
                        if route_complete && crate::grid_ipc::is_tab_renderer(index)
                            && !self.is_stopping()
                        {
                            // Drop every retirement/registry lock before
                            // notifying. A replacement may already exist;
                            // the UI treats this as invalidation only.
                            let app = self.retirement_app.lock().unwrap().clone();
                            if let Some(app) = app {
                                let _ = app.emit_to("main", "isolated-tab-renderer-recovery-changed",
                                    serde_json::json!({"id": crate::grid_ipc::renderer_label(index)}));
                            }
                        }
                    }
                }
                Ok(Ok(false)) => {}
                Ok(Err(error)) => self.remember_error(error),
                Err(_) => {
                    self.worker_failed.store(true, Ordering::SeqCst);
                    self.remember_error("GRID_NATIVE_CLEANUP_PANICKED_OWNER_RETAINED".into());
                }
            }
        }
    }

    pub(super) fn ensure_cleanup(
        self: &Arc<Self>,
        children: Children,
        listeners: Listeners,
    ) -> Result<(), String> {
        let mut slot = self.cleanup_worker.lock().unwrap();
        if self.cleanup_done.load(Ordering::SeqCst) {
            return Ok(());
        }
        if slot.as_ref().is_some_and(|worker| !worker.is_finished()) {
            self.wake.notify_all();
            return Ok(());
        }
        if let Some(worker) = slot.take() {
            if worker.join().is_err() {
                self.worker_failed.store(true, Ordering::SeqCst);
                self.remember_error("GRID_CLEANUP_WORKER_PANICKED".into());
            }
        }
        let lifecycle = Arc::clone(self);
        // The closure only captures shared registry references. A spawn error
        // leaves every process and worker in the original registry for retry.
        let worker = std::thread::Builder::new()
            .name("grid-cleanup".into())
            .spawn(move || lifecycle.cleanup_loop(children, listeners))
            .map_err(|error| format!("GRID_CLEANUP_START_FAILED:{error}"))?;
        *slot = Some(worker);
        Ok(())
    }

    fn cleanup_loop(self: Arc<Self>, children: Children, listeners: Listeners) {
        loop {
            let mut children_empty = false;
            if self.is_stopping() {
                // Never wait behind ordinary close/spawn on a manager mutex.
                if let Ok(mut live) = children.try_lock() {
                    for (index, child) in live.drain() {
                        self.retire(index, child);
                    }
                    children_empty = live.is_empty();
                }
                let connections = listeners
                    .try_lock()
                    .ok()
                    .map(|map| map.values().cloned().collect::<Vec<_>>());
                if let Some(connections) = connections {
                    for comms in connections {
                        comms.request_shutdown();
                    }
                }
            }
            // Native termination / I/O cancellation / joins are off the caller
            // thread and outside the registry locks. A stalled native call
            // retains this service owner and makes shutdown report Pending.
            self.poll_retirements();
            self.reap_workers();
            if self.is_stopping()
                && children_empty
                && self.operations.load(Ordering::SeqCst) == 0
                && self.retiring_total.load(Ordering::SeqCst) == 0
                && self.workers.lock().unwrap().is_empty()
            {
                // Re-read under the publication lock. The earlier drain's
                // empty snapshot can become stale while an admitted spawn
                // publishes its owner and releases its operation permit.
                if let Ok(live) = children.try_lock() {
                    if live.is_empty()
                        && self.operations.load(Ordering::SeqCst) == 0
                        && self.retiring_total.load(Ordering::SeqCst) == 0
                        && self.workers.lock().unwrap().is_empty()
                    {
                        if let Ok(mut map) = listeners.try_lock() {
                            map.clear();
                            self.retirement_app.lock().unwrap().take();
                            self.cleanup_done.store(true, Ordering::SeqCst);
                            self.wake.notify_all();
                            return;
                        }
                    }
                }
            }
            let lock = self.wake_lock.lock().unwrap_or_else(|p| p.into_inner());
            let _ = self
                .wake
                .wait_timeout(lock, Duration::from_millis(20))
                .unwrap_or_else(|p| p.into_inner());
        }
    }

    fn pending_error(&self, stage: &str) -> String {
        let detail = self.last_error.lock().unwrap().clone().unwrap_or_default();
        format!(
            "{stage}:operations={},retired={},workers={}:{}",
            self.operations.load(Ordering::SeqCst),
            self.retiring_total.load(Ordering::SeqCst),
            self.workers.lock().unwrap().len(),
            detail
        )
    }

    /// One tab may be closed/recovered while unrelated native owners are
    /// still retiring. Never use a global-empty receipt for that operation.
    pub(super) fn wait_route_retired_until(&self, index: u32, deadline: Instant) -> Result<(), String> {
        while self.is_retiring(index) {
            if Instant::now() >= deadline {
                return Err(format!("TAB_RENDERER_CLEANUP_PENDING:{index}"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }

    pub(super) fn wait_grid_retired_until(&self, deadline: Instant) -> Result<(), String> {
        loop {
            if !self.retiring.lock().unwrap().keys().any(|id| !crate::grid_ipc::is_tab_renderer(*id)) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(self.pending_error("GRID_PROCESS_CLEANUP_PENDING"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub(super) fn wait_retired_until(&self, deadline: Instant) -> Result<(), String> {
        while self.retiring_total.load(Ordering::SeqCst) != 0 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(self.pending_error("GRID_PROCESS_CLEANUP_PENDING"));
            }
            let lock = self.wake_lock.lock().unwrap_or_else(|p| p.into_inner());
            let _ = self
                .wake
                .wait_timeout(lock, remaining.min(Duration::from_millis(20)))
                .unwrap_or_else(|p| p.into_inner());
        }
        Ok(())
    }

    pub(super) fn shutdown_until(
        self: &Arc<Self>,
        children: Children,
        listeners: Listeners,
        deadline: Instant,
    ) -> Result<(), String> {
        self.request_shutdown();
        self.ensure_cleanup(children, listeners)?;
        loop {
            let (finished, worker) = {
                let mut slot = self.cleanup_worker.lock().unwrap();
                if slot.as_ref().is_some_and(JoinHandle::is_finished) {
                    // Check and take the same owner under one lock. A retry
                    // may otherwise replace an exited service with a live one.
                    (true, slot.take())
                } else {
                    (
                        slot.is_none() && self.cleanup_done.load(Ordering::SeqCst),
                        None,
                    )
                }
            };
            if finished {
                if worker.is_some_and(|worker| worker.join().is_err()) {
                    self.worker_failed.store(true, Ordering::SeqCst);
                }
                if self.worker_failed.load(Ordering::SeqCst) {
                    return Err("GRID_SHUTDOWN_WORKER_FAILED".into());
                }
                return Ok(());
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(self.pending_error("GRID_SHUTDOWN_PENDING"));
            }
            let lock = self.wake_lock.lock().unwrap_or_else(|p| p.into_inner());
            let _ = self
                .wake
                .wait_timeout(lock, remaining.min(Duration::from_millis(20)))
                .unwrap_or_else(|p| p.into_inner());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn registries() -> (Children, Listeners) {
        (
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    #[test]
    fn focus_intents_coalesce_and_replay_the_latest_after_an_inflight_send() {
        let lifecycle = GridLifecycle::default();
        let pending = LatestVisibility::default();
        assert!(pending.request(VisibilityMode::Blur, Duration::ZERO, Vec::new()));
        let old = pending.next(&lifecycle).unwrap();
        assert!(!pending.request(VisibilityMode::Show, Duration::ZERO, Vec::new()));
        assert!(pending.finish(old.revision));
        let current = pending.next(&lifecycle).unwrap();
        assert_eq!(current.mode, VisibilityMode::Show);
        assert!(!pending.finish(current.revision));
        assert!(pending.request(VisibilityMode::Minimized, Duration::ZERO, Vec::new()));
        assert!(!pending.request(VisibilityMode::Show, Duration::ZERO, Vec::new()));
        assert_eq!(pending.next(&lifecycle).unwrap().mode, VisibilityMode::Show);
    }

    #[test]
    fn failed_visibility_start_keeps_one_retryable_intent() {
        let pending = LatestVisibility::default();
        assert!(pending.request(VisibilityMode::Blur, Duration::from_millis(150), Vec::new()));
        pending.spawn_failed();
        assert!(pending.reserve_pending());
        assert!(!pending.reserve_pending());
        assert!(!pending.request(VisibilityMode::Show, Duration::ZERO, Vec::new()));
        let lifecycle = GridLifecycle::default();
        let latest = pending.next(&lifecycle).unwrap();
        assert_eq!(latest.mode, VisibilityMode::Show);
        assert!(!pending.finish(latest.revision));
    }

    #[test]
    fn grid_shutdown_retains_worker_owner_until_released() {
        let lifecycle = Arc::new(GridLifecycle::default());
        let (children, listeners) = registries();
        lifecycle
            .ensure_cleanup(children.clone(), listeners.clone())
            .unwrap();
        let (release, wait) = mpsc::channel();
        lifecycle
            .spawn("grid-test-held-worker", move || {
                let _ = wait.recv();
            })
            .unwrap();
        let result = lifecycle.shutdown_until(children.clone(), listeners.clone(), Instant::now());
        assert!(result.unwrap_err().contains("GRID_SHUTDOWN_PENDING"));
        assert_eq!(lifecycle.workers.lock().unwrap().len(), 1);
        assert!(lifecycle.enter().is_err());
        release.send(()).unwrap();
        lifecycle
            .shutdown_until(children, listeners, Instant::now() + Duration::from_secs(5))
            .unwrap();
        assert!(lifecycle.workers.lock().unwrap().is_empty());
        assert!(lifecycle.cleanup_worker.lock().unwrap().is_none());
    }

    #[test]
    fn grid_shutdown_does_not_wait_for_an_admitted_operation() {
        let lifecycle = Arc::new(GridLifecycle::default());
        let (children, listeners) = registries();
        let operation = lifecycle.enter().unwrap();
        let started = Instant::now();
        assert!(lifecycle
            .shutdown_until(children.clone(), listeners.clone(), started)
            .is_err());
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(lifecycle.enter().is_err());
        drop(operation);
        lifecycle
            .shutdown_until(children, listeners, Instant::now() + Duration::from_secs(5))
            .unwrap();
    }

    #[test]
    fn ordinary_grid_retirement_does_not_close_admission() {
        let lifecycle = Arc::new(GridLifecycle::default());
        lifecycle.wait_retired_until(Instant::now()).unwrap();
        assert!(lifecycle.enter().is_ok());
        let (children, listeners) = registries();
        lifecycle
            .shutdown_until(children, listeners, Instant::now() + Duration::from_secs(5))
            .unwrap();
    }
}
