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


## Capture task protocol

The existing main-only capture_full_page command now supports inspect/start/cancel/ack through one bounded native capture slot; legacy string-return callers share the same admission. Engine and publication work outlive the initiating invoke. Timeout requests cancellation but never releases native capacity. Terminal callbacks persist the in-process receipt, including saved path, and can be recovered from the ActivityBar native menu; unknown callbacks remain quarantined. Start uses host epoch and sequence, preventing a completed request from being started again. No engine hard-allocation claim; D02 best-effort current-document boundary unchanged. Source-only tests NOT_RUN. No ref or workflow changes.


## Cumulative source closure candidate

This round extends the same detached candidate: native Agent operation receipts, execution ownership for reviewed command and MCP, UI result acceptance, apply/discard Task closure, capture task inspection/cancellation/acknowledgement, bounded IPC observation and recovery sequence fences, capture document binding, native MCP cancellation and termination evidence, stable command Run ID, shared Foundation task lease, and authenticated legacy inventory. All changes are source-only, with no branch-reference update and no CI/Actions/build/test/gate invocation.

Migration policy: enroll exact legacy plaintext digests under the key initialization lock, seal the inventory with separate associated data, and reject later unenrolled or modified plaintext. Previously initialized encrypted stores without a trustworthy inventory cannot automatically authenticate unmarked plaintext; preserve such records without replay, requiring explicit migration review. No unprotected SQLite. No deletion of ambiguous history.

Honest acceptance boundary: platform-locked GTK hot paths remain untouched pending an actual native stack trace. Engine-internal screenshot allocation/termination is limited to available APIs, not represented as a hard memory-isolated renderer. Shared unmanaged editor files retain conflict detection and no-replace export rather than falsely claiming arbitrary-writer CAS. Coding uses the shared Foundation task state/lease and native tool ownership; a complete migration of all provider/tool protocols into the experimental Foundation AgentEngine is not established by these changes. Real Windows/Linux installed GUI/performance and regression results remain NOT_RUN. Target branch unchanged; no verified PASS.


## Follow-up: execution identity and complete model batches

Base ada4dc96. Foundation now rejects task retirement, detachment/resumption while a live execution lease owns the task, and removes the signal under the same task lock as retirement. Streaming and Coding batched adapters use the shared bounded argument parser; Coding validates the entire assistant function batch (IDs, types, names and JSON objects) before staging any proposal or storing the assistant message. A duplicate or malformed suffix can no longer leave a valid prefix staged with an inconsistent transcript. Linux script supervision observes the child with waitid/WNOWAIT, terminates leftover group members while the leader identity is still pinned, and only then reaps. No numeric group signal follows a successful reap; an uncertain wait leaves admission quarantined. Normal leader exit uses immediate descendant cleanup; running cancellation retains its existing TERM grace. Windows continues to use its owned Job. Agent supervisor polling no longer clones large running output snapshots. Added regression sources, NOT_RUN. This change does not claim strict containment of processes that escape a Unix process group, a full provider/tool engine migration, renderer memory isolation or real GUI evidence. No branch, workflow, CI, Action, build, test or gate updates.


## Native control-plane recovery follow-up

The tab receipt owner's existing sequence also advances for background/legacy admission and terminal ACK; capture ACK likewise fences older starts. Split owner/active-tab observations are invalidated if the owner changes during collection. Recovery adopts a tab list only after a matching terminal ACK and exactly one sequence advance (decimal-string arithmetic preserves u64 identity). Browser and capture commands share two fixed transport lanes per phase: normal plus recovery, including alternate read-only observation after timeout and a different ticket after native reconciliation. A permanently lost transport is not treated as a native completion; both lanes exhausted requires a safe frontend/application restart and further native reconciliation. No GTK layout primitive, new native command contract, workflow or checker changed. Test sources NOT_RUN; no CI/Actions or ref updates.


## Shared file conflict recovery (workspace editors)

