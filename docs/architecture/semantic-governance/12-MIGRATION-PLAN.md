# 12 · Migration Plan — 增量语义收敛路线图
> Chief（只读审计合成）· 基于 `01–11` 与 `16-REVIEW-REPORT.md`
> 原则：**严禁大重构**。每阶段只收敛一组语义，范围小、兼容可控、可自动测、可加 checker、可回滚。
> 不给出工期估算。阶段顺序遵循依赖关系，非优先级排名。

---

## Phase 0 — Evidence / Contracts / Tests / Checker（地基）
**Goal**: 把本轮审计成果固化为可机器执行的契约与看门狗，使后续迁移可在门禁保护下进行。
**Why now**: 现有检查器已互相矛盾（16 CLAIM-XC-01）且 IPC 门禁有正则盲区（RULE-005），不先修门禁，迁移会引入新漂移。
**Allowed scope**: `scripts/**`（仅改 checker 正则/断言）、新增 `scripts/check-*.{mjs,py}`、在 `docs/architecture/semantic-governance/**` 记录。
**Forbidden scope**: 任何 `src/`、`src-tauri/`、`package.json`、`Cargo.toml`、`AGENTS.md` 业务代码改动。
**Canonical contract**: `10-TARGET-SEMANTIC-CONTRACTS.md` 全文作为 Phase 1+ 的权威引用。
**Compatibility requirements**: 仅修 checker，不改变运行时行为；`check-command-set-consistency.py` 修改后须保持 GATE 通过（扩大正则不缩小通过集）。
**Tests**: 现有 `pre-merge.sh` 全门禁仍 PASS；新增 checker 自带 `--self-test`。
**Checker**: RULE-002（修 G2 漂移）、RULE-005（typed-invoke 正则 + 5 命令入 KNOWN）、RULE-012（删 `layout.aiNavOpen` 死重复，需先在源码移除——注意此条触及 `src/`，列入 Phase 1 非 Phase 0）、统一 SessionCloseDialog 三方口径。
**Runtime acceptance**: 不适用（无运行时变更）。
**Rollback boundary**: 仅脚本改动，`git revert` 单文件即可。
**Dependencies**: 无。
**Non-goals**: 业务代码收敛；语义重命名。

---

## Phase 1 — Browser & Grid semantics（第一批收敛）
**Goal**: 收敛"宫格可见/生命周期"与"离开宫格"意图分叉（最危险的 S4，16 CLAIM-S4-02）。
**Why now**: 满足 `RECOMMENDED_FIRST_BATCH` 全部六条件（真实歧义已证实 + 故障路径高概率 + 修改范围小 + 生命周期边界明确 + 兼容可控 + 可测可看门）。
**Allowed scope**: 引入 `exitGrid(mode:'hide'|'destroy')` 意图（`useBrowserStore`）；组件手拼 buildGrid/closeGridAll 收敛到此意图（RULE-004）；`isBrowserVisible` 公式守 `mainView==='browser'`（RULE-002）；`gridOpen` 仅 buildGrid/closeGridAll 赋值（RULE-001）。
**Forbidden scope**: 新增第三个"grid visible"标志；把 gridOpen 当可见短路；组件直接调 bridge 原生命令（RULE-006/009）。
**Canonical contract**: CONTRACT-GRID-LIFECYCLE、CONTRACT-BROWSER-VISIBILITY、CONTRACT-GRID-EXIT、CONTRACT-VIEW-SWITCH。
**Compatibility requirements**: `mainView` 仍由 `setView` 单一写入；`isBrowserVisible` 公式不变（仅守门）；外部裸 `layout.mainView=` 在 RULE-003 看门下逐步收敛（FileEditor.vue:11、TopBar.vue:21 等白名单过渡）。
**Tests**: `13-ACCEPTANCE-MATRIX.md` 的 Browser↔Grid 四层验收；新增 `scripts/check-grid-exit-intent.mjs`（RULE-004）、`scripts/check-view-switch.mjs`（RULE-003）。
**Checker**: RULE-001/002/003/004/006/009/013。
**Runtime acceptance**: browser→grid→browser 往返 ≥20 次、resize/maximize/restart 后不 recreate/reload/lose active URL（见 13）。
**Rollback boundary**: 仅前端 store/组件改动；无 Rust 契约变更（create_grid 契约已 CONSISTENT，不动）。
**Dependencies**: Phase 0（门禁就位）。
**Non-goals**: 改名 gridOpen→gridInstanceAlive（重命名属后续可选，非必须）。

---

