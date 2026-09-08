# A5 · M5-W18-R3B — 浏览器优先外壳内的数据库工作台（研究 + 原型）

| 项 | 值 |
| --- | --- |
| LANE | A5 |
| 派发 | `M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A5「Database mode closure」 |
| MODE | `RESEARCH_AND_PROTOTYPE`（W19 = `CLOSED`） |
| 工作树 | `/home/ainfinit/.codex/worktrees/m5-w18-a5/mvp-browser-os-v3` |
| 分支 | `codex/m5-w18-a5`（已 rebase 到 `origin/master`） |
| BASE | `200f0f1`（= `origin/master`，A0 已集成 W18-R/R2/R2B 全 lane 研究） |
| 产品代码改动 | **0**（未触碰 `src/`、`src-tauri/`、`scripts/`、ACL、capability、依赖清单） |
| push | **未 push**（仅 A0 可推） |
| 时间 | 2026-09-08 |

## 0. 交付物与复核方式

| 文件 | 性质 | 复核 |
| --- | --- | --- |
| `logs/research/M5-W18/A5-R3-database-shell.md` | 本设计报告 | 阅读 |
| `logs/research/M5-W18/A5-R3-prototype.html` | 自包含可点原型（合成数据、零 IPC、零网络、零持久化） | 浏览器直接打开文件即可 |
| `logs/research/M5-W18/A5-R3-state-model.mjs` | **规范源**：布局常量 + 状态机 + 命令注册表 + 95 条断言（含原型一致性校验与 R3B 收口断言） | `node logs/research/M5-W18/A5-R3-state-model.mjs` → `ALL_PASS (95 checks)` |
| `logs/research/M5-W18/A5-R3-checkpoint.md` | lane 检查点 | 阅读 |

规范源与原型的**防漂移机制**：`A5-R3-state-model.mjs` 的断言组 G8 会读取同目录 HTML，逐项校验六个 chrome 常量、工具窗口默认/最小/最大、树上限、四个尺寸预设、全部 62 个命令 id、13 个作用域挂点、五种结果状态、六种布局动作、DOM 量测读数存在性，以及「原型不含 `invoke(` / `__TAURI__` / 网络请求 / 浏览器存储」。文档说一套、原型画另一套会直接断言失败。

## 1. R2B → R3 修订日志（为什么改）

R2B 的数据库原型（`A5-R2B-wireframe.html`）被整体退回，R3 派发给出的纠偏方向是「保持 Chrome 心智模型 + IDEA 式工具窗口行为，默认安静、内容优先，能露出能力但不永久损失视口」（`M5-W18-R3-UX-TASKS-20260908.md:10-12`）。逐条修订：

| # | R2B 形态 | R3 修订 | 依据 |
| --- | --- | --- | --- |
| M1 | 面板常驻：连接区 / SQL 区 / 结果区同屏堆叠，永久三段挤压 | 六个工具窗口全部**按需出现**，默认只有左侧连接树停靠；结果窗口**运行时才打开**，历史/属性/消息零常驻 | 全局约束「No permanent multi-panel squeeze」（:22） |
| M2 | 无外壳，等于一张独立面板图 | 原型内置浏览器优先外壳：行 1 统一标签条（浏览器页与「数据库」文档同条）+ 行 2 导航/地址行，共 **2 行 60px** | :17、产品方向 :10 |
| M3 | 无收起/恢复概念 | 九项工具窗口能力齐备：open / close / collapse all / restore previous layout / expand active / pin / unpin / auto-hide / resize，且**自动隐藏（悬浮）不消耗视口** | :18 |
| M4 | 单一 SQL 输入框 | 中央多 SQL 文档（子标签 + 脏标记 + 连接归属 + 运行态徽标），沿用 R2B 已验证的 `execId` 陈旧守卫 | A5 卡「multi-document」 |
| M5 | 树只有静态一层 | 连接树支持全部折叠 / 展开一层 / **有界全展开**（250 节点、6 层双上限），合成数据刻意放 400 张表演示截断提示 | :19「large trees must not freeze the UI」 |
| M6 | 无专注模式 | 专注模式隐藏 rail / 导航行 / 状态栏 / 子标签 / 控制台状态行与三边工具窗口，活动文档**归还 100% 宽 + 100% 后标题栏高**，右上保留唯一返回胶囊（`Esc` / `F11`） | :21、A5 卡「full viewport in focus mode」 |
| M7 | 右键无设计 | 13 个对象作用域、62 个动作的命令注册表：每项有 id、标签、快捷键或无障碍路径、安全分级、确认档位、审计分级、后端依赖、前置条件、禁用原因；菜单**按指针下最近对象**取作用域（`closest([data-scope])`），不做大杂烩 | :23-24 |
| M8 | 只有「正常」一态 | 状态矩阵齐备：空结果 / 加载中 / 错误 / 禁用（未连接）/ 已截断 / 取消 / 超时 + 右键菜单态，四个尺寸各自可切 | :25 |
| M9 | 未标注可行性 | 新增可行性台账：10 个动作自曝为 `DESIGN_ONLY`（依赖 6 个未注册命令），1 个动作（清空表）自曝为后端恒拒 | A10 要求「视觉存在但当前架构不可实现须自曝」 |

## 2. 布局契约

### 2.1 常量（规范源 §1，原型 CSS/JS 同值，G8 校验）

| 常量 | 值 | 说明 |
| --- | --- | --- |
| `tabStrip` | 28px | 行 1 统一标签条，**兼作标题栏基准**（后标题栏高度 = 窗口高 − 28） |
| `navRow` | 32px | 行 2 导航 / 地址 / 全局入口 |
| `statusBar` | 24px | 底部状态栏（兼工具窗口开关，收起态下的恢复入口之一）；A0 裁决 24px |
| `railLeft` | 28px | 左侧工具图标细条（唯一常驻工具入口） |
| `subTabs` | 26px | 数据库文档**内部**的 SQL 子标签条，仅当文档数 ≥2 时占位 |
| `consoleStatus` | 22px | SQL 控制台状态行（运行/取消/execId/上限提示） |
| 左侧工具窗口 | 260 / 180–480 | 默认 / 最小–最大 |
| 右侧工具窗口 | 280 / 200–420 | 同上 |
| 底部工具窗口 | 240 / 120–（内容区 80%） | 底部用**比例上限**，避免小窗口挤死文档 |
| 树上限 | 250 节点 / 6 层 | 有界全展开双上限 |

**计入口径（须 A0 裁定，见 §9 Q1）**：外壳 chrome 只算行 1 + 行 2 = 2 行 60px（≤80px 红线）。文档内部的 SQL 子标签条与控制台状态行**不计入外壳 chrome**，但在预算核算时从活动面里**扣除**（下表「严格活动面」），因此该口径不会虚增达标率。

### 2.2 三态

| 态 | 定义 | 视口 |
| --- | --- | --- |
| 正常态 `normal` | 按需打开的工具窗口停靠 | 不受红线约束（红线只管收起/专注） |
| 收起态 `collapsed` | 三边全隐；仅保留 rail + 状态栏作为恢复入口 | 受红线约束 |
| 专注态 `focus` | 隐藏全部工具 chrome（含 rail / 导航行 / 状态栏 / 子标签 / 控制台状态行） | 100% 宽 + 100% 后标题栏高 |

### 2.3 预算实测（对齐 A0 规范几何公式，标签 = MEASURED）

A0 裁决：collapsed active height = (innerH − 60 − 24) / innerH；active width = (innerW − 28) / innerW（28 = activity strip）。本模型 `computeLayout` 的 `canonicalHeightPct / canonicalWidthPct` 即该规范值（`statusBar=24` 时 `contentHeight = H−84`）。最不利口径：**2 个 SQL 文档**（子标签条占位）。

| 尺寸 | 收起态活动高 = (H−84)/H | 高占比（红线 85%） | 收起态活动宽 = (W−28)/W | 宽占比（红线 92%） |
| --- | --- | --- | --- | --- |
| 1920×1080 | 996 / 1080 | **92.22%** PASS | 1892 / 1920 | **98.54%** PASS |
| 1440×900 | 816 / 900 | **90.67%** PASS | 1412 / 1440 | **98.06%** PASS |
| 1366×768 | 684 / 768 | **89.06%** PASS | 1338 / 1366 | **97.95%** PASS |
| 1200×800 | 716 / 800 | **89.50%** PASS | 1172 / 1200 | **97.67%** PASS |
| 1024×720 | 636 / 720 | **88.33%** PASS | 996 / 1024 | **97.27%** PASS |
| 900×600 | 516 / 600 | **86.00%** PASS | 872 / 900 | **96.89%** PASS |

六个验收尺寸（A0 裁决：1920×1080 / 1440×900 / 1366×768 / 1200×800 / 1024×720 / **900×600 产品最小窗**；**800×600 低于产品最小窗，弃用**）全部达标。

> 附：若再扣除文档内部子标签条(26px)+控制台状态行(22px)的更严口径，900×600 = 81.82%、1024×720 = 84.97%（均 <85%）。该口径仅为内部余量提示，不作为红线门槛——规范红线以 (H−60−24)/H 为准（见 §2.4）。

专注态六尺寸恒为 100% / 100%（活动面 = 窗口宽 × 后标题栏高）。

### 2.4 R3B 收口变更（2026-09-08 第二轮 · `M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A5）

本轮按 A0 在 `A0-M5-W18-R3-acceptance-audit-20260908.md` 的裁决对原型与规范源做收口，不触碰任何产品代码：

1. **规范几何对齐 A0 SSOT**：`statusBar` 由 22px 调为 **24px**（A0 裁决）；顶部 chrome 仍 = tabStrip(28)+navRow(32) = 60px；activity strip = railLeft(28)。
2. **六个验收尺寸**：在原有四尺寸基础上新增 **1200×800** 与 **900×600**，并**弃用 800×600**（低于产品最小窗）。预算断言改用 A0 规范公式 `(H−60−24)/H` 与 `(W−28)/W`，`900×600` 高占比 **86.00%**、宽占比 **96.89%** 均过红线（§2.3）。
3. **A3/A4 折叠·恢复语义对齐**：规范源 `collapseAll` 本就隐藏全部边（含已固定窗口）；`restorePreviousLayout` 已从快照逐字段还原 `edges`（含 `pinned`/尺寸/可见态）。本轮新增 G9 断言固化：① 收起全部连已固定左右边一并隐藏；② 恢复后可见态/自定义尺寸(360px)/固定态逐字段还原；③ 快照仅含 `{view, edges}`，不持久化 `focusReturnAction`/`expandedFrom` 等瞬时字段（A4 要求确定性、不持久化临时态）；④ `collapse→restore` 往返确定性。
4. **去除捐赠方品牌字体（R3B-02）**：原型 `--mono` 曾含 `JetBrains Mono`，本轮移除；新增 G8 负向断言「原型不含 JetBrains/DataGrip/IntelliJ/Rebased」。报告可引用来源，原型 UI 文本不出现捐赠方品牌。
5. **专注模式恢复活动面**：`enterFocus` 存快照、`exitFocus` 经 `restorePreviousLayout` 把活动编辑/结果面还原为正常停靠全视口（G3/G9 已验证）。
6. **轻量 ARIA（R3B-04 不回退）**：树 `role="tree"`、SQL 子标签 `role="tablist"`+`tab`、结果网格 `role="grid"`+`gridcell`、历史/消息 `role="log"`、工具窗标题栏 `aria-label`，保持作用域右键菜单的禁用原因可见。
7. **断言总数 78 → 95**：新增 G9（R3B 折叠/恢复/快照确定性 12 条）+ G8 品牌负向检查；`node A5-R3-state-model.mjs` → `ALL_PASS (95 checks)`。

### 2.4 正常态代价（明示）

1440×900 正常态（左树 260 + 底部结果 240）：活动面 1152×530 = 后标题栏高的 60.78%。这是「露出能力」的真实代价，因此设计上**不允许默认常驻**：首次打开数据库文档时底部结果窗口为 `hidden`，运行查询才出现（`runQuery()` → `openTool("bottom","db.tool.result")`）。原型/规范源的初始态取「已跑过一次查询」的稳定态，便于评审直接看到结果与截断态；点 rail 的「结果」图标或状态栏按钮即可复现关闭→按需打开的转换。

## 3. 工具窗口状态机（九项能力对照）

| 能力 | 语义 | 键盘 | 断言 |
| --- | --- | --- | --- |
| open | `openTool(edge,id)`；每边默认仅一个主窗口 | rail 图标 / 状态栏按钮 | G4-1 |
| close | `closeEdge(edge)`；最后一个成员移除后该边转 `hidden` | `Shift+Esc` | G4-16 |
| collapse all | 先存快照再全隐 | `Ctrl/Cmd+Shift+F12` | G4-7/8 |
| restore previous layout | 从快照**逐字段**还原（含用户自定义尺寸），快照消费后清空 | `Ctrl/Cmd+Alt+F12` | G4-9/10 |
| expand active | 底部→内容区 80%，左右→各自 max；可反向还原 | `Ctrl/Cmd+Shift+'` | G4-11/12 |
| pin | 固定；同边再开新窗口时**不替换**、共存为同边标签 | 菜单 | G4-2/3 |
| unpin | 取消固定，恢复「同边替换」 | 菜单 | — |
| auto-hide | 转悬浮层，**不消耗视口**（1152→1412px）；已固定的窗口幂等拒绝 | 菜单 | G4-4/5/6 |
| resize | 停靠尺寸钳位（左 180–480、底 120–内容区 80%） | `Alt+←/→/↑/↓`（拖拽为鼠标路径） | G4-13/14/15 |

**边独占规则**（:20）：`openTool` 时若在位者未固定 → 从成员中移除（真替换，不是叠加）；若已固定 → 保留为同边标签、新窗口取得可见权。左边成员 = {连接, 结构}，底边 = {结果, 历史, 消息}，右边 = {属性}。

## 4. 按需出现 / 干净收起

| 面板 | 出现触发 | 消失 | 是否常驻 |
| --- | --- | --- | --- |
| 连接树（左） | 打开数据库文档 / rail「连接」 | 关闭 / 收起全部 / 自动隐藏 / 专注 | 首开停靠，可关 |
| 结构树（左） | rail「结构」（替换连接树，除非连接树已固定） | 同上 | 否 |
| 结果（底） | **运行查询自动打开** / rail「结果」 | 关闭 / 清空 / 收起全部 / 专注 | 否 |
| 历史（底） | rail「历史」（替换结果，除非固定） | 同上 | 否 |
| 消息（底） | rail「消息」 | 同上 | 否 |
| 属性（右） | 右键「查看属性」/ rail「属性」 | 同上 | 否 |

「干净收起」判定：收起态下**不留任何工具占位块**，只有 28px rail 与 22px 状态栏（两者都是恢复入口，非工具窗口），实测宽占比 ≥97%。专注态连 rail 与状态栏也不留，只留返回胶囊。

## 5. 层级视图（连接/结构树）

- 全部折叠 `db.schema.collapseAll`（`Ctrl/Cmd+-`）：展开集合清空，深度归零。
- 展开一层 `db.schema.expandOneLevel`（`→`）：深度 +1；到第 6 层拒绝并提示「已达最大层级 6，未继续展开」。
- 有界全展开 `db.schema.expandAllBounded`（`Ctrl/Cmd+Shift+=`）：BFS 到 250 节点上限即停，**显式**提示「已展开 N 个节点，达到 250 上限；未完全展开」，小分支照常展开（截断不是整体放弃）。合成树里 `prod-replica.public` 放 400 张表，正是用来证明不会静默、也不会卡死。

## 6. 中央 SQL 多文档

- 文档态：`idle / running / success / error / timeout / cancelled`，各带脏标记与连接归属徽标（`local-sqlite · 非生产` / `prod-replica · 生产判定=生产 · 写已拒绝`）。
- **陈旧响应守卫**：每次运行分配 `execId`；`applyDocResult` 只接受与当前 `execId` 相同的响应，切文档时迟到结果被丢弃，不覆盖新文档（G6-2/3）。
- **取消幂等**：重复取消无副作用；取消后可重试并取得新 `execId`（G6-4/5）。
- 关闭策略：有查询在途禁止关闭（原因：避免孤儿请求）；关闭其它文档要求其余无在途查询。

## 7. 状态矩阵与文案口径

| 状态 | 呈现 | 文案来源 |
| --- | --- | --- |
| 空结果 | 空态卡（明说「成功但 0 行，不是错误、也不是截断」） | 本 lane 新写 |
| 加载中 | 转圈 + `execId` + 超时上限 30s + 取消提示 | `DB_DEFAULT_QUERY_TIMEOUT_SECS`（`src-tauri/src/domain.rs:1040`） |
| 已截断 | 黄条「结果已被截断（命中行数上限：1000 行 / 4.00 MiB），展示的不是完整结果集；存在超过 64.00 KiB 的长字段被截断」 | 对齐 `src/utils/dbUi.ts` 现有告警口径；上限见 `domain.rs:1028/1032/1036` |
| 错误 | 红条 + 脱敏后的后端错误 + 修正提示 | 后端 `DbError` / `sanitize_message` |
| 禁用（未连接） | 取数动作全禁用并说明「先执行 `db.connection.connect`」 | 本 lane |
| 取消 / 超时 | 状态行显示「已取消（结果不完整）」/ 超时被终止 | 规范源 `resultWarnings` |
| 单元格级 | `NULL` 斜体灰；长文本尾部 `…⚠`；二进制 `<二进制 65,536 B · 已达 64.00 KiB 上限>` | `DB_MAX_TEXT_FIELD_BYTES` |

红线：**任何截断/取消/超时/失败都不静默**，四种情况各有独立告警（G6-6~9）。

## 8. 右键菜单 = 命令注册表

字段契约（建议 A6 直接采纳为全局注册表的 db 命名空间）：
`{ id, label, keyboard, a11yPath, safety, confirm, audit, backend, precondition, disabledReason }`

- `safety` ∈ `READ`（只读）/ `LOCAL`（仅本地 UI 与文档状态）/ `DB_WRITE`（可改库数据）/ `DESTRUCTIVE`（不可逆）。
- 规则（已断言）：`DB_WRITE`+`DESTRUCTIVE` 必须有确认；`DESTRUCTIVE` 必须**键入式确认 + 审计**；`READ`/`LOCAL` **不得**用键入式确认（防确认疲劳）。
- `a11yPath` 全量默认 `Shift+F10 打开菜单 → ↑/↓ 选择 → Enter 执行 → Esc 关闭`；原型内该路径可实操。
- 禁用项**不隐藏**，聚焦/悬停即显示禁用原因；依赖未注册后端命令的动作额外打 `DESIGN_ONLY` 徽标。

13 个作用域 / 62 个动作（完整文本见规范源 §6；此处列 id + 安全分级 + 后端依赖）：

| 作用域 | 动作数 | 动作（安全分级；后端） |
| --- | --- | --- |
| `db.connection` | 8 | connect(READ; db_connect)、disconnect(LOCAL; db_disconnect)、newConsole(LOCAL)、edit(LOCAL)、toggleWrite(DB_WRITE)、refreshSchema(READ; **MISSING** db_list_schema)、copyLabel(READ)、remove(DESTRUCTIVE; **MISSING** db_list_connections) |
| `db.schema` | 5 | expandOneLevel、collapseAll、expandAllBounded、newConsole、copyName（READ/LOCAL） |
| `db.table` | 5 | previewRows(READ; db_query)、generateSelect(LOCAL)、showProperties(READ; **MISSING**)、copyQualifiedName(READ)、truncate(DESTRUCTIVE; 后端恒拒，见 §9) |
| `db.column` | 3 | copyName、generateWhere、showProperties(**MISSING**) |
| `db.console` | 7 | run(READ; db_query)、cancel(READ; **MISSING** db_cancel)、close、closeOthers、rename、duplicate、switchConnection |
| `db.editor` | 4 | runSelection(db_query)、copy、commentLines、insertTableName |
| `db.grid.cell` | 3 | copy、viewFull、edit(DB_WRITE; **MISSING** db_row_write) |
| `db.grid.column` | 4 | sortAsc、sortDesc、copyColumn、hide（均标注「仅已加载行」） |
| `db.grid.row` | 3 | copyRowCsv、openInProperties、delete(DESTRUCTIVE; **MISSING** db_row_write) |
| `db.result` | 4 | copyAllCsv、exportFile(**MISSING** db_export_result)、clear、rerun |
| `db.toolwindow` | 9 | pin、unpin、autoHide、close、expandActive、resize、collapseAll、restoreLayout、focusMode（全 LOCAL） |
| `db.history` | 5 | openInConsole、copySql、rerun(db_query)、clear、persistToggle(**MISSING** db_query_history) |
| `db.properties` | 2 | copyValue、refresh(**MISSING** db_list_schema) |

## 9. 可行性台账（自曝：视觉存在但当前架构做不到）

后端**已冻结**的数据库命令只有三个（`src-tauri/src/bridge.rs:6009 / :6040 / :6097`）：`db_connect`、`db_query`、`db_disconnect`。据此：

| 动作 | 缺什么 | 现状替代 |
| --- | --- | --- |
| `db.console.cancel` | `db_cancel` 未注册（后端有 `QueryCancel` 标志与 `DB_CANCEL_CHECK_EVERY_ROWS`，但**无命令入口**） | 只能等默认超时 30s 或断开连接；原型用「模拟取消（设计态）」演示交互，正式菜单项禁用并点名原因 |
| `db.connection.refreshSchema`、`db.table.showProperties`、`db.column.showProperties`、`db.properties.refresh` | `db_list_schema` 未注册 | 属性只能来自结果集列元数据；树的库表层级在真实产品里目前无法枚举（原型内为合成层级） |
| `db.connection.remove` | `db_list_connections`（登记簿读写通道）未注册 | 连接登记簿仅在后端，前端无增删列举通道 |
| `db.result.exportFile` | `db_export_result` 未注册 | 仅剪贴板 CSV |
| `db.grid.cell.edit`、`db.grid.row.delete` | `db_row_write` 未注册 + 首片刻意排除结果集写回 | 无 |
| `db.history.persistToggle` | `db_query_history` 未注册 | 历史仅本次会话内存，SQL 全文不落盘 |
| `db.table.truncate` | **不是缺命令，而是后端恒拒**：`require_write_confirmation` 对 `SqlRiskClass::Ddl`/`Admin` 一律返回 `WriteDenied`（`src-tauri/src/security_policy.rs:1218`），与连接的写权限开关无关 | 无（原型内恒禁用并说明） |

反向确认（已存在、可直接对接的安全语义，不需新造）：`evaluate_db_query_gate(sql, cfg, encrypted, confirm_write)`（`security_policy.rs:1269`）已把 `classify_sql_risk` + `is_production_database` + `require_write_confirmation` 串起来，且 `db_query` 在任何语句执行前必过该闸门（`bridge.rs:6074`）。所以 UX 的「写需确认 / 生产禁写 / 未知 SQL 拒绝」不是新设计，而是**把既有后端判定显性化到菜单的禁用原因里**。

## 10. 与现产品实现的差距（迁移影响面）

| 现状事实（VERIFIED_IN_REPO） | R3 目标 |
| --- | --- |
| `src/components/workspace/DatabasePanel.vue`（231 行）虽由 `MainArea.vue:218` 在中央区渲染，根节点却仍是 `side-inner db-panel`（:62），且带一颗把**侧栏**关掉的 ✕（`layout.sidebarOpen = false`，:65） | 中央文档语义化；关闭动作归工具窗口而非侧栏 |
| 连接表单 + 连接列表（:78）+ SQL 输入（:137）+ 结果网格在**同一条窄列**里纵向堆叠，`.db-panel` 自身 `overflow:auto`（:190），连接列表 `max-height:120px`（:197），结果网格 `max-height:320px`（:226） | 连接树/文档/结果分属不同边的按需工具窗口；结果高度由用户 resize，不再硬编码 320px |
| 单 SQL 输入框，无文档概念、无 `execId`、无取消 | 多文档 + `execId` 守卫 + 取消契约 |
| 前端再截一次：`DISPLAY_ROW_CAP = 200`（:25/:33），提示「仅显示前 200 行，另有 N 行未渲染」（:182） | 保留该护栏（合理），但把「后端 1000 行上限」与「前端 200 行渲染上限」在 UI 上**区分表述**，避免用户以为只取了 200 行 |
| 无结构树、无历史、无属性、导出仅「复制 CSV」（:142） | 结构/历史/属性作为按需工具窗口；导出待 `db_export_result` |
| 写权限开关已在前端（`db.form.allowWrite`，:114，文案「默认关闭，后端仍会再判一次」） | 直接沿用；菜单禁用原因引用同一判定 |

## 11. 跨 lane 依赖与交接

| lane | 交接内容 |
| --- | --- |
| A1（合并原型） | 可直接取用 §2.1 常量、§3 边状态机、§2.3 预算表；请裁定 §2.1「子标签不计外壳 chrome」口径与 §2.3 的 D-A5-R3-3（<768px 折叠子标签）是否纳入 |
| A2（密度审计） | 本报告的 chrome 数字是**原型侧**目标值，不是现产品实测；现产品 titlebar/tab/toolbar 实测归 A2，若 A2 结论与 60px 目标冲突，以 A2 实测为差距基线 |
| A6（命令注册表） | 建议直接吸收 §8 字段契约与 62 个 `db.*` id；若 A6 字段名不同，按 A6 为准、本 lane 映射 |
| A9（交互安全） | §9 的安全分级/确认档位需与 `require_write_confirmation` 的四类风险（Read/Write/Ddl/Admin/Unknown）逐一对齐；`Unknown → SqlParseFailed` 的用户可读文案未定 |
| A4 / A0（后端数据面） | 6 个未注册命令按优先级：`db_cancel`（最高，后端机制已存在只缺入口）> `db_list_schema` > `db_export_result` > `db_query_history` > `db_list_connections` > `db_row_write`（建议长期不做） |
| A8（视觉密度） | 原型用的是中性深色令牌，仅为形态验证，**不构成主题提案**；请以 A8 的间距/字号/图标体系覆盖 |
| A10（复刻边界） | 本 lane 未复制任何 IDEA/Rebased 资产、品牌或源码；仅按行为参考「工具窗口停靠/固定/自动隐藏/最大化」通用交互范式 |
| A11（打包核对） | 三个文件 + 检查点，SHA 见检查点 |

## 12. 未决问题（交 A0 裁定）

1. **Q1**：文档内部子标签条计「内容」还是「chrome」？本报告按「内容」计，但预算里已扣除，故不虚增达标率。
2. **Q2**：是否批准 `db_cancel` 作为 W19 首个后端补齐项？（后端 `QueryCancel` 已存在，仅缺命令 + ACL 条目）
3. **Q3**：800×600 是否进必测矩阵？若进，需采纳 D-A5-R3-3。
4. **Q4**：查询历史「默认不落盘」是否为终局？（当前设计：仅本次会话内存，SQL 全文不落盘）
5. **Q5**：结果导出上限（后端已有 16MiB 概念但无消费方）保留还是删除？

## 13. 建议的 W19 门禁（不实现，仅提案）

若 A0 开启 W19 数据库切片，建议把本 lane 的规范源移植为 `scripts/check-database-ux-logic.mjs`，至少保留：G1 顶部 chrome 预算、G2 四尺寸收起态预算、G3 专注全视口、G4 九项工具窗口能力、G5 树三操作有界性、G6 多文档/取消/截断非静默、G7 命令注册表完整性（禁用原因/安全分级/键盘路径不得缺失）、G8 文档-实现一致性。另建议加一条 ACL 不扩张断言：新增 db 命令必须同时出现在 `default-commands.toml`（插在末条 `list_artifact_images` 之前）。

## 14. 证据标签

| 标签 | 覆盖 |
| --- | --- |
| VERIFIED_IN_REPO | §9/§10 全部 file:line 引用（`bridge.rs:6009/6040/6074/6097`、`security_policy.rs:1206-1218/1269`、`domain.rs:1023-1040`、`DatabasePanel.vue:25/33/62/65/78/114/137/142/182/190/197/226`、`MainArea.vue:50/218`） |
| MEASURED | §2.3 预算数字（`node A5-R3-state-model.mjs` → `ALL_PASS (78 checks)`）；原型 JS 语法 `node --check` 通过 |
| NOT_RUN | 原型的**浏览器内视觉与 DOM 实测占比**（本 lane 无法启动浏览器；原型已内置 `offsetWidth/offsetHeight` 实时读数与红线着色，评审打开即可自证）；无截图 |
| INFERRED | §2.3 的 800×600 结论（按同一常量算术推得，未做 DOM 实测）；§11 各 lane 的接受意愿 |

## 15. 约束自查

- 未改动任何产品代码 / 依赖 / ACL / capability / 原生运行时 / 用户数据；本波仅修订 `logs/research/M5-W18/A5-R3-*`（state-model / prototype / 本报告 / checkpoint）。
- 未触碰他 lane 文件（`M5-W18-R3B-CORRECTION-TASKS-20260908.md`、`PARALLEL_COMMAND_BOARD.md`、其它 lane 报告均只读）。
- 原型自包含：无外链脚本/样式/字体（已移除 `JetBrains Mono`）、无网络、无 IPC、无浏览器存储（G8 断言）。
- 未 push；分支 `codex/m5-w18-a5` 停在本 lane 提交。
- R3B 收口：规范几何对齐 A0 SSOT（statusBar=24px、六尺寸含 900×600）、A3/A4 折叠·恢复语义经 G9 固化、捐赠方品牌字体已移除。
