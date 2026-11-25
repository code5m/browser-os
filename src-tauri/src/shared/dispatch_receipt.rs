//! A bounded mailbox for an already-owned native layout callback.
//! Revocation is only possible BEFORE entry; it prevents any late callback
//! from running. An entered callback has no timeout-based release path.
//! This is a helper of the existing Browser Owner, not a second scheduler.
use std::sync::{atomic::{AtomicUsize, Ordering}, Condvar, Mutex};
use std::time::{Duration, Instant};

const MAX_DISPATCHES: usize = 2;
static DISPATCHES: AtomicUsize = AtomicUsize::new(0);

enum State<T> { Pending, Entered, Finished(Option<T>), Revoked, Lost }
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WaitError { Revoked, Lost }
pub(crate) struct DispatchReceipt<T> {
    state: Mutex<State<T>>,
    changed: Condvar,
}
impl<T> DispatchReceipt<T> {
    pub(crate) fn new() -> Result<Self, &'static str> {
        DISPATCHES.fetch_update(Ordering::AcqRel, Ordering::Acquire,
            |count| (count < MAX_DISPATCHES).then_some(count + 1))
            .map_err(|_| "TAB_LAYOUT_DISPATCH_BUSY")?;
        Ok(Self { state: Mutex::new(State::Pending), changed: Condvar::new() })
    }
    pub(crate) fn enter(&self) -> Option<Entered<'_, T>> {
        let mut state = self.state.lock().ok()?;
        if !matches!(*state, State::Pending) { return None; }
        *state = State::Entered;
        self.changed.notify_all();
        Some(Entered { receipt: self, completed: false })
    }
    pub(crate) fn revoke_pending(&self) -> bool {
        let Ok(mut state) = self.state.lock() else { return false; };
        match *state {
            State::Pending => { *state = State::Revoked; self.changed.notify_all(); true }
            State::Revoked => true,
            _ => false,
        }
    }
    /// Only the queued, not-yet-entered phase is bounded by this deadline.
    /// A stuck entered callback must retain the Browser Owner until proof of
    /// completion or application/process isolation is available.
    pub(crate) fn wait(&self, queued_limit: Duration) -> Result<T, WaitError> {
        let deadline = Instant::now() + queued_limit;
        let mut state = self.state.lock().map_err(|_| WaitError::Lost)?;
        loop {
            match &mut *state {
                State::Pending => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        *state = State::Revoked;
                        self.changed.notify_all();
                        return Err(WaitError::Revoked);
                    }
                    let (next, _) = self.changed.wait_timeout(state, remaining).map_err(|_| WaitError::Lost)?;
                    state = next;
                }
                State::Entered => { state = self.changed.wait(state).map_err(|_| WaitError::Lost)?; }
                State::Finished(value) => return value.take().ok_or(WaitError::Lost),
                State::Revoked => return Err(WaitError::Revoked),
                State::Lost => return Err(WaitError::Lost),
            }
        }
    }
}
impl<T> Drop for DispatchReceipt<T> {
    fn drop(&mut self) { DISPATCHES.fetch_sub(1, Ordering::AcqRel); }
}
pub(crate) struct Entered<'a, T> { receipt: &'a DispatchReceipt<T>, completed: bool }
impl<T> Entered<'_, T> {
    pub(crate) fn complete(mut self, value: T) {
        if let Ok(mut state) = self.receipt.state.lock() {
            if matches!(*state, State::Entered) {
                *state = State::Finished(Some(value));
                self.completed = true;
                self.receipt.changed.notify_all();
            }
        }
    }
}
impl<T> Drop for Entered<'_, T> {
    fn drop(&mut self) {
        if !self.completed {
            if let Ok(mut state) = self.receipt.state.lock() {
                if matches!(*state, State::Entered) { *state = State::Lost; }
                self.receipt.changed.notify_all();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_unentered_callback_can_be_revoked_and_finished_receipt_survives_waiter_delay() {
        let revoked = DispatchReceipt::<u8>::new().unwrap();
        assert_eq!(revoked.wait(Duration::ZERO), Err(WaitError::Revoked));
        assert!(revoked.enter().is_none());
        drop(revoked);
        let completed = DispatchReceipt::new().unwrap();
        let entered = completed.enter().unwrap();
        assert!(!completed.revoke_pending());
        entered.complete(42);
        assert_eq!(completed.wait(Duration::ZERO), Ok(42));
        drop(completed);
        let lost = DispatchReceipt::<u8>::new().unwrap();
        drop(lost.enter().unwrap());
        assert!(!lost.revoke_pending());
        assert_eq!(lost.wait(Duration::ZERO), Err(WaitError::Lost));
    }
}
