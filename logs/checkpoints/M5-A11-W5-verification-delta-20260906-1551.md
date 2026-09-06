# M5-A11 · W5 验证增量（Verification Delta After A6/A7 W5 Outputs）

```text
LANE=A11
STATUS=PASS_WITH_DEBT（验证完成；集成门禁 pre-merge 当前 FAIL：唯一红灯=构建指标回归 cargo_warnings 2→27，属 A7 M5-7/8 图谱契约 dead_code，非正确性缺陷，须 A0 在最终 push 前消解）
BASE=0e76a89（W5 dispatch HEAD；本地 master 已与 origin 同步，领先 1=本车道 W4 提交）
HEAD=logs/checkpoints/M5-A11-W5-verification-delta-20260906-1551.md
FILES=logs/checkpoints/M5-A11-W5-verification-delta-20260906-1551.md
VERIFY=见 §1 全门表（实跑当前工作树，含 A6/A7 W5 代码，非 stale）
CHECKPOINT=本文件
MERGE_NOTES=见 §5（唯一红灯：cargo_warnings 2→27；消解选项见 §5）+ §7 债务台账
NEXT=待 A0 消解构建指标红灯后 final push；A6/A7 W5 产品代码当前以未提交（untracked/modified）形态存在于主仓工作树，由 A0 集成
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W5 Parallel Dispatch（行 135-170）→ **A11 = START VERIFICATION：「One verification delta」**（after A6/A7 outputs）；scope `logs/assist/M5-A11-W5-*.md`、`logs/checkpoints/M5-A11-W5-*.md`。
> 范围声明：本增量**只产出验证文档，零产品代码改动**。A6/A7 的 W5 产品代码当前以 **untracked/modified** 形态落在主仓工作树（尚未由 A0 提交），本车道针对该待集成状态做预验证。未 push。
> 姊妹件：W1 `M5-A11-W1-...`、W2 `M5-A11-W2-...`、W3 `M5-A11-W3-verification-delta-20260906-1413.md`、W4 `M5-A11-W4-verification-delta-20260906-1458.md`。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已经是最新的（0e76a89）
git status --short --branch          # 主仓工作树含 A6/A7 等多 lane 的 untracked/modified W5 交付（详见 §3/§4）
git log --oneline -8                 # 0e76a89 / 1610939 / d71f558 / 13c5279 / 562efb9 / f7ad35a / f8f1f49 / 12f1cff
```
- **调度匹配**：board 头部 `Current NEXT: M5-W5 parallel implementation`（行 7）一致。W5 仅开 A6（M5-6 Agent/Skill UI 纯逻辑/面板壳）与 A7（M5-7/8 graph 模型/存储策略切片）产品代码 lane。
- **关键事实修正**：W4 增量里我标注「A4/A5 W4 产品代码尚未集成、待 A0 集成后再验证」——现已落地为 `1610939`（A4 agent_memory + A5 Agent/Skill domain/policy shell），本增量 §2 正式补验。同时 A6/A7 的 W5 产品代码已落入主仓工作树（untracked/modified），§3/§4 验证。

---

