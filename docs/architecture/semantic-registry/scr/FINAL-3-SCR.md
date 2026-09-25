# Final 3 SCR — graph / plugin / workspace

> 机器真源：`docs/architecture/semantic-registry/{states,owners,intents,side-effects}.yaml`
> 本文件是对 `coverage-matrix.md` 中 `OWNER_PENDING_SCR = {graph, plugin, workspace}` 的最终裁决。
> 裁决原则：以**真实代码**为据（非 D3 旧文档）；不机械补 Registry 凑 `FULLY_GOVERNED=26`；
> 若代码事实足够明确则自主裁决为 FULLY_GOVERNED；仅当两方案都改变核心语义且同样合理才 HARD STOP。
> 现场代码证明三者均为**真实 capability（含 Vue store + manifest + public.ts + ui）**，故裁决 FULLY_GOVERNED。

---

## SCR-GRAPH

### CONTEXT
`src/capabilities/graph/` 真实存在：`state/useGraphStore.ts`、`manifest.ts`、`public.ts`、`ui/*`、`index.ts`。
历史 D3 文档曾把 graph 标为「无前端 store / LOCKED」，与现场不符 → 旧 `OWNER_PENDING_SCR` 是基于过期文档的误判。

### CURRENT FACT
- 前端 `useGraphStore` 持有：面板选择/过滤/容量信号 + 有界投影缓存（nodes/edges 来自后端 `graph_*` 命令的窄视图，删 props 第三闸）。
- 图谱**数据真源在 Rust `src-tauri/src/graph.rs`（`GraphStore` 有界内存）**；前端 store 的 nodes/edges 是**投影缓存**，非第二真源。
- 后端 `graph_*` 命令未落地前 `backendReady=false`，所有动作 no-op 且零 invoke（LIMITED START 口径）。

### PROBLEM
旧状态 `OWNER_PENDING_SCR` 未反映真实前端 owner，导致 graph 既未被治理也未诚实归类。

### DOMAIN BOUNDARY
前端 graph capability = **面板视图状态 + 有界投影缓存**；图谱数据语义由 `graph.rs` 拥有。前端不得重建图、不得引入第二份图数据真源。

### STATE MODEL
- STORED：`nodes`, `edges`, `selectedNodeId`, `selectedEdgeKey`, `loading`, `error`, `backendReady`(shared), `startId`, `truncated`, `inFlightRequestId`, `filter`
- DERIVED：`nodeList`, `edgeList`, `visibleNodes`, `visibleEdges`, `capacity`, `capState`, `readOnly`, `selectionText`, `selectedNode`, `selectedEdge`, `state`
- 注：`nodes/edges` 是原生 graph.rs 投影，标记为 `derived_from: graph.rs`（非独立 stored 真源）。

### OWNER MODEL
CANONICAL OWNER = `useGraphStore`。无第二 owner。

### WRITER MODEL
CANONICAL WRITER：`loadGraph`, `loadNode`, `loadStats`, `selectNode`, `selectEdge`, `setFilter`, `toggleKind`, `clearFilter`（含 `refresh`, `cancelInFlight`）。

### INTENT MODEL
loadGraph / loadNode / loadStats / refresh / selectNode / selectEdge / setFilter / toggleKind / clearFilter / cancelInFlight。

### SIDE EFFECT MODEL
NATIVE（bridge，读）：`graphQuery`, `graphNodeGet`, `graphStats`。graph 写/导出在 A7 中 LOCKED，前端不触发。

### PUBLIC CONTRACT
`public.ts` 导出 `useGraphStore`；`bridge.graphQuery/graphNodeGet/graphStats`。

### RESOURCE BOUNDARY
前端不持有图谱资源；`graph.rs` 拥有原生有界存储（GRAPH_MAX_NODES/EDGES）。

### NATIVE BOUNDARY
`graph.rs` 为 native owner；前端只读投影。

### ALTERNATIVES
A. 把 graph 标 NOT_APPLICABLE（无前端 stored 真源）→ 拒：前端确有面板状态（selection/filter/in-flight/truncated）需治理。
B. 把 nodes/edges 当独立 stored 真源 → 拒：会与 graph.rs 形成第二真源，违反 Phase B 红线 #20。

### DECISION
**FULLY_GOVERNED**。nodes/edges 显式声明为 `derived_from: graph.rs`（投影缓存），不制造第二真源。

---

## SCR-PLUGIN

### CONTEXT
`src/capabilities/plugin/` 真实存在：`state/usePluginStore.ts`、`manifest.ts`、`public.ts`、`ui/PluginManager.vue`、`index.ts`。

### CURRENT FACT
- 前端 `usePluginStore` 持有：list / detail / keys 投影缓存、filterState、busy、error、安装表单**瞬时输入**（manifestText/resourcePath，成功后清空不回显）。
- 插件**五态机（AVAILABLE/INSTALLED/ENABLED/ACTIVE/RESOURCE_EXISTS）真源在后端/原生**；前端 `detail.state` 是**只读投影**，前端不持有五态。
- `backendReady=false` 时所有 action 返回 false + 稳定错误文案（零 invoke）。

### PROBLEM
旧 `OWNER_PENDING_SCR` 未反映真实前端 owner。

### DOMAIN BOUNDARY（五态思想，严禁压平）
前端**不重复**五态；五态机由原生 plugin runtime 拥有。前端只缓存 `detail.state`（投影）并用 `actionsFor` 按状态机门控按钮可用性。

