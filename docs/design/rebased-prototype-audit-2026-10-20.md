# Rebased 产品原型审核（2026-10-20）

对象：`rebased-product-prototype.html`。本审核只评价 HTML 原型，不评价 BrowserOS 已安装客户端。原型是示意数据，不接真实 Git 仓库。

## 证据边界

- Rebased 官方 README：基于 IntelliJ Platform，保留其 Git 功能；Git Log 默认位于主编辑区，可在 Settings > Version Control > Log 调到底部。还说明了项目配置目录和文件索引选项。<https://github.com/DetachHead/rebased/blob/master/README.md>
- Rebased Releases：截至审核日显示 1.1.22；1.1.20 提到 Commit Options 面板和 commit checks；1.1.19 提到索引设置及 Rebased 专有设置标识；1.1.16 提到编辑区 Log 标签关闭行为。发布页不提供全部页面及按钮清单。<https://github.com/DetachHead/rebased/releases>
- 官方仓库还公开了 `screenshot.png`，证明项目存在产品截图入口；当前检索工具只能读取其元数据，无法把该图片作为本机像素验收证据。<https://github.com/DetachHead/rebased/blob/master/screenshot.png>
- IntelliJ Git Log 文档：分支、提交、变更文件、提交详情窗格；分支、作者、日期、路径、文本筛选；分支和提交的上下文操作。Rebased 对这些功能的继承来自官方 README 的产品定位，具体外观仍需 Rebased 实机核对。<https://www.jetbrains.com/help/idea/log-tab.html>
- 原型源码已逐项检查。当前没有 Rebased 安装版的逐屏截图；浏览器工具拒绝读取本地 `file://` 页面，像素一致性未验收。为验证交互，使用临时 `127.0.0.1` HTTP 服务打开原型，不等同于 Rebased 实机。

## 评分（76/100，2026-10-20 浏览器复测后）

| 项目 | 权重 | 得分 | 可复核依据 |
| --- | ---: | ---: | --- |
| 页面和布局 | 25 | 18 | 有 Log、Changes、Branches、Stashes、PR/CI、Settings；设置中的“主编辑区/底部工具窗口”已实际切换 `.log-bottom` 和底部高度（HTTP 浏览器验证）；仍缺少 Rebased 实机像素证据。 |
| 提交与文件审查 | 20 | 18 | 复测发现并修复提交联动问题：选中第二条提交后，SHA、文件数量、文件列表和编辑区均同步刷新；仍缺少真实 Git 内容与完整文件历史。 |
| 按钮行为 | 20 | 16 | 提交筛选、暂存/取消暂存、冲突继续、Fetch/危险操作预览和 Interactive Rebase todo 入口已通过原型检查；真实 Git 写操作仍未执行。 |
| 分支、暂存架、PR/CI | 15 | 9 | Branches 页面、Interactive Rebase todo 预览和操作入口已补齐；真实分支结果、PR/CI 检查仍未形成闭环。 |
| 冲突解决 | 10 | 8 | 浏览器复测已修复状态判断：接受冲突块后计数为 0，继续按钮进入 Merge/Rebase 预览；仍缺少真实 Git 操作与保存结果。 |
| 状态与可访问性 | 10 | 7 | `ahead 0` 时 Push 禁用，主标签和分隔器有基本语义；部分数量与示例数据不一致。 |
| **合计** | **100** | **78** | **仍不通过 97 分目标**。 |

评分是证据限定的设计与交互分，不是视觉还原百分比，也不是对真实软件功能完整性的抽样统计。

## 按钮和页面核对

| 区域 | 已画出 | 未闭环或无法证实 |
| --- | --- | --- |
| 顶部仓库操作 | Fetch、Pull、Push、更多、当前分支 | Fetch/Pull 只改提示；更多菜单不打开；没有真实状态反馈。Push 在 ahead 0 禁用是正确行为。 |
| Log | 文本检索、筛选区、提交图、文件列表、三种查看模式 | 筛选区的分支/作者/日期/路径/Hash 控件未参与查询；提交选择不联动文件和 Diff；提交图为重复的固定 SVG。 |
| 提交操作 | 比较、Cherry-pick、Revert、Reset、Tag、文件历史、Blame | 只有提示语，没有目标、差异预览、模式选择或结果状态；上下文操作未按当前提交状态禁用。 |
| Changes | 已暂存/未暂存分组、并排差异、暂存/取消/放弃/提交 | 文件选择不切换 Diff；按块暂存没有块选择；分组数量与可见行数不符；提交信息及 checks 面板缺失。 |
| Branches | 本地/远程、切换、比较、拉取、新建、Merge、Rebase | 新建输入不参与按钮结果；比较视图和合并/变基预览缺失；分支操作状态不联动 Log。 |
| Stashes | 新建、应用、弹出、删除、查看 | 无所选 stash 的 Diff、作用结果、删除确认或列表更新。 |
| PR/CI | 检查汇总、可合并状态、打开/创建 PR | 缺少 PR 详情、具体检查项和状态变化；此区更像 BrowserOS 扩展，不能仅凭 Rebased Releases 证明为 Rebased 原生页面。 |
| Settings | Log 位置、自动 Fetch、索引、项目配置、Git/SSH/Commit/Diff | 勾选项未改变原型行为；专有设置未标识；各子设置按钮只改提示。 |
| Conflict | 当前/编辑/目标三栏、接受块、返回、中止、继续 | 冲突状态与继续按钮不一致；中止和继续无预览；编辑结果未建立保存/标记解决流程。 |

## 修复顺序

1. 使 Log 默认主编辑区成立，并让设置中的位置选项真正切换布局。
2. 建立提交 → 变更文件 → 编辑区的单一状态模型；文件切换、筛选和 Diff 从同一份数据生成。
3. 为 Changes、Commit 和冲突流程建立可见的预览、确认及状态转移，不再以状态栏提示代替流程。
4. 补分支/暂存架的选择态、比较结果和操作约束；区分 Rebased 原生功能与 BrowserOS 扩展。
5. 用 Rebased 安装版逐屏核对菜单、按钮、弹窗、设置路径及不同窗口大小，之后再评视觉还原度。

## 验收状态

`PROTOTYPE_REVIEW_FAIL`。本轮补齐了两套原型的 Interactive Rebase todo 预览，并通过脚本复核。Rebased 实机逐屏审核、真实 Git 数据和像素审核仍待完成。不得将本报告的 78/100 表述为完整产品还原度；距离 97 分仍缺实机视觉证据、真实仓库数据闭环和真实 Git 操作结果。
