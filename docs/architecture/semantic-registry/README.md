# Semantic Registry

> 目的：为长期 AI / 多人协作建立**语义防退化机制** —— 防止未来重新创造
> 重复状态、重复 Intent、重复 Owner，或把带副作用的操作误当成无副作用操作。

---

## 1. 它解决什么

| 退化类型 | 例子 | 由谁拦 |
|---|---|---|
| 重复状态（第二真源） | 把 `desiredGridVisibility` 又存成 `gridVisible` | R1 |
| 治理域内新增状态未登记 | 在 `useBrowserStore` 加 `activeSurface` 却没人知道 | R2 |
| Owner 越界 | 组件直接调 `closeGridAll` / 直写 `mainView` | R3 |
| 重复 Intent | 已有 `activateGrid`，又加 `showGridView` / `enterGrid` | R4 |
| 副作用误判 | 以为 `gridPosition()` 只是移动，实际是 show/hide | R5 |

运行：

```bash
node scripts/check-semantic-registry.mjs              # 扫描真实仓库
node scripts/check-semantic-registry.mjs --self-test   # 自检（含 false-positive fixture）
node scripts/check-semantic-registry.mjs --strict      # 提示级(R5)也判失败
node scripts/check-semantic-registry.mjs --json        # 机器可读
```

## 2. 文件索引

| 文件 | 内容 |
|---|---|
| `states.yaml` | 状态真源：含义、不表达什么、owner、谁能写、是否派生 |
| `intents.yaml` | 一个意图一个入口：允许/禁止的效果、重复名、已否决项 |
| `owners.yaml` | 谁拥有什么；owner-only 内部原语 vs public intent |
| `side-effects.yaml` | 真实行为（不是函数名）：`position` 实际含 show/hide |

## 3. 状态分类（写入每个条目）

| 分类 | 含义 | 可否被当作现状引用 |
|---|---|---|
| `CURRENT_FACT` | 代码中真实存在、已确认 | ✅ |
| `ACCEPTED_ADR` | 已裁决并冻结（有 ADR 编号） | ✅ |
| `TARGET_CONTRACT` | 目标契约（尚未落地） | ❌ 仅作目标 |
| `PROPOSED_CHANGE` | 拟议，代码中不存在 | ❌ **严禁当作现状** |
| `KNOWN_DEBT` | 已知债务 | ✅ 但不得扩展 |

> **铁律：禁止把 Proposal 伪装成 Current。**
> 本 registry 中已明确标注的不存在项：
> - `switchTab` / `closeTab`（真实入口是 `tabSwitch` / `closeTabNow`）→ `PROPOSED_CHANGE`
> - `sync_browser_scene` → `NOT_FOUND`（全仓 grep 无命中，未纳入）
> - `exitGrid(mode)` → `REJECTED`（已 ADR 否决，出现即阻断）

## 4. 新增语义的流程

```text
新增状态 / Intent / Owner / Side Effect
        ↓
查询本 Registry（states / intents / owners / side-effects）
        ↓
已有语义？── YES ──► 使用已有语义（禁止再造一个）
        │
        └── NO ──► 填写 SCR（docs/architecture/semantic-changes/SCR-template.md）
                     ↓
                   Reviewer 裁决
                     ↓
                   ADR（如需）
                     ↓
                   更新 Registry YAML
                     ↓
                   才允许写业务代码
```

## 5. Checker 规则与手段

| 规则 | 级别 | 判定手段 |
|---|---|---|
| R1 `SEMANTIC_DUPLICATE_STATE` | fail | 识别**存储声明**形态（`const X = ref/computed/reactive(`），排除对象字段名 |
| R2 `SEMANTIC_UNREGISTERED_STATE` | fail / info | 仅在 `states.yaml` 声明的 `governed_files` 内生效；已登记未治理只报 info |
| R3 `SEMANTIC_OWNER_VIOLATION` | fail | owner-only **内部原语** vs **public intent** 区分；组件直写 `mainView`；非 owner 调 `bridge.closeGrid` |
| R4 `SEMANTIC_INTENT_DUPLICATE` | fail | 检测重复入口定义；已否决项（exitGrid）出现即阻断 |
| R5 `SEMANTIC_SIDE_EFFECT_UNKNOWN` | warn（--strict 下 fail） | 调用点附近查找 `side-effect:` 认知声明（默认 6 行窗口） |

不是"只写 regex"：脚本自带 YAML 子集解析器读取 registry 结构、引号感知的注释剥离、
作用域限定、声明形态识别、副作用认知窗口分析。语义真源在 YAML，**改语义请改 YAML（走 SCR），不要改脚本**。

## 6. 边界（避免过度冻结）

- 本 Registry **登记两批**：第一批 Browser/Grid + View Navigation（Phase 1）；第二批 Workspace/FilePanel（Phase 2，useWorkspaceStore.ts + FilePanel.vue），不冻结全部业务设计。
- 治理域外的状态**不判失败**（避免误报、避免阻止合理扩展）。
- `observed_not_governed` 是**显式登记**，不是绕过用的 allow-list：它们只报 info，纳入治理仍需 SCR。
- R5 默认仅提示，不阻断；需要严格时用 `--strict`。
- 需要人工判断的（是否真属重复语义）**不强行自动化** —— checker 只报"疑似"，裁决在 SCR/Reviewer。

## 7. 防退化闭环（已接入 pre-merge）

```text
Agent / 开发者 修改代码
        ↓
读取 Semantic Registry（states / intents / owners / side-effects）
        ↓
检查语义冲突（是否与既有语义重复 / 越权 / 副作用未声明）
        ↓
pre-merge 门禁（scripts/pre-merge.sh）
        ↓
阻止重复语义进入代码
```

接入点（两处，均为新增，未削弱任何既有检查）：

| 位置 | 行为 |
|---|---|
| 正式门禁 `run_pre_merge` | `check-semantic-registry` 已加入 Phase 03 checker 循环，默认模式运行；**仅 fail 阻塞**，warn 不阻塞 |
| 自检 `run_self_test` | 校验脚本存在 + `--self-test` 必须 ALL_PASS |

验证结果：`bash scripts/pre-merge.sh --self-test` 中本项 PASS（未新增失败；
仍存在的 `check-terminal-policy.py` / `check-terminal-ui-logic.mjs` 失败为既有债 Debt-004，与本层无关）。

## 8. 与既有门禁的关系

| 门禁 | 职责 |
|---|---|
| `check-view-intent.mjs` | Phase 1 冻结的 Browser/Grid 契约（C1–C11） |
| `check-semantic-registry.mjs` | 跨语义的重复/越界/副作用防退化（本 Registry 驱动） |

两者互补：前者钉住 Phase 1 结论，后者防止未来语义漂移。
本 checker **未**接入 `npm run check`（那需要改 `package.json`，属独立改动，不在本阶段范围）。