## 1. W5 验证矩阵（当前工作树实跑，含 A6/A7 W5 代码）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **370 passed；0 failed**（361 + graph.rs 9） | ✅ |
| cargo check 告警 | `cargo check --locked` | **27 warnings（全部 dead_code，见 §5）** | ⚠️ 红灯 |
| core 边界门 self-test | `python3 scripts/check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| core 边界门 default | `python3 scripts/check-core-boundary.py` | `all invariants hold`（ACTIVE=7，core 文件=3） | ✅ |
| core 边界门 pending | `python3 scripts/check-core-boundary.py --expect-pending` | `CORE_PENDING_RESULT=NONE` | ✅ |
| M5-2 MCP 门 self-test | `python3 scripts/check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS`（ACTIVE=5，PENDING=9） | ✅ |
| M5-2 MCP 门 default | `python3 scripts/check-mcp-policy.py` | `MCP_POLICY=PASS` | ✅ |
| M5-2 MCP 门 pending | `python3 scripts/check-mcp-policy.py --expect-pending` | `MCP_PENDING_RESULT=FAIL`（按设计：产物已落地→PENDING→ACTIVE 翻转待 M5-2.b） | ⚠️ 设计内 |
| M5-3 agent-memory 门 self-test | `python3 scripts/check-agent-memory-policy.py --self-test` | `AGENT_KV_POLICY_SELF_TEST=PASS`（ACTIVE=5） | ✅ |
| M5-3 agent-memory 门 default | `python3 scripts/check-agent-memory-policy.py` | `AGENT_KV_POLICY=PASS` | ✅ |
| M5-3 agent-memory 门 pending | `python3 scripts/check-agent-memory-policy.py --expect-pending` | `AGENT_KV_PENDING_RESULT=FAIL`（按设计：产物已落地→应接入默认门禁，已接入） | ⚠️ 设计内 |
| M5-4/5 agent-skill 门 self-test | `python3 scripts/check-agent-skill-policy.py --self-test` | `AGENT_SKILL_POLICY_SELF_TEST=PASS`（ACTIVE=2，PENDING=4） | ✅ |
| M5-4/5 agent-skill 门 default | `python3 scripts/check-agent-skill-policy.py` | `AGENT_SKILL_POLICY=PASS` | ✅ |
| M5-4/5 agent-skill 门 pending | `python3 scripts/check-agent-skill-policy.py --expect-pending` | **`error: unrecognized arguments: --expect-pending`**（该脚本未实现此模式，4 个 PENDING 码位仍 stale） | ⚠️ 不一致 |
| M5-7/8 graph 门 self-test | `python3 scripts/check-graph-policy.py --self-test` | `GRAPH_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| M5-7/8 graph 门 default | `python3 scripts/check-graph-policy.py` | `GRAPH_POLICY=PASS` | ✅ |
| M5-7/8 graph 门 pending | `python3 scripts/check-graph-policy.py --expect-pending` | `GRAPH_PENDING_RESULT=FAIL`（按设计：graph.rs 已落地→应翻转 PENDING→ACTIVE） | ⚠️ 设计内 |
| M2-4 执行通道 self-test | `python3 scripts/check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS` | ✅ |
| tools 策略 self-test | `python3 scripts/check-tools-policy.py --self-test` | `self-test OK` | ✅ |
| db 策略 self-test | `python3 scripts/check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（ACTIVE=14，PENDING=1） | ✅ |
| db 策略 default | `python3 scripts/check-database-policy.py` | `all invariants hold`（ACTIVE=14） | ✅ |
| db 策略 pending | `python3 scripts/check-database-policy.py --expect-pending` | `DB_PENDING_RESULT=NONE` | ✅ |
| scheduler 策略 self-test | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（ACTIVE=23，PENDING=0） | ✅ |
| scheduler 策略 default | `python3 scripts/check-scheduler-policy.py` | `all invariants hold`（ACTIVE=23） | ✅ |
| scheduler 策略 pending | `python3 scripts/check-scheduler-policy.py --expect-pending` | `SCHED_PENDING_RESULT=NONE` | ✅ |
| A6 Agent/Skill UI 逻辑 | `node scripts/check-agent-skill-ui-logic.mjs` | **57 assertions passed, 0 failed** | ✅ |
| Rust fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | `FMT_CLEAN` | ✅ |
| 工作树 diff | `git diff --check` | `DIFF_CHECK_CLEAN` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-*.js`=162.50 kB（gzip 58.28 kB） | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=FAIL`（EXIT=1）；失败行唯一=build metrics regression，见 §5** | ❌ 红灯 |

> 注：`pre-merge.sh` 除构建指标外全部子门（core/mcp/agent-memory/agent-skill/graph/script-exec/tools/db/scheduler 策略、fmt、npm build、cargo check、各 self-test）均 PASS；构建指标对比仅 `cargo_warnings` 一项触发 `warnings_increased`（2→27）。前端体积 `total_bytes_pct=15.67%`，低于 A0 于 2026-09-06 书面抬至的 **16%** 增长上限，故尺寸本身合规。

---

## 2. A4/A5 W4 集成（`1610939`）补验 —— W4 递延项结案

`1610939` 集成了 W4 A4/A5 产品代码，本增量正式补验（W4 时尚未集成）：

| 交付 | 验证 |
|---|---|
| `agent_memory.rs`（868 行，A4） | 隐私三重闸（`SENSITIVE_KEY_NAMES`×15 + `SENSITIVE_VALUE_PATTERNS`×8，C-5 字段名+值双扫）→ `PrivacyViolation` 拒；有界（per-agent 1MiB、agent 上限 32、总记录 5000、单值 ≤64KiB、LRU 淘汰，测试 T-priv/T-cap/T-ttl/T-audit）；审计仅记 `key_hash`（`AgentKvAuditEntry` 无 value 字段，编译期保证不落值）；`cargo test agent_memory` 通过 |
| `agent.rs`（104）/ `skills.rs`（147，A5） | `AgentDef`/`SkillDef` DTO + `validate()` 调 `security_policy::contains_credential_leak` → `CredentialLeak` 错误（测试 `credential_in_system_prompt_rejected`/`credential_in_description_rejected`）；纯逻辑，无执行 runtime |
| `domain.rs`（+148） | `AGENT_KV_*` 常量 + Agent/Skill/Graph DTO 单一真源 |
| `security_policy.rs`（+88） | `contains_credential_leak` / 脱敏助手 |
| `check-agent-memory-policy.py`（303）/ `check-agent-skill-policy.py`（410） | 已接入 `pre-merge.sh`（行 424-435 / 432-435）；self-test+default 均 PASS |
| `main.rs` | 仅 `mod agent; mod agent_memory; mod skills; mod graph;`（模块声明，**无新命令**） |

**命令表面核查（W5 硬停止「新命令须原子含 ACL/源检/bridge/types/策略/tests，prefer 无命令」）**：
- `grep '#[tauri::command]' agent.rs skills.rs graph.rs` → 无；`main.rs` 的 `generate_handler!` 未含 `agent_/skill_/graph_` 命令；ACL 无 agent/skill/graph 条目 → **A4/A5/A7 均未暴露新 Tauri 命令** ✅
- 边界核查：agent_memory/agent/skills/graph 均无 `use tauri`/`AppHandle`/`crate::bridge`/`std::process`/`tokio`/`reqwest`/`rmcp` 导入（仅文档注释提及「不引桥/不联网」）✅

---

## 3. A6 W5（M5-6 Agent/Skill UI 纯逻辑/面板壳）验证

**形态**：以 untracked 落入主仓工作树（未提交）。
- 组件：`src/components/workspace/{AgentChatPanel,AgentManagerPanel,PermissionPreviewModal,RunHistoryModal,SkillManagerPanel}.vue`
- 状态/逻辑：`src/stores/useAgentStore.ts`、`src/utils/agentSkillUi.ts`
- 策略/测试：`scripts/check-agent-skill-ui-logic.mjs`（**57 assertions passed**）
- 改动：`bridge.ts`/`types.ts`/`ActivityBar.vue`/`MainArea.vue`/`useLayoutStore.ts`/`M5-6` 卡（DTO 类型与面板挂载，非命令）

| 验收（W5 硬停止） | 结果 |
|---|---|
| 无执行 runtime / 安装器 / 网络/模型调用 / 新后端命令 | ✅ 无 `#[tauri::command]`；前端 27 处 agent/skill/graph 引用均为 **DTO 类型定义**（AgentDef/SkillDef/GraphNode 展示型），非命令封装 |
| 无新依赖 | ✅ 未改 `package.json`/`Cargo.toml` |
| 无 live execution | ✅ 面板壳 + 纯逻辑（`agentSkillUi.ts`），`check-agent-skill-ui-logic.mjs` 57 断言通过 |
| UI 逻辑测试 PASS | ✅ 57/57 |

