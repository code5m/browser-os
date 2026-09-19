# 02 — Intent Closure Audit

> 一个用户意图 = 一个 canonical 入口。区分「同一用户意图」vs「不同生命周期动作」。
> 对照 `intents.yaml`（含 `rejected_intents`）与代码入口（store action / bridge wrapper / Tauri command）。

## 1. 已闭合意图（GOVERNED，范围内）

| Intent | Meaning | Entry（canonical） | Owner | Decision |
|---|---|---|---|---|
| activateView/Browser/Home | 通用/浏览器/主页导航 | useLayoutStore.setView | useLayoutStore | GOVERNED |
| activateGrid / openGrid / rebuildGrid / closeGrid / closeGridCell | Grid 生命周期（创建/重排/销毁/关格） | useBrowserStore 动作 | useBrowserStore | GOVERNED |
| tabSwitch / closeTabNow | 页签切换/关闭 | useBrowserStore 动作 | useBrowserStore | GOVERNED |
| openFile / openFileInline / enterDir / saveFile / closeFileEditor | 文件编辑/导航 | useWorkspaceStore 动作 | useWorkspaceStore | GOVERNED |
| add / remove / toggle / importFile / togglePanel | 收藏夹 | useBookmarkStore 动作 | useBookmarkStore | GOVERNED |
| addTermPane / killTerm / termWrite / setActiveTerm / restartTerm / bindTermWriter / replayTermHistory | 终端生命周期 | useSystemStore 动作 | useSystemStore | GOVERNED |
| saveCredential / getCredential / deleteCredential / fillBrowserCredential / listBrowserCredentials | 凭据读写/填充 | bridge → KeyringStore | credential | GOVERNED |

## 2. 已否决意图（红线，代码中不得出现）

| Rejected Intent | Reason | Checker |
|---|---|---|
| `exitGrid` | 多义万能 API（hide/destroy/切视图） | R4 阻断（代码出现即失败） |
| `mergeOpenFileIntoOpenFileInline` | 覆盖编辑器 vs 行内编辑器是独立 UX | 设计约束 |
| `mergeBookmarksIntoHome` | 收藏夹 vs 主页快捷方式生命周期不同 | 设计约束 |
| `componentWritesTermPanes` | 组件直写面板注册表破坏 INV | R3 阻断 |
| `exposePassword` / `copyPassword` / `exportCredential` | 凭据泄露红线 | R4 + S1 阻断 |

## 3. 范围内 — 意图清晰度结论

- **无重复入口**：范围内每个用户意图均有唯一 canonical 入口，duplicate_names 已显式枚举并拦截。
- **closeGridCell** 当前无 UI 调用方（Debt-003 orphan API），但意图语义仍唯一，非重复。
- 结论：范围内 Intent **CLOSED**。

## 4. 范围外 — 新增域意图（无登记，标注 gap）

| 域 | 代表用户意图（代码已有入口，但无 registry 登记） | Owner | Decision |
|---|---|---|---|
| Git | `gitStage/gitUnstage/gitDiscard/gitCommit/gitCreateBranch/gitCheckoutBranch/gitPush`（bridge → request_git_write） | 无 | OUT_OF_SCOPE → 需 SCR |
| Agent/Skill | `agentParse/agentValidate/agentChat/skillInstall/confirmSkill/...` | 无 | OUT_OF_SCOPE → 需 SCR |
| Task | `taskAdd/taskUpdate/taskRemove/taskRunNow` | 无 | OUT_OF_SCOPE → 需 SCR |
| Database | `dbConnect/dbListConnections/dbQuery/dbCancel/dbDisconnect` | 无 | OUT_OF_SCOPE → 需 SCR |
| Graph | `graphQuery/graphNodeGet/graphStats` | 无 | OUT_OF_SCOPE → 需 SCR |
| Script/Snippet | `scriptAdd/scriptUpdate/scriptRemove/runScript/runCommand/snippet*` | 无 | OUT_OF_SCOPE → 需 SCR |
| Session | `sessionSave/sessionDiscard/sessionExport/sessionRestore` | 无 | OUT_OF_SCOPE → 需 SCR |
| Plugin/Image/Resource/Vault/Settings/Home/Workbench | 各自 bridge 入口 | 无 | OUT_OF_SCOPE → 需 SCR |

> 同一用户动作是否多入口？范围内已确认无；范围外因无 registry 无法自动判定，但代码层每个动作通常单一
> store action + 单一 bridge wrapper（封装良好），人工引入重复的风险较低，仍需 registry 固化以防回归。

## 5. 小模型风险

| DUP-ID | Intent | WHY_SMALL_MODEL_MAY_MISUNDERSTAND | POSSIBLE_WRONG_CHANGE | PREVENTION |
|---|---|---|---|---|
| DUP-101 | Grid 生命周期 | openGrid/rebuildGrid/closeGrid 都"碰 grid" | 用 openGrid 替代 rebuildGrid（Regression B） | intents.yaml 显式分离 + R4 |
| DUP-102 | 文件打开 | openFile/openFileInline 都是"打开文件" | 合并二者（丢失行内/覆盖隔离） | rejected mergeOpenFileIntoOpenFileInline |
| DUP-103 | 终端新建 | addTermPane/newTerm/openTerm 同名 | 引入 newTerm 重复入口 | rejected duplicate_names 拦截 |
| DUP-104 | 凭据写入 | saveCredential/dbConnect/configureRepo 都"写 keyring" | 在前端 reactive 存原始凭据 | R3 + R7 护栏 |
