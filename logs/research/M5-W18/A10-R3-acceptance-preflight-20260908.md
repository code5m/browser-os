# A10 — M5-W18-R3 验收前检清单（给 A11 / A0 的 G1–G8 落点）

- Lane：A10；分支 `codex/m5-w18-a10`；基点 `200f0f1`；接续 `2a0722b` / `7d60cfc` / `b0bd696`
- 时间：2026-09-08
- 用途：A2–A9 的 R3 产物落地后，A10 出终审前，先把**所有可机械执行的检查项**集中成本文件，供 A11 验收 / A0 集成直接复用。
- 状态：`origin/master` 仍为 `200f0f1`，**A2–A9 R3 产物尚未出现**，本清单为预置（preflight），非终审结论。

---

## 0. A10 已交付的基线（终审将直接引用）

| 基线 | 文件 | 关键数字 |
|---|---|---|
| 参照身份/许可 | `A10-R3-reference-and-feasibility-review-20260908.md` | rebased SHA `2896562e…`；Build Terms v1.3；NOTICE 3 行 |
| 视口算术 | `A10-R3-viewport-budget.py` | 1440 侧栏 ≤115.2px / 1024 ≤81.9px；底部 ≤130.2px(1440) |
| 原型自包含门禁 | `A10-R3-prototype-selfcontainment-check.py` | 6 规则，exit 0/1 |
| 次级参照 | `A10-R3-reference-addendum-20260908.md` | SourceGit MIT 已核；slio-git UNVERIFIED |
| 当前外壳基线 | `A10-R3-current-chrome-baseline-20260908.md` | 一级 5 项；☰ 16 项；StatusBar 26px |

---

## 1. 复核闸门 G1–G8（终审逐条打勾）

| 门 | 检查内容 | A10 将如何核验 | 关联发现 |
|---|---|---|---|
| **G1** 复用证据 | 任何 `COPY`/`ADAPT` 含 仓库+SHA+路径+许可+NOTICE | grep 原型/文档里的引用，调 API 复核 SHA | F01/F02/F16 |
| **G2** JetBrains 来源 | JetBrains 作者文件不得标 `COPY`/`ADAPT`（v1.3 未接受） | 比对路径前缀 `platform/`、`plugins/` | F01/F03 |
| **G3** 无品牌/资源 | 原型 UI 不得出现 Rebased/IntelliJ/JetBrains 视觉元素 | 跑 `A10-R3-prototype-selfcontainment-check.py`（`PROTO_NO_BRANDING`） | F03 |
| **G4** 动作可落地 | 每个可见动作→ACL 命令 ID 或 `PROPOSED_NEW`(含 lane) | 对照 `git_status/git_diff/git_branch_list` + 写闸门 7 项；无孤儿 | F07/F10 |
| **G5** 数字可核实 | 每个 px/% 有 file:line 或 GUI 实测，估算标 `estimate` | 抽审 A2/A3/A8 文档的编号 | F05/F13 |
| **G6** 视口算术 | 通过 `A10-R3-viewport-budget.py` 上限或显式偏离 | 重算 A1 原型四尺寸 | F08/F09 |
| **G7** 状态唯一归属 | 每个状态字段单一 owner | 比对 A3/A4 状态文档是否重叠 | F11 |
| **G8** 原型自包含 | 不 import 产品源码/依赖/ACL、不 invoke 原生、不存数据 | 跑 `A10-R3-prototype-selfcontainment-check.py` | — |

---

## 2. R3 卡验收项的 A10 落点（逐条）

| R3 卡要求 | A10 必须看到 | 现状基线 |
|---|---|---|
| "Rebased 作为参照" | A7 映射表含固定 SHA + 逐单元 COPY/ADAPT/REJECT/REIMPLEMENT | 主报告 §2/§5（已给） |
| "git2 可行性" | A7 标 git2 0.19 API 出处；不引新依赖 | 主报告 §5 矩阵（已给） |
| "折叠态 ≥92% 宽 / ≥85% 高" | A1 四尺寸重算表 + 定义"折叠态" | 视口脚本（已给） |
| "去掉常驻按钮 / 不更挤" | A2 五分类 + 重复项标记；A1 默认态无 left260+right240+bottom240 | 外壳基线 §1.4 |
| "上下文菜单=命令注册表身份" | A6 每动作 disabled-reason + safety class；注册表非 CommandSnippetPanel | F10（二次确认见 §3） |
| "Git 成完整工作流" | A7 命令矩阵；A1 每动作不常驻 | 主报告 §5 + §4 缺失动作表 |

