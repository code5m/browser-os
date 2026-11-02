# 2026-11-02 Capability / Agent / Native hardening — follow-up to PR #51

Base: branch `fix/capability-runtime-agent-chat-20261102` at `7aff355a` (master `dd32cc82`). This work stays on the branch; it does **not** alter the user's GUI acceptance master.

## Implemented in this follow-up

1. `CapabilityRuntime.releaseAsync` validates owner/governance/dependents **before** any Owner side effect, caches the completed receipt for sequential idempotence, clears the receipt only on an explicit new enable/unregister/reset, and prevents new dependents from registering/enabling/activating while their dependency is closing. Tests RT-28..RT-31 exercise duplicate release, preflight denies, second lifecycle, and dependent races. A cleanup receipt proves Owner hook completion, **not** PID/FD zero leaks.
2. Production first paint in `src/main.ts` no longer awaits the full capability bootstrap; shell mounting and `m0_ready` are independent of slow async Owner hooks. Background bootstrap errors remain observable via `debugLog`. `tests/check-async-production-bootstrap.mjs` verifies the entry order and the actual real bootstrap. **Remaining:** shell capabilities can initially be inactive; GUI must test loading/feature gating and eventual recovery. There is still no hard end-to-end timeout for an unresponsive Owner hook.
3. `createPluggableRuntime.activateAsync` now re-enables previously disabled capabilities before activating and does not trust a stale `active` boolean; a new async activate -> deactivate -> activate regression covers the flow.
4. `ai_service` records a bounded 60-second UUIDv4 cancellation tombstone when the cancel arrives before chat registration. The subsequent chat request consumes the cancelled signal and must reject before model transport. Exact-match IDs, at most 32 active/tombstone entries, and RAII cleanup remain. Rust unit `cancel_before_register_prevents_late_provider_start` added.
5. **Single task state-machine Owner for the two live production paths:** `src-tauri/src/capabilities/agent/execution_owner.rs` now uses one process-wide Foundation `TaskRegistry` for both ordinary chat and `coding_service`. Result success/failure and abort use the same `TaskStatus` transitions, without storing user prompt or tool arguments in the owner. Foundation now has `retire_terminal` for non-durable terminal records; production RAII guards prune their own task states after completion/cancel so the registry cannot grow with completed chat turns. Coding-specific workspace/tool confirmation remains solely in Coding Owner; the experiment Agent Channel remains isolated. This standardizes **shared production task state Owner**, not yet a persisted task scheduler or unified old Agent/ACP/RMCP write path.
6. **Native permissions:** static audit located all 58 historical `permission: '-'` command names in the actual installed main-window `default-commands` Tauri allowlist. The 56 other than `report_resources` / `report_title` are not in `remote-collect`; those two are intentionally remote-allowed telemetry. The normal `scripts/check-native-owner-effect-audit.mjs --check` enforces this against capabilities/permissions and rejects any new `permission: '-'`, and a per-command source list is in `docs/architecture/native-boundary/NATIVE_58_CALLER_SCOPE_REVIEW_20261102.md`. The 58 previously undeclared Tauri caller scopes are now stated explicitly in the Native command registry (`permission: '-'` is **0**). The same 58 command names remain in `pending_side_effect_reviews` for Owner action/confirmation, argument-scope, untrusted WebView denial, and real installation checks. This separates the **resolved caller-ACL declaration debt** from the **unresolved effect-level authorization evidence**; it does not falsely claim the latter is done. Tauri ACL scope is not the same as Owner approval.

## Scope of verified tests

- `node scripts/check-capability-runtime.mjs`: 31/31 PASS.
- `node scripts/check-pluggable-lifecycle.mjs`: PASS including new reactivation.
- `node tests/check-async-production-bootstrap.mjs`: PASS.
- `node scripts/check-native-owner-effect-audit.mjs --check`: PASS, with 58/58 Tauri main allowlist and 2 intentional remote telemetry exceptions.
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`: PASS.
- `npm run check`, `npm run build`, and **exact-commit** Rust tests require separate final-run evidence; success at previous SHA is not proof for the new changes.

## Unclosed full-release gates

- Installed Ubuntu GNOME `.deb` GUI smoke/first paint, interactive model configuration, abort-before-register network regression, PTY/WebView/DB process-tree PID/FD/RSS/PSS leak test.
- A durable and fully routable Agent TaskRegistry shared by legacy Agent, Coding, chat, and opt-in Agent Channel; the current production chat and Coding share an in-memory Foundation registry, while old Agent and experimental channels are not silently enabled. Business write tools must keep separately approved Owner ports.
- 58 individual Native side-effect authority/confirmation reviews. The new main vs remote Tauri boundary proof reduces ambiguity but does **not** complete these 58 per-command security acceptances.

Do not describe this branch as fully production rearchitected. The remaining criteria are tracked independently, and manual GUI acceptance belongs to the user after any separately approved merge into master.
