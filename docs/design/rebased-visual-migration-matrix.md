# Rebased 视觉方案 → GitStudio 迁移矩阵

唯一视觉基准：`docs/design/rebased-screenshot-replica.html`。
真实实现：`src/capabilities/git/ui/GitStudio.vue`。

| 视觉方案区域 | 原型内容 | GitStudio 当前实现 | 状态 |
|---|---|---|---|
| 顶部仓库/分支 | 仓库、分支、Update、Push、搜索、设置 | `source`、仓库选择、分支菜单、Update/Push 真实预览确认、历史搜索聚焦、持久化 Git 设置；Update 支持 Fast-forward only / Merge / Rebase 且由原生适配器校验 | 已对应，待安装版视觉验收 |
| 左侧提交区 | Commit / Shelf、文件选择、移动改动、Stage、Amend、展开/折叠 | `rebasedChanges`、`rebasedUnversioned`、`selected`、`stashes`、`rebasedAmend`；工具条接入真实 stage/unstage、discard、stash、diff、选择已打开文件和分组展开/折叠 | 已对应，待安装版视觉验收 |
| 中央历史区 | Log、筛选、提交选择、提交上下文菜单 | `commits`、`search`、`author`、`revision`、`logSelection` | 已对应 |
| 右侧详情区 | 提交文件树、文件状态、父提交 | `commitFiles`、`visibleCommitFiles`、`selectCommitFile` | 已对应 |
| 中央 Diff/预览 | 文件、Split、Patch、仅变更、折叠 | `historicalDiffMode`、`historyOnlyChanges`、`historyEditorCollapsed` | 已对应 |
| 三列主布局 | 左 Commit/Shelf、中 Log/编辑器、右提交文件树 | `rebasedLeftWidth`、`rebasedRightWidth`、`beginRebasedResize`；选择提交文件后中栏从 Log 切到历史文件编辑/预览 | 已对应，待安装版视觉验收 |
| 冲突编辑 | 当前版本 / 冲突编辑 / 目标版本、上/下一个冲突、逐行采用当前或目标 | `conflict`、`resolution`、`conflictRows`、`moveConflict`、`applyConflictLine`，最终仍走 preview/confirm | 已对应，待真实冲突仓库验收 |
| Merge Commit 父提交 | 多父提交选择后重算文件列表和 Diff | `StudioInput.parent` 已接入，原生校验父提交归属；仍需安装版真实 Merge Commit 验收 | 已接入，待 GUI 验收 |
| Shelf Diff | 搁置项、保存、查看、恢复 | `stashes`、`inspectStash`、`stash_save`、`stash_apply`，真实写操作继续走 preview/confirm | 已对应；删除仍保留在完整 Stash 工作区 |
| 文件历史 | 独立文件历史标签和返回 Log | `inspectFile('follow')`、`fileHistoryContext`、`historicalContent` | 已对应 |
| 远程操作结果 | 独立进度、错误、刷新结果 | `remote`、`preview`、`notice`、`error` | 已对应 |
| 设置 | Log 位置、`.idea`、索引 | `gitPreferences.ts` 持久化 `logPlacement`、`writeIdeaMetadata`、`indexEnabled`，并由真实 GitStudio 读取应用 | 已对应，待安装版视觉验收 |

## 当前证据边界（2026-10-22）

- `check-rebased-source-audit.mjs`：76 项通过，覆盖真实 Vue/Rust 实现而不是只检查原型按钮。
- `check-rebased-adapter-alignment.mjs`：34 项通过，覆盖 Update 策略、历史文件、冲突和写操作适配边界。
- Git 原生单元测试：12 项通过，覆盖提交历史文件、冲突元数据、范围提交、Merge/Rebase/Cherry-pick 安全边界。
- `npm run build`、`check:real-ui`、Workspace 启动和图片懒加载门禁通过。
- 当前桌面进程已完成 Vue mount、首帧根节点和 1920×1080 布局探针；桌面截图通道在本机会返回全黑，因此不能据此宣称 `GUI_PASS`。
- `tests/visual/git-studio-rebased.html` 直接挂载生产 `GitStudio.vue`，只以确定性 Git 数据替换原生读取通道；已实际验证 Log 首屏、选择提交后的递归文件树、选择文件后的历史编辑器、并排 Diff，以及编辑器模式的无遮挡工具条。该夹具不是第二套产品 UI。
- 整仓 `pre-merge.sh` 仍因本轮范围外的危险区 `AGENTS.md` 修改，以及整仓体积相对历史基线增长 36.39%（上限 35%）失败；不得通过删除其他人的改动或放宽基线掩盖。

## 迁移规则

1. 真实 GitStudio 的行为和 Adapter 是权威，原型只提供视觉和交互基准。
2. 不得为了复刻视觉而绕过 `preview → confirm → execute` 安全边界。
3. 原型中的示例数据不得直接进入真实仓库状态。
4. 每完成一行迁移，必须同时通过 `check:developer-studio`、`check:real-ui` 和 `check:rebased-adapter`。
5. 安装版 GUI 验收必须在迁移完成后单独执行，浏览器原型加载不能替代安装版证据。