> **小缺口（非阻断）**：`check-agent-skill-ui-logic.mjs` 尚未接入 `pre-merge.sh`（graph 策略已接入，行 441-444；agent-skill UI 逻辑未接）。建议 A0 补一行以覆盖 A6 UI 逻辑门禁。

---

## 4. A7 W5（M5-7/8 graph 模型/存储策略切片）验证

**形态**：以 untracked 落入主仓工作树（未提交）。
- `src-tauri/src/graph.rs`（16.6 KB）：纯逻辑（校验/容量/脱敏/bounded store/bounded query），文档头声明不引 `tauri`/`AppHandle`/`crate::bridge`/网络/命令/第二执行路径；持久化（SQLite 单连接）留待 M5-8 接 `database.rs`。
- `scripts/check-graph-policy.py`：self-test PASS（ACTIVE=7）+ default PASS，**已接入 `pre-merge.sh`（行 441-444）**。
- `domain.rs`（+Graph* 类型）、`main.rs`（`mod graph;`）、`M5-7/8/9` 卡。
- `cargo test` +9（graph.rs 测试），编译通过。

| 验收（W5 硬停止） | 结果 |
|---|---|
| 纯逻辑 / 无 live UI / 无 agent 消费 / 无后台重建 worker / 无网络 | ✅ graph.rs 仅 `use crate::domain::*; use serde; use std::collections`；无命令、无 tauri、无网络 |
| 有界 store/query / 隐私扫描 / 容量接入 | ✅ `GRAPH_MAX_NODES=5000`/`GRAPH_MAX_EDGES=20000`/`GRAPH_PROPS_MAX_ENTRIES=64`/`GRAPH_LABEL_MAX_BYTES`/`GRAPH_NODE_ID_HEX_LEN` 等常量 + `graph_props_contain_secret` 双扫；策略门守「常量缺失/隐私未扫值/容量未接/遍历无界/id 非十六进制/第二执行路径」 |
| 无新命令 | ✅ 见 §2 命令表面核查 |
| 策略 self-test/default PASS | ✅ |

