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
| DATA-02 Workspace save/recovery | Prior partial CODE_PASS, GUI_PENDING | editorFilePath and encrypted journal plus advisory path lock, but NOT unconditional atomic CAS against noncooperating external writers |
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
| UI-01 universal Document Manager | NOT COMPLETE | separate document stores/versions still exist |
| UI-02 browser/module Tab Registry | NOT COMPLETE | logical UI projection is not unified underlying WebView/resource ownership |
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