Both overlay and inline text saves now synchronously persist the intended encrypted draft before the native best-effort write and observe the file after successful publication before clearing the recovery record. Uncertain saves retain the original baseline/local text and expose a scoped read-only disk observation. The user can explicitly create a new conflict copy through the existing no-replace create_file command; this does not rebase, overwrite or discard the original. Source-only; shared unmanaged files do not acquire arbitrary-writer CAS through a post-write observation. No CI, Actions, validation or branch updates.


## Shared production complete-message engine

Chat, Legacy and Coding now use the existing Foundation engine's bounded complete-message batch driver. Transport, permissions and durable transcript remain explicit host adapters. Chat/Legacy advertise zero tool calls and hold Foundation execution leases; Coding validates whole batches before its host can stage changes or pending approvals. The experimental Agent Channel streaming driver and its signed, read-only Public API still use the same Foundation task/lease authority; this change does not turn the experimental single-suggestion protocol into a general persisted multi-tool execution engine. Actual provider integration and all new tests remain NOT_RUN. No runtime/command permission expansion, branch update, workflow, CI or validation.


## Linux process-tree containment

Script/command and Linux MCP owners share a small descriptor-bound cgroup-v2 primitive. An operator may set BROWSEROS_DELEGATED_CGROUP_ROOT to an existing, user-owned, non-root delegated subtree containing the BrowserOS host; no mount, sudo, chmod, controller enablement or installation is performed. Each execution receives a fresh leaf and joins it in pre_exec before user code can fork. Descendants that call setsid or double-fork remain in the scope. Cleanup uses cgroup.kill and confirms the recursive populated field is zero before successful completion. Configured-but-unavailable delegation fails before dispatch; unconfigured hosts retain existing process-group behavior without a strict containment claim. Same-UID malicious cgroup migration, an external privileged actor, and host-crash orphan scope reconciliation remain explicit limitations; never adopt or kill an arbitrary preexisting cgroup. Windows Job behavior unchanged. MCP start failure cleans up the owned group; successful MCP receipt also requires its output reader to finish. Kernel contract: https://docs.kernel.org/admin-guide/cgroup-v2.html. Source-only tests NOT_RUN; no refs, CI, builds or checks.


## End-of-round boundary and recovery tightening

Cgroup preparation does not grant kill authority; a successful child spawn arms cleanup. Agent recovery examines all authenticated run records and running task status under session ownership, pauses interrupted tasks and retains unknown external effects. Unsupported platforms may not fall back to plaintext Coding session persistence. Linux PNG encoding observes cancellation at write boundaries; Windows CDP PNG parsing/decoding moves to a bounded worker with cancellation/document checks, and the capture receipt remains owned until decode completion. The existing watchdog receives non-blocking metadata-only capture phase and age in addition to tab mutation metadata. No screenshot renderer hard-memory-cap or forced renderer-exit guarantee is inferred from host-side bounds or callback cancellation. No GTK layout/lifecycle change. Test sources NOT_RUN.

### Nine-item review disposition (source candidate, not acceptance)

- Agent execution loop: Chat/Legacy/Coding share the Foundation complete-message engine; the experimental streaming Channel and existing approval/provider adapters are not a fully migrated universal engine.
- Native permanent IPC loss: bounded lanes, generation fencing and unknown quarantine remain authoritative; safe same-host reuse after permanently lost native completion still needs proof of executor isolation, never force-release a slot.
- GTK freeze: actual affected-host thread stack and reproducer unavailable (device offline). Metadata is evidence scaffolding, not a root-cause repair.
- Capture: bounded preflight, worker decode/encode, document generation and receipts are implemented in source; hard-isolated renderer with whole-document fidelity and confirmed termination remains unimplemented.
- Agent recovery: encrypted Coding records, leases, all unfinished-run recovery and external-effect reconciliation; not every experimental/manual path has a durable run owner.
- Linux process tree: optional delegated cgroup-v2 path for script/MCP with recursive empty receipt; delegation must exist, fallback groups are not strict containment, crash-orphan reconciliation and other process owners remain outside this guarantee.
- File conflict: encrypted pre-save drafts, post-save observation and explicit no-replace conflict copy, under D03 best-effort unmanaged-file policy.
- IPC and module responsibilities: reuse existing receipt, engine, process and draft owners; no claim all paths and state machines are converged.
- No installed-package, GUI, build, test, gate, CI or Actions verification performed; target branch/ref unchanged. Real-device evidence and permission to validate remain prerequisites for an acceptance decision.