> **部分消费导致 dead_code（见 §5）**：graph.rs 消费了 `GraphNodeKind`/`GraphEdgeKind`/`GraphProps`/`GRAPH_*` 部分常量，但 `GraphNode`/`GraphEdge`/`GraphStore` 结构体、`validate_graph_node/edge/props/id`/`scan_graph_value`/`is_hex`/`graph_props_contain_secret` 函数、`GRAPH_MAX_DEPTH`/`GRAPH_QUERY_LIMIT` 等仍无调用方（M5-8 store / M5-9 UI / agent 消费未落地）→ 27 个 dead_code 告警。

---

## 5. 唯一红灯：pre-merge FAIL（构建指标回归）

`bash scripts/pre-merge.sh` → `PRE_MERGE_RESULT=FAIL`（EXIT=1），失败行唯一：
```
[pre-merge] FAIL: build metrics regression vs logs/m0-build-metrics/build-metrics-4f0e8ab.json
```
根因（`measure-build-metrics.py` 退出逻辑：`return 1 if exceeds_growth_limit or warnings_increased`）：
- `total_bytes_pct=15.67%` < A0 抬至的 **16%** 上限 → `exceeds_growth_limit=false`（尺寸合规）
- **`cargo_warnings=27`（基线 2）→ `warnings_increased=true`** → 退出 1 → FAIL

**27 告警全为 dead_code**，落在 A7 M5-7/8 图谱契约：
`GraphNode`/`GraphEdge`/`GraphStore` 结构体未构造；`GraphNodeKind`/`GraphEdgeKind`/`GraphError` 枚举未用；`validate_graph_node/edge/props/id`/`scan_graph_value`/`is_hex`/`graph_props_contain_secret` 函数未用；`GRAPH_*` 常量（MAX_NODES/EDGES/DEPTH/QUERY_LIMIT/PROPS_MAX_BYTES/PROPS_MAX_ENTRIES/NODE_ID_HEX_LEN/LABEL_MAX_BYTES）未用；graph.rs 自身 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS` 与 A4 重复定义且未用。

**性质**：非正确性缺陷，是「核心切片」建立契约但消费方（M5-8 store、M5-9 UI agent 消费）尚未落地的预期中间态。W4 时 `1610939` 将这些 Graph* 类型写入 `domain.rs`（死代码），彼时 pre-merge 尚未跑（W4 HEAD=f7ad35a 在 1610939 之前，故 W4 增量报 ALL_PASS 正确）；W5 集成图.rs 后死代码仍在 → 触发 M0-1.a「warning 只减不增」红线。

**消解选项（交 A0，不属本车道产品代码范围）**：
1. **（推荐，预期路径）** A7 完成 M5-8 图谱 store/runtime，构造 `GraphNode`/`GraphEdge`/`GraphStore` 并调用 `validate_*`/`graph_props_contain_secret`/`scan_graph_value`/`is_hex` → 死代码自然清零；
2. **若延后**：按 IF-4 给每项未用符号加 `#[allow(dead_code)]` 并注明命名消费方（A0 可接受文档化例外，目标回基线 2）；
3. 将前向声明的未用类型移出编译树（如 feature gate / 收归 graph.rs 模块作用域）直至被消费。