---

## 3. 缺失动作枚举（用户裁决 → 产品缺口）

用户裁决要点："感觉更小更不挤、别更挤、去掉常驻功能按钮、别再加"。R3 要求 Git 成完整工作流。**A10 桥接结论**：这些 Git 动作一律**不得成为常驻按钮**，须进入 命令入口/右键菜单（带命令 ID），否则直接违反用户裁决。

缺失动作（产品当前无，R3 要求；按主报告 §5 判定类）：

| 缺失动作 | 判定 | 进入方式（建议） | 禁令 |
|---|---|---|---|
| hunk/行级暂存 | D | 右键菜单（选区上下文） | 不得常驻按钮 |
| log / graph | B | 命令入口（Ctrl+L 类）+ 底部/工具窗（折叠态隐藏） | 不得占中央内容 |
| worktree | B | 右键/命令入口 | — |
| stash | B | 右键/命令入口 | — |
| merge | C | 右键菜单 | 含冲突解决子流 |
| 交互式 rebase | C | 命令入口 + 序列编辑器 | 跨重启持久（A4） |
| cherry-pick / revert / reset | B/C | 右键菜单 | 破坏性须 A9 定级 |
| 冲突解决 | C | 专用 3 路编辑器 | 净新增 UI |
| blame / 文件历史 | B | 右键菜单 | — |
| patch（apply/format） | D | 命令入口 | format 无 API→REJECT |
| tag | B | 右键/命令入口 | — |
| remote fetch/pull | C | 命令入口（凭据走 keyring + A6/A9） | — |
| 命令日志 | B | 命令入口（复用 audit） | — |

> 二次确认（F10 不变）：`src` 内 `registerCommand|executeCommand|commandRegistry|command_registry` **0 命中**；`commands` 视图实为 `CommandSnippetPanel`（命令**片段**面板，`MainArea.vue:24,208`），**不是**命令注册表/面板。故"每个上下文动作有命令注册表身份"是**净新增基础设施**。

---

## 4. 不可能 / 高风险状态转移（A10 必须看到定义）

| 转移 | 为何不可压缩 | 须由谁定义 |
|---|---|---|
| 交互式 rebase 进行中 → 进程崩溃 → 重启 | sequencer 状态机须持久（跨重启） | A4 store + 关闭协调 |
| diff 选区 → 暂存选中行 | git2 无高层 API，需手写 index blob 写入 | A7 |
| 合并冲突 → 用户编辑 → 写回工作树 | 3 路编辑器 + 审计 tick 内禁写约束 | A1/A9 |
| log 图 → 滚动到 10 万提交 | 须虚拟化 + 既有图容量护栏（`WORKSPACE_IDENTITY` 已要求） | A3/A8 |
| 工具窗口 resize/re-parent → 原生子 WebView 位置 | 历史 `queue_resize/set_size_request/webview.hide()` 已失败 | **无 lane 认领（F12，请 A0 指派）** |

---

## 5. 给 A11 的验收前检（复制即用）

A11 在 `master` 出现 A1–A9 R3 产物后，逐条：

1. `python3 logs/research/M5-W18/A10-R3-prototype-selfcontainment-check.py` → 所有 `A1*R3*.html` exit 0（G8/G3）
2. `python3 logs/research/M5-W18/A10-R3-viewport-budget.py` → 记录 A1 四尺寸是否通过（G6）
3. 核 A7 映射表：rebased 固定 SHA `2896562e…`、JetBrains 路径无 `COPY/ADAPT`、slio-git 标 `UNVERIFIED`（G1/G2/F16）
4. 核 A2：一级常驻按钮仍为 5、☰ 16 项、重复项标记；无新增常驻按钮（§2/§3）
5. 核 A6：每个上下文动作有 disabled-reason + safety class；注册表非 `CommandSnippetPanel`（F10）
6. 核 A4：每个状态字段单一 owner；无与 A3 重叠（F11）
7. 核 A0：预算重采干净值；R3 分片预算来源（F05）

任一项 FAIL → A10 终审给 `NOT_IMPLEMENTABLE_AS_DRAWN` / `NEEDS_REWORK`，退回对应 lane。

---

## 6. 范围

- 仅新增 A10 lane 自有文件；零产品代码/依赖/ACL 改动；**未 push**。
- 未改已提交历史（`2a0722b`/`7d60cfc`/`b0bd696`）。
