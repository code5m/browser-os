# 官方截图对照与源码核对

参考： https://github.com/DetachHead/rebased/blob/master/screenshot.png

已通过 GitHub Contents API 获取并实际查看原图，尺寸为 1583 × 919。
截图是仓库 master 的静态参考，不代表所有发布版本及所有页面。

## 截图中的结构

- 顶部约 44px：产品图标、主菜单、仓库、分支、同步快捷操作、搜索、设置、窗口控制。
- 最左约 40px：竖向工具窗口栏。
- 左侧约 300px：Commit / Shelf、工作区修改列表、底部 Amend 和提交消息、Commit / Commit and Push。
- 中间约 845px：Log: HEAD 标签、筛选工具栏、带作者和日期列的提交图。
- 右侧约 350px：可展开文件树，底部所选提交详情。
- 深灰背景、细边框、紧凑行高、蓝色选中行；原型旧稿的浅色大块按钮不符合此参考。

## 当前稿

`rebased-screenshot-replica.html` 按上述区域重建；旧稿保留。
`browseros-idea-git-layout.html` 使用相同深色配色，但遵循用户要求保留左侧详情、
中央编辑、底部分支与历史。它是适配设计，不以区域位置与 Rebased 完全相同为验收条件。

## 源码依据

https://github.com/DetachHead/rebased/blob/master/plugins/git4idea/frontend/resources/intellij.vcs.git.frontend.xml

实际读取确认：

| 注册 | 对应能力 | 核对边界 |
| --- | --- | --- |
| GitToolbarWidgetAction | 顶部分支组件 | 尚需读取组件实现确定菜单和状态 |
| UpdatePushQuickAction | 更新/推送快捷操作 | 注册本身不能证明当前截图每个箭头的提示文字 |
| CommitQuickAction | 提交快捷操作 | 需继续核对提交检查与消息校验 |
| HistoryQuickAction | 历史快捷操作 | 需继续核对导航目标 |
| RollbackQuickAction | 回滚快捷操作 | 需继续核对确认与选择范围 |

## 尚存差异

1. 顶栏和侧栏仍有字符近似图标；Commit 工具栏已替换为描摹的线框 SVG，尚非官方图标资产。
2. 提交图已增加两处分叉、两色轨道和节点；复杂拓扑不是通用图算法。
3. 已按截图录入 29 条可辨认的标题、作者和时间。它们是截图转录，不是从真实仓库加载的提交。右侧文件数与树节点尚未完全对应，切换提交也尚未联动整棵文件树。
4. User / Date 已能组合筛选并显示空结果；Paths、Shelf 和仓库/分支入口尚未完成。
5. 文本文件已有双栏示例 Diff 和未变更行折叠；二进制明确说明缺少内容。删除、重命名和真实图片预览仍缺失。
6. 已完成一轮同尺寸浏览器对照；顶栏图标、筛选工具栏缺失按钮、树细节还需修正，不能据此宣称像素级还原。

此前递增的评分缺少完整点击证据，不再作为当前验收分。以上待完成项闭合后重新审核。

## 2026-10-21 实测记录

范围：仅 `rebased-screenshot-replica.html`，BrowserOS 适配稿和旧稿保留。本轮不代表两个原型全部完成。

浏览器视口为 1583 × 919，与参考图相同。通过真实 DOM 几何读取和截图确认：

| 区域 | 当前测量 | 原图人工读数 | 结论 |
| --- | --- | --- | --- |
| Commit 面板 | x=42, width=300, height=843 | x≈42, width≈300 | 主要位置对齐 |
| Log 面板 | x=347, width=841 | x≈348, width≈846（包含右侧边界） | 仍有边界差异 |
| 文件树面板 | x=1193, width=350 | x≈1195, width≈348 | 约 2px 边界差异 |
| fixes from merge 选中行 | y=252, 行高26 | y≈252, 行高26 | 纵向对齐 |

实际点击验证：