## Production run ownership for one-shot adapters

The existing encrypted checkpoint owner now protects a domain-separated per-run receipt for Chat, Legacy and Agent Channel. An OS lock spans admission, provider/approved read execution and terminal publication. Explicit chat request UUIDs cannot be replayed after process loss; legacy callers/Channel without a request UUID get a native UUID. A repeated interrupted receipt is paused, stale approval cleared and replay denied. Metadata only, no prompt or response persisted; Coding remains the encrypted session owner, and synthetic A/B benchmark remains offline. Run records are bounded per record, not yet an indexed retention/recovery UI; generated-ID callers cannot deduplicate a new invocation. No new native command or plaintext/SQLite fallback. Tests NOT_RUN.


## Shared signed native approval boundary

Foundation HostApprovalIssuer/ApprovalBroker now serve both registered Channel tools and Coding's already-reviewed command/MCP native owner. Coding restores the same paused Task with a deterministic workspace context, binds a signed decision to the exact persisted operation and review digest, consumes the permit for the common execution lease, and persists the signed claims in its encrypted effect receipt before invoking the original native adapter. Registered tools still execute only through ToolRegistry; the native-owner claim only acquires a lease and rejects SQLite-backed registries. The actual command/MCP owners retain their existing review validation and termination semantics. The main UI remains the trusted local approval boundary (not a new user identity service). Tests NOT_RUN. Read-only frontend handoffs, manual MCP entrypoints, and experimental Rig orchestration still require separate convergence; do not treat this as universal Agent closure.


## Queued native execution fencing and layout completion mailbox

The existing tab mutation Owner now distinguishes admitted-but-not-entered worker execution from entered execution under the same lock. Cancel/queue expiry may produce a terminal receipt only before entry; delayed workers cannot cross the fence. Existing create/activate main-thread layout callbacks use a two-slot shared completion mailbox (not the old ephemeral channel). If a callback never enters, revocation makes its later execution impossible; already committed tab metadata is reported as a terminal partial failure requiring native-list reconciliation. Entered callbacks are never released on timeout; panic/lost receipt remains unknown. No changes to GTK allocation, offscreen coordinates, WebView creation/close order or Tauri request parameters. Actual indefinitely blocked entered GTK work remains a process-lifecycle boundary. Added regression sources NOT_RUN.


## Screenshot native callback entry fence

CaptureCancellation now carries an independent, monotonic queued/entered/revoked native dispatch token. Cancellation may publish a terminal receipt only if the native closure has been irrevocably fenced before entry; both WebKitGTK and WebView2 closures check the token before touching the engine. A two-slot native dispatch permit counts queued callbacks until actual destruction/entry, so repeated cancellation cannot create unbounded abandoned callback work. Closed reply channels are only recoverable when the pending token can be fenced; callbacks that already entered retain the Unknown/CancelRequested owner. Late results cannot overwrite a terminal receipt. This is control-plane isolation, not renderer-process memory isolation. The live-document FullDocument/CDP engine limitations in D02 remain. Test sources NOT_RUN.


## Remaining local approval entrypoints

Coding read-only claims now persist a signed Foundation approval, Run ID and process epoch before handoff to the existing UI owner. Claims from earlier process epochs (including legacy executing claims without a receipt) become unknown; late UI results cannot complete them. The lease covers native admission, not the UI read's lifetime. Direct user-approved MCP tools/list and tools/call use encrypted production run receipts, signed approval and the common execution lease, and retain the existing revision/credential/process owner. Uncertain direct MCP results are recorded paused/operation_unknown, never automatically replayed. Manual callers still lack stable caller request IDs and a receipt catalog/reconciliation UI; new explicit invocations are distinct runs, not deduplication of remote effects. Tests NOT_RUN.


## Common model adapters

