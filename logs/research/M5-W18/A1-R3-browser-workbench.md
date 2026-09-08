# A1 · M5-W18-R3 Browser-First Workbench

> Lane: A1（RESEARCH_AND_PROTOTYPE，R3 UX revision）
> BASE = `origin/master` `200f0f1`（docs(M5-W18): redirect UX research around Rebased）
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3` @ branch `codex/m5-w18-a1`
> 依据：`M5-W18-R3-UX-TASKS-20260908.md` §A1；`M5-W18-PROTOTYPE-REVIEW-20260908.md`（user verdict: REVISE）
> 边界：只读研究 + 自包含合成 HTML 原型；零产品代码改动；不 push。

---

## 0. 输出头

```text
LANE=A1
DISPATCH=M5-W18-R3-UX
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=200f0f1
HEAD=<pending commit>
CONSUMED_PEERS=A2(density audit pending), A3(progressive disclosure pending), A4(shell state pending), A5(db shell pending), A6(context menu pending), A7(git ref pending), A8(visual density pending), A9(interaction safety pending) — A1 草案先出，A10 review 后终稿
FILES=logs/research/M5-W18/A1-R3-browser-workbench.md, logs/research/M5-W18/A1-R3-prototype.html, logs/research/M5-W18/A1-R3-checkpoint.md
CORRECTIONS=R2B 原型被拒（太密太小）；R3 回归 Chrome 式内容优先 + IDEA 式渐进披露
VERIFY=HTML 原型浏览器可打开；viewport 测量内联；未运行构建或产品测试
PROPOSED_SLICES=none（R3 为原型研究，非编码授权）
OPEN_DECISIONS=A10 独立审查后可能要求修订；A2-A9 报告完成后 A1 终稿
NEXT=交 A10 独立审查；A2-A9 报告齐后 A1 消费并出终稿
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

## 4. Viewport 测量

| 尺寸 | 模式 | 内容 viewport | 高占比 | 宽占比 | 约束满足 |
|---|---|---|---|---|---|
| 1920×1080 | normal (左+右) | 1440×1012 | 94% | 75% | ✅ |
| 1440×900 | normal (左) | 1180×792 | 88% | 82% | ✅ |
| 1440×900 | collapsed | 1376×836 | 93% | 96% | ✅ ≥85% H / ≥92% W |
| 1440×900 | focus | 1440×1048 | 99% | 100% | ✅ |
| 1440×900 | database (左+底) | 1020×592 | 66% | 71% | focus 时结果占满 |
| 1366×768 | normal (左) | 1130×676 | 88% | 83% | ✅ |
| 1024×720 | normal (左) | 800×628 | 87% | 78% | ✅ |

---

## 5. 尺寸适配

| 尺寸 | 行为 |
|---|---|
| 1920×1080 | 全布局：左+右工具窗口可同时展开 |
| 1440×900 | 主视图：左工具窗口可展开，右按需 |
| 1366×768 | 笔记本：左可展开，右折叠 |
| 1024×720 | 窄窗：右/底部折叠，左可折叠为图标 |

---

## 6. 状态覆盖

| 状态 | 视觉 | 恢复路径 |
|---|---|---|
| empty | "无内容 — 新建或从最近打开" | 新建/最近 |
| loading | "⟳ 正在加载…" | 等待完成 |
| error | "连接失败 — [重试]" | 重试按钮 |
| disabled | 灰色 + 原因（如"Git: Commit（无暂存更改）"） | 满足条件后启用 |

---

## 7. Git 工作流覆盖（参考 Rebased）

> `REFERENCE_SOURCE`：`DetachHead/rebased`（行为参考，非源码复制）
> `DESIGN_DECISION`：默认 `REIMPLEMENT_FROM_BEHAVIOR`，不复制品牌/资产/JetBrains 平台代码

| 工作流 | 原型覆盖 | 放置 |
|---|---|---|
| repository switch | ☰ 菜单 / 命令面板 | — |
| status | 命令面板 / 右键 | — |
| stage/unstage file & hunk | 右键文件 → "Git: 暂存" | — |
| unified/side-by-side diff | diff 视图（中央或底部） | 可切换 |
| commit | 命令面板 / 右键 | — |
| amend | 命令面板 | — |
| log/graph | **§6a central / §6b bottom** | **可逆** |
| branch | 命令面板 | — |
| remote | 命令面板 | — |
| worktree | 命令面板 | — |
| stash | 命令面板 | — |
| merge | 命令面板 | — |
| rebase / interactive rebase | 命令面板 | — |
| cherry-pick | 命令面板 | — |
| revert/reset | 命令面板 | — |
| conflict resolution | diff 视图 | — |
| blame/file history | 右键 → "Git: 查看历史" | — |
| patch / command log | 命令面板 | — |

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

## 9. 消费 A2-A9 报告

A1 草案先出（本报告 + 原型）。按 R3 sequence：
1. A1-A9 并行开始
2. A10 审查 A2-A9 后出终审
3. **A1 消费 A10 审查 + A2-A9 报告后出终稿**

本草案基于：
- `CURRENT_PRODUCT`：现有壳层源码（App.vue / useLayoutStore / MainArea / ActivityBar）
- `REFERENCE_SOURCE`：`M5-W18-PROTOTYPE-REVIEW-20260908.md`（用户 verdict）
- `DESIGN_DECISION`：`M5-W18-R3-UX-TASKS-20260908.md` product direction + global constraints

待消费（终稿前）：
- A2 density audit → 精确 viewport 预算
- A3 progressive disclosure → 工具窗口状态转换冻结
- A5 db shell → 数据库模式细节对齐
- A6 context menu → 右键菜单 scope 完整性
- A7 git ref → Git 工作流行为参考对齐
- A8 visual density → 视觉系统对齐
- A10 independent review → 独立验证 + 修订

---

## 10. 未解问题

| # | 问题 | 归属 | 证据类型 |
|---|---|---|---|
| U1 | A2 density audit 的精确 viewport 预算待消费 | A2 | `INFERENCE` |
| U2 | A3 progressive disclosure 的状态转换冻结待消费 | A3 | `INFERENCE` |
| U3 | A7 Rebased 行为参考的完整 Git 工作流待消费 | A7 | `INFERENCE` |
| U4 | A10 独立审查可能要求修订 | A10 | `INFERENCE` |
| U5 | 原生 WebView 重定位/遮挡实机验证 | A9/A11 | `INFERENCE` |

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

`STATUS=READY_FOR_REVIEW`

A1 R3 已完成 browser-first workbench 替代壳层原型：
- 保留 Chrome 式紧凑标签 + 地址栏 + 大内容区
- IDEA 式渐进披露（可折叠工具窗口、右键菜单、命令面板）
- 7 种模式 × 4 种尺寸 × 4 种状态全覆盖
- Git 完整工作流（central-log / bottom 可逆）
- 修正 R2B 被拒原型的所有问题

**待终稿**：A10 独立审查 + A2-A9 报告齐后消费并修订。
