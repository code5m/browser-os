# BrowserOS / 8-file audit closure matrix — 2026-11-03

Integration branch: fix/master-audit-closure-20261103 (base master@791c91b8).
Original user-provided materials: 8 untracked files in 需要解决的问题/ and the independent .bevel/ directory. Preserve them unchanged; never stage, delete or overwrite them.
Review policy: one integration branch and one PR, never 8 approvals. CODE_PASS, PACKAGE_PASS and GUI_PASS are separate verdicts.

## Eight original sources (historical code SHA must not be treated as the current state)

| Original filename | Scope |
| --- | --- |
| # BrowserOS 最新 Master 架构复审 | BIGINT, Coding apply recovery, Capability disable, Agent Runtime, native performance, physical owners |
| # BrowserOS 最新 master：第三轮异步架构审查 | Hot-Plug state divergence, native synchronous Grid, shutdown, eval_in_tab, Terminal, DB connect |
| ## BrowserOS 最新 master 重新审查 | Production Task/Session/Cancel, event-retirement memory, Rig terminal states, approval identity |
| ## BrowserOS 最新 master：第三轮高耦合审查 | Document Manager, tab registry, Browser/Grid resources, UI module boundaries |
| ## 一、最新 master 审查结果 | Workspace file identity/save/CAS, inline drafts, editor overlap, Monaco worker |
| Untitled-1 | Five database exports, i64 DTO, lifecycle, Database Studio interactions |
| 处理进度.md | Old 911a164f baseline and historical statuses; do not mistake for latest master |
| 我再按最新远程 master 做一次代码审查，不沿用上次“CI 通过”的结论。这 | PR #54 data projection regressions, binary lossless export, pagination, clipboard, preview |

## De-duplicated action register

