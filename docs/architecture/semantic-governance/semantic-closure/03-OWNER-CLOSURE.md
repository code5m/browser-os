# 03 — Owner Closure Audit

> 检查：多 Owner 争用、越界调用（Component/Store 跨域直写）、Registry 与代码一致性。

## 1. 当前 Owner 注册（7 + 1 native）

| Owner | Owns | owner_only_api | Status |
|---|---|---|---|
| view_navigation (useLayoutStore) | mainView, gridToolbarOpen | setView/activate*/openModule | ACCEPTED_ADR |
| browser_grid_lifecycle (useBrowserStore) | gridOpen, grid lifecycle, derived visibility | buildGrid/closeGridAll/closeGridOne | ACCEPTED_ADR |
| native_execution (bridge/Rust) | bounds/visibility/webview position | gridPosition/tabPosition/hide*/createGrid/closeGrid | ACCEPTED_ADR |
| browser_tabs (useBrowserStore) | activeTabId, tabs, url | tabSwitch/closeTabNow/tabNew | CURRENT_FACT |
| workspace_filepanel (useWorkspaceStore) | filePath/inlineFile/previewDir/pathInput/currentLocalPath | enterDir/openFile*/saveFile/closeFileEditor/... | CURRENT_FACT |
| bookmark (useBookmarkStore) | items/loaded/busy/error/panelOpen/sorted | add/remove/toggle/importFile/togglePanel/load | CURRENT_FACT |
| terminal (useSystemStore) | terminalOpen/termPanes/termGrid/termGridCount/activeTermId/.../m0* | addTermPane/killTerm/termWrite/setActiveTerm/... | CURRENT_FACT |
| credential (KeyringStore) | gitRepoToken/dbCredential/browserCredential + keyring lifecycle | save_token/get_token/delete_token（前端禁触） | CURRENT_FACT（R3 已通用化） |

## 2. 多 Owner 争用检查

| ID | 资源 | 涉及 owner | 是否冲突 | 结论 |
|---|---|---|---|---|
| O-01 | `closeGrid` | browser_grid_lifecycle（intent）/ native_execution（native primitive） | 否 | 分层：store 持生命周期意图，bridge 持 native destroy 原语，单向调用，非争用 |
| O-02 | `mainView` vs `terminalOpen` | view_navigation / terminal | 否 | 导航到 term 表面（mainView）与终端面板开合（terminalOpen）是两层，无争用 |
| O-03 | `gridSession` (useBrowserStore:23) | 前端 epoch vs Rust webview 生命周期 | **边界含糊** | 计数在 store，生命周期在 Rust；建议明确为"前端镜像 epoch"，owner 维持 useBrowserStore 但注明生命周期真源在 Rust（见 01-S-18） |

> 范围内**无真正的多 Owner 争用**。O-01 是刻意分层，非冲突。

## 3. 越界调用检查（代码对齐）

- **Component → 生命周期原语**：R3 / R2 在 `governed_files` 内拦截（`COMPONENT_WRITES_MAINVIEW`、
  `COMPONENT_CALLS_LIFECYCLE_PRIMITIVE`、`COMPONENT_WRITES_TERMINAL` 等）。Phase 5.1 真实扫描 fail=0，
  代码无 Component 直写 gridOpen/mainView/termPanes/activeTermId/filePath/items。
- **前端 → KeyringStore 原语**：Phase 5.1-A R3 通用化后，前端任何文件直调 `save_token/get_token/delete_token`
  或引用 `KeyringStore` 即失败。真实扫描 0 命中（DatabasePanel 仅 `db.connect(password.value)` + 清空）。
- **跨 store 直写**：Registry 的 `forbidden_callers` 已声明（如 useBrowserStore 不得直写 mainView）。
  代码未见违规（self-test + real scan 通过）。

> 范围内越界调用 **CLOSED**（有 Registry + R3/R2 机器约束，真实扫描通过）。

## 4. 范围外 — 无 Owner（最大风险）

| 域 | 资源/动作 | 应有 Owner | 现状 |
|---|---|---|---|
| Git | status/branches/diff/repoId；git_write 动作 | 无 | 无 owner 约束 |
| Agent/Skill | skills/agents/sessions；install/run 动作 | 无 | 无 owner 约束 |
| Task | tasks/runs；task_run 动作 | 无 | 无 owner 约束 |
| Database | connections/configs/schema；db_connect/query | 无 | 无 owner 约束 |
| Graph | nodes/edges | 无 | 无 owner 约束 |
| Session/Vault/Image/Plugin/Resource/Settings/Home/GridArchive/Workbench | 各自 | 无 | 无 owner 约束 |

> 这些域的任何 Component 直写、跨 store 越界、重复入口，现有 R1–R7 **完全不拦截**（R2 仅 `governed_files`、
> R3 仅 3 个 owner）。这是 Owner Closure 的最大开放缺口。

## 5. 小模型风险

| DUP-ID | Concept | WHY_SMALL_MODEL_MAY_MISUNDERSTAND | POSSIBLE_WRONG_CHANGE | PREVENTION |
|---|---|---|---|---|
| DUP-201 | closeGrid 双 owner 登记 | 两个 owner 都列了 closeGrid | 误以为重复登记要删一个 | 保留分层（intent vs native primitive） |
| DUP-202 | 凭据 owner | "credential" owner 是 Rust KeyringStore，前端看似无 owner | 在前端 store 加 password 字段 | R3/R7 护栏 + states sensitive |
| DUP-203 | 终端面板 | termPanes/activeTermId 在 useSystemStore | 在 TerminalPane.vue 直写面板列表 | R3 COMPONENT_WRITES_TERMINAL |
| DUP-204 | 范围外域 | git/agent 无 owner | 模型在组件里直接调 git/agent 命令并自管状态 | 需扩展 R3（见 08） |
