# Detached follow-up repair candidate (2026-11-25)

Base: `04d12d7c103996d1ffd909d81eecdfcccf47478b`. The user's local `08c8cd8b` and dirty worktree were unavailable.
This is an unverified, detached Git object; do not update a branch or open a PR
without authorization. No CI, Actions, builds, tests, gates or installation
were executed. Regression cases are source only, NOT_RUN.

## Implemented candidate changes

- Native capture: pass a per-request GCancellable on Linux; request cancellation
  when the IPC wait expires or is dropped; retain capacity until native
  completion, skip abandoned encoding, and avoid blocking an extra wait worker.
  WebKit's public full-document snapshot API does not bound renderer allocation
  or prove immediate renderer interruption. Its finish callback, not the
  cancellation signal, owns terminal completion.
- Windows capture: query Page.getLayoutMetrics before screenshot dispatch and
  pass a bounded explicit CSS clip (16 million CSS pixels, 16,384 edge limit).
  Missing/malformed metrics fail closed. Device-scale/internal engine memory
  limits are not proven. CDP calls do not provide a per-call abort here:
  post-dispatch cancellation retains the permit until callback. Both metrics
  and capture synchronous dispatch errors complete the same one-shot callback.
  Real Windows behavior, document changes between metrics and capture, and
  WebKit native behavior remain unverified.
- Foundation durable recovery: lease check, task load, state transition,
  events and outbox publication share one SQLite IMMEDIATE transaction.
  Recovery rolls back atomically on error. Revision increment is checked.
  No production durable feature activation, storage permission or encryption
  changes. Added deterministic transaction rollback regression source.

## Outstanding (not closed)

1. Linux true pre-allocation renderer bound and renderer-side cancellation;
   Windows renderer-level memory and cancellation guarantees, and stale page
   handling. No silent substitution of visible-only screenshots for full page.
2. General Browser mutation IPC operation IDs, native terminal receipts,
   generation fencing and explicit unknown-outcome isolation require approved
   Tauri protocol design. Do not release serial mutation ownership on timeout.
3. Run/Session/Task production integration, shared model/tool/approval loop,
   protected SQLite directory/database/WAL/SHM and encryption, and side-effect
   reconciliation. Do not enable durable-sqlite in production as-is.
4. Strict external-writer CAS is not provided by ordinary rename after
   comparison. Need an enforced exclusive writer primitive or explicit
   conflict/no-overwrite contract; do not claim extra hashing or advisory locks
   solve arbitrary external writes.
5. GTK thread stacks and reproducible incident required before locked heat-path
   changes. Existing positioning, visibility and lifecycle rules untouched.
6. Windows native behavior and installed Linux GUI/performance require real
   target environments and explicit authorization to run verification.
7. Remaining P2 structure changes must be tied to actual ownership/duplication
   defects. Unrelated publishing/WeChat files and governance workflows untouched.

Overall: BLOCKED / GUI_PENDING. No PASS claim.


## Second detached source repair

Parent: `81399f21f6db08d4187c428a9a5b2e125d096d9a`. No branch refs, workflows, builds, tests, or gates were run or changed.

- Browser owner: replace the Promise-tail backlog with a physically bounded,
  removable queue; distinguish waiting timeout from UNKNOWN in-flight outcome.
  An in-flight watchdog rejects the caller and discards queued work but retains
  the original operation and execution slot until settlement. History/reload/
  legacy close share the owner path. Capability cleanup processes confirmed
  close receipts individually. Added controlled late-settlement regressions.
  Native non-return, transport rejection versus actual terminal completion,
  engine cancellation, and recovery after frontend/process restart still
  require the approved native operation protocol. No timeout releases a slot.
- Capture: recheck native WebView identity before the Windows metrics-to-
  capture transition and before the host saves results. Also reject missing
  or oversized legacy CDP layout metrics in addition to CSS dimensions.
  No renderer-wide memory guarantee, document-generation fence, or Linux
  pre-allocation hard bound is claimed.
- Durable foundation: require the persisted Running revision before lease or
  effect dispatch. Unknown effects block additional effects, execution leases,
  and transitions back to Running until reconciliation. Lease deadlines no
  longer permit stealing or recovery; an orphaned lease fails closed pending a
  protected owner-death recovery mechanism. Prioritize cancellation before
  tool polling and check revision after durable intent. Use checked revision
  increments. Added deterministic lease and unknown-effect regressions.
  Production owner remains in-memory; no unsafe durable activation.
- Linux workspace staging: retain our actual opened stage handle and refuse
  name-based cleanup when the stage name was replaced; failed exclusive stage
  creation no longer unlinks someone else's name. Added a deterministic
  stage-replacement regression. This is a specific cleanup fix, not strict
  external target CAS; ordinary target rename/unlink remains non-conditional.
- GTK hot paths, WebView lifecycle/position primitives, publishing/WeChat, and
  governance were not modified. Real Windows/Linux/GTK/GUI and final gates
  remain NOT_RUN by user instruction. A strict existing-file replacement policy
  must either use a proven enforced primitive or explicitly decide on a
  non-overwriting conflict workflow; silently disabling normal editing is not
  approved.

Status: BLOCKED / GUI_PENDING. This is unverified source, not a completed
seven-item closure.