| ID | Status | Evidence and remaining acceptance |
| --- | --- | --- |
| LIFE-01 Manager stop | CODE_PASS in this branch | manager.ts now awaits runtime.releaseAsync; PLT2-22i reproduced RED then GREEN |
| LIFE-02 Hot-Plug disable/remove | CODE_PASS in this branch | hotplug.ts and orchestrator.ts await Owner receipt; PLT2-14g-q test asynchronous failures, no premature DISABLED, add/disable/enable/remove |
| LIFE-03 Resource Governor fallback | CODE_PASS in this branch | resourceGovernor.ts awaits releaseAsync; PLT2-14j-k |
| AGENT-01 Rig oversized result | CODE_PASS in this branch | rig_driver.rs validates output before success and records Failed; experimental-rig-run test |
| AGENT-02 retired Task memory | CODE_PASS in this branch | event.rs and task.rs bound retained terminal history to 256 retired tasks; recent replay retained |
| DATA-01 DB large integers | Prior partial CODE_PASS, GUI_PENDING | master contains bigint-safe normalization and MySQL/PostgreSQL E2E; exact clipboard/download in packaged GUI unverified |
| DATA-02 Workspace save/recovery | Partial CODE_PASS, GUI_PENDING | Existing editorFilePath/encrypted journal/native advisory CAS plus this branch's cross-workbench identity, shared dirty claims, save admission and directory mutation guard; noncooperative external editors remain outside OS-level CAS guarantees |
| DATA-03 Coding multiple-file apply | Prior partial CODE_PASS, GUI_PENDING | durable journal plus fault rollback and crash detection; real external side-effect reconciliation pending |
| DATA-04 DB CSV/TSV/JSON/SQL/Markdown | Prior CODE_PASS, GUI_PENDING | pure function and component contracts, actual clipboard/download still not accepted |
| DATA-05 binary lossless export | NOT IMPLEMENTED | current API contains placeholder byte count, intentionally blocks invalid export; needs explicitly designed byte stream/download contract |
| AGENT-03 unified Task/Session/Cancel across products | NOT COMPLETE | shared in-memory TaskRegistry is not durable authoritative Coding session/approval/recovery |
| AGENT-04 Host identity and effect audit | NOT COMPLETE | real principal, workspace revision, revocation and external side-effect outcomes not end-to-end confirmed |
| AGENT-05 legacy agent_run/agent_install | NOT COMPLETE | intentionally restricted behind AGENT_SKILL_COMMANDS_AVAILABLE=false |
| NATIVE-01 Grid Moved/Resized | BLOCKED by native acceptance | main.rs still invokes reposition_visible synchronously; native event changes require true GUI regression under locked project rules |
| NATIVE-02 window shutdown | BLOCKED by native acceptance | main.rs still synchronously calls shutdown on CloseRequested/ExitRequested; native resource release evidence missing |
| NATIVE-03 sync eval_in_tab/Terminal | BLOCKED by protected bridge.rs |  protected source; do not modify without explicit native risk approval and real desktop evidence |
| NATIVE-04 real side effects of 58 commands | GUI_PENDING | static source/origin and permission inventory PASS; real effects and remote-webview refusal not demonstrated |
| UI-01 universal Document Manager | PARTIAL, CODE_PASS for shared identity arbitration | New src/application/documentOwnership.mjs coordinates absolute paths, dirty owners, in-flight saves and directory mutations across Workspace overlay/inline and Coding Workbench; each still owns its own text/model/saved hash, so no universal single canonical Document Manager yet |
| UI-02 browser/module Tab Registry | PARTIAL, logical projection CODE_PASS | New src/application/tabResourceProjection.mjs renders one typed/owner-bound tab list in UnifiedTabBar.vue; WebView lifecycle still belongs exclusively to browser Owner, module resources belong to layout Owner (as required by locked 1 tab = 1 WebView policy). No native pool or physical resource unification. |
| UI-03 Database and Git dock/drag | CODE_PASS in simulator; GUI_PENDING | formal studio-folding/database-idea-dock tests pass; installed WebView geometry unverified |
| UI-04 Ctrl+N focused native WebView | CODE_PASS in shell fixtures; GUI_PENDING | JS/Monaco/terminal focus guards exist; native child-webview real events must be checked |
| WECHAT-01 official account draft-only | CODE_PASS with mock transport; REAL ACCOUNT PENDING | keyring, media, draft/get flows exist; no real account material/image/draft verification; no publish/mass-send |
| VERIFY-01 packaged real GUI, PID/RSS/PSS/FD/Grid p95 | GUI_PENDING | same SHA release install, cold-start and repeated resource cleanup evidence required |
| VERIFY-02 final branch checks | IN PROGRESS | npm run check, 607 Rust tests PASS, production build and full pre-merge require final SHA |

## Evidence for this branch

- check-capability-platform.mjs: 63 PASS / 0 FAIL including added tests.
- check-hot-plug-acceptance.mjs: PASS.
- capability-demo.mjs runtime-demo bookmark: REGISTER/ACTIVE/DISABLE/ENABLE/UNREGISTER PASS.
- check-capability-runtime.mjs: 31/31 PASS.
- check-pluggable-lifecycle.mjs: PASS.
- agent-foundation experimental-rig-run Rust tests: 29 unit tests + 4 contract tests PASS.
- npm run check: PASS (current branch worktree as checked).
- cargo check/test + production build/pre-merge: record separately when completed.

Code-only success is never actual GUI acceptance. Untracked original files are excluded from Git.

## Regression note after the initial matrix

PLT2-14q reproduced failed Hot-Plug reactivation that left the production Runtime marked enabled while the Platform was DISABLED. Owner activation rollback now disables Runtime metadata only **after** Owner rollback completes; an unconfirmed cleanup instead leaves a visible non-executable SUSPENDED diagnostic state. Its test went RED then GREEN. The correct current baseline for the combined Capability gate is 63 PASS / 0 FAIL. A successful automated gate never certifies native WebView or packaged GUI behavior.

## 验证追加说明（此分支提交前）

