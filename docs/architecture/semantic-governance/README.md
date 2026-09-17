# README — 语义治理（Semantic Governance）文档导航

> 本目录是 **mvp-browser-os-v3** 的"单语义治理"长期资产。
> 任何 Agent / 开发者修改本项目前，推荐阅读顺序：

```
AGENTS.md
  ↓
docs/architecture/semantic-governance/README.md          （本文件）
  ↓
00-EXECUTIVE-SUMMARY.md                                  （结论导航，先读）
  ↓
10-TARGET-SEMANTIC-CONTRACTS.md                          （目标契约 = 应遵守的语义）
  ↓
14-DECISIONS.md                                          （已接受/提议的 ADR）
  ↓
与当前任务相关的专题文档（01–09, 11, 12, 13）
```

---

## 文档分类（务必区分"事实"与"设计"）

### CURRENT FACT（描述现状，已被源码证据支持）
- `01-SEMANTIC-INVENTORY.md` — 当前语义清单（SEM-001..016）。
- `02-STATE-SOURCES.md` — 当前状态源分类与风险。
- `03-ACTION-ENTRYPOINTS.md` — 当前动作入口。
- `04-OWNERSHIP-MATRIX.md` — 当前所有权矩阵。
- `05-LIFECYCLE-MODEL.md` — 当前生命周期状态机（红线现状）。
- `06-IPC-CONTRACTS.md` — 当前 IPC 契约闭包。
- `07-SIDE-EFFECT-MAP.md` — 当前原生副作用映射。
- `08-DUPLICATE-SEMANTICS.md` — 当前重复/歧义语义。
- `09-SMALL-MODEL-FAILURE-MODES.md` — 当前小模型误改风险。
- `15-EVIDENCE-INDEX.md` — 证据总账（标注每条 FACT/INFERENCE/UNVERIFIED）。
- `16-REVIEW-REPORT.md` — 独立复核结论（确认/部分确认/矛盾/证据不足）。

### TARGET DESIGN（目标态，尚未全部落地，RECOMMENDATION）
- `10-TARGET-SEMANTIC-CONTRACTS.md` — 各语义的"应然"契约（CONTRACT-*）。
- `12-MIGRATION-PLAN.md` — 增量收敛路线图（Phase 0–6）。

### PROPOSED（提议，待裁决，非现状）
- `11-CHECKER-DESIGN.md` — 机器可强制的看门狗规则（RULE-001..013，部分已满足、部分待落地）。
- `13-ACCEPTANCE-MATRIX.md` — 四层验收矩阵（迁移后如何验证）。
- `14-DECISIONS.md` 中标记为 `PROPOSED` 的 ADR。

### ACCEPTED DECISION（已接受，长期契约）
- `14-DECISIONS.md` 中标记为 `ACCEPTED` 的 ADR（如 ADR-GRID-001 视图切换不销毁宫格、ADR-BROWSER-001 浏览器家族≠纯浏览器视图、ADR-NATIVE-SHOW-001 无独立 show 命令）。

---

## 防误用警告
- **不要把"建议方案（10/11/12/13 的 RECOMMENDATION 部分、PROPOSED ADR）"当成现状。** 现状以 01–09 + 15 + 16 为准。
- **不要把"设计建议"写成"系统现在就是这样"。** 所有文档严格区分 FACT / INFERENCE / RECOMMENDATION / UNVERIFIED。
- 证据行号只是辅助；每条证据同时记录 **file + symbol**，代码变化后请以 symbol 重新定位（见 15 §D）。

## 严重度定义（S0–S4，本仓库统一口径，见 ADR-SEVERITY-001 提议）
- S0 = 编译/运行期崩溃可捕获。
- S1 = lint/类型可捕获。
- S2 = 编译通过、局部可检测/可恢复。
- S3 = 编译通过、特定条件下错误、中等可检测（多为架构耦合，需补原语/看门）。
- **S4 = 编译通过、生产中静默错误、核心 UX/数据、难检测**（红线违反 / 静默语义错误）。
- 注：`close==kill==destroy`、`position==show` 属**架构耦合**，按 ADR-SEVERITY-001 归 S3 并标注"需补原语"，不归 S4。

## 审计元信息
- 审计类型：READ-ONLY 语义治理审计（不修改业务代码、不 commit、不 push）。
- 流程：真实源码 → 多 Agent 并行调查 → Evidence Ledger → 交叉 Review → 冲突裁决 → Markdown 长期资产。
- 完成条件：本目录 17 份文件（README + 00–16）全部就位即 `RESEARCH_COMPLETE`。
- 本轮产出基线：`master` @ `e05160a`，工作树干净，未做任何业务代码改动。