Remote Chat/Legacy and local Ollama now share Foundation's zero-tool complete-message adapter. Local Chat enters the encrypted production Run owner and uses an absolute transport read deadline. Streaming Foundation and the official experimental Rig adapter share a single revision-bound model-turn lease/cancellation/deadline implementation. Rig retains its real SDK turn algorithm for the intentional offline A/B experiment; this does not claim remote providers physically terminate on dropping a future. Coding keeps its existing bounded complete-batch host. No network/provider tests run.


### 2026-11-25 — Host screenshot postprocessing budget and exit receipt

- Linux pixel-copy allocation uses fallible bounded reservation; PNG writer bounds the requested growth capacity. Windows CDP JSON normally borrows the base64 field and decodes fixed-size chunks into a fallibly reserved, length-checked output. Cancellation and document generation are checked between chunks.
- Both host CPU workers now return their result to an off-main-thread joiner. The completion callback runs only after the owned worker thread has exited; panic produces a failure result after join. These are **host postprocessing** limits/exit receipts, not a renderer process limit or termination proof.
- Remaining D01/D02 boundaries: already-entered hung GTK/WebKitGTK/WebView2 operations cannot be force-cancelled safely in this live-WebView architecture. Queue fencing cannot release an entered unknown operation. Independent renderer process ownership, allocation enforcement and a validated exit receipt for the actual live document remain unimplemented. No hidden page reopen, forced shared-renderer kill, new Tauri command, or locked layout change was made.
- Remaining D04 boundaries: experimental Rig retains its SDK orchestration algorithm (common model-turn gate); one-shot receipt metadata is not a complete resumable conversation store, and manual MCP still lacks receipt catalog/reconciliation and a caller-stable retry identity. Frontend read handoff is not a full native execution lease. No unsupported claim of total closure.
- New regression source covers cross-chunk decoding, interior padding, cancellation. NOT_RUN. No CI, build, tests, validation, installation, or ref update.


## 2026-11-25 — Native run payload and bounded receipt admission

Base b9dcd23e. Extend the existing encrypted ProductionExecution/RunCheckpoint owner, without introducing a new database, IPC command, or parallel engine.

- Remote Chat, Legacy and local Ollama now persist normalized text-only model input before dispatch, and eligible response payload in the terminal receipt before IPC success. The native credential snapshot is not serialized. On the same request UUID and exact normalized input, a completed eligible response is returned from the authenticated receipt without another provider invocation; mismatched input, an old metadata-only receipt, or an interrupted dispatch cannot replay. General Chat offers explicit same-turn retry in the existing panel. An unrecorded original request may dispatch on that explicit retry; a recorded interrupted one never does. No conversation text or credential is stored in frontend localStorage.
- Existing per-run locks remain authoritative. Catalog admission scans canonical .run identities under a private interprocess catalog lock; maximum 4096 records, 1 MiB authenticated plaintext per record, and a bounded directory scan. Existing admitted runs can publish terminal outcomes at capacity. This is non-destructive retention/admission, NOT a user-configurable pruning or archive policy. No automatic deletion of unresolved effects, history, sealed markers or run IDs. No claim of a metadata-index/UI catalog yet.
- Manual MCP records approved request digest, bounded target/tool labels, actual transport outcome digest and reported disposition. Recovering a crashed approved effect marks it operation_unknown instead of ordinary interrupted. The remote effect is not proven by a response digest. Raw MCP arguments, credential refs/values and response contents are not copied into the effect evidence. Manual callers still need an explicitly approved stable request identity and a query/reconciliation interface.
- New source-level regression cases cover exact replay recovery, mismatch rejection, ciphertext and interrupted run refusal. NOT_RUN.
- D01/D02 still block replacing the live native rendering ownership architecture without specific lifecycle adjudication and affected-platform evidence. No locked GTK/WebView lifecycle change, new command parameters, branch-ref mutation, CI/Actions, installation, build, test or check. D04/D05 still need the catalog/read/continue/reconcile contract, full multi-turn Session continuity and retention decisions for all callers; one-shot eligible reply recovery does not implement them by itself.


## 2026-11-26 — Text run catalog and explicit Task continuation

