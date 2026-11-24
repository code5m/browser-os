//! Bounded admission for synchronous native work off the event/runtime threads.
//!
//! Each capability owns its own lane and capacity. There is no unbounded
//! pending queue: saturation returns a stable error before work is scheduled.
//! The permit belongs to the blocking closure, not the awaiting IPC future.
//! Dropping that future cannot admit replacement work while its OS call runs.
//! This bounds concurrency; it does not make a blocking OS syscall cancellable.

use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct BlockingLane {
    active: AtomicUsize,
    limit: usize,
    busy_error: &'static str,
    worker_error: &'static str,
}

impl BlockingLane {
    pub(crate) const fn new(
        limit: usize,
        busy_error: &'static str,
        worker_error: &'static str,
    ) -> Self {
        Self {
            active: AtomicUsize::new(0),
            limit,
            busy_error,
            worker_error,
        }
    }

    pub(crate) fn reserve(&'static self) -> Result<BlockingPermit, String> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                if active < self.limit {
                    Some(active + 1)
                } else {
                    None
                }
            })
            .map_err(|_| self.busy_error.to_string())?;
        Ok(BlockingPermit { lane: self })
    }

    pub(crate) async fn run<T, F>(&'static self, operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        self.reserve()?.run(operation).await
    }
}

pub(crate) struct BlockingPermit {
    lane: &'static BlockingLane,
}

impl BlockingPermit {
    pub(crate) async fn run<T, F>(self, operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, String> + Send + 'static,
    {
        let worker_error = self.lane.worker_error;
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = self;
            operation()
        })
        .await
        .map_err(|_| worker_error.to_string())?
    }
}

impl Drop for BlockingPermit {
    fn drop(&mut self) {
        self.lane.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_waiter_retains_capacity_until_native_work_finishes() {
        static LANE: BlockingLane = BlockingLane::new(1, "OWNER_BUSY", "OWNER_WORKER_FAILED");
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let waiter = tokio::spawn(async move {
            LANE.run(move || {
                started_tx
                    .send(())
                    .map_err(|_| "START_SIGNAL_FAILED".to_string())?;
                release_rx
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|_| "RELEASE_SIGNAL_FAILED".to_string())?;
                Ok(())
            })
            .await
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert!(matches!(LANE.reserve(), Err(error) if error == "OWNER_BUSY"));
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(permit) = LANE.reserve() {
                drop(permit);
                break;
            }
            assert!(
                Instant::now() < deadline,
                "native worker retained its permit"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    #[tokio::test]
    async fn worker_failure_releases_capacity_and_does_not_become_success() {
        static LANE: BlockingLane = BlockingLane::new(1, "OWNER_BUSY", "OWNER_WORKER_FAILED");
        let result = LANE
            .run(|| -> Result<(), String> { panic!("native worker fixture") })
            .await;
        assert_eq!(result, Err("OWNER_WORKER_FAILED".to_string()));
        assert_eq!(LANE.run(|| Ok(7)).await, Ok(7));
    }

    #[test]
    fn callback_retains_capacity_after_waiter_timeout() {
        static LANE: BlockingLane = BlockingLane::new(1, "CAPTURE_BUSY", "CAPTURE_WORKER_FAILED");
        let permit = LANE.reserve().unwrap();
        let (send, recv) = mpsc::channel();
        let native_callback = move || {
            let _ = send.send((vec![1_u8], permit));
        };
        assert!(recv.recv_timeout(Duration::from_millis(1)).is_err());
        drop(recv);
        assert!(matches!(LANE.reserve(), Err(error) if error == "CAPTURE_BUSY"));
        native_callback();
        assert!(LANE.reserve().is_ok());
    }
}
