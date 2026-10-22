# Rebased Git 客户端行为契约（原型对应版）

> 本文是 `rebased-screenshot-replica.html` 的开发行为契约。它定义真实 Git adapter 接入时必须保持的 UI 语义，不代表当前原型已经执行真实 Git 命令。

## 1. Git 根状态

| 状态 | 分支入口 | Update/Push | 可用动作 |
| --- | --- | --- | --- |
| `NO_PROJECT` | 隐藏 | 禁用 | 打开项目/仓库 |
| `LOADING` | 显示 `Loading branches…` | 禁用 | 取消或等待 |
| `NO_VCS` | 显示受限入口 | 禁用 | Create Git Repository |
| `UNTRUSTED` | 显示受限入口 | 禁用 | Trust Project |
| `SINGLE_ROOT` | 分支名及 ahead/behind | 按远程状态 | Branches Popup、Checkout、Compare |
| `MULTI_ROOT` | 根目录 + 分支范围 | 按根目录汇总 | Select root、Fetch all roots |

状态变化必须先清除旧的分支菜单和旧的提交选择，避免在仓库尚未加载时继续操作旧数据。
状态也必须通过统一的 `repositoryState` 字段对外暴露，供 UI、自动化测试和真实 adapter 订阅；不能只依赖按钮文案推断状态。

状态映射固定为：`NO_PROJECT`、`LOADING`、`NO_VCS`、`UNTRUSTED`、`MULTI_ROOT`、`AHEAD_BEHIND`、`SINGLE_ROOT`。新增状态必须先更新契约和校验器，不能在页面中临时拼接状态字符串。

## 2. 远程动作

### Update

输入：当前 Git roots、当前分支、更新策略。

过程：`IDLE → CHECKING → UPDATING → REFRESHING → IDLE`。

成功：刷新 roots、branch、commit graph、working tree。

失败：保留当前 Log 和文件选择，在固定状态栏展示错误码、原因和重试入口。

Update 预览必须展示 incoming 提交和更新策略（Merge、Rebase、Fast-forward only）；确认后才执行真实 adapter，没有 incoming 提交时明确显示空状态。

远程结果必须区分 `idle`、`success`、`error`，并保留最近一次操作的标签和可重试信息；不能用成功状态覆盖失败原因。

### Fetch

只刷新远程引用，不修改工作树；多根项目支持选择单根或全部 roots。

Fetch 预览必须列出目标 remote/root，并在确认后刷新引用；取消不得改变当前工作树、Log 选择或文件 Diff。

### Push

没有 outgoing 提交时仍可打开 Push 预览，但显示“没有待推送提交”；有 outgoing 时显示待推送提交列表、目标远程和分支。Push 成功后必须刷新 ahead/behind 和 Log。

所有远程动作共享 busy lock，进行中禁止重复触发，但不能互相覆盖错误信息。

## 3. 提交流程

### Commit / Stage / Amend

提交前必须依次校验：

1. 至少选择一个文件；
2. 提交消息非空；
3. Stage 模式下确认暂存集合；
4. Amend 模式下确认存在可修改的上一条提交；
5. 检查未版本控制文件、文件冲突和 hook 配置。

失败只更新固定结果区，不清空用户选择或提交消息。

提交区的 `Checks…` 入口用于在提交前显式运行检查并查看 hook 输出；真实实现失败时必须保留文件选择、提交消息和可重试入口，不能用成功提示覆盖 hook 错误。

提交结果至少区分 `idle`、`checking`、`success`、`error`；失败必须保留当前选择、消息和 Amend/Stage 状态，成功后才触发工作区刷新事件。

原型状态快照还应明确暴露 `commitMode`（`DIRECT` / `STAGE`）、`amend`、`stagedFiles` 和 `shelfCount`，使真实 Commit 组件可以区分直接提交、暂存后提交和 Amend，而不是只读取按钮文案。

多 Git root 状态还必须暴露 `selectedRoot`；Fetch 与分支菜单应以当前 root 为范围，`Fetch all roots` 才能扩大到全部远程引用。

成功后：刷新 working tree、Changes 数量、Log、当前分支 ahead/behind；Commit and Push 继续进入 Push 预览，而不是直接假定远程成功。

## 4. 提交、文件树与 Diff

提交选择必须生成新的 `CommitSnapshot`：

