# BrowserOS V2 最终源码冻结与交付边界 — 2026-11-27

Status: **SOURCE_FROZEN / NOT_RUN / GUI_PENDING / REF_PUBLICATION_PENDING**.

This is a source handoff, not a compile, runtime, release, CI or GUI acceptance result.

## 1. Immutable candidate

- Runtime/code baseline: `d5183041190f3c7e15d2da213c5c0e098867f5a1`
- This successor changes documentation only. No product source, dependencies, lockfiles, tests, workflows, gates, build configuration or platform policy change.
- The candidate is an immutable Git commit object. No branch, tag or master reference is moved. The prior feature-branch tip must not be mistaken for this candidate. Record the exact successor SHA when fetching or publishing; ref publication requires a separate authorized step that does not silently start workflows.
- Scope is frozen. Do not add theoretical defenses, new architecture, new acceptance criteria, or a minimum number of fixes. Reopen source only for a concrete, reproducible blocker against this candidate or a clearly demonstrated compile/data-integrity/security defect. Preserve the user's explicit no-build/no-test/no-CI constraints until changed.

## 2. Final source inspection performed

The review was source-only and sampled the existing production handoff, not an exhaustive proof of the repository.

| Area | Source inspected | Disposition |
| --- | --- | --- |
| Rig model transport | `src-tauri/src/framework/ai/rig_transport.rs`; `framework/ai.rs`; exact upstream v0.43.0 source | HTTP trait methods, lazy-body types, OpenAI configuration/chat model, request builders, unary call and response fields compared; no confirmed signature mismatch in the inspected interfaces |
| Coding Rig / Native multi-turn | `agent-foundation/src/rig_driver.rs`, `engine.rs`, `src-tauri/src/capabilities/agent/coding.rs`, `execution_owner.rs` | Checked typed history, model/run/paired tool-result APIs, bounded session serialization, provisional final response, persisted approval and apply state. No new confirmed delivery blocker from this limited review |
| Independent tab ownership | `src-tauri/src/bridge/isolated_tabs.rs`, `grid_ipc.rs`, `grid_process.rs`, `grid_process_windows.rs`, `grid_process_unsupported.rs` interfaces | Linux/Windows create returns the bound receipt; platform selection and window route identity inspected. Kernel capability and installed behavior remain runtime questions |
| Desktop evidence | `src-tauri/src/native_visibility_review.rs`, `scripts/native-visibility-installed.py`, `docs/operations/native-visibility-installed-review.md` | Nine-stage manual fixture exists, includes terminal evidence freeze. No captures performed |
| Package/release | existing package and workflow configuration, installed fixture instructions | Existing gates retained. No installed binary, package hash or fresh test result produced |

Upstream reference: [Rig v0.43.0](https://github.com/0xPlaygrounds/rig/tree/654567eb64274fca00cab86cdd32c86b9913769e). Inspected `crates/rig-core/src/completion/request.rs`, `completion/message.rs`, `client/openai.rs`, `providers/openai/wire.rs`, `providers/openai/wire/chat.rs`, `driver.rs`, `crates/rig-http/src/http_client/mod.rs`, `http_client/erased.rs`, and `crates/rig-agent/src/run/mod.rs`. This resolves the sampled SDK API question at source level only; it does not establish compilation or network interoperability.

## 3. Fixed release acceptance criteria

Use the exact frozen revision for both platforms. Execute only when the user permits runtime validation, in a suitable disposable environment. Keep existing governance/quality gates; do not relax any gate.

| ID | Core goal | Required observable result | Evidence now |
| --- | --- | --- | --- |
| A1 | Rig production transport | With approved production routing enabled, a real configured HTTPS Chat Completions provider completes a text turn, a function-call turn and cancellation/error handling without duplicate local dispatch or exposed credentials | NOT_RUN |
| A2 | Coding multi-turn | Read a project file, propose a change, explicitly approve/apply, resume the same Task, receive final answer; rejected approval has no effect; recover existing persisted state | NOT_RUN |
| A3 | Independent browser process | Release launches with `default_isolated_kernel_ready` on each supported target with containment available; ordinary tabs open, switch, close and recover through correct native owner. No implicit backend migration | NOT_RUN |
| A4 | Native visible web content | All nine existing installed visibility fixture phases pass on Linux X11 and Windows, then normal user interactions confirm actual content, switching and absence of residual windows; fixture alone is not full GUI acceptance | GUI_PENDING |
| A5 | Windows/Linux installed package | Exact candidate compiles and builds under existing gates, installs/launches in disposable environments, normal core scenarios pass, exits cleanly; preserve source/package/executable evidence | NOT_RUN |

A release decision requires all five applicable criteria and existing mandatory gates on the actual candidate. Source freeze does not certify that no defects remain. Old CI/GUI results from different SHAs do not prove this revision, and no failure count is invented for it.

## 4. Explicit retained decisions and limits

- Rig is connected to actual production entrypoints **through configuration/experiment routing**, not forced globally. Native remains the existing fallback when Rig routing is not enabled. Do not enable production rollout silently. If unconditional Rig-only rollout is a mandatory product requirement, that is a separate explicit scope decision, not a passing item.
- Independent tabs default to isolated for release builds **only when the platform containment preflight succeeds**. Unsupported or non-delegated systems select the initial embedded compatibility backend. Explicit isolation does not silently migrate documents on failure. Linux cgroup delegation/Windows Job Object behavior still needs installed evidence.
- Supported transport is bounded unary OpenAI-compatible Chat Completions; arbitrary provider-specific reasoning/replay content, streaming parity and remote cancellation proof are not asserted.
- Existing Coding capacity limits and fail-closed missing transaction journal policy retained. Historic over-budget sessions or missing prepared journals may require manual recovery; no automatic truncation, replay or invented rollback evidence.
- Existing visibility fixture covers its disposable single-monitor desktop configuration, not Wayland, hardware scanout, every page/monitor arrangement or arbitrary user interaction.
- No additional speculative work is queued. The next legitimate phase is the fixed acceptance checklist, after permission. If acceptance is not authorized, handoff stays SOURCE_FROZEN / NOT_RUN rather than pretending release completion.

## 5. Stop rule

Once the frozen candidate satisfies A1-A5 and existing mandatory gates, close this remediation effort. Only a demonstrated blocker can reopen it. Improvements and expanded compatibility belong to a separate version/backlog. No quota of edits per round.
