# A4 M5 BUG-HUNT Scheduler Closeout

## Scope

- Persist the scheduled trigger reservation before starting a process.
- Serialize scheduler and `task_add/update/remove/run_now` task-store transactions.
- Add crash-window and concurrent load-modify-save tests.

## Delivered

`scheduler::tick` now advances `last_fired_at`, `next_run_at`, and `updated_at`, persists the reservation, and only then enters the process-start path. A persistence failure rejects the run instead of starting an unrecorded execution.

`tasks::task_store_lock` is held across scheduler and task-command load-modify-save transactions, preventing a stale command snapshot from overwriting a scheduler reservation.

## Verification

- `cargo test --manifest-path src-tauri/Cargo.toml scheduler`: 10/10 PASS
- `cargo test --manifest-path src-tauri/Cargo.toml`: 447/447 PASS
- `npm run build`: PASS
- `bash scripts/pre-merge.sh`: `PRE_MERGE_RESULT=ALL_PASS`
- `git diff --check`: PASS

Existing warnings remain limited to the pre-existing `grid_process.rs` dead-code warnings; the frontend build retains its existing dynamic-import advisory.

## Remaining boundary

Only A0 may integrate, commit, and push the combined worktree. Native GUI acceptance remains user-side evidence.
