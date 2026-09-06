# Lane A4 · M5-W6 Review Note — Graph UI (A8) / Plugin Manifest-Lifecycle (A9) 与 agent memory KV 的隐私/容量交互审计

> Lane=A4 · WAVE=M5-W6 · Status=**SUPPORT/REVIEW ONLY**（看板 §M5-W6：A4 仅评审 A8/A9 与 agent memory 的隐私/容量交互，无产品代码）
> 评审对象：A8（M5-9 graph UI pure logic/panel shell，over A7 DTOs）、A9（M5-10/11 plugin manifest/lifecycle policy slice）
> 评审镜头（看板原话）：*"Review graph/plugin interactions with agent memory privacy/capacity"*
> 边界真相源：① A4 W4 agent memory KV 契约（`src-tauri/src/agent_memory.rs` + `scripts/check-agent-memory-policy.py`，5 ACTIVE 码）；② A7 W5 已集成图谱契约（`src-tauri/src/graph.rs` + `scripts/check-graph-policy.py`，7 ACTIVE 码）

## 0. 仓库实况（@ `77b1e3e`，已 `git pull --ff-only`，工作树 clean）

| 项 | 状态 | 证据 |
|---|---|---|
| A4 agent-memory 政策 | **PASS** | `check-agent-memory-policy.py --self-test` → `AGENT_KV_POLICY_SELF_TEST=PASS（ACTIVE=5）`；默认扫描 `AGENT_KV_POLICY=PASS` |
| A7 graph 政策 | **PASS** | `check-graph-policy.py --self-test` → `GRAPH_POLICY_SELF_TEST=PASS（ACTIVE=7）`；默认扫描 `GRAPH_POLICY=PASS` |
| A7 图谱代码 | **已集成** | `domain.rs:2114-2116` 含 `GRAPH_MAX_NODES=5_000`/`GRAPH_MAX_EDGES=20_000`；`graph.rs` 含 `validate_graph_node/edge` + `GraphStore` bounded + 隐私双扫 |
| **A8 W6 产品代码** | **未落地** | `src/components/**/*Graph*.vue` 0 命中；`scripts/check-graph-ui-logic.mjs` 0 命中 |
| **A9 W6 产品代码** | **未落地** | `src-tauri/src` 无 `PluginDef`/`PluginManifest`/`plugin.rs`；`scripts/check-plugin-policy.py` 0 命中 |

→ 本评审为**前瞻式契约审计 + 红线条目**（A8/A9 代码未进 mainline）。W5 缺口复核见 §4。

## 1. 边界契约（A8/A9 必须对齐）

来自 A4 的 5 码 + A7 的 7 码，本评审聚焦三族：

| 族 | A4 agent_memory | A7 graph | 对 A8/A9 含义 |
|---|---|---|---|
| 隐私双扫 | `AGENT_KV_PRIVACY_DOUBLE_SCAN` | `GRAPH_PRIVACY_DOUBLE_SCAN` | 任何持久化/展示层接触敏感值，必须**同时扫字段名（token/password/secret/api_key）+ 值模式（sk-/AKIA/Bearer /eyJ/-----BEGIN）** |
| 容量全局上限 | `AGENT_KV_BOUNDED_TOTAL`（5MiB / 5000 条 / 32 agent / LRU） | `GRAPH_BOUNDED_STORE`（5000/20000）+ `GRAPH_TRAVERSAL_BOUNDED`（depth 4 / limit 1000） | 所有 store/map/list 必须有**全局上限兜底**，不能只有单条/单查询上限 |
| 审计脱敏 | `AGENT_KV_AUDIT_NO_VALUE` | （M5-8 抽 Extractor 时补 audit-no-value） | 审计只记 key/摘要，**绝不序列化完整 value/body** |
| 无第二执行路径 | M4 护栏 | `GRAPH_NO_SECOND_PATH` | 纯策略/纯逻辑，不引入 install/exec/网络/第二进程 |

A7 `graph.rs:7` 明示：**"图谱不存 `agent_kv` 值（仅 `Memorizes` 关系边）"**；Skill/Agent 节点**按 id 引用** A5 `SkillDef.id`/`AgentDef.id`（64-hex 完整性 `GRAPH_REF_NODE_INTEGRITY`）。→ 当前无跨 store 泄露路径。

## 2. A8（M5-9 graph UI）隐私/容量交互评审

**结论：直接耦合低，但须守 5 条红线（代码落地后按 §5 复核）。**