```ts
type CommitSnapshot = {
  hash: string
  subject: string
  author: string
  parents: string[]
  files: FileChange[]
}

type FileChange = {
  path: string
  oldPath?: string
  status: 'M' | 'A' | 'D' | 'R' | 'TYPECHANGE' | 'BINARY' | 'SUBMODULE'
  modeBefore?: string
  modeAfter?: string
  size?: number
}
```

右侧文件树只能由当前 `CommitSnapshot.files` 渲染，禁止保留上一个提交的固定节点。选择文件后打开 Diff；关闭 Diff 必须恢复原提交、父提交选择和 Log 滚动位置。
文件树摘要必须显示变更状态统计；节点标题至少保留状态、路径，并在适用时展示旧路径、模式变化和大小，保证提交详情中的文件数量与树节点可核对。

特殊边界：

- 初始提交：没有父版本，新增文件不能提供“应用父版本”；
- Merge Commit：必须选择 Parent 1 或 Parent 2；
- Merge/冲突预览：选择 Merge Commit 后，Diff 必须切换为“当前版本 / 冲突编辑区 / 目标版本”三栏；中间区域可编辑，接受箭头只改变当前会话，不能绕过真实 adapter 直接写工作树；
- Binary/large file：显示元数据，禁止强制当作文本 Diff；
- Submodule：显示目标 commit，不把子模块内容当普通文件展开；
- Rename/typechange：显示旧路径、新路径和模式变化。

Diff 编辑区必须提供：

- `只显示变更` / `显示全部内容` 切换，并保留当前文件滚动位置；
- 上一处/下一处变更导航；
- 父版本/当前版本应用箭头；
- 当前版本编辑预览、结束编辑和保存编辑预览状态；
- 二进制、子模块和大文件不得进入可编辑文本区域。
- 子模块和超过预览阈值的大文件必须在 Diff 中显示元数据并明确不可编辑原因。

这些操作在原型中只改变当前 Diff 会话，不直接写入工作树；真实 adapter 接入后必须明确区分“应用到编辑缓冲区”和“写入工作树”两种结果。

## 5. History 语义

Branch 筛选必须真正影响中央 Log：选择 `HEAD`、`master` 或清除筛选后重新渲染提交集合，并在状态快照中保留当前筛选；不能只更新状态栏文字。

状态快照还必须提供动作可用性：`update`、`push`、`fetch`、`branch` 各自包含 `enabled` 和 `reason`，使开发实现能区分“操作进行中”“没有项目”“分支加载中”“无 VCS”“未信任项目”和正常可执行状态。
多仓库状态不是禁用 Update/Push 的同义词；选择具体 Git root 后，Update/Push 针对该 root 可执行，Fetch all roots 作为单独动作。
远程操作结果必须进入固定状态区并保留到下一次远程操作，至少区分成功、错误和空闲，不得只依赖瞬时 toast 或消失的状态文字。
Update/Push/Fetch 的确认、成功和失败状态必须同时投影到该结果区与状态快照，避免同一动作出现两套反馈语义。
README 设置项 `Store project settings outside root` 与 `Enable indexing files for search` 必须投影到页面状态和快照，分别对应 `.idea` 存储策略与索引开关，不能只改变提示文字。
`Log in editor` / `Log in tool window` 必须同步到历史面板的 `data-log-location` 挂点，后续真实布局切换不得重新猜测设置来源。