- 作者 detachhead：7 行。
- 同时筛选 15 June：显示空结果，而不是残留旧记录。
- 恢复筛选并搜索 jediterm：1 行。
- 点击 action.yml：中央切换到父提交/当前提交双栏示例。
- 折叠未变更行：两侧仅保留第 5 行差异；展开后恢复 8 行未变更上下文。
- 工作区 Collapse all / Expand all：展开节点数分别为 0 / 2。
- 本次浏览器错误日志为空；HTML 内联脚本语法检查通过。

边界：这些是 HTML 原型的视觉及交互证据，不是 Tauri 安装版验收，不使用 GUI_PASS。

## 2026-10-21 新增源码证据

已通过 GitHub Contents API 读取以下实现，而非仅依赖动作注册：

- [GitQuickActions.kt](https://github.com/DetachHead/rebased/blob/master/plugins/git4idea/frontend/src/com/intellij/vcs/git/frontend/GitQuickActions.kt)：UpdatePushQuickAction 绑定 Vcs.UpdateProject / Vcs.Push；CommitQuickAction 绑定 CheckinProject / Git.Commit.Stage；HistoryQuickAction 绑定 Vcs.ShowTabbedFileHistory；RollbackQuickAction 绑定 ChangesView.Revert。此文件只确定动作映射，不证明确认对话框细节。
- [GitToolbarWidgetAction.kt](https://github.com/DetachHead/rebased/blob/master/plugins/git4idea/frontend/src/com/intellij/vcs/git/frontend/widget/GitToolbarWidgetAction.kt)：仓库已加载时进入 GitBranchesPopup；未加载时等待初始化；NoVcs 进入建仓动作组，未信任项目则显示信任入口；没有项目或 DoNotShow 时不显示；incoming/outgoing 图标由同步状态决定。

后续必须按这些状态补交互，不能把分支组件实现成始终可用的静态下拉，也不能把 HistoryQuickAction 错映射为整个仓库历史。

## 2026-10-21 按钮交互复核

已把复刻稿中的可见入口改为可操作原型。每个入口都会改变页面状态或状态栏；涉及真实 Git 写入的动作只显示演示结果，不修改仓库。

| 页面入口 | 源码映射 | 原型行为 | 当前边界 |
| --- | --- | --- | --- |
| Commit / Shelf | Commit 工具窗口与 Shelf | 切换面板，状态栏说明当前面板 | Shelf 数据为空，未实现真实 stash |
| Refresh | 工作区刷新动作 | 更新状态栏 | 未重新读取真实仓库 |
| Rollback | `ChangesView.Revert` | 显示需选择文件的回滚说明 | 不改工作树 |
| Move changes | 变更移动动作 | 显示选择范围说明 | 不改工作树 |
| Shelve changes | Shelf 动作 | 显示搁置说明 | 不创建 stash |
| Show diff / Preview | Diff / 文件预览 | 打开选中文件的双栏 Diff | 使用示例文本 |
| Expand / Collapse | 文件树常用展开动作 | 统一展开或折叠节点 | 只作用于原型树 |
| Commit / Commit and Push | `CheckinProject` / `Vcs.Push` | 检查选中文件并显示提交说明 | 不写 Git、不访问远程 |
| 仓库 / 分支 / Update | `GitToolbarWidgetAction`、UpdatePushQuickAction | 展示菜单和状态 | 未实现分支切换、fetch、push |
| User / Date / Paths | Log 过滤入口 | User/Date/Paths 均按示例数据过滤 | 仍未连接真实 Log 查询 |
| History menu | `Vcs.ShowTabbedFileHistory` 相关入口 | 展示详情、刷新、新标签演示项 | 不创建真实 IDE 标签 |

本轮实际点击证据：Commit/Shelf、Refresh、Rollback、仓库菜单、分支菜单、Checkout、分支筛选、路径菜单、Show diff、Commit、Commit and Push 均可点击且产生可见状态变化；浏览器错误日志为空。源码对应动作仍需在下一轮补齐真实数据模型和状态机，不能把这些演示动作当作仓库操作已完成。

## 旧版完整工作区回归

`rebased-product-prototype.html` 也已复核：Push 不再因为 `ahead 0` 被静态禁用，而是进入 Push 预览；Fetch、Pull、分支切换、Cherry-pick、Revert、Reset、Tag、文件历史、Blame、Merge、Rebase、Interactive Rebase、Diff 模式切换和三栏冲突入口均能进入相应预览或编辑状态。Interactive Rebase 的首行上移、末行下移仍保持禁用，这是边界条件而不是失效按钮。实测该页面共 94 个按钮，只有这两个合法边界按钮禁用；Push 预览和冲突“接受目标”均已点测通过，错误日志为空。

> 注：本文件按时间追加记录。下方“仍未宣称完成”的列表属于该轮审核时点的历史快照，不能覆盖文档后续的“最新审核矩阵”和 `rebased-source-audit.md` 当前结论。

## 2026-10-21 产品方案复核与本轮落地（历史快照）

正式产品方案唯一指向 `rebased-screenshot-replica.html`。本轮按 Rebased 官方源码复核并落地了四项行为边界：

- 顶部 Update 与 Push 拆为独立入口；
- History 在没有文件上下文时给出约束提示，选择文件后指向文件历史语义；
- Commit / Commit and Push 在原型层校验至少选择一个文件且提交消息非空；
- 选择提交时同步更新提交标题、作者和状态说明，明确右侧文件树后续应按提交刷新。

浏览器实测：未选择文件点击 Commit 时显示“提交前校验失败：请选择至少一个变更文件”；未选择文件点击 History 时显示“请先选择文件，再打开该文件历史”；控制台错误为 0；前端构建、脚本语法和 `git diff --check` 通过。

仍未宣称完成的源码一致性项：

1. `GitToolbarWidgetAction` 的 NoVcs、未信任项目、加载中、多仓库、incoming/outgoing 状态机尚未接入真实数据；
2. Update、Push、Fetch 仍是预览行为，不执行 Git 命令；
3. 提交选择、文件树和 Diff 仍使用示例数据，尚未按真实 commit 刷新；
4. Paths 尚未真正过滤日志；Shelf 尚未接入真实 stash；
5. 菜单定位、键盘导航、右键菜单和提交多选仍待实现。

因此当前状态应记录为：`VISUAL_PASS / PROTOTYPE_BEHAVIOR_PARTIAL / REAL_GIT_PENDING`，不得写成真实 Git 客户端已完成。

本轮后续回归补充：Paths 已按示例路径分组过滤；提交记录支持 Shift 多选、上下方向键移动和右键 Compare/Cherry-pick/Revert/Create branch 演示；菜单支持按触发按钮定位、点击外部关闭和 Esc 关闭。浏览器回归确认路径筛选后显示 9 条示例提交，键盘选择可更新提交详情，控制台错误为 0。

最新回归补充：

- Settings 菜单已覆盖 Log 主编辑区/工具窗口、`.idea` 存储位置和索引开关三个产品选项；
- Shelf 已显示示例搁置项，切换时隐藏 Commit 输入区，切回后恢复；
- Update、Push、Fetch、Commit、Commit and Push 具有进行中、重复点击拦截和完成状态；
- 选择 `action.yml` 后打开 History，可见文件历史标题、提交列表和“返回仓库 Log”；
- 选择不同提交会更新右侧文件计数和可见文件集合；
- Amend last commit 会切换为 Amend Commit，并与提交文件/提交消息校验联动；顶部 Push 与 Update 共用忙碌锁、阶段状态和重复点击拦截；右侧 Show diff/Preview 会优先使用当前已选文件；
- 提交切换现在会重建右侧变更文件树，而不是只隐藏固定节点；新树中的文件仍可进入 Diff 和 File History；
- 分支入口补充了 No project、Loading branches、No VCS、Untrusted project、incoming/outgoing 五类可审计状态；Loading/No project 状态不再弹出可执行分支菜单。
- Commit 区补充 Stage selected files 与 Amend 两个独立状态；提交成功演示会清除选择并反馈工作区刷新，Commit and Push 会进入 Push 阶段提示。
- Merge Commit 记录现在显示 Diff parent 选择器，切换父提交会同步状态栏，避免把合并提交误当成单父提交 Diff。
- Push 现在区分无 outgoing 提交与有 incoming/outgoing 状态：初始同步分支显示“没有待推送提交”，切换到 `master ↑1 ↓2` 后才进入 Push 预览。
- Shelf 搁置项已补 Restore、Show diff、Delete 菜单；恢复/删除会从当前 Shelf 列表移除示例项并反馈状态，仍明确标注未写入真实 stash。
- Diff/File History 返回 Log 时保存并恢复中央列表滚动位置，同时保留当前提交上下文；浏览器实测关闭 Diff 后状态栏显示当前提交仍为 5。
- 分支按钮补充右键菜单：Checkout、Compare、New Branch、Rename、Delete；No VCS、未信任、Loading、No project 状态仍按约束收敛可用入口。
- 提交记录右键菜单补充 Reset current branch to commit 与 Create tag at commit，仍以预览提示表达需要确认/真实仓库边界。
- 分支状态机新增 `2 Git roots` 多仓库状态；可选择 `platform` / `plugins` 根目录、打开多仓库 Branches Popup 或 Fetch all roots，右键也提供 Select root/Compare roots。
- 菜单关闭逻辑统一恢复触发按钮焦点；Esc 和点击外部关闭后可继续键盘操作。浏览器实测 Esc 后 `activeElement=branchMenu`，菜单已移除，错误日志为空。
- 浏览器回归控制台错误为 0，HTML 脚本语法、`npm run build`、`git diff --check` 通过。
- 菜单键盘导航回归修复：菜单打开时 ArrowUp/ArrowDown 不再触发提交列表的全局导航重绘，避免菜单焦点丢失；菜单关闭仍恢复触发按钮焦点。脚本语法、构建和 `git diff --check` 重新通过。
- 菜单进一步采用 `role=menu/menuitem`，打开时自动聚焦首项并维护 roving `tabIndex`；浏览器实测 No VCS 菜单打开后首项获得焦点，ArrowDown 移到 `Create Git Repository`，Esc 后恢复 `branchMenu`，错误日志为空。
- 文件树补充 A/D/R 状态样例：新增文件显示“父提交中不存在”、删除文件显示“当前提交中已移除”、重命名文件显示路径变化说明；未配置具体内容时仍生成可读的双栏示例 Diff。浏览器实测新增文件 `test.cmd` 可进入 Diff，错误日志为空。
- `No project` 状态现在真正隐藏分支组件；通过 Product 菜单的 `Open repository` / `New project` 可恢复 `master` 和 Git root 1。浏览器实测隐藏状态与恢复状态均正确，错误日志为空。
- 文件 Diff 补充 IDEA 风格的局部操作条：上一处变更、下一处变更、应用父版本、应用当前版本；操作只更新原型状态栏，明确不写入工作树。浏览器实测新增文件 Diff 的四个入口均可见，应用父版本反馈正确，错误日志为空。
- History 按源码语义改为上下文启用：未选择文件时按钮禁用，选择文件后启用并打开 File History。浏览器实测 `before=false / after=true`，错误日志为空。
- 顶部 Update/Push 现在随分支状态同步禁用：Loading、No project、No VCS、Untrusted project 等受限状态不可执行远程操作，恢复仓库后重新启用。浏览器实测 Loading/No project 均为 disabled，Open repository 后恢复 enabled，错误日志为空。

## 最新审核矩阵（原型层）

| 源码/产品要求 | 当前证据 | 状态 |
| --- | --- | --- |
| 分支状态机与无项目隐藏 | `branchStates`、多 Git root、No VCS/Untrusted、No project 隐藏与仓库恢复 | 已覆盖原型 |
| Update / Push 独立动作 | 独立按钮、忙碌锁、受限状态禁用、无 outgoing 提示 | 已覆盖原型 |
| 文件 History 语义 | 无文件禁用，选中文件后启用 File History | 已覆盖原型 |
| Commit / Stage / Amend 校验 | 文件选择、消息校验、未版本控制文件、成功刷新提示 | 原型覆盖，Hook/真实提交未接入 |
| 提交到文件树到 Diff | 提交切换重建文件树，A/D/R/Binary、Merge parent、Diff 导航 | 原型覆盖，真实 commit 数据未接入 |
| 菜单键盘/焦点/定位 | `role=menuitem`、roving tabIndex、Esc、外部关闭、触发器定位 | 已覆盖原型 |
| 真实 Git 执行链 | commit、fetch、push、checkout、merge、rebase、stash | 未覆盖，需真实 adapter |

本轮审计结论：视觉方案继续保持唯一正式方案；原型行为覆盖显著提高，但不能升级为完整开发验收稿，直至真实 Git adapter 和真实仓库回归证据补齐。

因此此前“文件历史、设置、Shelf、操作状态机尚未进入原型”的审核项已关闭。剩余未关闭项仍是：真实 Git adapter、真实提交/推送/抓取/切换/stash、真实 Diff 数据、真实多仓库状态和真实分支操作。

## 2026-10-21 源码一致性再审核

本次按 Rebased 仓库当前公开源码与 README 的产品定位重新核对，而不是只按截图核对像素：

| 审核面 | 源码/产品事实 | 本原型状态 | 差异与边界 |
| --- | --- | --- | --- |
| 顶部 Git Widget | Git 根状态、加载中、无 VCS、未信任、多根、incoming/outgoing 会改变入口 | 已模拟 | 仍是示例状态，未接真实 VCS 检测 |
| Quick Actions | Update、Push、Commit/Stage、File History、Rollback 是独立动作语义 | 已模拟 | 原型状态反馈，不执行 Git 命令 |
| Log 编辑器定位 | README 将 Log 视为主编辑区，并允许移动到工具窗口 | 已覆盖 | 当前稿固定截图布局，尚未实现可拖入工具窗口 |
| Commit → 文件 → Diff | 选中提交后查看变更文件，再进入 Diff/History | 已覆盖 | 文件集合和 Diff 内容为示例数据 |
| 文件状态 | A/D/R、Binary、submodule、large file 需要区别展示 | 已补充 | 当前只展示代表性状态与元数据，未读取真实文件模式/大小 |
| Merge commit | Diff 必须能选择 Parent 1/Parent 2 | 已覆盖 | 父提交列表为示例，不解析真实拓扑 |
| 键盘与菜单 | 菜单需要定位、焦点、Esc、方向键和右键上下文动作 | 已覆盖 | 未接 IDE action system 与真实快捷键映射 |

本轮新增的文件元数据演示包括：图片/图标的 binary 与大小提示、`Platform.kt` 的 submodule 提示、重命名文件的状态说明。它们用于明确产品设计应保留的语义，不代表已经获得真实仓库元数据。

同时将 README 中的三个设置入口改为有状态的原型控件：Log 主编辑区/工具窗口二选一、项目设置是否移出 `.idea`、是否启用索引；菜单会保留当前选择并以勾选状态反馈，而不是一次性状态栏提示。

Commit 闭环也进一步补齐了原型层的刷新语义：Commit 或 Commit and Push 完成后会清空提交选择、清空提交消息、将 Changes/Unversioned Files 计数归零，并显示工作区已刷新；这仍然是模拟刷新，不代表真实 commit、hook 或 push 已执行。

为避免原型行为与后续实现继续漂移，新增[Git 行为契约](/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/docs/design/rebased-git-behavior-contract.md)，固定 Git 根状态、远程动作阶段、CommitSnapshot、文件状态、History 语义和错误/焦点边界。

同时新增 `scripts/check-rebased-prototype.mjs`，对正式原型的关键入口、状态机、Commit 检查、Merge parent、文件状态和设置状态做静态契约检查；当前结果为 `PASS (10 markers, script valid)`。

审核结论：当前稿已经可以作为“源码对照型视觉与交互原型”，但不能称为 Rebased 的完整实现。下一阶段若进入开发，优先级应为：

1. 以统一 Git adapter 提供真实 root/branch/commit/file status 数据；
2. 用真实 commit tree 驱动右侧树和 Diff parent；
3. 将 Update/Fetch/Push/Checkout/Commit/Shelf 接入命令执行与错误状态；
4. 再实现 Log 编辑器与工具窗口之间的可移动布局。