- **R8-1（容量·消费 bounded DTO）**：UI 必须只消费 `GraphStore`/`GraphNode` bounded DTO；列表/搜索结果须遵守 `GRAPH_QUERY_LIMIT=1000`，**不得整表无界加载**，>1000 须分页。容量/错误/空态须映射到 `GraphError` 变体（`NodeCapacityExceeded`/`EdgeCapacityExceeded`/`SecretInProps`/`RefIdNotHex` 等），不得吞异常。
- **R8-2（隐私·展示脱敏·纵深防御）**：`graph.rs` 已在写入侧用 `graph_props_contain_secret` 拒绝敏感 props（G7-2 已解决），但 UI 仍须**纵深防御**：节点详情摘要不得渲染原始 props 值；即便未来数据从其他源导入绕过了双扫，UI 也不得泄露疑似凭据。建议 UI 侧对 props key 也跑一次 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS` 过滤（同 A4/A7 同源常量）。
- **R8-3（隐私·前端态·W6 硬停）**：graph 搜索/过滤输入、以及任何（未来）agent 消费流内容，须**组件内局部持有**，禁写入审计/日志/前端持久化。W6 硬停明令"no token/cookie/Authorization/body/prompt-secret logging or persistence"；A8 **禁止调用 live agent consumption**（看板 W6 硬停），故本项聚焦搜索态。
- **R8-4（不跨 store）**：A8 UI **不得直接读取 agent memory KV store**（独立子系统）；若需"记忆视图"，须走未来带脱敏的 bridge 命令（返回 `AgentKvRecordSummary`，无 value）+ 分页 ≤1000。
- **R8-5（引用完整性）**：渲染 Skill/Agent 节点时，仅展示 graph 节点自带的 id+label，**不得额外 fetch/缓存 A5 `SkillDef`/`AgentDef` 内部**（避免误拉 agent memory）；id 须按 `GRAPH_NODE_ID_HEX_LEN=64` 校验展示。

## 3. A9（M5-10/11 plugin manifest/lifecycle）隐私/容量交互评审

**结论：与 agent memory 的交互面在"权限 manifest 不得越权读 KV"与"manifest 本身不含 secret"，须守 6 条红线。**

- **R9-1（隐私·manifest 无 secret）**：plugin manifest 元数据**不得包含 secret 字段**（无 token/password/api_key/Authorization）；且 `scripts/check-plugin-policy.py` 必须含**隐私双扫**（复用 A4/A7 同款 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS`），对 manifest 字段名+值做断言。manifest 字段（name/version/description/author）须有长度上限（对齐 `GRAPH_LABEL_MAX_BYTES=256`/`MAX_TEXT_FIELD_BYTES=64KiB` 思路）。
- **R9-2（容量·bounded registry）**：若 A9 引入内存"已装插件登记表"，须有**全局上限**（插件数 / 单插件字段数 / 总字节），不得无界 map/list（对齐 `AGENT_KV_BOUNDED_TOTAL`/`GRAPH_BOUNDED_STORE`）。W6 硬停："All stores/maps/lists must be bounded"。
- **R9-3（不跨 store / 不读 agent memory）**：plugin **permission manifest 的能力白名单不得包含 agent memory KV 的读/写**（A5 `security_policy.rs` 能力枚举不得新增 agent_kv 读写权）。plugin 生命周期（install/enable/disable/uninstall）为**纯状态机**，不得（W6 硬停）install/uninstall/文件变更/网络/执行，**不得路径穿越进入 agent memory KV 存储目录**。
- **R9-4（无第二执行路径 / 无 install runtime）**：W6 硬停"no install/uninstall file mutation runtime, no downloaded plugins, no signature enforcement beyond pure validation unless fully local"。plugin lifecycle = 纯策略；与 A7 `GRAPH_NO_SECOND_PATH` + A4 M4 护栏"无第二执行路径"同源。
- **R9-5（未来命令须原子）**：若 W6 内新增任何 plugin 命令，必须同包内齐 source check + ACL（插 `list_artifact_images` 之前）+ 前端 bridge/types + 政策覆盖 + 测试；**W6 偏好不加命令**。
- **R9-6（审计脱敏）**：若 plugin lifecycle 落审计（如 plugin_install/enable 事件），审计 detail 不得含 manifest secret/路径超所需；对齐 `AGENT_KV_AUDIT_NO_VALUE`（只记 key/摘要）。

## 4. 跨 A8/A9 系统性发现（承接 W5 评审）