- Rust 全量：607 PASS / 0 FAIL / 18 条条件跳过；`cargo check --locked` PASS，两条既有未使用函数告警。
- Agent Foundation `experimental-rig-run`：29 个单元测试、4 个契约测试 PASS；覆盖输出超限终态、短任务日志回收。
- `npm run check`：PASS；Capability Platform 63 PASS / 0 FAIL；Hot-Plug Acceptance PASS。
- 生产 Vite 构建、最终 SHA 的 Pre-merge 和 GitHub PR 检查须在本次提交后独立核验。

## Follow-on implementation on the SAME branch (2026-11-03)

- Cross-workbench document ownership layer is a single process-local metadata coordinator, not a content database. Workspace overlay/inline and Coding Workbench publish active dirty documents into it, with synchronous update semantics.
- Coding save, multi-file Agent apply and Workspace CAS save now reserve the same absolute path. An independently dirty editor, concurrent save, or in-progress save under a directory rejects mutation before any irreversible native operation. Native saved-hash/CAS remains the final authority for competing processes; no unsafe bypass has been added.
- UnifiedTabBar.vue now renders one read-only projection carrying logical tab owner, type, path and native resource ID (only web tabs have actual WebView IDs). Close/activate calls retain the original Browser/Module Owners and do not alter WebView creation/hide/destroy or ordinary tab-close policy.
- npm run check:module-contracts now includes all cross-editor and tab resource tests; module contracts 20/20 PASS, CROSS_WORKBENCH_DOCUMENT_OWNERSHIP=PASS, CROSS_WORKBENCH_FILE_STORE=PASS, UNIFIED_TAB_RESOURCE_PROJECTION=PASS.
- npm run check: PASS; npm run check:coding-ui: PASS for real Chrome/Monaco with mocked file/model/native transport, not installed GUI acceptance.
- Remaining nontrivial gaps: physical unified resource ownership (would cross locked native/WebView interfaces), durable production Task/Session/Approval authority (requires approved key encryption/identity and recovery), lossless binary database transport (current Tauri API only exposes binary lengths), native blocking/GTK shutdown (locked native boundary), real WeChat account and installed GUI resource evidence. They cannot be marked complete without actual authority/evidence and required contract approval.

## Latest build/evidence status after document and tab projection

- Source tests: npm run check PASS, including 20 module contracts and the three new cross-workbench/tab tests; npm run check:coding-ui PASS with real Chrome and Monaco, simulated backend transport.
- Full local Vite build **NOT PASS** on this worktree. A 2-GiB V8 heap scope was terminated deliberately by the operator before a system-wide OOM because memory and swap were exhausted by concurrent desktop programs. The subsequent 1.5-GiB V8 heap limit failed cleanly with V8 heap exhausted during transform. These are resource constraints, not a test of production correctness. Do NOT inherit previous SHA's build or Pre-merge PASS.
- The project requires at least a clean production build and full same-SHA Pre-merge gate for merging. Real installed GUI remains GUI_PENDING; no release install, screen capture or WeChat credential verification was performed.
- One intentionally missing guarantee remains: metadata-only document ownership does not replace separate content buffers/Monaco models, and only coordinates writes **within this process**. Native CAS still provides cross-process validation.

## WeChat draft retry guard (subsequent work)

