> **历史工作记录，非当前分支状态：** PR #50 已于 2026-11-02 合并到 `master@dd32cc82`。本文的 “only on this branch” 仅描述合并前的工作阶段。最新下一轮改造请查 `2026-11-02-lifecycle-chat-continuation.md`。

# 2026-11-02 Master-based architecture remediation progress

Base: `master@c90eb564`; target: `research/architecture-reconstruction-audit-20261101`. Changes apply **only** on this branch; this is **not** a production completion certificate.

## Implemented and checked

- Added separate **ordinary AI chat without a project root** under Agent Manager (`GeneralChatPanel.vue`): uses the existing authorized `aiService.chat('default', ..., modelId)` provider, has model selection, typed, bounded user/assistant conversation history validated by the Rust AI Owner, independent send and new-conversation controls, no access to workspace, code tools, or side-effect execution. Existing project-scoped Coding Workbench is untouched. Conversation is multi-turn at the existing provider boundary but is not a durable Agent runtime; no session persistence or streaming claims.
- Capability Runtime synchronous `activate` / `suspend` now reject promise-returning hooks instead of announcing a transition before the promise resolves. The existing `activateAsync` / `suspendAsync` remain the correct entrypoints for async lifecycle. Added RT-17 and RT-18 negative tests. **Remaining**: migrate all production callers to a single async transition and verify cleanup and rollback; rejecting an already-started async hook does not cancel it.
- Hot-Plug orchestrator executes the loader only once (in `requestAdd`, with await), no longer drops its promise. Loader registration failure triggers contribution cleanup. Runtime hot-add requires a deliberate `permit` callback (default DENY); two existing test/demo callers explicitly provide a permissive **fixture** callback, which must never be reused for production authorization.
- Corrected the stale registry source paths for `reveal_path` and `move_path` from `src-tauri/src/fs_cmds.rs` to `src-tauri/src/capabilities/workspace/fs_cmds.rs`. Marked the old 148-command physical-boundary report as a historical snapshot (current registry = 183). Updated outdated Agent/Skill owner comments.

## What is **NOT** complete

| User gate | Current state | Blocker for declaring completion |
|---|---|---|
| Unique Owner for **every** business state | PARTIAL | Full state-level writer graph and canonical owner enforcement not completed, old Coding and new Foundation task owners coexist by design. |
| Formal Public API for **all** new cross-domain features | PARTIAL | Rust `framework/agent_channel.rs` still directly invokes domain commands; dedicated approved stable capability facades and negative checks needed. |
| All Native commands physically owned | PARTIAL | All 183 registry entries have Owner, but 100 still point to `bridge.rs`. Moving them safely is a separate broad Rust refactor. |
| Lifecycle of all critical resources | PARTIAL | Synchronous hook fail-closed added; all production paths, cancellation and OS resource cleanup remain to be proven. |
| No unapproved cross-module internal reference | NOT VERIFIED | TS alias and Rust call graph scanning plus allowlist approvals not completed. |
| All critical side effects guarded | PARTIAL | Existing guards remain; incomplete global permission/confirmation proof; 58 registry entries have no explicit permission label. |
| Every module test independently runnable | PARTIAL | Foundation and several modules have tests, not universal separate per-module test entrypoints. |
| Documentation matches all code | PARTIAL | Targeted stale entries corrected, old audit explicitly dated; exhaustive doc/source drift checker not done. |
| Installed GUI real acceptance | NOT VERIFIED | No matching-SHA Ubuntu GNOME manual GUI test or .deb package run by this patch. |
| Before/after performance baseline | NOT VERIFIED | No paired same-machine CPU/RSS/PSS/startup comparison. |

## Evidence

- `node scripts/check-capability-platform.mjs`: 51 PASS / 0 FAIL (includes hot-add fixtures)
- `node scripts/check-capability-runtime.mjs`: 18 PASS / 0 FAIL (RT-17/18 negative cases)
- `node scripts/check-native-capability-boundaries.mjs`: 183 commands, 0 failure, 0 warning
- `node scripts/check-hot-plug-acceptance.mjs`: PASS
- Frontend build result and branch commit must be recorded separately from this document; a build PASS does not establish installed GUI PASS.

## Safety notes

The new ordinary chat is read-only from BrowserOS's perspective; provider requests necessarily send the user's text to the configured model provider. Historical context is bounded and kept in page memory only; chat can be reset. No silent root/default project directory is invented.

The `permit: () => true` callbacks in test/demo fixtures are explicitly non-production. Production Hot-Plug callers **must** validate the authenticated principal, target capability, permissions, and resource effects at the Host before permitting install.

## Structured chat history follow-up

Existing `ai_service` now accepts an optional `history` array containing at most 12 validated user/assistant turns (total length limited to 18,000 UTF-8 bytes) and rejects `system` role injection and credential-like content. The plain chat UI caps earlier history to 16,000 characters, asks the configured provider without any root path, and does not enable tools. The Rust unit test `chat_history_is_bounded_and_only_accepts_user_assistant_roles` covers history-role and length constraints. The actual provider network must still be verified with a configured model and installed GUI.

## Second-round regression corrections

- Resource Governor has moved to awaited asynchronous `activateAsync` / `suspendAsync`; its `destroy` awaits suspend before disable. This fixes an actual terminal idle-suspend false failure exposed by the full repository gate. `scripts/check-composition-profiles.mjs` awaits the new public API; 11/11 PASS. This is **not proof of all OS process/PTY cleanup**.
- New `GeneralChatPanel.vue` is registered in `docs/architecture/ui-system/ui-components.yaml` (95 entries). UI boundary gate PASS.
- `src-tauri/src/framework/ai.rs` now validates bounded structured chat history (12 turns, max 18,000 UTF-8 bytes), rejects non-user/assistant roles and credential-like content, and dispatches to the same existing authenticated `ai_service` provider. This retains the existing read-only AI Owner and adds no project root or new native command.
- `npm run check` completed PASS locally on this branch. `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` PASS. Rust compile/test, installed GUI and model-provider integration were not executed locally due disk capacity; they need independent same-SHA evidence.

## Owner Public API boundary and static guard

- Added narrow read-only Rust `public_api.rs` ports in Browser, Database, Git and Agent owners; `framework/agent_channel.rs` invokes only these Owner facades instead of direct `commands.rs` or `bridge::tab_list`. Owner ports delegate to existing native command handlers with the original source checks and 100-item/size limits, preserving business ownership.
- The standard `scripts/check-native-capability-boundaries.mjs` now enforces NATIVE-08 for this Agent Host, with positive/negative fixtures. This is a targeted consumer contract **not** a proof that all other Rust cross-module calls are migrated.
- `scripts/check-capability-boundaries.mjs` now resolves `@/` and root-relative `src/` imports, with a negative alias-import fixture, removing one previously unaudited path class.
- Native command registry remains 183 entries, 100 still physically in `bridge.rs`. These are **not migrated** by the facade patch.