### 4.1 W5 缺口复核
| W5 缺口 | 状态 | 证据 |
|---|---|---|
| G7-1（缺全局 node/edge 上限） | **已解决** | `domain.rs:2114-2116` `GRAPH_MAX_NODES=5000`/`GRAPH_MAX_EDGES=20000`；`graph.rs` `insert_node/insert_edge` 在容量满时拒（`NodeCapacityExceeded`/`EdgeCapacityExceeded`），`from_json` 再校验；`GRAPH_BOUNDED_STORE` 码 PASS |
| G7-2（AGRAPH_2 不查敏感模式） | **已解决** | `graph.rs:18-20` 与 A4 同款 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS`；`graph_props_contain_secret` 双扫；`GRAPH_PRIVACY_DOUBLE_SCAN` 码 + 测试 `validate_rejects_secret_in_props`/`from_json_rejects_secret` PASS |
| G7-3（缺 audit-no-value 码） | **暂不适用** | `graph.rs` 为纯内存 core 切片，无 `graph_extract`/无 audit 路径（M5-8 持久化+抽取器留待后续）。**待 A7 M5-8 落 audit.json 时须补 audit-no-value**（detail 无 props/body），列入跟踪 |

### 4.2 残留 DRY 项（建议，非阻塞）
- **F1（敏感常量重复）**：`SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS` 目前在 `agent_memory.rs:134` 与 `graph.rs:18` **各定义一份**（值当前一致、均被政策覆盖）。与 M5-W2"常量集中 `domain.rs`"原则相悖；若敏感模式集演进（如增 `ghp_`/`xoxb-`），两处须同步改。建议：抽为 `domain.rs` 单一真源 `pub const SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS`，`agent_memory.rs` 与 `graph.rs` 改为 `use crate::domain::*;` 引用。A9 的 `check-plugin-policy.py` 也应复用同一份，避免第三份拷贝。

## 5. 解锁条件与逐行复核清单（A8/A9 落地后由 A4 执行）

**Unblock condition**：A8 的 `src/components/**/*Graph*.vue` + `src/stores/` + `scripts/check-graph-ui-logic.mjs`、A9 的 `src-tauri/src/plugin.rs`/`domain.rs` PluginDef + `scripts/check-plugin-policy.py` 进入 mainline 后，A4 做第二次 concrete review。

复核清单：
- [ ] A8：是否只消费 bounded DTO；列表/搜索遵守 `GRAPH_QUERY_LIMIT=1000` 且 >1000 分页；容量/错误/空态映射到 `GraphError`。
- [ ] A8：节点详情是否渲染原始 props 值（须不渲染 / 双扫过滤）。
- [ ] A8：搜索/过滤态是否组件内局部、无审计/日志/持久化外泄；是否调用 live agent consumption（应无）。
- [ ] A8：是否直接读 agent memory KV store（应无）。
- [ ] A9：`check-plugin-policy.py` 是否含隐私双扫（字段名+值模式，复用 A4/A7 同源常量）。
- [ ] A9：manifest 是否含 secret 字段（应无）；字段是否有长度上限。
- [ ] A9：插件能力白名单是否含 agent_kv 读/写（应无）；registry 是否 bounded。
- [ ] A9：lifecycle 是否纯状态机（无文件变更/exec/网络/路径穿越进 agent memory 目录）。
- [ ] A9：若新增命令，是否 source check+ACL+bridge/types+政策+测试同包。
- [ ] 交叉：A8/A9 新增敏感常量是否复用 `domain.rs` 单一真源（消除 F1 重复）。

## 6. 与 W6 Hard Stops 对齐

- "Only A8 and A9 may write product code in W6" → 本评审**零产品代码**，符合。
- "All stores/maps/lists must be bounded" → R8-1（graph UI 消费 bounded）+ R9-2（plugin registry bounded）。
- "no token/cookie/Authorization/body/prompt-secret logging or persistence" → R8-3 + R9-1（manifest 无 secret）。
- "A8 must not add live agent consumption… A9 must not install/delete/download/execute" → R8-3 + R9-4。

## 7. LANE 输出模板

```text
LANE=A4
STATUS=PASS（REVIEW ONLY，无产品代码）
BASE=77b1e3e
HEAD=logs/assist/A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md
FILES=logs/assist/A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md
VERIFY=git status clean @77b1e3e；check-agent-memory-policy.py --self-test PASS(ACTIVE=5)+默认 PASS；check-graph-policy.py --self-test PASS(ACTIVE=7)+默认 PASS；A8/A9 W6 代码未落地（*Graph*.vue/check-graph-ui-logic.mjs/PluginDef/check-plugin-policy.py 均 0 命中）；SENSITIVE 常量重复定义已确认(agent_memory.rs:134 + graph.rs:18)
CHECKPOINT=logs/assist/A4-M5-W6-memory-privacy-capacity-review-20260906-2237.md
MERGE_NOTES=本 Lane W6 仅评审，无冲突；W5 缺口 G7-1/G7-2 已确认由 A7 W5 集成解决，G7-3 暂不适用（待 M5-8 补 audit-no-value）；给 A8 五红线（R8-1~R8-5）、A9 六红线（R9-1~R9-6）；残留 DRY 项 F1（敏感常量抽 domain.rs 单一真源）；边界真相源=A4 W4 5 码 + A7 W5 7 码
NEXT=待 A8/A9 落地 W6 代码后，按 §5 清单做第二次 concrete review；F1 抽常量可建议 A0 套用补丁
```
