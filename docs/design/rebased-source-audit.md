# Rebased 产品方案源码审核（当前轮）

审核基准：

- 唯一正式视觉方案：[rebased-screenshot-replica.html](./rebased-screenshot-replica.html)
- 真实 Git 实现：`src/capabilities/git/ui/GitStudio.vue`
- 原生安全边界：`git_studio_read → git_studio_preview → git_studio_confirm`
- 迁移矩阵：[rebased-visual-migration-matrix.md](./rebased-visual-migration-matrix.md)

## 结论

这份方案可以继续作为唯一正式视觉基准，但不能把 HTML 原型的按钮反馈当成真实 Git 完成证据。真实实现已经覆盖主要 Git 工作流，当前验收边界是“源码/UI 对齐通过，安装版桌面 GUI 尚未在本轮验证”。

## 源码对照结果

| 审核项 | 当前证据 | 结论 |
|---|---|---|
| 提交详情与变更文件 | `selectCommit` → `commit_files` → `selectCommitFile` | 通过 |
| 重命名文件 | 原生 `--name-status -z` 保留 R/C 状态，文件树显示旧路径 → 新路径，Diff 使用新路径 | 通过 |
| 提交文件统计 | 当前提交详情显示新增/修改/删除/重命名统计，数量与动态文件树一致 | 通过 |
| 仓库状态投影 | `repositoryState`：无项目 / 加载中 / 非 Git / 路径无效 / 未受信任 / 已加载；异常状态进入固定状态面板 | 通过 |
| 仓库范围与同步状态 | `repositoryMode`：单仓库 / 多仓库根目录 / 有待同步提交，并通过 `data-repository-mode` 暴露 | 通过 |
| incoming/outgoing | 原生 `%(upstream:track)` → `branchTracking` → 当前分支/列表徽标 | 通过 |
| 远程操作结果 | Fetch/Update/Push 独立记录 checking、executing、success/error/cancelled，并保留固定结果区 | 通过 |
| Push/Update 空状态 | Push 无 outgoing、Update 无 incoming 时仍可预览并明确显示空状态 | 通过 |
| 文件历史动作 | `inspectFile('follow')` → 原生 `log --follow`，按钮标注 `Vcs.ShowTabbedFileHistory` | 通过 |
| 文件历史上下文 | 进入文件历史后显示明确的文件路径上下文，并可返回仓库 Log | 通过 |
| 历史上下文状态 | `historyContext` 明确区分 `LOG`、`FILE_HISTORY`、`DIFF`、`CHANGES`，并通过 `data-history-context` 暴露 | 通过 |
| 历史路径筛选 | 提交说明、作者、Paths 可筛选；进入文件历史自动锁定当前文件，返回仓库 Log 清除约束 | 通过 |
| 提交右键操作 | 历史行右键打开布局内菜单；比较工作区、挑选提交、从提交新建分支均复用 preview/confirm | 通过（真实写操作仍需确认） |
| 提交多选 | Ctrl/⌘ 多选、Shift 范围选择；两条提交可进入 Compare，写操作仍保持单提交契约 | 通过（多提交写操作未擅自扩展） |
| 分支右键操作 | 本地/远程分支项支持 Checkout、Compare、新建分支、删除已合并分支入口 | 通过（真实写操作仍需确认） |
| 菜单键盘行为 | 提交/分支菜单打开后首项聚焦，方向键循环，Home/End 首末项定位，Esc 关闭 | 通过 |
| 编辑器 / Diff | 文件、并排 Diff、Patch、全文三版本、部分行 | 通过 |
| 只显示变更 / 显示全部 | `historyOnlyChanges` 与 `historicalRows` | 通过 |
| 编辑器折叠 | `historyEditorCollapsed` | 通过 |
| 底部分支与历史 | `history-bottom`、真实 local/remote refs | 通过 |
| 可拖动布局 | `beginHistoryResize('inspector'/'bottom')` | 通过 |
| 三栏冲突 | ours / 中央 resolution / theirs；冲突定位和逐行候选应用 | 部分通过：仍缺少后端 hunk 级语义和已解决状态持久化 |
| Shelf | 读取、Diff、保存、应用、弹出、删除 | 通过 |
| 写操作安全边界 | preview → checkbox acknowledgement → confirm → refresh | 通过 |
| 提交并推送 | 提交确认成功后刷新仓库，再自动进入独立 Push 预览；推送仍需二次确认 | 通过（真实远端仍需安装版/网络验收） |
| 提交暂存约束 | Commit/Amend/提交并推送仅允许已暂存文件，混选未暂存文件时阻止并解释原因 | 通过 |
| 分支危险操作边界 | 远程分支不可 Checkout；当前、`master`、`main` 不可删除；仅非当前本地分支可删除 | 通过 |
| Merge/初始提交 Diff | Merge Commit 可选择合法父提交并刷新文件列表/Diff；初始提交明示无父提交 | 通过 |
| 原型与真实实现一致性 | `check:rebased-source-audit`、adapter alignment | 通过 |

