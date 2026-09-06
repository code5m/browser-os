# Lane A4 · M5-W5 Review Note — Agent Memory KV 与 A6/A7 的隐私/容量交互审计

> Lane=A4 · WAVE=M5-W5 · Status=**SUPPORT/REVIEW ONLY**（看板 §M5-W5：A4 仅评审 A6/A7 的 memory privacy/capacity 交互，无产品代码）
> 评审对象：A6（M5-6 Agent/Skill UI pure logic/panel shell）、A7（M5-7/8 graph model/store policy slice）
> 评审镜头（看板原话）：*"Review A6/A7 for memory privacy/capacity interactions"*
> 边界真相源：本 Lane 在 W4 落地的 agent memory KV 契约（`src-tauri/src/agent_memory.rs` + `scripts/check-agent-memory-policy.py`，已随 `1610939 feat(M5): add agent memory and skill policy shells` 集成）

## 0. 当前仓库实况（@ `0e76a89`，已 `git pull --ff-only`）

| 项 | 状态 | 证据 |
|---|---|---|
| A4 W4 agent memory KV 契约 | **已集成** | `domain.rs:1596-1604` 含 `AGENT_KV_MAX_TOTAL_BYTES=5MiB` / `MAX_TOTAL_RECORDS=5000` / `MAX_PER_AGENT_BYTES=1MiB` / `MAX_AGENTS=32`；`agent_memory.rs` 存在；`check-agent-memory-policy.py` 默认/自测 PASS |
| A5 W4 Agent/Skill 域 | 已集成 | `src-tauri/src/agent.rs`、`src-tauri/src/skills.rs`、`scripts/check-agent-skill-policy.py` 存在 |
| **A6 W5 UI 代码** | **未落地** | `src/components/agent/` 不存在；无 `useAgentStore.ts`/`ChatPanel.vue` 等 |
| **A7 W5 graph 代码** | **未落地** | 无 `graph*.rs`；无 `scripts/check-graph-policy.py`；`domain.rs` 无 `Graph*` 类型 |

→ 本评审为**前瞻式契约审计 + 红线条目**；A6/A7 落地后，A4 将按 §4 清单做逐行复核（解锁条件见 §4）。

## 1. 边界契约（A6/A7 必须对齐的 A4 agent memory KV 不变量）

`scripts/check-agent-memory-policy.py` 的 5 个 ACTIVE 码位即为 A4 侧权威边界：

| 码 | 不变量 | 对 A6/A7 的含义 |
|---|---|---|
| `AGENT_KV_CONSTANTS_PRESENT` | 容量常量单一真源在 `domain.rs` | A6/A7 新增任何容量常量须同样落在 `domain.rs`，不得散落字面值 |
| `AGENT_KV_PRIVACY_DOUBLE_SCAN` | 隐私闸**同时扫字段名（token/password/secret/api_key）+ 字符串值（sk-/AKIA/Bearer /eyJ/-----BEGIN）** | 任何持久化/展示层若接触敏感值，必须复用同一套 `SENSITIVE_KEY_NAMES` / `SENSITIVE_VALUE_PATTERNS` |
| `AGENT_KV_PER_AGENT_BYTES_NOT_COUNT` | per-agent 软配额=**字节（1MiB）**；agent 数上限=**32 独立** | "32" 是 agent 数上限，不是单 agent 条目数；容量三不变量（总/per-namespace/per-agent）逻辑分离 |
| `AGENT_KV_BOUNDED_TOTAL` | store 必须接总容量/总条目上限 + LRU 淘汰 | 所有持久化 store 必须有**全局总上限**兜底，不能只有单条/单查询上限 |
| `AGENT_KV_AUDIT_NO_VALUE` | 审计只记 `key_hash`，绝不序列化完整 `record.value` | 任何审计/日志不得含敏感正文 |

## 2. A6（M5-6 Agent/Skill UI）隐私/容量交互评审

**结论：直接耦合度低，但有两处必须在 A6 落地时守死。**