> 此红灯为 **A0 final push 的发布阻断项**；其余门禁（含全部策略脚本、fmt、npm build、370 单测）全绿。

---

## 6. Before-A0 冲突扫描

- **空文件**：`git status` 中 A6/A7 等 untracked 文件均有实质内容（graph.rs 16.6KB、各 Vue 组件、脚本等），无空文件。
- **stale STOPPED 冒充 PASS**：各策略脚本 self-test/default 均真实 PASS（非 STOPPED 伪装）。
- **重复命令名**：`main.rs` 的 `generate_handler!` 与各模块 `#[tauri::command]` 无 agent/skill/graph 新命令，无重复。
- **ACL 顺序**：ACL 末条恒为 `list_artifact_images`；无新命令插入其后。
- **bridge/main/types 命令奇偶**：因无新命令，平凡一致；`bridge.ts`/`types.ts` 新增均为 Agent/Skill/Graph **DTO 类型**（展示用），与命令解耦。
- **docs NEXT 一致性**：board 头部 `Current NEXT: M5-W5` 与 §M5-W5 dispatch 一致。
- **lane scope 漂移**：W5 产品代码仅 A6（UI 纯逻辑/壳）、A7（graph 纯逻辑/策略）；均未越界加命令/runtime/网络/依赖；A4/A5 集成（1610939）属其 W4 授权；其余 lane 仅 docs。无漂移。

---

## 7. 债务台账更新（相对 W4）

| 项 | W4 状态 | W5 状态 |
|---|---|---|
| 集成门禁 | ALL_PASS | **FAIL（唯一红灯：cargo_warnings 2→27）** |
| U-2（core seam） | CLOSED（f8f1f49） | 维持 CLOSED |
| U-4（capability 真源） | 已立项（12f1cff） | 维持；graph/agent 领域常量亦入 domain.rs 单一真源 |
| W3 发现②（pre-merge 接入 MCP 门） | CLOSED | 维持；agent-memory/agent-skill/graph 三门均已接入 |
| A4/A5 W4 产品代码补验 | 递延（未集成） | **结案**（§2 全绿） |
| **cargo_warnings 2→27（dead_code on A7 图谱契约）** | 未现（W4 HEAD 在 1610939 前） | **新增红灯，发布阻断，交 A0/A7（§5）** |
| MCP `--expect-pending` | FAIL by design | 维持（待 M5-2.b 翻转） |
| agent-memory `--expect-pending` | FAIL by design | 维持（已接入默认门禁） |
| graph `--expect-pending` | — | FAIL by design（待 M5-8 翻转） |
| agent-skill 策略缺 `--expect-pending` 模式 | — | **不一致**：该脚本无此模式，4 个 PENDING 码位 stale（兄弟脚本 mcp/agent-memory/graph 均有） |
| agent-skill UI 逻辑测试未接入 pre-merge | — | **小缺口**：`check-agent-skill-ui-logic.mjs` 未接入 `pre-merge.sh` |
| 构建指标基线文件 | 单一 | 现存在两份（`4f0e8ab` + `6f4e554`）；pre-merge 以 `sort|head -1` 取最旧（更严），行为 OK 但建议归档旧基线 |

---

## 8. 声明（避免误读）

- 本车道**零产品代码改动**（W5 的 graph.rs / Vue 组件 / 脚本均为 A6/A7 落盘，非本车道写入）。本增量仅新增验证文档。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；未引用旧报告（遵守 IF-5 不 stale 要求），且针对**含 A6/A7 W5 代码的当前工作树**复跑。
- 工作树中 A6/A7 及其他 lane（A1/A2/A4/A5/A8/A9/A10）的 untracked/modified 文件**本车道未触碰**；提交动作仅 `git add` 本文件，不带入他 lane 改动。
- 唯一必须消解项：§5 的 cargo_warnings 2→27 红灯，由 A0 在 final push 前处理（优先走 §5 选项 1）。
