# Rebased Git Adapter 契约（原型到开发实现）

本文是 `rebased-screenshot-replica.html` 的实现边界。项目真实 GitStudio 已通过 `GitStudio.vue → bridge.ts → git_studio_read/git_studio_preview/git_studio_confirm/git_studio_cancel` 接入；本文约束的是截图复刻原型与该真实实现之间的映射，不把静态示例数据误认为真实仓库结果。

## 1. 命令边界

| UI 动作 | Adapter 命令 | 输入 | 成功结果 | 失败结果 |
|---|---|---|---|---|
| Update | `Vcs.UpdateProject` | root、策略（merge/rebase/ff-only） | 重新读取状态、Log、Changes | 保留原上下文，返回可读错误 |
| Fetch | `Git.Fetch` | root 或 all roots、remote | 更新 refs，不改变工作树 | 显示 remote、认证或网络错误 |
| Push | `Vcs.Push` | remote、branch、selected commits | 更新 outgoing、refs、Log | 保留待推送列表与错误 |
| Commit | `CheckinProject` | message、selected files、amend | 刷新 Changes、Log、当前提交 | 保留选择和消息 |
| Stage | `Git.Commit.Stage` | selected files | 更新 staged/unstaged 分组 | 保留原分组并显示错误 |
| File History | `Vcs.ShowTabbedFileHistory` | root、path | 返回文件历史提交列表 | 保留当前文件和 Log 上下文 |
| Merge | `Git.Merge` | source、target、strategy | 更新分支状态或进入冲突 | 返回冲突文件集合 |
| Rebase | `Git.Rebase` | source、target、strategy | 更新分支状态或进入冲突 | 返回冲突文件集合 |

### 当前实现映射说明

上表是正式产品动作契约；当前 GitStudio 使用 `update` 作为 `Vcs.UpdateProject` 的内部动作标识，并保留旧的 `pull` 输入兼容。
策略由预览明确选择：Fast-forward only（默认）、Merge 或 Rebase，并分别映射到 `git pull --ff-only`、`--no-rebase`、`--rebase`。

因此：

- Fetch 与 Push 是独立入口；
- 当前 Update 已支持三种策略的预览、确认和原生命令映射；真实远端仓库和安装版冲突事件仍需验收；
- 完整 `Vcs.UpdateProject` 验收仍需覆盖策略执行结果、冲突事件映射和刷新快照。

## 2. 统一状态

```ts
type RepositoryState =
  | 'NO_PROJECT' | 'LOADING' | 'NO_VCS' | 'UNTRUSTED'
  | 'SINGLE_ROOT' | 'MULTI_ROOT' | 'AHEAD_BEHIND';

type OperationPhase =
  | 'IDLE' | 'CHECKING' | 'FETCHING' | 'UPDATING'
  | 'PUSHING' | 'COMMITTING' | 'REFRESHING' | 'CONFLICT';
```

`NO_PROJECT` 必须清空提交、文件树和文件选择；`LOADING` 禁止分支写操作；`NO_VCS` 只提供 Create Git Repository；`UNTRUSTED` 只提供 Trust Project；多仓库必须要求明确 root。

## 3. 事件顺序

所有写操作必须遵循：

```text
validate → preview → confirm → execute → phase events → refresh snapshot
```

Adapter 不得直接修改 UI。它只发布：

```ts
type GitAdapterEvent =
  | { type: 'phase'; phase: OperationPhase; label: string }
  | { type: 'snapshot'; snapshot: RepositorySnapshot }
  | { type: 'conflict'; files: ConflictFile[] }
  | { type: 'error'; operation: string; code: string; message: string }
  | { type: 'completed'; operation: string };
```

## 4. 刷新不变量

- Commit 成功只移除已提交文件，未选择文件必须保留。
- Update/Fetch/Push 成功后必须重新获取 branch、incoming/outgoing、Log 和 Changes，不能只更新状态栏。
- File History 返回 Log 时恢复原提交、文件选择和滚动位置。
- Diff 关闭时恢复进入 Diff 前的 `historyContext`。
- 错误不得清空用户的提交消息、文件选择、Shelf 选择或冲突编辑内容。
- 原生 Git 非零退出必须保留受限长度的 stderr；Hook、冲突、认证和远端拒绝应进入固定错误状态区，不得只返回无上下文的通用失败文字。
- 二进制、子模块和超大文件只返回元数据，不进入文本编辑器。

## 5. 当前原型边界

截图复刻页面通过 `window.__rebasedPrototype.getState()` 暴露只读状态快照；该接口只用于视觉/交互验收，不能替代真实 GitStudio。真实 GitStudio 已有 Adapter，但两套界面仍需逐项对照：仓库状态、提交文件、Diff、冲突、Shelf、远程操作和刷新事件。安装版 GUI 验收也必须独立完成，不能用原型浏览器加载结果替代。

## 6. Merge Commit parent 扩展（已实现，需安装版验收）

当前 `StudioInput` 已包含可选 `parent` 字段。`commit_file_diff` 默认使用 first parent；选择父提交后按指定父提交生成文件列表和 Diff：

```text
parent: optional parent oid or parent index
```

- `commit_files` 返回完整 parents；
- 普通提交和初始提交保持现有行为；
- Merge Commit 缺省使用 first parent；
- 指定 parent 必须属于目标 commit，否则拒绝；
- parent 选择变化必须重新读取文件列表和 Diff；
- preview/confirm 继续绑定原仓库 fingerprint；
- 前端已提供 Merge Commit 父提交选择器；仍需在真实 Merge Commit 仓库的安装版 GUI 中验证切换、非法 parent 和并发变更拒绝。