### 本轮新增收口

- 真实 GitStudio 顶部现在投影“分支组件”入口：只有仓库已加载时显示当前分支与 incoming/outgoing 徽标，点击后进入底部分支/历史区；没有项目时入口不渲染。
- 真实 GitStudio 增加未受信任项目状态及固定布局提示，避免把 `UNTRUSTED` 误当成普通单仓库继续操作。
- 提交右键已补齐 Reset 入口；soft/mixed/hard 均走预览确认，hard 额外要求显式确认标记，并在原生层做模式校验。
- 源码审核门禁当前为 68 项，新增“受信任状态可见”“分支组件进入历史上下文”“Reset 预览契约”和“无 VCS 初始化预览”检查。

## 仍然存在的产品差异

1. **冲突编辑仍低于 IDEA 级别。** 当前已补充原生 hunk 元数据、冲突定位、逐行“当前 → 编辑区 / 目标 → 编辑区”和整块采用；解决内容会写回工作区并自动暂存，专项 Rust 测试已覆盖该闭环。仍缺少桌面端多文件冲突批量处理、冲突标记可视化和安装版人工验收。
2. **原型仍是示例数据。** 原型可用于视觉和交互评审，不能替代真实仓库读写、远端网络、权限和安装版验收。
3. **设置迁移已完成 UI 偏好闭环，但原生效果仍有边界。** 真实 GitStudio 已持久化 Log 位置、`.idea` 写入和索引开关，并实际切换 Log 布局；`.idea` 写入策略和索引开关尚未下沉到原生 Git 执行层，需单独进行桌面版验收。
4. **源码级门禁不等于桌面 GUI 通过。** 仍需在明确授权后执行安装包冷启动和真实窗口验收。

5. **Merge Commit 多父 Diff 已补齐首轮契约。** `StudioInput.parent` 支持选择父提交；原生层校验父提交必须属于目标 commit，`commit_files` 返回全部父提交和当前选中父提交；切换父提交后文件列表与 Diff 重新读取。后续仍需在真实 Merge Commit 仓库上做安装版 GUI 验收。

### Merge Parent 扩展验收条件

- 初始提交：没有父提交时不显示选择器；
- 普通提交：只有一个父提交时保持现有行为；
- Merge Commit：列出全部父提交并默认 first parent；
- 切换父提交后，文件列表和 Diff 同步刷新；（已实现）
- 非该提交父对象、非法 OID 和并发变更必须拒绝；
- 预览确认链路必须继续使用当前 fingerprint 安全校验。

## 当前评分

| 维度 | 分数 | 说明 |
|---|---:|---|
| 截图布局还原 | 89 | Rebased 四区结构稳定 |
| 视觉细节 | 83 | 编辑器、底部历史、拖动分隔线、文件历史上下文已补齐 |
| 按钮覆盖 | 91 | 提交/分支右键、多选比较、文件历史和安全写操作入口已覆盖 |
| 源码行为一致性 | 99 | 真实 Adapter 链路、状态映射、仓库范围状态、历史上下文、远程操作结果、Push/Update 空状态、提交两阶段流程、标签/撤销/Reset/初始化预览确认、多父 Diff、重命名文件语义、提交文件统计和 68 项源码审核门禁已建立 |
| Git 工作流完整性 | 87 | 读、预览、确认、刷新闭环以及 Update 三策略映射已建立；远端网络和桌面冲突验收仍待完成 |
| 可开发性 | 89 | 迁移矩阵、行为契约、Adapter 契约和边界说明已建立 |
| 综合 | **93** | 可作为正式产品方案和开发基准，尚不能宣称完整桌面验收通过 |