- Reused encrypted RunCheckpoint, OS lock, existing ai_service and Foundation TaskRegistry. Bounded authenticated ID pagination; main-only catalog/restore UI; no arbitrary record paths or project effect data through the text route.
- A persisted predecessor reserves a single successor ID and exact input before dispatch. An interrupted text-only task restores the same paused Task and Session under a new Run; a normal chat turn creates a Task in the same Session. Current provider configuration is checked again and credentials are acquired only in the native adapter. Unpublished reservations can be recovered explicitly. Committed successors return eligible saved results, never dispatch again. No automatic model replay. A failed/cancelled or forgotten record cannot resume as a pending Task.
- Normal chat panel uses the prior run as continuation authority; historical local/legacy text requests can be viewed/resumed without tools, but are not ordinary remote chat continuations. Explicit per-record terminal payload forgetting preserves IDs and unresolved successor reservations; other records may retain context copies. The existing 4096-record cap remains non-destructive; full-store archival/compaction and expiry policy are still separate unresolved concerns.
- Narrowly permit authenticated paused project-free text records in Foundation; no change to workspace approval semantics. Regression sources added for same Task, no fork, no replay and forgotten receipt. NOT_RUN.
- Original D01/D02 strict renderer process ownership and entered-native hang recovery are not modified. No native lifecycle or Tauri parameter change, no ref update, no CI/Actions/validation.


## 2026-11-26 — Manual MCP recovery and reconciliation interface

Manual workbench calls now generate their Run UUID before dispatch, use it at the existing coding_service boundary, and expose the original receipt in the MCP pane even after an IPC failure. Legacy callers without a stable ID still get a native-generated one. New receipts include a digest of the native workspace directory identity and the exact reviewed call revision, bounded safe target/tool labels, response digest and transport disposition. Read/catalog actions are authenticated and project scoped. Manual execution holds the existing workspace OS ownership lock through the native reviewed call. A successful transport return is not a universal exactly-once side-effect proof.

User-entered observations of applied/not-applied/unresolved effects are encrypted, bounded (16 observations per receipt, 2048 bytes each), revision-checked and retained with their timestamps. They do not transition operation_unknown to completed, remove the original Run ID, authorize reuse of a stale grant, or automatically invoke MCP again. Old path-scoped receipts are marked as not bound to physical directory identity. This is a real reconciliation evidence UI, not a remote-system proof or cross-Run idempotency protocol; a separately approved new Run remains a separate effect attempt. New tests cover stale observations, unknown-state retention and denied original-ID replay, NOT_RUN. No native lifecycle or command parameter change, no workflows/gates/refs/CI/Actions/validation.


## 2026-11-26 — Non-destructive terminal receipt archive

Explicit terminal text-payload forgetting now compacts the authenticated receipt and moves its identical encrypted bytes into a private immutable archive, using no-replace publication, readback and version-checked active-copy removal under the same per-run and catalog OS locks. The old Run ID, Task/Session lineage and sealed marker remain; both-copy interruption is recoverable, disagreement fails closed. Reads/catalog pages include archive records and deduplicate IDs; archived IDs cannot be recreated or written. Active capacity is released only after active-copy removal is confirmed. The UI distinguishes successful content forgetting from pending archival and offers an explicit retry. There is no automatic content deletion, no pruning of unresolved effects, and no claim of erasing copies retained by other turns or backups. Default bounds: 4096 active receipts (1 MiB each) and 16384 archived compact receipts (64 KiB each); the finite archive cap requires explicit future storage review rather than silently discarding anti-replay evidence. Manual effect/experimental receipt compaction remains outside this text-only policy. Missing key initialization markers beside production run records fail closed rather than initializing a replacement key. New regression sources cover duplicate publication, repeated archive, catalog visibility and denied replay. NOT_RUN. No ref updates, native lifecycle changes, CI, Actions or validation.


## 2026-11-26 — External whole-application recovery owner