### 2.1 风险点
- **R6-1（隐私·前端态保密）**：A6 `ChatPanel.vue` 流式渲染 + `RunHistoryModal` 会触达 Agent/Skill 的对话/运行内容，可能包含 token/密码/Authorization/body。看板 W5 Hard Stop 明令"no token/cookie/Authorization/body/prompt-secret logging or persistence"。Agent memory KV 的隐私双扫原则（只记 key_hash、不持久化 value）应同等适用于 A6 前端态：**流式内容必须组件内局部持有，不得写审计/日志/前端持久化 store**。
  - `skills.rs:113` 现有测试夹具 `d.description = "token sk-abc123"` 仅是域测试数据，非持久化；但提示 A6 的 `SkillManager` 详情展示若渲染 `description`，须确认该字段不外泄到审计/日志（它属于 Skill 域元数据，非 agent memory KV value，但仍受 W5 硬停约束）。
- **R6-2（容量·未来 memory 浏览器）**：M5-6 本期不含 agent memory KV 浏览面板（W5 A6 仅 chat/manage/permission）。但若后续加"记忆查看器"，必须复用 `agent_memory.rs::list` 返回的 `AgentKvRecordSummary`（**无 value**），并遵守 32 agent / 1000 per-namespace / 5000 total 的**分页上限**，不得整表加载。
- **R6-3（能力预览安全）**：`PermissionPreviewModal` 逐项展示 `capabilities`，来源是 `security_policy.rs` 的 `AGENT_CAPABILITY_V1`/`SKILL_CAPABILITY_V1` 白名单（A5 `check-agent-skill-policy.py` 已守单一真源 + ACL 末条 K1），**不触及 agent memory KV value**，无泄露面。✅