- 中央 `Log`：仓库提交历史；
- `Refresh Log` 必须重新渲染当前筛选后的提交集合，并保留当前提交选择、滚动位置和 Branch/Paths 筛选；
- `File History`：当前选中文件的提交历史，对应 `Vcs.ShowTabbedFileHistory`；
- Merge、Rebase、Interactive Rebase 必须作为独立 Git action contract 进入分支/提交上下文菜单，并在确认前展示目标分支、策略和冲突处理入口；
- 正常工作流必须存在可见的 `Git workflow` 入口，至少提供 Merge、Rebase、Interactive Rebase 和 Resolve conflicts；三栏冲突编辑区不能只能通过隐藏状态演示进入。
- 无项目、分支加载中、无 VCS 或未信任项目时，`Git workflow` 与分支操作入口都必须隐藏或禁用，不能提供没有仓库上下文的 Merge/Rebase 操作；无 VCS/未信任状态只保留 Create Git Repository/Trust Project 等修复入口。
- 状态快照必须提供 `workflowVisible`，让自动化验收明确判断 Merge/Rebase 入口是否按仓库状态显示。
- Merge/Rebase 预览必须记录 `workflow.action`、`workflow.target` 和 `workflow.result`（`preview`/`confirmed`/`cancelled`），确认或取消都不得丢失可观测状态。
- Merge、Rebase、Interactive Rebase 预览必须提供对应策略选择，并把 `workflow.strategy` 写入确认状态。
- Merge、Rebase、Interactive Rebase 预览必须提供目标分支选择，并把最终选择写入 `workflow.target`。
- Merge/Rebase 预览必须提供“打开三栏冲突编辑器”入口，并将 `workflow.result` 标记为 `conflict-editor`；该状态不等于操作已确认完成。
- 冲突编辑器必须记录当前冲突文件 `workflow.conflictFile`；没有选中文件时使用当前提交的首个变更文件作为明确回退，不能出现无上下文的冲突编辑区。
- 没有文件上下文时，File History 禁用；
- File History、Diff、Log 是互相可返回的标签上下文，不能混用标题和数据源。
- 状态快照必须区分 `historyContext`：`LOG`、`FILE_HISTORY`、`DIFF`；关闭 Diff 或文件历史后恢复到正确的上一级上下文。
- 动作映射必须保留官方命令标识：`Vcs.UpdateProject`、`Vcs.Push`、`Git.Fetch`、`CheckinProject`、`Git.Commit.Stage`、`Vcs.ShowTabbedFileHistory`，真实 adapter 不应从界面文案反向推断命令。

## 5.1 Shelf 语义

- Shelf 列表必须显示实际搁置项数量和每个搁置项的文件范围；
- `Restore shelf` 只把该搁置项恢复到当前 Changes，不得清除其他未提交文件；
- `Show diff` 打开独立 Shelf Diff 上下文，关闭后返回 Shelf；
- `Delete shelf` 只删除当前搁置项并刷新数量；
- 恢复、Diff、删除失败时保留 Shelf 选择和固定状态区错误信息；
- 原型可以使用示例内容，但真实实现必须由 Shelf adapter 提供内容、路径和状态。

## 6. 错误与焦点

提交区和文件详情区的工具栏图标必须是真实按钮而不是静态字符，至少提供 Refresh changes、Rollback、Move changes、Shelve、Show diff、Expand/Collapse tree、File History，并具备 `aria-label`、键盘焦点和固定状态区反馈。

错误不得使用遮挡浏览器内容的弹窗；进入固定状态区或独立布局面板。菜单必须：

- 锚定触发按钮矩形定位；
- 支持外部点击关闭和 `Esc` 关闭；
- 打开时聚焦首项；
- 方向键在菜单项之间移动；
- 关闭后恢复触发按钮焦点。
- 菜单支持 `Home`/`End` 将焦点移动到首项/末项，避免只能逐项导航。
- 提交记录、文件树和 Diff 内的上下文菜单必须锚定实际触发元素，不能使用固定的页面坐标。
- 提交记录支持单选与 Shift 多选；Compare、Cherry-pick、Revert、Reset、Create Branch、Create Tag 必须读取当前选择集合。
- Compare 必须恰好选择两个提交；少于或多于两个都不得派发比较命令。比较视图展示双方 hash、父提交数量和变更文件集合，并可返回原 Log 选择与滚动位置。
- Cherry-pick、Revert、Reset、Create Branch、Create Tag 必须先进入确认预览，确认后才向真实 adapter 派发命令；取消不得改变当前提交选择。
- 分支 Checkout、Compare、New Branch、Rename 等写操作同样必须先确认；加载中、无项目、无 VCS 和未信任项目状态不得派发分支命令。

## 7. 当前实现边界

当前原型已经演示上述状态和交互，`GitStudio.vue` 已接入真实仓库读取、提交文件/Diff、文件历史、暂存提交、标签、撤销、分支与远程操作 adapter。进入桌面验收前，仍必须用真实仓库和远端网络验证 Update/Fetch/Push/Commit/Checkout/Shelf 的命令执行、错误映射、刷新事件以及安装版行为；HTML 原型中的 `CommitSnapshot` 仅用于视觉回归。

原型提供只读 `window.__rebasedPrototype.getState()` 快照用于自动化验证；它只能观察状态，不能替代真实 Git 命令或修改工作树。
在 `NO_PROJECT` 状态下，快照中的 `selectedCommit` 和 `snapshot` 必须为 `null`，不得泄漏上一个项目的数据。