Added an explicit `--supervised-recovery` launch mode, not a change to default installed startup or a replacement for the locked GTK layout. A parent without Tauri owns the exact spawned child and its kernel process domain (Linux delegated cgroup v2, Windows suspended creation and existing Job Object assignment). A per-launch random authenticated loopback probe acknowledges only after a callback enters the main UI loop; one dispatch worker and one outstanding probe prevent accumulation. After at least one genuine UI acknowledgement, a 60-second unacknowledged live probe may terminate the owned domain. Replacement is permitted only after kernel recursive-empty accounting and Child exit confirmation; at most one replacement; no command-line URL replay. Ordinary exits, initial startup without acknowledgement, a lost probe connection, and reported graceful-shutdown preflight do not trigger automatic restart. A second unresponsive generation is retired without further relaunch. Nested cooperating Linux ProcessScope owners are confined inside the supervising scope. No PID discovery/adoption, no sudo or automatic delegation, no new Tauri command or WebView lifetime change.

This opt-in mode may lose volatile web state or unsaved edits when forcibly recovering a hung app. It does not guarantee shutdown race immunity, recovery from supervisor death on Linux, containment of externally launched services, or preservation of in-memory DOM/session state. It does not separate the live renderer from its host, so D02 strict independent screenshot isolation is still open. Default launcher unchanged pending native lifecycle evidence and owner review. Source-only regression for recovery decisions, NOT_RUN; no CI, refs, build, test, or installation.


## 2026-11-26 — Channel checkpoint, continuation and history

The actual experimental Channel entrypoints are deterministic offline fixtures and explicit read-only Public API calls. Both legacy commands and main-only AI-service actions use the existing encrypted Run checkpoint (historical payload field classified by Task goal). A fixture pins its assignment; a read pins the resource/arguments. Eligible results are committed before terminal success. Stable Run IDs support committed-result recovery. Interrupted tasks reserve one successor, restore the same Task/Session, and keep the predecessor OS lock through the async attempt. Read continuation requires a fresh local confirmation, current catalog/resource validation and signed permit, never the prior grant. Existing BlockingLane remains occupied by the actual blocking read if ToolRegistry times out. A failed request receipt is not proof the blocking worker exited. History, restore, explicit resume and terminal compaction are exposed in the existing experimental panel. Historical metadata-only receipts remain view-only. No third runtime, live model promotion of synthetic Rig tests, or remote-effect replay guarantee. Source regression NOT_RUN. No default launch or GTK lifecycle change, branch refs, CI, Actions, builds or checks.


## 2026-11-26 — Explicit independent capture render job (source candidate)

Added a native-only opt-in `--isolated-capture HTTPS_URL ABSOLUTE_NEW_PNG` entry. This is a **new anonymous/fresh-profile render** explicitly requested by the user, never a substitute for current-live-document screenshots. No default launch, normal browser session, layout/hide behavior, Tauri command parameters, remote capability, or existing profile changes. The same TabManager/capture_bound_document path is reused with a private, unique profile, actual page-load observation and current-document check. The independent process has no ordinary app plugins/IPC handlers, is visibly separate, denies downloads/popups and non-HTTPS main navigation, and does not transfer cookies, credentials, DOM or live-session state. Fresh HTTPS responses can differ from the live page.

The same native process ownership infrastructure as the application supervisor creates the worker domain **before** execution. Defaults chosen for this opt-in candidate: 2 GiB cgroup-accounted memory / Windows Job committed-memory budget, Linux swap disabled and group OOM behavior, 60s elapsed job deadline, existing 16M pixel / 16384 edge / 32 MiB output admission. Failure to configure and read back budget refuses startup. Parent only publishes bounded PNG via native no-replace after original child exit and recursive process-domain empty confirmation. Timeout/worker failure retires the owned domain; unknown exit retains private scratch, never publishes an output. Cleanup is native root-anchored and identity-checked after exit; cleanup failure cannot delete published output. No downgrade to unbounded capture or live-tab reopen. FullDocument/CDP are still the engine mechanisms; no claim of tiled rendering or exact fidelity, whole GPU/external broker memory accounting, parent hard-crash containment on Linux, or arbitrary same-UID security isolation. No new UI or change to live capture receipt protocol. Verify actual WebKit/WebView2 descendants, profile separation, load/error behavior, memory/OOM, deadline, exit and package launch on real Windows/Linux before calling strict mode supported. Source regression tests added NOT_RUN. No builds, checks, CI, Actions, installs or ref updates.