### STATE MODEL
- STORED：`list`, `detail`, `keys`, `filterState`, `busy`, `error`, `manifestText`(ephemeral), `resourcePath`(ephemeral)
- DERIVED：`backendReady`(shared computed), `actionsFor`

### OWNER MODEL
CANONICAL OWNER = `usePluginStore`。

### WRITER MODEL
CANONICAL WRITER：`refreshList`, `getDetail`, `install`, `enable`, `disable`, `refreshKeys`, `addKey`, `removeKey`, `selectFromList`。

### INTENT MODEL
refreshList / getDetail / install / enable / disable / refreshKeys / addKey / removeKey / selectFromList / clearError。

### SIDE EFFECT MODEL
NATIVE（bridge）：`pluginList`, `pluginGet`, `pluginInstall`, `pluginEnable`, `pluginDisable`, `pluginKeysList`, `pluginKeysAdd`, `pluginKeysRemove`。
红线：不持久化/不回显签名原文、公钥原文、资源绝对路径、metadata 正文、凭据、请求/响应体、stdout/stderr。

### PUBLIC CONTRACT
`public.ts`；`bridge.plugin*`。

### RESOURCE BOUNDARY
插件资源（resourcePath）由原生 install side effect 管理；前端不持有。

### NATIVE BOUNDARY
Rust plugin commands 为 native owner。

### ALTERNATIVES
A. 为凑治理把五态压成 enabled/active/installed → 拒：违反红线 #19（plugin five-state collapse），且前端本就不持有五态。
B. 标 NOT_APPLICABLE → 拒：前端确有 list/detail/keys 缓存与生命周期 action 需治理。

### DECISION
**FULLY_GOVERNED**。五态机明确记为 NATIVE（前端投影），不压平、不重复。

---

## SCR-WORKSPACE

### CONTEXT
`src/capabilities/workspace/state/useWorkspaceStore.ts` 真实存在；Phase 8C-0 已完成 **God Store 收敛**：Files→useFileStore、Artifact→useArtifactStore、Repo→useRepoStore、Script→useScriptStore、Snippet→useSnippetStore（均已在 registry 治理）。

### CURRENT FACT
`useWorkspaceStore` 现仅持有：
- `audit`：Governance AUDIT 镜像（`bridge.auditLog()` 投影，归 Governance 域，8C-0 不动）
- `recents`：跨域（url+file）最近访问，`localStorage` 有界 30 条
- `refresh()`：聚合编排（`useArtifactStore().loadTree()` + `useRepoStore().loadRepos()` + `bridge.auditLog()`），**不持子域内部状态**

明确禁止回存 Files/Artifact/Repo/Script/Snippet 子域内部状态（WS_OWNER_* 门禁守护）。

### PROBLEM
旧 `OWNER_PENDING_SCR` 未反映：God Store 已分解，Workspace Core 仅剩审计镜像 + recents + 编排。

### DOMAIN BOUNDARY（严禁 God Store 回归）
Workspace = **AGGREGATE / ORCHESTRATOR / CONTAINER**，非子域状态的归属。子域状态由各自 owner 拥有，Workspace Core 仅消费/编排。

### STATE MODEL
- STORED：`audit`, `recents`
- DERIVED：无
- 注：`recents` 是跨域 UI 历史（url+file），有界 30，持久化于 localStorage；不是子域状态。

### OWNER MODEL
CANONICAL OWNER = `useWorkspaceStore`（仅 audit + recents + 编排）。

### WRITER MODEL
CANONICAL WRITER：`refresh`, `loadRecents`, `addRecentUrl`, `addRecentFile`, `openRecent`。
合法协作写入：`useFileStore` 调 `addRecentFile`（跨域写 recents 的 file 分支，非第二 owner，受控）。

### INTENT MODEL
refresh / loadRecents / addRecentUrl / addRecentFile / openRecent。

### SIDE EFFECT MODEL
NATIVE（bridge，读）：`auditLog`。

### PUBLIC CONTRACT
`public.ts`；`bridge.auditLog`。

### RESOURCE BOUNDARY
`recents` 是 UI 历史（localStorage）；子域资源各自 owner。

### NATIVE BOUNDARY
audit 由 governance/audit 后端拥有；前端仅镜像。

### ALTERNATIVES
A. 把子域状态收回 useWorkspaceStore → 拒：违反红线 #18（workspace God Store regression），且 8C-0 已分解。
B. 标 NOT_APPLICABLE → 拒：Core 确有 audit+recents 真实状态需治理。

### DECISION
**FULLY_GOVERNED**。Core 仅拥有 audit+recents+编排，子域状态保持各自 owner（不吸收）。

---

## 汇总裁决

| DOMAIN | OLD | NEW | REASON |
|---|---|---|---|
| graph | OWNER_PENDING_SCR | FULLY_GOVERNED | 真实前端面板状态 + 有界投影缓存；数据真源 graph.rs（derived_from 标注） |
| plugin | OWNER_PENDING_SCR | FULLY_GOVERNED | 真实 list/detail/keys 缓存 + 生命周期 action；五态机为 NATIVE（前端投影） |
| workspace | OWNER_PENDING_SCR | FULLY_GOVERNED | God Store 已分解（8C-0）；Core 仅 audit+recents+编排，子域各自 owner |

OWNER_PENDING_SCR → **0**（均有代码事实支撑，非机械补 Registry）。
