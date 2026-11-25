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


## Third detached source repair: generation-fenced captures and protected Coding checkpoints

Parent: `6ac6ce54abe385ce8f2e6fae2238f9b37008ec5b`. Source-only; no ref update, CI, Actions, builds, gates,
installation or tests executed. No permission or WebView positioning rule changes.

- Capture: track observed document generations independently of a reusable tab
  ID. Invalidate on native load/URI observations, explicit navigation, zoom,
  and close. Reject stale metrics/results. Linux performs an isolated-world
  numeric preflight with native scale/zoom and a conservative 16-million
  physical-pixel budget. The result callback bounds the raw pixel buffer and
  moves PNG compression off GTK onto the existing admitted capture worker
  path. No remote content is returned by the preflight. The WebKit API accepts
  FullDocument but not an allocation ceiling, so dynamic DOM growth remains
  a hard engine limit that this preflight cannot enforce. The Windows CDP
  request still has no per-call native abort. Do not call either guarantee
  fully solved without an engine/process boundary.
- Coding: use the existing OS-owned lock primitive for session and workspace
  ownership, including across cooperating processes. Persist the last run ID
  and the complete transcript before model dispatch and between complete tool
  batches; recover an interrupted turn without reissuing the provider call.
  Claimed side effects stay claimed. Session checkpoints on Linux/Windows
  are authenticated AES-256-GCM envelopes bound to the session UUID, using
  the existing OS keyring and private directory/native versioned storage.
  Missing encryption keys fail closed; no fallback plaintext writes. Legacy
  plaintext sessions are read under the owner and migrate on the next save.
  Existing backup/manifest formats and other platforms are not migrated.
  This does not enable the unprotected Foundation SQLite storage or replace
  the current Coding tool loop with Foundation. The test key is unit-test-only.
- Remaining strict external target CAS: general Linux/Windows name replacement
  has no compare-content-and-replace primitive against arbitrary external
  writers. A strict no-overwrite fallback requires an explicit product change;
  keep normal editor semantics rather than silently disabling editing.
- GTK actual userspace thread stacks, Windows real native behavior, Linux
  installed-client GUI and sustained performance require the real device.
  Its Desktop Commander connection remains offline. Do not modify the
  locked GTK layout/visibility path without that evidence.
- The existing browser queue quarantines unknown outcomes. A host/native
  operation-ID protocol and out-of-band authoritative receipts are still
  required to recover permanently lost replies without releasing in-flight
  native work; changing the command protocol/lifecycle needs explicit Owner
  approval. This candidate does not manufacture cancellation or completion.

Overall: BLOCKED / GUI_PENDING. All added tests are NOT_RUN.


## Owner decision record (2026-11-25)

The Owner approved the reviewed architecture targets and asked that they remain visible to later audits. The binding decisions D01–D08 and acceptance criteria are recorded in `docs/architecture/semantic-governance/ADR-20261125-native-owner-recovery.md`, linked from AGENTS.md and PROJECT-RULES.md. No gate exceptions. Approval is not implementation or verification. Current detached candidate remains BLOCKED / GUI_PENDING; no tests, CI, Actions, installation or branch update.


## D03/D04 implementation candidate

Coding's production Task is created/resumed inside the session owner, not for read/list calls. The existing Foundation TaskRecord is persisted with the session and bounded Run receipts; pause/review/approval are not terminal task success. Unknown external command/MCP outcomes block execution until an explicit user-confirmed reconciliation with evidence. A separate changeset export uses native no-replace and preserves original project files. Shared in-place apply remains a reviewed best-effort external-file operation (D03). The existing tool adapter loop and approval dispatch still need full Foundation convergence. Added behavior regression sources; NOT_RUN. No branch or gate updates.


## Subsequent implementation candidates — 2026-11-25

- D03/D05: c0c0a1c and 8d95e25 encrypt Coding backup blobs, manifests and recovery journals with record-specific authenticated data, keep private migration markers, and bind new per-store keys to native directory identity. Legacy records are read only before migration. Migration/key loss/oversize scans fail closed, never generate a replacement over sealed data. A crash between marker creation and sealed publication may require explicit recovery; original bytes remain. Windows new-file staging now publishes using the original secured handle; Linux retains the staging descriptor and avoids deleting an unowned temp entry. Approved AI publication opens and verifies the reviewed ancestor before relative traversal and write. This binds the object, not arbitrary external pathname CAS; rollback paths still need independent review. No overwrite export remains available.
- D01/D06/D08: 0bced479 introduces a bounded native receipt owner for the seven main-window tab mutations, including compatibility wrappers sharing its admission. Start is ticket-scoped; cancellation before dispatch is terminal, running cancellation is advisory; unknown or unacknowledged work keeps admission. Creation/activation IPC now waits off the GTK thread for the host callback, with the activation reservation transferred into the callback. Navigation dispatch is not renderer completion. Existing hide/position primitives remain unchanged. Native watchdog diagnostics include metadata-only ticket/phase/age. Internal non-IPC native paths and delayed renderer operations still require independent reconciliation. Frontend receipt inspection/cancel/ack primitives are exposed; a complete restart recovery UI remains a target. Native inventory and SCR are recorded without modifying checks. See browser-native-hang-evidence-20261125.md for the real incident evidence procedure.
- D02/D04/D07: strict engine memory isolation and true in-flight renderer abort, full Foundation model/tool/approval convergence, protected Foundation durable-store lease recovery, actual GTK reproduction, installed Windows/Linux GUI, performance and final gates remain open. Existing Coding encrypted checkpoint and explicit reconciliation are implementation progress, not proof of all Agent persistence.

No branch refs updated; all source changes and regression fixtures NOT_RUN. No CI/Actions invocation, test/build/install or checker modification. Overall: BLOCKED / GUI_PENDING.


## Follow-up review and focused repair

Base edb3db3a. Frontend control lanes now remove timed-out waiters rather than repeatedly attaching to a permanently pending IPC Promise. Recovery discards queued intents and compares a second owner snapshot before adopting the native list. Native Prepare no longer holds its owner mutex while capturing resource identity. Coding publication journals record the exact parent identity from the pinned native handle; rollback/delete use the same verified parent, and older attempted journals lacking that proof are retained, not replayed. Historical Windows CAS lock files are recognized without deletion; authenticated old records establish a migration marker on read. Updated stale bootstrap test to use the encrypted session owner and current Task API. Source changes only; all test sources NOT_RUN. No branch ref, workflow, build, installation or gate action. Remaining constraints and gaps: full external command/MCP ownership, task lifecycle across apply/discard, screenshot task recovery, renderer isolation, actual GTK stacks and platform validation. ADR D01–D08 unchanged.


## Native Coding operation ownership (fcaf3fb6)

Command and MCP proposal execution enters execute_operation under the encrypted Coding session's native/process lock, with pre-dispatch Run/Task receipt. Reviewed command uses the same snippet validation and script supervisor; shutdown no longer publishes a fake terminal status. The UI uses the owned execution route and accepts only the stored native result. Unknown effects require separately confirmed termination and effect evidence, never automatic replay. Apply/discard close a paused Task when no unresolved approvals remain. Cancellation is requested via the exact operation Run ID (MCP cancellation transport still requires native adapter completion). All regression source fixtures NOT_RUN; no refs, CI, Actions, builds or tests.