- Draft creation now records a non-secret pending marker in local browser storage before sending; an interrupted or unsuccessful request remains blocked across page reload until the user explicitly confirms that the official draft box contains no duplicate. If local storage cannot be read or written, draft creation remains blocked. No account credentials or article contents are stored in this marker.
- A returned draft ID with failed detail verification also retains the pending marker and prevents another submission until verification succeeds or the user reconciles externally. Its media ID is stored locally alongside the marker so verification can resume after reload; article content and account credentials are not stored there. The Chrome/Vue fixture simulates the transport and checks unknown results, reload, no automatic resubmission, explicit reconciliation, storage-write failure and returned-but-unverified drafts. It does not establish real WeChat account behavior, native GUI acceptance, or resilience to manually cleared browser storage.
- Explicit native errors known to precede `draft/add` (input validation, missing or invalid account, HTTP client setup, token network/missing token) clear the pending marker so the user may correct the issue. Generic provider/response and network errors remain conservative because the failing request stage is not distinguishable. The fixture covers the input-validation case.
- Starting a new creation clears the previous result identity before the native request. A regression reproduces a first verified draft followed by a second unknown outcome and checks that the old ID cannot be verified to unlock the new submission.
- Negative WeChat API error numbers are preserved in sanitized UI status (for example `WECHAT_API_-1`), while provider-supplied detail remains hidden. A Chrome/Vue mock regression failed before the fix and passed afterwards.
- Latest frontend build attempt for these uncommitted changes: `systemd-run --user --scope -p MemoryMax=3500M env NODE_OPTIONS=--max-old-space-size=2048 npm run build` failed with exit 143; systemd reported `Result: oom-kill` for the isolated scope. Build and final pre-merge remain pending, not passed. The fixture, workspace startup, image lazy-loading and `git diff --check` passed; WeChat real-account and installed GUI validation remain pending.
- Subsequent build on the same source worktree: 4.5 GiB isolated scope failed with `Result: oom-kill`. After available memory rose, `systemd-run --user --scope -p MemoryMax=6G -p MemorySwapMax=0 env NODE_OPTIONS=--max-old-space-size=2048 npm run build` completed successfully (3,616 modules, exit 0). This establishes a frontend build for the uncommitted worktree only; final same-commit pre-merge, independent budget verification, real-account and GUI acceptance remain pending.
- The WeChat browser fixture is now part of the managed coding UI regression runner and uses its dynamically allocated local server. The complete managed UI suite passed with real Chrome/Vue/Monaco and simulated native/provider transport; engineering-governance check also passed. This does not supersede the failed production build or establish real-account/installed GUI acceptance.

## Workbench fold restoration (subsequent work)

- Global tool folding now stores the prior sidebar, browser dock and AI navigation visibility alongside the collapsed preference, and restores that snapshot after recreating the store. Legacy saved preferences retain their default fallback; malformed snapshots are rejected. The store-level regression first reproduced incorrect restoration, then passed and is included in `check:studio-folding`. It does not prove native WebView placement or installed-desktop behavior.

## Document identity input guard (subsequent work)

- Relative document paths now require an explicit absolute project root; the previous empty-root conversion incorrectly treated `relative.ts` as `/relative.ts`. The cross-workbench ownership regression reproduced that input failure before the fix, and the ownership and file-store fixture tests pass afterwards. This input guard does not resolve symlink aliases or provide cross-process atomicity.
- Coding Workbench new-file input now rejects empty, dot, parent, backslash and NUL path components so alternative spellings cannot create separate buffers for the same lexical path. The real Vue/Monaco browser fixture reproduced dot-segment acceptance before the change and passed afterwards. Symbolic-link aliases and native filesystem behavior are outside this fixture.
- The new-file validation was changed after the earlier successful frontend build and budget audit. Those results do not cover the latest source; production build, budget verification and final pre-merge remain pending for the final code state.
- Workspace deletion now preserves overlay and inline text edited while the native delete request is in flight, retains the original path, and warns that the disk item was deleted and the draft needs copying or saving elsewhere. A deferred mock-delete fixture reproduced loss before the change and passed for both editors afterwards. This is recovery of in-memory edits after deletion, not prevention of the deletion, cross-process locking, or proof of journal persistence after an immediate crash. The source changed after the earlier build and requires final build/gate revalidation.
- Workspace delete, rename and move now reserve their source/destination paths in the process-local document coordinator until the request finishes; overlapping editor saves and mutations are rejected, and locks are released on failure. Tests cover directory ancestry, sibling paths, active saves, dirty buffers, a deferred delete racing a save, failed-delete release, source and destination blocking during rename, and failed-move release. This coordinates BrowserOS editors within one process only; external applications, symlink aliases, native atomicity and installed GUI remain unverified.
- If a document becomes dirty while rename or move is already in flight, the UI now warns the user to check the old and new paths before saving. Deferred simulated rename/move tests cover these warnings; they do not retarget Coding Workbench buffers or prove crash recovery.