### 2.2 给 A6 的红线条目（落地时必守）
- A6 面板/**不得**直接读取 agent memory KV store（W5 无 agent_memory 命令；任何 memory 视图须走未来带隐私闸的 bridge 命令）。
- 流式/历史内容组件内局部持有，禁写入 `audit.json` / 浏览器持久化 / 普通日志。
- 若新增 memory 视图：列表走 `AgentKvRecordSummary` + 分页（≤1000/页），绝不在 UI 渲染 `record.value`。

## 3. A7（M5-7/8 graph model/store）隐私/容量交互评审

**结论：A7 已有 per-node/per-query 上限，但存在两处与 A4 边界不对齐的缺口，须在 W5 落地时补齐。**

### 3.1 缺口
- **G7-1（容量·缺全局总上限）**：A7 契约（`M5-8` §4.2 / A7 impl-card §2.3）只定义 `GRAPH_PROPS_MAX_BYTES=64KiB`、`GRAPH_LABEL_MAX_BYTES=256`、`GRAPH_MAX_DEPTH=4`、`GRAPH_QUERY_LIMIT=1000`——**均为单条/单查询上限，无全局 node/edge 总数或 graph.db 体积上限**。这与 A4 `AGENT_KV_BOUNDED_TOTAL`（总 5MiB + 5000 条 + LRU 淘汰）不对齐。看板 W5 Hard Stop："All stores/maps/lists must be bounded"。建议 A7 增 `GRAPH_MAX_NODES` / `GRAPH_MAX_EDGES`（或 graph.db 体量软上限），超容按 source 优先级淘汰 `Extract`/`Ai` 节点（保留 `Manual`）——复用 agent memory KV 的"永久记录优先"思路。
- **G7-2（隐私·AGRAPH_2 只查字节不查敏感模式）**：A7 `check-graph-policy.py` 规划码 `AGRAPH_2` 仅断言 `props`/`label` 字节 ≤ 上限（A7 impl-card §3）。**未复用 A4 的隐私双扫**（字段名 + 值模式 sk-/AKIA/Bearer /eyJ/-----BEGIN）。后果：`graph_node_upsert` 可把 `{"token":"sk-abc123"}` 写进 graph.db 而 `AGRAPH_2` 不报（仅 64KiB 内即通过）。建议 AGRAPH_2 扩展为：字节上限 **且** 对 `props` 递归跑 `scan_json_value` 同款双扫（直接复用 `agent_memory.rs::SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS`，或把该常量抽到 `domain.rs` 单一真源供两脚本引用）。
- **G7-3（审计·缺 audit-no-value 码）**：A7 `graph_extract` 审计 `detail` 禁记原文（K3），但 A7 impl-card §3 的 `AGRAPH_1..8` 表**未列 audit-no-value 码**。建议补 `AGRAPH_9`：`graph_extract` 审计 detail 不得含 `props`/`body`/响应正文（对齐 `AGENT_KV_AUDIT_NO_VALUE`）。
- **G7-4（未来 M5-9 agent 消费）**：W5 A7 抽取源为 workspace 文档/脚本输出/tab（非 agent memory KV），当前无跨 store 泄露路径。但 M5-9（Graph UI + Agent 消费）若从 agent memory KV 派生图节点，须**只取 key/summary、绝不取 value**，并遵守全局容量——与 A4 审计脱敏同源。

### 3.2 给 A7 的红线条目（落地时必守）
- 增 `GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES`（或 DB 体量软上限）作为全局兜底，超容淘汰非 `Manual` 节点。
- `AGRAPH_2` 同时跑隐私双扫（字段名 + 值模式），与 agent memory KV 共用同一套敏感常量。
- 补 `AGRAPH_9`：graph 审计 detail 无 `props`/`body`/响应正文。
- `GraphNode.props` 不存主数据正文（K7）已守；与 A4 隐私原则一致。✅

## 4. 解锁条件与逐行复核清单（A6/A7 落地后由 A4 执行）

**Unblock condition**：A6 的 `src/components/agent/*` + `src/stores/useAgentStore.ts`、A7 的 `src-tauri/src/graph*.rs` + `scripts/check-graph-policy.py` 进入 mainline 后，A4 做第二次 concrete review。

复核清单：
- [ ] A6：是否新增任何 agent memory KV 直读/渲染 value 的路径（应无；若有须走带隐私闸的 bridge 命令）。
- [ ] A6：流式/历史内容是否组件内局部、无审计/日志/持久化外泄（grep `useAgentStore` 是否写盘；`audit.json` 是否含对话正文）。
- [ ] A7：`domain.rs` 是否新增 `GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES` 全局上限 + `graph_store.rs` 淘汰逻辑。
- [ ] A7：`check-graph-policy.py` `AGRAPH_2` 是否含隐私双扫（字段名+值模式），与 `agent_memory.rs` 敏感常量同源。
- [ ] A7：`AGRAPH_9` 是否守 graph 审计 detail 无 `props`/`body`。
- [ ] A7：ACL 9 条 `graph_*` 是否插 `list_artifact_images` 之前（K1，A4 W4 已守同序）。
- [ ] 交叉：A6/A7 新增容量常量是否全在 `domain.rs`（对齐 `AGENT_KV_CONSTANTS_PRESENT`）。

## 5. 与 W5 Hard Stops 对齐

- "Only A6 and A7 may write product code in W5" → 本评审**零产品代码**，符合。
- "All stores/maps/lists must be bounded" → A6 前端 store 须局部有界；A7 graph store 须补全局上限（G7-1）。
- "no token/cookie/Authorization/body/prompt-secret logging or persistence" → A6 前端态（R6-1）+ A7 graph props/audit（G7-2/G7-3）须共同守死。

## 6. LANE 输出模板

```text
LANE=A4
STATUS=PASS（REVIEW ONLY，无产品代码）
BASE=0e76a89
HEAD=logs/assist/A4-M5-W5-memory-privacy-capacity-review-20260906-1526.md
FILES=logs/assist/A4-M5-W5-memory-privacy-capacity-review-20260906-1526.md
VERIFY=git status clean @0e76a89；确认 A6/A7 W5 代码未落地（src/components/agent/ 与 graph*.rs 均不存在）；agent memory KV 常量已集成(domain.rs:1596-1604)；check-agent-memory-policy.py 默认/自测 PASS
CHECKPOINT=logs/assist/A4-M5-W5-memory-privacy-capacity-review-20260906-1526.md
MERGE_NOTES=本 Lane W5 仅评审，无冲突；给 A6 两条红线（R6-1/R6-2）、给 A7 三处缺口（G7-1 全局上限/G7-2 隐私双扫/G7-3 audit-no-value）作为落地守门建议；A4 W4 agent memory KV 契约为边界真相源
NEXT=待 A6/A7 落地 W5 代码后，按 §4 清单做第二次 concrete review；若 A0 决定，将 G7-1/G7-2/G7-3 抽成 A7 的 policy 脚本补丁由 A0 套用
```