## Phase 2 — Bookmark / UI state
**Goal**: 收敛收藏夹高亮/挂载不一致（SEM-006）、`currentLocalPath` 派生重算（SEM-010）。
**Why now**: 风险 S2/S3，修改范围小，纯前端派生收敛。
**Allowed scope**: 统一用 `bmPanelOpen` 派生做高亮与挂载（ActivityBar 改用派生）；`currentLocalPath` 收敛为单一写入入口或显式字段。
**Forbidden scope**: 新增 UI 私存 boolean 副本。
**Canonical contract**: CONTRACT-VIEW-SWITCH（派生归一处）。
**Compatibility requirements**: 非 browser 视图下收藏夹点击应"先切回 browser 再开"（已有 ActivityBar.onToggleBookmarkPanel 逻辑，仅统一判据）。
**Tests**: 四层验收（非 browser 视图点收藏夹 → 高亮且挂载一致）。
**Checker**: 现有 `check-ui.mjs` 派生一致性断言增强。
**Rollback boundary**: 纯前端。
**Dependencies**: Phase 0。
**Non-goals**: 收藏夹数据模型改造。

---

## Phase 3 — Workspace / FilePanel
**Goal**: 收敛 `currentLocalPath` 四级 fallback（SEM-010）、破坏型 FS 命令仅经 store（RULE-010）。
**Why now**: S2，数据丢失风险需看门。
**Allowed scope**: `useWorkspaceStore` 单一入口封装 FS 写/删/改名；`revealPath` 只读豁免。
**Forbidden scope**: 组件直连 `bridge.writeFile/deletePath/renamePath/movePath/createFile`（RULE-010）。
**Canonical contract**: CONTRACT-FS（RULE-010 隐含）。
**Tests**: FS 破坏性操作四层验收（含 undo/回收站语义）；checker 门禁。
**Checker**: 新建 `scripts/check-fs-intent.mjs`（RULE-010）。
**Rollback boundary**: 纯前端。
**Dependencies**: Phase 0。
**Non-goals**: 文件系统后端重构。

---

## Phase 4 — Terminal
**Goal**: 收敛终端生命周期多 owner（terminal spawn/kill 在 `useSystemStore` 与 `TerminalPane` 双出口），核实 resize 去重归属（FEAT-008）。
**Why now**: S3，PTY 存活于视图切换已是正确行为，但 kill 入口分散。
**Allowed scope**: 所有 spawn/kill 经 `useSystemStore`；`TerminalPane` 仅渲染。
**Forbidden scope**: 组件直接 `system.killTerm`（RULE 待补）。
**Canonical contract**: CONTRACT-TERMINAL（归纳为单 owner）。
**Tests**: 视图切换后 PTY 存活、回放；restart/kill 行为四层验收。
**Checker**: 新增 `scripts/check-terminal-intent.mjs`（建议）。
**Rollback boundary**: 纯前端 + 既有 `term_kill`/`terminate_session` 不变。
**Dependencies**: Phase 0。
**Non-goals**: PTY 后端重构。

---

## Phase 5 — Credentials
**Goal**: 收敛凭据无 store owner（STORE_OWNERSHIP_LEAK + NATIVE_POLICY_LEAK）为 `useCredentialStore` facade；守密码红线（RULE-011）。
**Why now**: S3，安全域与 UI 域混用（FEAT-009）。
**Allowed scope**: 新 `useCredentialStore` facade 封装 list/fill/import；`CredentialList.vue` 经 facade。
**Forbidden scope**: 前端持有明文 password / 写 Pinia ref / 进 localStorage / 进日志（RULE-011 B）。
**Canonical contract**: CONTRACT-CREDENTIAL-OWNER。
**Tests**: 凭据填充 exact-origin 匹配；导入/填充四层验收；密码不进前端断言。
**Checker**: 新建 `scripts/check-credential-owner.mjs`（RULE-011）。
**Rollback boundary**: 新增 facade + 组件改调 facade；Rust keyring 不变。
**Dependencies**: Phase 0。
**Non-goals**: keyring 后端改造。

---

## Phase 6 — Optional deeper refactor
**Goal**: 仅在 Phase 1–5 门禁稳定后，考虑 `gridOpen`→`gridInstanceAlive` 重命名、`exitGrid` 抽取独立 hide 原语（补 RULE-013 的 hide 语义）、`sync_browser_scene` 真正落地为单一场景适配器（CONTRACT-NATIVE-SHOW）。
**Why now**: 可选；消除架构性耦合（close==kill==destroy、position==show、native 双层）。
**Allowed scope**: 重命名、新增 hide 原语、适配器落地。
**Forbidden scope**: 改变"视图切换不销毁"红线；引入 `set_visible(false)`（WebKitGTK 死锁，bridge.rs:567-569）。
**Canonical contract**: 全部 CONTRACT-*。
**Tests**: 全量四层验收 + 现有 `pre-merge.sh` 门禁。
**Checker**: 所有 RULE-001~013 全绿。
**Rollback boundary**: 需 Rust 契约评审；单 PR 小步。
**Dependencies**: Phase 1–5。
**Non-goals**: 重写浏览器内核 / 宫格子进程模型。