## Agent Foundation task-signal ordering (subsequent work)

- `TaskRegistry` previously exposed a new task status before replacing or signalling its cancellation channel, permitting a pause/resume or start/cancel race to affect the wrong generation. Start, pause, cancel and finish now update the corresponding signal while holding the task-state lock, in task-then-signal order.
- A 500-iteration threaded pause/resume regression checks that the old generation is cancelled and the resumed generation is not; final cancellation is also checked. Default Foundation tests passed (24 unit and 4 contract), `durable-sqlite` tests passed (29 unit, 4 contract, 4 fault-injection), and `cargo check --manifest-path src-tauri/Cargo.toml --locked` passed with two existing unused-function warnings. This does not unify production Chat/Coding owners, establish persisted approval identity, or validate real provider cancellation.
- `AgentEngine::drive_turn` now listens for cancellation while opening a provider stream, not only after the stream opens. A hanging-provider regression timed out before the fix and passes after it; the durable Foundation suite passes (30 unit, 4 contract, 4 fault-injection). Dropping the local provider future does not prove a remote server stopped processing, and no real provider was contacted.
- Foundation `AgentEngine` progress, tool proposal and terminal transition now validate the original task revision while holding the task-state lock. A stale generation after pause/resume cannot record progress, request approval or complete the newer generation; the durable suite passes (31 unit, 4 contract, 4 fault-injection). This change covers `AgentEngine` only: audit the experimental Rig driver and tool-result event path separately for equivalent check-then-act races before claiming AGENT-03 complete.
- Follow-up Rig and tool path: Rig model completions/proposals now commit only at the drive's captured revision; pending tool results are bound to task ID, call ID and the post-approval revision, and applying a host result holds the task-state lock across revision validation and Rig-state update. Tool-result events also validate the active revision. A wrong-task result regression failed before the change and passes afterwards; a stale-generation result is rejected without consuming the pending operation. Experimental Rig tests pass (33 unit, 4 contract), durable SQLite tests pass (31 unit, 4 contract, 4 fault-injection), and BrowserOS Rust check passes with two existing warnings. The method still trusts the host to supply the actual executed result; unified production owners, real provider behavior, external-effect reconciliation and installed GUI remain unverified.
- Task transitions now publish their committed events before releasing the registry lock, preventing concurrent transitions in the same registry from publishing in a different order than their state changes. The 500-iteration pause/resume test checks event transition continuity; durable SQLite (31 unit, 4 contract, 4 fault-injection) and experimental Rig (33 unit, 4 contract) suites pass. This does not establish ordering between independent processes or prove real-provider behavior.

## Workspace move-path containment defect (new finding)

- `src-tauri/src/capabilities/workspace/fs_cmds.rs` checks `src_canon.starts_with(&dst_canon)` while claiming to reject a destination inside the source. This is reversed: a source inside an ancestor destination is rejected, while a destination inside the source does not match this guard. Fixing the native `move_path` command requires the explicit native-code authorization required by `src-tauri/AGENTS.md`; no command signature change is necessary. Cover same path, child destination, parent destination, sibling destination, and same-name collision with real temporary directories, then run the Rust and applicable file-workspace regression gates. Existing `rename`/OS failures are not proof of the intended guard.

## Focused rechecks on the current worktree

