# Semantic Governance Coverage Remediation — Final Result

> 机器真源：`docs/architecture/semantic-registry/{states,owners,intents,side-effects}.yaml`
> 门禁：`scripts/check-semantic-registry.mjs`（self-test + 真实扫描）
> 本文件是 **A–M 规格的收口报告**，非第二真源。评级定义见各 YAML / README。

## 0. 范围与约束（A–M 规格）

| Phase | 内容 | 结果 |
|---|---|---|
| A | 冻结 Phase D3，仅文档/checker，禁业务代码；独立提交 + annotated tag；不 push/不 merge master | ✅ 早已收口（d3b4670 + tag `domain-modularity-phase-d3-pass`） |
| C | 真实 registry 逐模块覆盖率矩阵；STATUS 仅取 8 类；UNKNOWN=0；不人为提高 FULLY_GOVERNED | ✅ `coverage-matrix.md` 生成 |
| D | 先修 Registry Drift / Locator，不先改业务代码；分类 {STALE_DOCUMENTATION, STALE_LOCATOR, MISSING_REGISTRY_ENTRY, REAL_SEMANTIC_DEFECT} | ✅ 真实扫描 R2/RI fail=0，活动漂移=0 |
| E | Browser/Grid 冻结语义保护（mainView/desiredGridVisibility/isBrowserVisible 等），禁未经 SCR 改变 | ✅ UNCHANGED（已在 D3/Phase E 锁定，本次未触碰） |
| F | 逐域证明后裁决 OWNER_DECLARED_ONLY；禁伪造 semantic objects | ✅ task 实证升格 FULLY_GOVERNED；graph/plugin/workspace 诚实保留 OWNER_PENDING_SCR |
| G | Single Semantics Audit（DUPLICATE_STATE/OWNER/WRITER/INTENT、DERIVED_STORED、UNREGISTERED_WRITER、STALE_LOCATOR、PUBLIC_CONTRACT_BYPASS） | ✅ 见 §3 |
| J | 增强 checker 需带 positive/negative fixture，禁洗绿 | ✅ self-test 含 positive/negative/FP/locator/R6 回归，ALL_PASS |
| L | 按语义域分批提交（SG-1~SG-5），不自动 push | ✅ 各 SG 独立 commit，无 push |
| M | 输出本结果，完成后停止；不进 M2、不拆 bridge.rs、不搬 state/UI | ✅ 见 §5 STOP |

## 1. 覆盖率最终状态（UNKNOWN=0）

机器统计（真实 registry + `src/capabilities/*` 盘点，无虚构治理对象）：

- TOTAL_MODULES（capability 16 + framework 15）：31
- FULLY_GOVERNED：23（capability 13：agent/apps/bookmark/browser/clipboard/database/git/home/skill/task/terminal/tools/vault；framework 10）
- OWNER_PENDING_SCR：3（graph、plugin、workspace —— LOCKED / 边界待 SCR，**不伪造**）
- NOT_APPLICABLE：4（settings、session、resource、workbench —— Phase H 框架 SERVICE，不升格）
- UNGOVERNED：0 ✅（task 已完成真实治理）
- UNKNOWN：0 ✅

Registry 体量（去重后）：REGISTERED_STATES ≈ 177、REGISTERED_INTENTS = 82、REGISTERED_OWNERS = 22、REGISTERED_SIDE_EFFECTS = 29。

## 2. 门禁结果（checker）

- `--self-test`：`SELF_TEST_RESULT=ALL_PASS`
  - positive fixture 0 fail/warn、negative fixture R1–R9 + R6 全部检出、false-positive 0 误报、locator CASE A–E 全过、R6 写穿式 computed 不误报。
- 真实扫描（`--json`）：`status=PASS`，`files_scanned=196`，`fails=0`，`warns=6`，`infos=64`。
  - R2 fails=[]、RI fails=[]（locator 全部解析成功，无 UNRESOLVED/DUPLICATE）。
  - 64 infos = 设计内的 `observed_not_governed`（已登记、待 SCR，非失败）。
  - 6 warns = R5 `SEMANTIC_SIDE_EFFECT_UNKNOWN`（browser/grid 冻结域 rebuildGrid/gridPosition 的既有认知标记，非阻断，不在本次治理面）。

