# Lane A7 · M5-W6 图谱契约评审笔记（SUPPORT/REVIEW ONLY）

> 续 W5（`logs/checkpoints/Lane-A7-M5-7-8-graph-core-20260906-2223.md`，W5 已交付图谱 core 切片并集成于 `4b438ef`）。
> 本文件是 **M5-W6 Parallel Dispatch（board `77b1e3e docs(M5): dispatch W6 graph UI and plugin lanes`）** 下 Lane A7 的评审交付。
> 依据 board §M5-W6：`A7 = SUPPORT/REVIEW ONLY`，任务 = “Review A8 graph UI against A7 DTO/query helpers; no graph product code unless fixing docs only.”
> 评审对象：A8 W6 图谱 UI 预案 —— `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` 的 `[W6 next-card acceptance criteria]` 段（AC-1~4 / HS1~5）+ `logs/assist/A8-M5-W5-graph-ui-delta-20260906-1525.md`。

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git pull --ff-only` | 已是最新（HEAD=`77b1e3e`，W6 dispatch 已含） | ✅ |
| NEXT/M5 | board §M5-W6：仅 A8 + A9 可写产品代码；A7 为 **REVIEW ONLY** | ✅ 本包合规（docs/review，无图谱产品代码） |
| 是否越界 | 仅产出本评审笔记（`logs/assist/`，docs）；**未**改 `graph.rs`/`domain.rs`/`main.rs`/`bridge.ts`/`src/types.ts`/`src/components/**` | ✅ |
| 工作树脏文件 | `M logs/checkpoints/M5-20260906/M5-0-overview.md` 为 **A1 W5 reconciliation**（diff 头标注 Lane A1），非本 Lane 改动 | 不纳入、不触碰 |

## 1. A7 权威契约面（A8 W6 必须消费的真源）

全部来自 A0 已集成提交 `4b438ef`：`src-tauri/src/domain.rs` L2039-2116（DTO + 7 常量）+ `src-tauri/src/graph.rs`（校验/容量/脱敏/bounded store/query helper）。

### 1.1 JSON 形状（serde：`#[serde(rename_all = "snake_case")]` + `#[serde(default)]`）

```jsonc
GraphNode = {
  "id": "string",                                  // 普通 sha256("kind:path")；Skill/Agent = sha256("skill:"+SkillDef.id) / sha256("agent:"+AgentDef.id)，64 位 hex
  "kind": "file|dir|tab|script|skill|agent|tag|topic",
  "label": "string",
  "props": { "string": "string" }                  // 可选(serde default={})；已脱敏，UI 禁显
}
GraphEdge = {
  "from": "string",
  "to": "string",
  "kind": "in_dir|references|related_to|tagged_with|uses|a2a_with|memorizes",
  "weight": 0,                                    // 可选(serde default=0)，类型 u32（非 i64）
  "props": { "string": "string" }                  // 可选(serde default={})；已脱敏，UI 禁显
}
GraphStore = { "nodes": [GraphNode...], "edges": [GraphEdge...] }
```

- `GraphProps = std::collections::BTreeMap<String, String>` → 序列化为**按 key 排序**的 JSON 对象。
- **枚举字符串值必须精确镜像**（前端 union 类型逐字一致，尤其 `a2a_with`/`related_to`/`tagged_with`/`in_dir` 带下划线）：
  - `GraphNodeKind`：file · dir · tab · script · skill · agent · tag · topic（8 个）
  - `GraphEdgeKind`：in_dir · references · related_to · tagged_with · uses · a2a_with · memorizes（7 个）

### 1.2 容量 / 脱敏常量（单一真源，A8 必须 import 镜像，**禁止字面量重定义**）

| 常量 | 值 | 用途 |
|---|---|---|
| `GRAPH_PROPS_MAX_BYTES` | = `MAX_TEXT_FIELD_BYTES`(64KiB=65536) | 单 prop 值上限（后端守；UI 仅展示 label，不直接触碰） |
| `GRAPH_LABEL_MAX_BYTES` | 256 | `truncateLabel(label, 256)` 截断依据 |
| `GRAPH_NODE_ID_HEX_LEN` | 64 | Skill/Agent 节点 id 完整性（AGRAPH-10） |
| `GRAPH_MAX_DEPTH` | 4 | 后端邻居遍历深度上限（前端静态快照无此概念，但模式须一致） |
| `GRAPH_QUERY_LIMIT` | 1000 | 查询/返回节点数上限 → 前端 list 分页每页 ≤ `GRAPH_QUERY_LIMIT/10 = 100` |
| `GRAPH_MAX_NODES` | 5000 | 容量提示阈值 `node_count >= 0.8*5000` |
| `GRAPH_MAX_EDGES` | 20000 | 边硬上限 |
| `GRAPH_PROPS_MAX_ENTRIES`（graph.rs 内部，非公开常量） | 64 | 单节点 props 条目上限（前端不重定义） |

### 1.3 查询助手语义（graph.rs，供 A8 在 TS 侧镜像 bounded 行为）

- `bounded_neighbors(start, depth, limit)`：把 `depth.min(GRAPH_MAX_DEPTH)`、`limit.min(GRAPH_QUERY_LIMIT)`，从 `start` 出发**无向** BFS，结果按层级稳定排序。
- `bounded_subgraph(start, depth)`：取 `bounded_neighbors` 节点 + 其**间**边，节点 ≤ `GRAPH_QUERY_LIMIT`。
- `to_json` / `from_json`：逐条校验（容量/脱敏/完整性）后序列化。
- 脱敏（AGRAPH-9）：`graph_props_contain_secret` 字段名黑名单 `token/password/secret/api_key` + 值模式 `sk-/AKIA/Bearer /eyJ/-----BEGIN` 双重扫描，fail-closed 拒绝。

## 2. 契约评审发现（Critical / Major / Minor）

### 🔴 F1（Critical）`summarizeNode` 白名单含 DTO 不存在的字段
M5-9 卡 W6 AC-1（L46）规定 `summarizeNode` 输出 `id / kind / label / source / source_ref / created_at / updated_at / extractor_version + 邻居计数`。
但 **A7 W5 `GraphNode` 仅有 `id / kind / label / props` 四个字段**（`domain.rs` L2081-2088 为权威真源）。且 `id = sha256("kind:path")`，是**不可逆哈希**，无法反解出 `source`/`source_ref`/`created_at`/`updated_at`/`extractor_version`。
→ **结论**：W6 `summarizeNode` 实际可输出字段仅为 `{ id, kind, label, neighborCount }`。`source/source_ref/created_at/updated_at/extractor_version` 在当前 DTO 下**不可得**，A8 不得假设它们存在，否则前端拿不到值（undefined）。
→ **处置建议（docs-only，A7 可提）**：请 A1/A0 修订 M5-9 卡 W6 AC-1 的 `summarizeNode` 白名单，改为 `{ id, kind, label, neighborCount }`；若业务确需 source/时间戳，应作为**后续 wave（非 W6）** 由 A7 在 DTO 上扩字段（属产品代码，W6 review-only 不做）。A8 当前应按真实可用字段实现并通过 AC-4 隐私 grep。

### 🔴 F2（Critical）无 `graph_*` 后端命令 → W6 必须吃静态快照
grep `graph_query|graph_neighbors|graph_store|invoke('graph` 全仓库 **0 命中**（`bridge.ts`/`commands`/`default-commands.toml` 均无）。W6-HS1 明文禁止新增后端命令。
→ **结论**：A8 W6 **不得**包装尚未存在的 `graph_query` 命令。W6 实现应基于**静态 `GraphNode[]` 快照 / fixture**（A8 W5 delta §2 已确认需 `backendReady` guard + live 接线留 W6+）。
→ **处置建议**：`useGraphQuery`/`useGraphStore` 在 W6 持有静态快照数组，纯 helper（`filterGraphNodes`/`searchGraphNodes`）直接对 `GraphNode[]` 运算；真实命令接线与 RAG 注入（`useGraphRag`、`/graph` 前缀）按卡 §4.3 冻结到 W7+。与 W6 AC-1「纯函数、无 Tauri invoke、无网络、无 fs」一致。

### 🟠 F3（Major）TS 枚举/字段须精确镜像（含下划线形变）
- `kind` 字符串必须逐字匹配 snake_case：`a2a_with`、`related_to`、`tagged_with`、`in_dir` 含下划线，易错写成 `a2aWith`/`relatedTo`/`taggedWith`/`inDir`。
- `weight` 为 `u32`（**非负整**），`#[serde(default)]` → TS 用 `weight?: number`（缺省 0），**不要用 `number|null` 或负数语义**。
- `props` 为 `#[serde(default)]` → TS 用 `props?: Record<string,string>`（缺省 `{}`），且**永不渲染**。
- 建议在 `src/types.ts` 用 `export type GraphNodeKind = 'file'|'dir'|'tab'|'script'|'skill'|'agent'|'tag'|'topic'` 等字面量联合，避免魔法字符串漂移（A11 验收 `src/types.ts` 与 A7 DTO 0 drift）。

### 🟠 F4（Major）容量常量单一真源
AC-4 用 `grep -nE 'GRAPH_MAX_NODES\s*=|GRAPH_LABEL_MAX_BYTES\s*=' src/utils/graphUi.ts` 强制 0 命中 → 容量字面量**只能**定义在 `src/types.ts`（镜像 `domain.rs`），`graphUi.ts` 须 `import`。A8 切勿在 helper 内写 `5000`/`256` 字面量。

### 🟠 F5（Major）隐私 K7 双闸（前端亦禁 props）
A7 后端 `graph_props_contain_secret` 已双扫 props；但 A8 前端仍须守 K7：node detail / list / search / filter / `summarizeNode` **不得显示 `props` 任何 key 或 value**。AC-4 隐私断言要求 `grep` 0 命中 `props` 于 `summarizeNode` 输出。W6-HS5 同义。前端是防御纵深，非可省。

### 🟡 F6（Minor）bounded 语义镜像
前端 `searchGraphNodes`/`filterGraphNodes` 返回数组须 cap ≤ `GRAPH_QUERY_LIMIT(1000)`；list 分页每页 ≤ `100`；`truncateLabel(label, GRAPH_LABEL_MAX_BYTES=256)` 截断后字节 ≤ 256（AC-4 容量断言）。后端 `GRAPH_MAX_DEPTH=4` 仅作用于后端邻居遍历，前端静态快照无 depth 概念，但分组/分页上限须与常量同源。

### 🟡 F7（Minor）Skill/Agent 节点 id 为 64-hex 哈希
前端**不得**尝试解析/反查 id（W6-HS2 禁 live agent 消费）。仅展示 `kind` 标签 + 截断后的 id（如 `sha256…ab12`）。`Memorizes`/`A2aWith`/`Uses` 边语义引用 A4 `agent_kv` / A5 `SkillDef.id`，UI **仅展示边类型名**，不得 fetch 真实值（K7 + W6-HS2）。

### 🟢 F8（Advisory）复用 A6 W5 面板壳范式
A8 `GraphPanel.vue` 应复用 A6 W5 已集成范式（`f99d2eb`：`src/components/agent/SkillManager.vue` / `PermissionPreviewModal.vue`）：`useLayoutStore` + `MOD_META` no-router 锚点 + Agent/Skill 校验/permission preview 模式，确保与既有 Agent/Skill UI 视觉/交互一致（A6 W6 负责 UI 一致性评审）。

### 🟢 F9（Advisory）不引 d3 整包（W6-HS4）
W6 仅列表 + 静态边框；力导向布局（`d3-force`/`d3-zoom`/`d3-drag`）留 W7+ 真接 layout 时引入，W6 **禁止**新增 npm 依赖。

## 3. 给 A8 的推荐 TS 镜像（消 F1/F3/F4，可直接落地）

```ts
// src/types.ts —— 镜像 domain.rs，容量常量单一真源
export type GraphNodeKind =
  | 'file' | 'dir' | 'tab' | 'script' | 'skill' | 'agent' | 'tag' | 'topic';
export type GraphEdgeKind =
  | 'in_dir' | 'references' | 'related_to' | 'tagged_with' | 'uses' | 'a2a_with' | 'memorizes';
export type GraphProps = Record<string, string>; // 永不渲染

export interface GraphNode {
  id: string;
  kind: GraphNodeKind;
  label: string;
  props?: GraphProps;
}
export interface GraphEdge {
  from: string;
  to: string;
  kind: GraphEdgeKind;
  weight?: number; // u32, default 0
  props?: GraphProps;
}

// 容量常量（必须 import，禁止字面量）
export const GRAPH_LABEL_MAX_BYTES = 256;
export const GRAPH_QUERY_LIMIT = 1000;
export const GRAPH_MAX_NODES = 5000;
export const GRAPH_MAX_EDGES = 20000;
export const GRAPH_MAX_DEPTH = 4;
export const GRAPH_NODE_ID_HEX_LEN = 64;

// src/utils/graphUi.ts —— 纯函数，无 invoke/网络/fs
export interface NodeSummary { id: string; kind: GraphNodeKind; label: string; neighborCount: number; } // F1: 仅真实字段
export function filterGraphNodes(nodes: GraphNode[], f: GraphFilter): GraphNode[] { /* 按 kind/source/label 包含 */ }
export function searchGraphNodes(nodes: GraphNode[], q: string, opts?: {}): SearchResult[] { /* label 模糊 + id 精确; q 长度 0-200; 空 q 返回 [] */ }
export function summarizeNode(node: GraphNode, neighborCount: number): NodeSummary { /* 仅 id/kind/label/neighborCount，绝不碰 props */ }
export function emptyStateFor(reason: 'list'|'filtered'|'no-result'): EmptyStateDescriptor {}
export function errorStateFor(err: 'load'|'parse'|'action'): ErrorStateDescriptor {}
export function truncateLabel(label: string, maxBytes = GRAPH_LABEL_MAX_BYTES): string { /* 按字节截断 ≤ 256 */ }
```

## 4. 给各 Lane 的 NEXT

- **给 A0**：本评审为 docs/review，可直接 drop（`logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md`）；**F1 要求修订 M5-9 卡 W6 AC-1 的 `summarizeNode` 白名单**（docs，A1 职责），否则 A8 实现会被卡死在不存在的字段。
- **给 A1**：请按 F1 修订 M5-9 卡 W6 AC-1 `summarizeNode` 白名单为 `{id,kind,label,neighborCount}`；如需 source/时间戳，走后续 wave 由 A7 扩 DTO（非 W6）。另：`logs/checkpoints/M5-20260906/M5-0-overview.md` 当前为 A1 W5 reconciliation 脏文件，与本评审无关。
- **给 A8**：W6 按 F1~F9 实现（静态 `GraphNode[]` 快照 + 纯 helper + 第 3 节 TS 镜像）；`summarizeNode` 仅用真实字段；`grep` 0 命中 `props`；容量常量全部 import。
- **给 A6**：A8 `GraphPanel.vue` 视觉/交互须与 A6 W5 Agent/Skill UI 壳一致（F8），A6 W6 负责一致性评审。
- **给 A10**：W6 复审重点 = K7 零泄露（`summarizeNode`/node detail 不显 props）+ 无新命令（W6-HS1）+ 无 live agent（W6-HS2）+ 无 model call/rebuild worker（W6-HS3）+ 无 npm 依赖（W6-HS4）。
- **给 A11**：W6 收口核对 `src/types.ts` 与 A7 DTO 0 drift + `grep` 容量字面量 0 命中 + `grep props` 0 命中于 helper 输出。

## 5. LANE 输出模板

```
LANE=A7
STATUS=PASS（REVIEW ONLY，无产品代码）
WAVE=M5-W6
BASE=77b1e3e
HEAD=logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md
FILES=logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md
VERIFY=docs/review only；评审基于 A7 W5 已集成 DTO(domain.rs 4b438ef)+graph.rs；grep 确认无 graph_* 后端命令(0 命中)；未改任何产品代码；git diff --check 未涉及本 Lane 文件
CHECKPOINT=logs/assist/A7-M5-W6-graph-contract-review-20260906-2239.md
MERGE_NOTES=W6 A7 为 REVIEW ONLY，评审 A8 图谱 UI 预案(M5-9 卡 W6 AC + A8 W5 delta)对照 A7 W5 DTO/查询助手；核心发现：F1 summarizeNode 白名单含 5 个 DTO 不存在字段(source/source_ref/created_at/updated_at/extractor_version，且 id 为 sha256 不可反解)须由 A1 修订卡；F2 无 graph_* 后端命令须吃静态 GraphNode[] 快照；F3 enum 须精确 snake_case 镜像(含 a2a_with/related_to/tagged_with/in_dir)、weight=u32 可选、props 可选；F4 容量常量单源 import；F5 K7 双闸前端亦禁 props；F6 bounded 语义镜像；F7 Skill/Agent id 为 64-hex 不反查；F8 复用 A6 面板壳；F9 不引 d3 整包。提供可直接落地的 TS 镜像
NEXT=A1 修订 M5-9 卡 W6 AC-1 后 A8 实施；如需 source/时间戳字段，后续 wave 由 A7 扩 DTO（非 W6）
```
