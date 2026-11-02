# BrowserOS lifecycle / Agent / chat continuation (2026-11-02)

- **Base:** `master@dd32cc828e21f923e61aca0ed66b647e8c957e41` (PR #50 merged)
- **Implementation branch:** `fix/capability-runtime-agent-chat-20261102`
- **Scope:** P0 lifecycle and Owner cleanup contract; P1 Agent Owner gateway and project-free chat; P2 independent contract and negative tests, docs. Source branch only; no changes to master or the user's `browseros-ui2-aplus-20261101` checkout.
- **Status:** implementation and static/unit tests are not GUI or real-provider acceptance. Use exact commit SHA from the branch before publishing any test verdict.

## P0 — execution transitions and resource release

Source: `src/capability/runtime.ts`, `src/capability/types.ts`, `src/capability/resourceGovernor.ts`.

1. Per-capability `pending` record holds a Promise and AbortController. Concurrent calls to the same `activateAsync`/`suspendAsync` share one transition; conflicting transition/disable/unregister/reset is blocked rather than committing an inconsistent state.
2. `cancelTransition(id)` aborts activation cooperatively. On rejection or late cancellation, the Owner's `onDeactivate` is awaited before leaving the original state; cleanup failure is recorded and propagated rather than being reported as success. A known `AsyncFunction` lifecycle hook is rejected **before invocation** by the legacy synchronous entrypoint.
3. `releaseAsync(id)` serializes concurrent closes, cancels pending activation, waits for suspension, invokes the Owner's `onDestroy` (or `onDeactivate` fallback), and only then disables metadata. New release receipts distinguish `ownerCleanup=true` (hook finished) from `ownerCleanup=false` (no release hook). It **does not** claim a verified process/PTY/WebView handle count; real Owners must supply and test their release hook.
4. `ResourceGovernor.destroy()` delegates to `runtime.releaseAsync`; ACTIVE non-suspendable resources no longer silently fall through to an impossible disable.
5. Contract tests `RT-19`..`RT-27` exercise delayed activation, duplicate calls, cancellation, rollback, cleanup failure, no premature reactivation during close, and concurrent close idempotence.

**Important remaining limitations:** Existing synchronous startup bootstrap is kept for compatibility. A non-`async` function that dynamically returns a Promise may still start work before the legacy sync API rejects it; the full conversion of all bootstraps to `activateAsync` and real OS resource close hooks remains work. AbortSignal cancellation is cooperative, not forcible OS-process cancellation; OS-resource zero-leak testing is still required.

## P1 — Agent execution Owner and formal APIs

- The read-only Coding LSP status gateway is now implemented by `src-tauri/src/capabilities/agent/public_api.rs` instead of reaching backwards from that Owner into `framework/ai.rs`. It retains the main WebView source check, canonical-root validation, and the Agent-owned `coding_lsp::execute` implementation.
- `src-tauri/src/framework/agent_channel.rs` still uses the four existing Owner facades. Existing git/database guards remain unchanged.
- The Native boundary checker now verifies that declared command **file paths match actual Rust `#[tauri::command]` locations** (`NATIVE-07`) and rejects resurrection of the old Agent-to-Framework gateway (`NATIVE-09`). Positive and negative fixtures are included.
- The Browser `tab_list` command and source-of-truth tab table remain centralized in `bridge.rs`. A proposed physical move was **reverted** because `check-native` marks it a protected dangerous area requiring explicit source migration approval. No governance checker was bypassed.
- `agent-foundation` is still a separate opt-in execution implementation; old `agent_chat/run` and real `coding_service` are not falsely claimed to share one durable Task Owner. A single production execution owner requires a separate, behavior-preserving migration.

## P1 — ordinary project-free chat

- `model/chatHistory.mjs` selects up to 12 recent user/assistant turns with code-point safe UTF-8 **byte** truncation (3000 bytes/turn, 16000 bytes sent total). The current prompt is rejected in UI if it exceeds 24000 UTF-8 bytes. Rust still validates independently.
- Model switch creates a fresh in-memory conversation. Late replies from a previous session/model are dropped; the active request has a cryptographically random UUIDv4 (fallback for older WebKitGTK secure random generation).
- The existing `ai_service` now supports a bounded `request_id` and `action:'cancel'`, restricted to the trusted `main` WebView via the existing source policy. Rust stores only an in-memory UUID->AtomicBool map (max 32 requests), and cancellation terminates the already bounded `curl` provider worker through the existing cancellation monitor. RAII unregisters completed requests. Switching from ordinary chat to Coding/Blueprint also requests cancellation of any active plain-chat worker; the component remains mounted to preserve earlier completed messages. An aborting UI does not append a late response.
- No root path, file editing, Git, SQL, shell, or MCP side-effect tools are authorized in this chat. Model replies still use the configured external provider; nothing is claimed about persistent transcripts or streaming output.
- Tests: `node tests/check-agent-chat-history.mjs`, Rust `chat_request_cancellation_is_scoped_and_cleanup_is_guaranteed`, and existing native source checks.

**Residual:** A cancellation arriving before server-side request registration returns `cancelled=false` rather than creating an unbounded cancellation tombstone; the UI remains busy until the initial worker exits (bounded provider timeout). Live installed-GUI/provider tests must check this fast race and WebKit compatibility. No chat disk persistence or streaming yet.

## P2 — modular tests and doc/code parity

- Every one of the **20** `src/capabilities/*/manifest.ts` manifests can be independently compiled and checked by `node tests/check-capability-module-contract.mjs <capability>` or with `--all`; this validates ID consistency, unique declared semantic Owner, formal Public API locators and entrypoints. **This is a manifest/contract smoke check, not a full isolated test of 20 business implementations.**
- New negative Native fixtures validate source locator drift and Agent Public API reverse dependencies. Existing `NATIVE-01..08` gates remain.
- Updated stale `agent-foundation/Cargo.toml` description and Agent README Stage E Skill reference, and annotated the historical 2026-11-02 work report so pre-merge claims are not read as current facts.
- `package.json` includes named chat history and all-capability contract checks in the normal `npm run check` sequence. Native command count stays **183**, with **100** still in `bridge.rs` and **58** registry permissions shown as `-`. Those are residual evidence gaps, not new unverified permission grants.

## Verification checklist (must be filled from real execution)

| Command or evidence | Purpose | Status |
|---|---|---|
| `node scripts/check-capability-runtime.mjs` | 27 transition/rollback cases | PASS (27/27) |
| `node scripts/check-capability-platform.mjs` | existing hot-plug contracts | PASS (51/51) |
| `node tests/check-agent-chat-history.mjs` | UTF-8/UUIDv4 | PASS |
| `node tests/check-capability-module-contract.mjs --all` | 20 module contracts | PASS (20/20) |
| `node scripts/check-native-capability-boundaries.mjs --self-test` + normal run | Rust Owner/physical locator negatives | PASS |
| `python3 scripts/generate-rust-crosslayer-contracts.py --check` | Native frontend ABI registry | PASS (183 commands) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | Rust formatting | PASS |
| `npm run check` | whole repository static/integration checks | PASS locally after final source changes; 20/20 manifests in gate |
| `npm run build` | frontend production bundle | PASS locally (2m11s) before the final visibility watcher addition; exact-commit CI rebuild still required |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | compiled Rust regressions | NOT VERIFIED until exact branch SHA CI completes |
| Installed `.deb` Ubuntu GNOME GUI | interactive real acceptance | PENDING USER |
| Before/after paired runtime CPU/RSS/PSS and latency | production performance baseline | NOT MEASURED |

**Definition of done:** all ten original architectural goals still require evidence beyond the focused fixes above. This branch is an incremental implementation, **not** an ALL_PASS rearchitecture tag.

## Second continuation — production bootstrap, Owner routing, and evidence (2026-11-02)

### Actual application startup now uses asynchronous transitions

- `src/main.ts` calls `bootstrapCapabilityRuntimeAsync()` and mounts Vue **after** the awaited capability setup. It reports startup failures without preventing the shell from mounting, and schedules the `m0_ready` first-paint probe after mount (not before).
- `src/capability/index.ts` adds an idempotent production-only async bootstrap for profile, custom Assembly and enabled-config routes. Dependencies register and resolve before activation; each Owner's `onActivate` is awaited. A failed startup collects reverse-order rollback errors instead of falsely returning `activated=true`. The legacy sync bootstrap remains for source-compatible tests, not the GUI startup.
- `src/capability/platform/pluggable.ts` adds async activation, awaited rollback and async Owner release/deactivation while retaining the original sync test surface. `scripts/check-pluggable-lifecycle.mjs` covers delayed Owner hooks, no premature ACTIVE/contribution and activation failure.
- `tests/check-async-production-bootstrap.mjs` executes the real bundled production path and checks idempotency, framework-only, minimal and full 22-capability boot.

### Coding requests now enter one Agent Owner API

- `src-tauri/src/framework/ai.rs` retains provider transport, model selection and Tauri source authorization, but delegates the **actual Coding action dispatch** to `src-tauri/src/capabilities/agent/public_api.rs::dispatch_coding_request`. This Owner routes Coding, LSP and read-only context requests and continues to own every approval/write gate. Cancellation queries also go through the same facade.
- Native boundary rule `NATIVE-10` rejects production Framework imports of `Agent::coding(_context/_lsp)::execute`; its negative fixture validates the rule. `#[cfg(test)]` Rust fixtures are explicitly excluded, not production code.
- This removes one reverse dependency, but **does not unify old Agent Blueprint, Coding task state and Agent Foundation's TaskRegistry into one durable execution state machine**. Productionizing such a migration requires compatible session/approval replay and live failure-injection tests.

### Real baseline data: limited but measured

- `scripts/measure-lifecycle-comparison.mjs` measures the **same TS activation/suspend sequence** in isolated Node workers, comparing baseline `dd32cc82` with current worktree. It records per-cycle latency/CPU and benchmark-worker RSS/PSS, hardware and explicit limitations, and supports `--self-test` and a JSON artifact.
- Local first sample (900 iterations × 6 samples): median latency ratio current/baseline ≈ 1.78; CPU ratio ≈ 1.66. Absolute median latency was around **0.00123 ms per cycle**; the extra guards cost microseconds, not seconds. Results are runtime microbenchmarks **NOT** an installed BrowserOS performance improvement claim.
- For native real resources, the existing Tauri `ShutdownCoordinator` owns real cleanup tasks `close-tabs`, `kill-terminals`, `shutdown-grid`, `kill-running-scripts`, and session flush. These implementations and Rust tests **do not establish that all real installed WebView/PTY/DB handles have reached zero** after GUI lifecycle. No BrowserOS process was running on the authorized Linux desktop during this audit, so a real PID comparison was not fabricated.

### Native Owner & effect debt gate

- `scripts/check-native-owner-effect-audit.mjs` generates and verifies `docs/architecture/native-boundary/NATIVE_OWNER_EFFECT_AUDIT_20261102.md`: **all 183 commands** have explicit owner, physical file and effect metadata. The **58 legacy `permission: "-"` commands remain unverified** and are frozen in `native-permission-debt-baseline.json` by exact command name. New unreviewed permission gaps fail the gate; this does **not** grandfather any hidden permission or imply that the 58 paths passed side-effect authorization.
- A proposed relocation of Browser `tab_list` from `bridge.rs` was reverted by the `check-native` protected-surface gate. `bridge.rs` still hosts about 100 Native commands. Bulk physical refactoring without per-Owner approved migration and installed GUI tests would falsely claim safety and remains blocked.

### Remaining mandatory completion gates

1. Durable unified execution task/approval Owner: old Agent Blueprint, Coding state, and Foundation must share one authoritative lifecycle and replay policy; currently incomplete.
2. Native 100-command physical extraction and 58 effect-permission debt items: currently incomplete; require actual authorization/negative proof.
3. Real installed GUI on exact branch/master build, same host and same scenarios: **pending user interactive acceptance**.
4. Paired pre/post application startup latency, CPU, process-tree RSS/PSS and open WebView/PTY/DB/Worker handles: **not measured**; the Node microbenchmark is a different measurement, not a substitute.

All claims are intentionally scoped. The branch MUST stay unmerged until required checks/approvals are satisfied.

### Native cleanup process fixture and build capacity

- `src-tauri/src/shutdown.rs::native_shutdown_reaps_real_child_pid_and_joins_owned_worker` spawns a **disposable test-owned** `sleep` process and thread, registers their cleanup with the existing production `ShutdownCoordinator`, and asserts both cleanups and repeat-shutdown idempotence. This creates a real PID for the test, but is **not** a BrowserOS terminal/WebView/DB leak measurement. It also fixes the stale module header claiming that `ShutdownCoordinator` was not yet wired; production wiring already exists in `main.rs` and `bridge.rs`.
- A second, longer same-machine TS lifecycle microbenchmark is recorded in `docs/architecture/performance/lifecycle-dd32-vs-runtime-20261102.json` (3000 iterations × 8 samples, independent Node workers). Median overhead ratio ranged between approximately 1.8x and 2.8x across runs; median cost remains under 0.003 ms per activation/suspension pair. **This is a regression in microbenchmark throughput, not evidence of improved system performance.**
- The authorized Ubuntu desktop's default Terser production build was killed after Vite began transforming while system swap was exhausted and IntelliJ + ChatGPT desktop were in use; no user application was force-closed. The **exact commit's GitHub CI build and Rust compilation must be used as final source verification**, rather than calling that local attempt PASS.