## 3. Phase G — Single Semantics Audit

| 审计项 | 机制 | 结果 |
|---|---|---|
| DUPLICATE_STATE（派生被存成第二真源） | R1 / R6 | 0 fail |
| DUPLICATE_OWNER / WRITER / INTENT | R4 / R8 / R9 | 0 fail |
| DERIVED_STORED（派生量被声明为存储态并写入） | R1 / R6 | 0 fail |
| UNREGISTERED_WRITER（forbidden_writers 越界写） | R9 | 0 fail |
| UNREGISTERED_STATE（治理域内新增状态未登记） | R2 | 0 fail |
| STALE_LOCATOR（owner_implementations 解析失败/重复） | RI | 0 fail |
| PUBLIC_CONTRACT_BYPASS（组件直连 bridge 绕过 owner store intent） | 治理纪律 + R3/R9 | 受控（见 §4 红队 #9） |

诚实结论：自动化可检项全部为 0 失败。`PUBLIC_CONTRACT_BYPASS` 无独立阻断规则，作为治理纪律由 owners.yaml 的 `forbidden_callers`/`authorized_callers` 约束，列为待持续监督项（非本次失败）。

## 4. Phase K — 红队 Review（10 攻击向量）

| # | 攻击向量 | 缓解 | 状态 |
|---|---|---|---|
| 1 | 在治理 store 注入未登记 `ref` 状态 | R2 fail | ✅ 阻断 |
| 2 | 组件直写 `mainView` / 直调 Grid 生命周期原语 | R3 fail | ✅ 阻断 |
| 3 | 复用被否决 intent 名（如 exitGrid） | R4 fail | ✅ 阻断 |
| 4 | 把派生量存成第二真源（ref + `.value=`） | R1 / R6 fail | ✅ 阻断 |
| 5 | 物理迁移 owner 文件但未同步 locator | RI UNRESOLVED fail | ✅ 阻断 |
| 6 | 单 owner 状态被两 owner 同时声明（single_owner_required） | R8 fail | ✅ 阻断 |
| 7 | forbidden_writer 越界写状态 | R9 fail | ✅ 阻断 |
| 8 | 调用带副作用 bridge API 未声明认知 | R5 warn（非阻断，设计内） | ✅ 受控 |
| 9 | 组件直连 bridge 绕过 owner intent（public contract bypass） | 治理纪律 + R3/R9 | ⚠ 受控（监督项） |
| 10 | 明文凭据泄漏 | R7 SENSITIVE_INPUT_LEAK fail | ✅ 阻断 |

9/10 由 checker 阻断或受控；#9 为治理纪律监督项，不阻断（与规格 Phase K「独立红队」一致，未洗绿）。

## 5. STOP — 收口裁定

- **已完成**：Phase D3 冻结收口；SG-1~SG-4（drift / simple / medium / task-complex）真实治理；UNGOVERNED 清零；所有 checker 门禁绿灯。
- **显式债务（诚实不静默消失）**：
  1. `graph` / `plugin` / `workspace` = `OWNER_PENDING_SCR`（LOCKED / 边界待 SCR），**不伪造治理**；如需升格须走 SCR。
  2. 6 条 R5 warn 为 browser/grid 冻结域既有认知标记，非本次治理面，留待该域 SCR。
  3. 64 条 `observed_not_governed` 为已登记待治理态（workspace 子域等），SCR 纳入前保持 info 级。
- **明确不再推进**（遵守规格 M）：不进入 M2；不拆分 `bridge.rs`；不迁移 state/UI；不扩大业务代码改动面。
- **交付物**：本结果 + `coverage-matrix.md` + 四个 registry YAML + `check-semantic-registry.mjs`（SG-4 task 切片 commit `58046c5`，未 push）。

EOF