## 本轮验证命令

```text
npm run check:rebased-source-audit
npm run check:rebased-adapter
npm run check:rebased-prototype
npm run check:developer-studio
npm run check:real-ui
npm run build
git diff --check
```

本轮额外把原型自身的交互契约纳入源码审核门禁（当前共 68 项）；Adapter 对齐门禁当前为 22 项：

原生 Git Studio 定向 Rust 回归测试当前为 29 passed、3 ignored（数据库 E2E 需要可丢弃 MySQL 服务）、0 failed；其中包含提交历史、提交文件 Diff、受控提交、Merge/Rebase/Cherry-pick 安全边界、冲突 hunk、stderr 错误保留和工作区 fingerprint 检查。

- Update / Push 独立入口，以及 Fetch 入口可达性；
- Commit 消息、文件选择、Stage、Amend 的前置校验；
- 提交选择 → 变更文件 → Diff 的联动标记；
- 提交右键操作、多选提交状态；
- 菜单定位、焦点恢复、Esc/方向键操作；
- 分支状态矩阵和“文件历史 ≠ 仓库历史”的语义约束。
- 提交并推送的二次确认链路、暂存文件约束、分支危险操作边界、Merge/初始提交 Diff 语义提示。

这 68 项源码审核和 22 项 Adapter 对齐只证明原型与源码约定没有静态漂移，不证明示例数据已经连接到真实远端，也不替代桌面安装版验收。

其中安装版 GUI 仍必须单独验证；未执行安装版脚本时，结论只能是源码/UI 对齐通过，不能写成 `GUI_PASS`。

## 后续收口顺序（按阻断程度）

| 优先级 | 收口项 | 当前证据 | 完成条件 |
|---|---|---|---|
| P0 | 安装版 Tauri GUI | 本轮未执行安装版验收 | 无 Vite/1421 进程冷启动，真实窗口可见，GitStudio 菜单/编辑器/分支区可操作 |
| P0 | 真实远端 Update/Fetch/Push | 当前只有 Adapter 预览链和 fast-forward 子集 | 真实 remote 结果、进度、拒绝原因和刷新快照均可追溯 |
| P1 | Merge Commit 多父 Diff | `StudioInput.parent` 已接入，非法父提交会拒绝 | 在真实 Merge Commit 仓库和安装版 GUI 上验证父提交切换、非法 parent、并发变更拒绝 |
| P1 | hunk 级冲突状态 | 当前为行级候选预览 | 后端返回冲突 hunk、每块 resolved 状态，并能安全恢复/确认 |
| P2 | Revert/Reset/Tag 等扩展动作 | Revert/Tag 已有预览链；Reset 已接入 soft/mixed/hard 和 hard 确认边界 | 仍需在真实安装版和真实仓库验证执行后刷新、失败恢复与用户提示 |

在 P0/P1 未完成前，综合结论保持“正式产品方案 / 源码对齐通过，完整开发验收未通过”，不得升级为 `READY_TO_MERGE` 或 `GUI_PASS`。

## 原型冷加载证据

正式 HTML 使用系统 Google Chrome headless 冷加载成功：

- 退出码：`0`；
- 生成 DOM：约 `78 KB`；
- `branchMenu`、`updateProject`、`pushProject`、`rows`、`status` 核心节点均存在；
- Chrome 日志未出现 `SyntaxError`、`ReferenceError`、`TypeError` 或 `Uncaught`。

这只证明原型页面可运行，不替代真实 GitStudio 或安装版 Tauri GUI 验收。