- After adding a WeChat cover-path/account-change regression and invalidation behavior, `npm run check:coding-ui:managed` stopped in the first Coding Workbench fixture: unexpected page errors `CODING_EDITOR_DISPOSED`. The WeChat fixture was not reached; root cause is not yet established. The preceding frontend build and budget audit predate this WeChat change and do not verify the final source.
- After the latest frontend changes, a fresh production `npm run build`, `check-workspace-startup.mjs` and `check-image-preview-lazy-loading.mjs` passed. Vite reported large chunks and mixed static/dynamic imports; build success is not packaged-client or GUI acceptance.
- The new build's independent SHA-256 inventory verification passed for 171 files, and its configured partitioned budget passed (total 13,071,376 bytes; editor-exclusive 11,532,663; core/shared 1,538,713). This does not imply the historical 45% comparison passed or validate installed size or runtime memory.
- `check-quick-create-shortcuts.mjs` and `check-file-quick-create.mjs` passed for shell shortcut routing and mocked file creation. These do not exercise keyboard delivery inside the native child WebView or an installed client.
- `npm run check:module-contracts` passed all 20 capability contracts, tab projection, document identity and mocked cross-workbench file-store checks with the current uncommitted changes. These results do not replace native filesystem or full build acceptance.
- After the 6 GiB-scoped frontend build, `npm run check` passed on this uncommitted worktree, including `check:studio-folding` and its new restoration test; the managed Chrome/Vue suite including the WeChat fixture had passed separately. Native command effects, real account, installed GUI, approved byte-budget verification and same-commit full pre-merge remain unverified.
- Independent `report-coding-editor-budget.mjs --verify-dist --enforce-budget` subsequently passed on that build: all 171 emitted files matched the independent SHA-256 inventory and the configured partitioned budget passed. This is not a same-commit full pre-merge result, installed package/GUI validation, or real account validation.

## Handoff checkpoint — 2026-11-04

### 本阶段已完成

- 微信公众号素材/草稿流程已具备：系统密钥库配置、封面永久素材上传、正文图片上传与插入、草稿创建、草稿详情核验；创建前持久化不确定结果标记，禁止不明结果自动重试。
- 草稿不确定结果支持刷新后恢复 media_id、API 详情核验和人工核实解锁；凭据、正文和图片内容不写入浏览器本地标记。
- Ctrl+N 快捷创建、工作区文件创建和工具栏/侧栏折叠状态持久化已有代码与模拟回归；跨工作区文档保存、删除、重命名、移动的进程内冲突保护已有实现。
- Agent Foundation 任务取消、版本代际校验、Rig 工具结果校验和同任务事件发布顺序已有实现；Rust 单元/契约/故障注入测试及 `cargo check` 通过。
- 最新生产前端构建、工作区启动检查、图片懒加载检查、产物哈希清单和批准的独立编辑器预算均有通过证据。

### 交接前的阻断状态

- 最后一次 `npm run check:coding-ui:managed` 在首个 Coding Workbench fixture 失败，页面出现 `CODING_EDITOR_DISPOSED`；公众号 fixture 因此未执行。失败根因尚未确认，不能宣称最终交互通过。
- `src-tauri/src/capabilities/workspace/fs_cmds.rs` 的 `move_path` 子目录 containment 判断方向疑似反转；修改受原生目录授权规则约束，尚未改动。
- 真实微信公众号账号、真实草稿箱/图片、安装包、桌面 GUI、原生子 WebView 键盘事件均未验收。
- 完整同一工作树 pre-merge 尚未重新通过；当前工作树仍有已修改和未跟踪文件，未提交、未合并、未推送。

### 建议后续 Agent 顺序

1. 先定位并修复 `CODING_EDITOR_DISPOSED`，最小化重跑 `npm run check:coding-ui:managed`，确认公众号 fixture 实际执行。
2. 在获得明确原生授权后修复 `move_path` containment，并运行 Rust 与文件工作区回归。
3. 重新运行受影响的前端检查、Rust 检查和最终 pre-merge；不要用旧构建结果覆盖新源代码。
4. 获得真实公众号凭据和桌面验收条件后，单独记录真实草稿/图片结果；模拟测试不能替代账号验收。
5. 最后再决定是否提交、合并、推送或安装；这些动作不应从本交接节点自动推断授权。

### 本节点结论

`HANDOFF_READY_WITH_BLOCKERS`：已完成的实现与证据已记录，剩余工作和阻断项已明确；不表示目标完成，也不表示 GUI 或真实公众号验收通过。
