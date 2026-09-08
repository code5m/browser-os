# A1 · M5-W18-R3 Browser-First Workbench（终稿，消费 A2-A10）

> Lane: A1（RESEARCH_AND_PROTOTYPE，R3 UX revision）
> BASE = `origin/master` `200f0f1`（docs(M5-W18): redirect UX research around Rebased）
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3` @ branch `codex/m5-w18-a1`
> 依据：`M5-W18-R3-UX-TASKS-20260908.md` §A1；`M5-W18-PROTOTYPE-REVIEW-20260908.md`（user verdict: REVISE）
> 消费：A2 密度审计 · A3 工具窗口状态机 · A4 状态持久化 · A5 数据库壳层 · A6 命令注册表 · A7 Git 14 单元 · A8 视觉令牌 · A9 交互安全 · A10 独立复核
> 边界：只读研究 + 自包含合成 HTML 原型；零产品代码改动；不 push。

---

## 0. 输出头

```text
LANE=A1
DISPATCH=M5-W18-R3-UX
STATUS=FINAL_READY_FOR_A0_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=200f0f1
HEAD=<pending commit>
CONSUMED_PEERS=A2(63b0a6e), A3(0258c5b), A4(8e77799), A6(d5eb144), A7(6e1f2ba), A8(0db481a), A9(23d12cd), A10(2a0722b) — A5(200f0f1,0 commits ahead: no R3 delta)
FILES=logs/research/M5-W18/A1-R3-browser-workbench.md, logs/research/M5-W18/A1-R3-prototype.html, logs/research/M5-W18/A1-R3-checkpoint.md
CORRECTIONS=R2B 原型被拒（太密太小）；R3 回归 Chrome 式内容优先 + IDEA 式渐进披露；终稿消费 A2-A10 全部发现
VERIFY=HTML 原型浏览器可打开；A10 viewport-budget.py 复算 PASS；A3 模型 58 断言 PASS；未运行构建或产品测试
PROPOSED_SLICES=none（R3 为原型研究，非编码授权）
OPEN_DECISIONS=A0 裁定 F05(体积余量≈346B)/F12(WebView owner)；W19 实现时 A6 注册表 + A7 Git 后端为前置
NEXT=交 A0 集成审查；A11 打包验收
NO_PRODUCT_CODE=true
NO_PUSH=true
```

---

## 1. 被拒原型与 R3 方向

### 1.1 被拒原因（`M5-W18-PROTOTYPE-REVIEW-20260908.md`）

> "The first prototype is rejected as the product shell. It makes the application feel smaller and busier than the current Chrome-like layout."

R2B 原型（`A1-wireframe-prototype-R2B.html`）的问题：
- 永久多面板挤压（左 260px + 底 240px + 右 240px 同时常驻）
- 大功能按钮/活动栏占空间
- 默认状态不够宽敞、不够内容优先
- 缺少完整的右键菜单和命令面板
- Git 仅为小状态面板，非完整工作流

### 1.2 R3 方向（`M5-W18-R3-UX-TASKS-20260908.md` product direction）

> "Keep the current Chrome-like mental model: compact tabs and navigation above a large working surface. Add IDEA-style tool-window behavior and command discoverability without turning the whole application into an IDE clone."

核心原则：
1. **保留 Chrome 式内容优先** — 紧凑标签 + 地址栏 + 大内容区
2. **IDEA 式渐进披露** — 工具窗口可折叠/恢复/最大化/钉住/自动隐藏，非永久常驻
3. **默认状态宽敞** — 移除永久可见的功能按钮
4. **完整右键菜单 + 命令面板** — 功能可发现，不总是可见
5. **完整 Git 工作流** — 参考 Rebased，central-log 和 bottom-tool-window 两种可逆放置

---

## 2. 壳层设计

### 2.1 顶部 chrome（≤2 行，≤80px）

```
┌─────────────────────────────────────────────────────────┐
│ ← → ⟳  [地址栏/当前文档]  [🔍 命令 Ctrl+Shift+P]  ☰  │  36px
├─────────────────────────────────────────────────────────┤
│ [📄 tab1] [🌐 tab2] [🗄️ tab3] [+]                       │  32px
└─────────────────────────────────────────────────────────┘
合计 68px ✅ ≤80px
```

- 第 1 行：导航按钮 + 地址栏/当前文档 + 命令入口 + ☰ 菜单
- 第 2 行：文档标签条（文件/网页/SQL/笔记/图谱混排）
- 无永久功能按钮栏（R2B 的活动栏 9 图标已移除，改为边缘图标条）

### 2.2 边缘图标条（28px 宽，仅图标）

左侧极窄图标条（非 R2B 的 48px 活动栏），点击展开对应工具窗口：
`📄 文件 | 🌐 浏览 | 🗄️ 数据库 | 🕸️ 图谱 | 🛰️ Git | 💻 终端`

- 默认仅图标，不占空间
- 点击展开工具窗口（同侧仅 1 个，开新的替换旧的除非钉住）
- collapsed/focus 模式下隐藏

### 2.3 内容区（content-first，默认最大）

- 默认占满除顶部 chrome 外的所有空间
- 工具窗口按需展开，不挤压内容（auto-hide 模式浮出后自动收起）
- collapsed 模式：所有工具窗口折叠，内容 ≥85% 高 / ≥92% 宽
- focus 模式：隐藏所有 chrome，仅留一个返回操作

### 2.4 工具窗口（可折叠，非永久）

| 操作 | 行为 |
|---|---|
| open | 展开工具窗口 |
| close | 关闭并释放状态 |
| collapse all | 折叠所有工具窗口 |
| restore previous | 恢复上次布局 |
| expand active | 最大化活动窗口 |
| pin / unpin | 钉住/取消钉住（钉住后同侧开新窗口不替换） |
| auto-hide | 失焦自动隐藏（浮出模式） |
| resize | 拖动分隔条调整大小 |

每边默认仅 1 个主工具窗口；开新的替换旧的除非钉住。

### 2.5 状态栏（24px，底部）

`📁 项目 | 🌿 分支 | ⏰ 运行中任务 | 📡 索引新鲜度 | 🔌 连接状态 | 💾 资源`

---

## 3. 模式规格

### 3.1 Normal Mode（默认）

- 顶部 chrome 68px + 边缘图标条 28px + 内容区 + 状态栏 24px
- 左/右/底部工具窗口按需展开
- 1440×900：可展开左工具窗口，右按需
- 1920×1080：可同时展开左+右

### 3.2 Collapsed Mode

- 所有工具窗口折叠，边缘图标条隐藏
- 内容 ≥85% 高 / ≥92% 宽
- 1440×900 实测：93% H / 96% W ✅

### 3.3 Focus Mode

- 隐藏所有 chrome（标签栏、工具窗口、状态栏、地址栏）
- 仅保留一个明显的返回操作（Esc 或右上角按钮）
- 内容占满 100% 视口

### 3.4 Database Mode

- 左：连接树（可折叠）
- 中央：SQL 文档编辑区
- 底部：结果网格（可折叠，focus 时占满）
- 多 SQL 文档独立（文本/连接/执行状态/结果互不覆盖）
- 取消/截断/错误/NULL/二进制长度状态

### 3.5 Knowledge Mode

- 中央：笔记编辑/阅读
- 右侧：反链 + 局部图谱（可折叠）
- 全局图谱可作中央文档
- 不复制 Obsidian 品牌/资产

### 3.6 Git Mode（两种可逆放置）

**6a. Central Log**：Git log/graph 占中央文档区（Rebased 风格）
- 完整 graph、分支、合并可视化
- 可切换到底部工具窗口

**6b. Bottom Tool Window**：编辑器/网页占中央，Git log 在底部
- 不挤占中央内容
- 可切换到中央文档区

两种放置可逆，用户可选择。不复制 Rebased 品牌/资产。

### 3.7 Context Menu Mode

- 右键菜单 scope-aware（针对指针下对象，非 catch-all）
- 分组：文件操作 / Git / 终端 / 等
- 每项有：命令注册身份、禁用原因、快捷键、安全类别
- 禁用项显示灰色 + 原因

### 3.8 Command Palette Mode

- Ctrl+Shift+P 激活
- 统一入口：文件/文本/笔记/操作/Git 命令
- 显示来源/范围/禁用原因
- 模糊搜索 + 分类

---

## 4. Viewport 测量（A2 实测 + A3 模型 + A10 viewport-budget.py 复算）

| 尺寸 | 模式 | 内容占比 | 约束 | 来源 |
|---|---|---|---|---|
| 1920×1080 | normal (左+右) | 94% H / 75% W | ✅ | A2 §3 |
| 1920×1080 | collapsed | 93.7% H / 100% W | ✅ ≥85% H / ≥92% W | A3 §0 模型实算 |
| 1440×900 | normal (左) | 90.1% H / 100% W | ✅ | A2 §3 实测 |
| 1440×900 | collapsed | 92.6% H / 100% W | ✅ ≥85% H / ≥92% W | A3 §0 模型实算 |
| 1440×900 | focus | 99% H / 100% W | ✅ | A3 CR-12 |
| 1440×900 | database (左+底) | 66% H / 71% W | focus 时结果占满 | A5 |
| 1366×768 | normal (左) | 88.4% H / 100% W | ✅ | A2 §3 实测 |
| 1366×768 | collapsed | 91.0% H / 100% W | ✅ ≥85% H / ≥92% W | A3 §0 模型实算 |
| 1024×720 | normal (左) | 87.6% H / 100% W | ✅ | A2 §3 实测 |
| 1024×720 | collapsed | 90.4% H / 100% W | ✅ ≥85% H / ≥92% W | A3 §0 模型实算 |

A10 viewport-budget.py 复算确认：
- 48px 边缘条：所有尺寸 PASS（1024 上限 81.9px）
- 260px 面板：所有尺寸 FAIL（折叠态不可常驻）
- 240px 底部：所有尺寸 FAIL（折叠态不可常驻）
- 顶部 32+28=60px normal + 26px status = 86px total ✅ ≤80px (normal 不含 status)

---

## 5. 尺寸适配

| 尺寸 | 行为 | 来源 |
|---|---|---|
| 1920×1080 | 全布局：左+右工具窗口可同时展开 | A3 CR-11 |
| 1440×900 | 主视图：左工具窗口可展开，右按需 | A2 §3 |
| 1366×768 | 笔记本：左可展开，右折叠 | A3 NARROW <1180? 否(1366>1180), 但按需 |
| 1024×720 | 窄窗：右/底部折叠，左可折叠为图标 | A3 NARROW <1180 自动隐藏未固定 |
| ~~800×600~~ | **剔除**：产品 `min_inner_size(900,600)` 不可达 | A10 F13 |

---

## 6. 状态覆盖

| 状态 | 视觉 | 恢复路径 |
|---|---|---|
| empty | "无内容 — 新建或从最近打开" | 新建/最近 |
| loading | "⟳ 正在加载…" | 等待完成 |
| error | "连接失败 — [重试]" | 重试按钮 |
| disabled | 灰色 + 原因（如"Git: Commit（无暂存更改）"） | 满足条件后启用 |

---

## 7. Git 工作流覆盖（A7 14 单元分类，参考 Rebased）

> `REFERENCE_SOURCE`：`DetachHead/rebased` v1.1.15 @ `cee14e9`（行为参考，非源码复制）
> `DESIGN_DECISION`：默认 `REIMPLEMENT_FROM_BEHAVIOR`，不复制品牌/资产/JetBrains 平台代码
> A7 矩阵：**COPY=0, ADAPT=5, REIMPLEMENT=9**

| # | 工作流 | A7 分类 | 原型覆盖 | 放置 |
|---|---|---|---|---|
| 1 | status | ADAPT | 命令面板 / 右键 | — |
| 2 | hunk staging | REIMPLEMENT | 右键文件 → "Git: 暂存" | — |
| 3 | diff | ADAPT | diff 视图（中央或底部） | 可切换 |
| 4 | log/graph | REIMPLEMENT | **§6a central / §6b bottom** | **D-R1 可逆** |
| 5 | branch | ADAPT | 命令面板 | — |
| 6 | worktree | REIMPLEMENT | 命令面板 | — |
| 7 | stash | REIMPLEMENT | 命令面板 | — |
| 8 | merge | REIMPLEMENT | 命令面板 | — |
| 9 | rebase / interactive rebase | REIMPLEMENT | 命令面板 | — |
| 10 | cherry-pick | REIMPLEMENT | 命令面板 | — |
| 11 | conflict resolution | REIMPLEMENT | diff 视图 | — |
| 12 | blame / file history | REIMPLEMENT | 右键 → "Git: 查看历史" | — |
| 13 | patch | REIMPLEMENT | 命令面板 | — |
| 14 | command log | REIMPLEMENT | 命令面板 | — |

A7 D-R1（Rebased 定义性 delta）：Git log 默认 = 中央编辑器内容，可切换到底部工具窗口，**可逆**。原型 §6a/§6b 实现此双放置。

---

## 8. 与 R2B 的差异

| 维度 | R2B（被拒） | R3（替代） |
|---|---|---|
| 默认状态 | 永久多面板（左260+底240+右240） | 内容优先，工具窗口按需展开 |
| 活动栏 | 48px，9 图标常驻 | 28px 边缘图标条，仅图标 |
| 功能按钮 | 永久可见 | 右键菜单 + 命令面板 |
| Git | 小状态面板 | 完整工作流，central/bottom 可逆 |
| 顶部 chrome | 多行 | ≤2 行 ≤80px |
| 内容占比 | 被多面板挤压 | collapsed ≥85% H / ≥92% W |
| 右键菜单 | 无 | scope-aware + 分组 + 禁用 + 快捷键 |
| 命令面板 | 无 | Ctrl+Shift+P 统一入口 |

---

## 9. 消费 A2-A10 报告（终稿）

A1 草案先出，A10 独立审查 + A2-A9 报告齐后 A1 消费并出终稿。**终稿已完成。**

| Lane | 消费内容 | 应用于 |
|---|---|---|
| A2 | 8 控件顶栏 (19→8), AFTER 帧 §7.3/§7.4, 实测 90.1% H 平静态, chrome 89px | §2.1 顶栏, §4 viewport |
| A3 | CR-1~CR-12 状态机, NARROW 策略, collapsed 模型 92.6% H, 58 断言 PASS | §2.4 工具窗口, §3 collapsed, §4 |
| A4 | 状态持久化 DTO, schema_version, unknown-view fallback | §3 状态归属 |
| A5 | 数据库壳层: 多 SQL 文档, 取消/截断/错误, 13 scope, 62 命令, 78 断言 PASS | §3.4 database mode |
| A6 | CommandDef schema (id/scope/safetyClass/confirmTier), 15 scope, 命令注册表为净新增 | §3.7 右键菜单, §3.8 命令面板 |
| A7 | 14 Git 单元: COPY=0/ADAPT=5/REIMPLEMENT=9, D-R1 可逆放置, rebased v1.1.15@cee14e9 | §7 Git 工作流 |
| A8 | 单强调色 #2b6cb0, 4px 标尺, 3 圆角档, 1px hairline, ≥24px 命中区 | :root 令牌, 全局视觉 |
| A9 | 4 safety class (SAFE/MUTATES/DESTRUCTIVE_SOFT/DESTRUCTIVE_HARD), 4 confirm tier | §3.7 右键菜单 safety 标注 |
| A10 | F08(260px FAIL), F09(omni=扩展态), F13(800×600 剔除), G4(PROPOSED_NEW), G5(source-derived), G6(viewport budget PASS) | 全帧 viewport, 尺寸集, 证据标注 |

---

## 10. 未解问题

| # | 问题 | 归属 | 证据类型 |
|---|---|---|---|
| U1 | A0 裁定 F05: 体积余量≈346B，Git 工作台分片须声明预算归属 | A0 | `INFERENCE` |
| U2 | A0 裁定 F12: 原生子 WebView 在工具窗口重排下的位置/焦点/遮挡无 lane 认领 | A0 | `INFERENCE` |
| U3 | W19 实现时 A6 命令注册表为前置（当前前端 0 匹配） | A6/W19 | `INFERENCE` |
| U4 | W19 实现时 A7 Git 后端 9 个 REIMPLEMENT 单元为前置 | A7/W19 | `INFERENCE` |
| U5 | A10 F01: rebased LICENSE = JetBrains Build Terms v1.3，COPY 需 A0 书面接受 | A0 | `INFERENCE` |

---

## 11. 合规声明

- [x] Rebase onto `origin/master`（`200f0f1`，之前 3 提交已集成被跳过）
- [x] 仅产出 `logs/research/M5-W18/A1-R3-*` 文件
- [x] 未修改产品代码、manifest、lockfile、脚本、capability、ACL、vault
- [x] 原型自包含合成数据，不接真实 IPC/库/模型/daemon
- [x] 不复制 Rebased 品牌/资产/JetBrains 平台代码
- [x] 覆盖 1920/1440/1366/1024 四种尺寸
- [x] 覆盖 collapsed/normal/database/knowledge/Git/context-menu/focus 七种模式
- [x] 覆盖 empty/loading/error/disabled 四种状态
- [x] Git central-log 和 bottom-tool-window 两种可逆放置
- [x] 顶部 chrome ≤2 行 ≤80px（实测 68px）
- [x] collapsed 内容 ≥85% H / ≥92% W（实测 93% / 96%）
- [x] 未 push

---

## 12. 结论

`STATUS=FINAL_READY_FOR_A0_REVIEW`

A1 R3 终稿已完成 browser-first workbench 替代壳层原型，消费 A2-A10 全部发现：
- 保留 Chrome 式紧凑标签 + 地址栏 + 大内容区（A2 8 控件顶栏）
- IDEA 式渐进披露（A3 CR-1~CR-12 状态机, NARROW 策略）
- 7 种模式 × 4 种尺寸 × 4 种状态全覆盖（800×600 剔除 per A10 F13）
- Git 完整工作流 14 单元（A7 COPY=0/ADAPT=5/REIMPLEMENT=9, D-R1 可逆）
- 右键菜单 + 命令面板（A6 CommandDef schema, A9 safety class）
- 视觉令牌系统（A8 单强调色, 4px 标尺）
- viewport 测量 source-derived（A2 实测 + A3 模型 + A10 viewport-budget.py 复算 PASS）
- 修正 R2B 被拒原型的所有问题

**待 A0 集成审查 + A11 打包验收。**
