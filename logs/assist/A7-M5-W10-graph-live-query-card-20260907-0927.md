# Lane A7 · M5-W10 图谱 live-query 实现卡片（GRAPH DOCS ONLY · 后续 wave 实施）

> 续 W5（`4b438ef` 图谱 core）+ W6（契约评审 `A7-M5-W6-graph-contract-review-20260906-2239.md`）+ W7（桥卡片 `A7-M5-W7-graph-bridge-card-20260907-0725.md`）+ W8（桥卡片 `A7-M5-W8-graph-bridge-card-20260907-0756.md`）+ W9（契约定稿 `A7-M5-W9-graph-bridge-contract-20260907-0903.md`）。
> 本文件是 **M5-W10 Controlled Runtime Prep Dispatch（board `3792115`，A0 2026-09-07 16:00 添加）** 下 Lane A7 的 docs-only 交付。
> 依据 board §M5-W10（line 198）：`A7 = START GRAPH DOCS ONLY`，任务 = *“Prepare graph live-query implementation card for a later wave, including command names, result limits, cancellation, and privacy; do not implement commands.”* 交付物 = 本**图谱 live-query 实现卡片**（无产品代码）。
> **本波 W10 不写任何图谱命令/前端代码**；仅产出供后续 runtime wave 原子实施的蓝图。

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git pull --ff-only` | 已是最新（HEAD=`3792115`，W9 runtime-free polish 已合） | ✅ |
| NEXT/M5 | board §M5-W10：仅 A3 可碰 MCP stdio-prep；图谱 live-query 属“later wave”，**W10 不开** | ✅ 本包合规（仅卡片） |
| A3 排除遵守（沿用 W8+） | 不触碰 `mcp.rs`/A3 文件；图谱 live-query **零 MCP 依赖、不需 `mcp` feature gate** | ✅ |
| 是否越界 | 仅产出本卡片（`logs/assist/`）；**未**改 `graph.rs`/`domain.rs`/`main.rs`/`bridge.ts`/`default-commands.toml`/`src/types.ts`/`src/components/graph/**` | ✅ |
| 工作树脏文件 | W10 其他 lane（A1/A2/A3/A4/A5/A6/A8/A9/A10/A11）并发产物非本 Lane，不纳入、不触碰 | — |

## 1. W10 事实基线（board L179 + 仓库核查）

- W9 全绿：Agent/Skill bridge tests 26 passed、MCP tests 9 passed、plugin tests 10 passed、Agent/Skill UI logic **99 断言**、**Graph UI logic 43 断言**、npm build passes、MCP/Agent policy scripts pass。
- W10 仅 A3 可碰 MCP runtime-prep（stdio-only、feature-gated `mcp`、无 listener/网络、无 rmcp side-effect、无 file/db/script/plugin 执行）；Plugin/Agent 执行仍锁定。
- **A7 `graph.rs`/`domain.rs` 在 W9→W10 间零改动**（git diff 空）；`graph.rs` 仍 **0 命令**、`src/types.ts` 仍**无 `GraphNodeView`**、`graph.rs` 无 `mcp`/`feature` 引用 → 图谱 live-query 确属“later wave”，且**与 A3 `mcp` feature 解耦**（默认构建即包含，无需 feature gate）。

## 2. 图谱 live-query 实现卡片（后续 runtime wave 原子实施）

### 2.1 Command names（锁定，不再变更）
| 命令 | 用途 | 只读 |
|---|---|---|
| `graph_query` | 从 `start_id` 出发的有界子图查询 | ✅ |
| `graph_node_get` | 单点详情（Skill/Agent id 走 64-hex 校验） | ✅ |
| `graph_stats` | 容量概览（供 UI 黄牌） | ✅ |

> 命名沿用 W8/W9 定稿；实施 wave 直接采用，不再讨论。

### 2.2 Result limits（格式化约束，全部引用 `domain.rs` 单源常量）
- **遍历有界**：`depth = min(requested_depth, GRAPH_MAX_DEPTH=4)`；`limit = min(requested_limit, GRAPH_QUERY_LIMIT=1000)`；`bounded_subgraph`/`bounded_neighbors` 已强制。
- **容量硬上限**：`GRAPH_MAX_NODES=5000` / `GRAPH_MAX_EDGES=20000`（store 插入与 `from_json` 双守）。
- **字段长度**：`GRAPH_LABEL_MAX_BYTES=256`、`GRAPH_PROPS_MAX_BYTES=65536`（= `MAX_TEXT_FIELD_BYTES`）、`GRAPH_NODE_ID_HEX_LEN=64`。
- **返回截断信号**：`graph_query` 出参含 `truncated: bool`（触 `GRAPH_QUERY_LIMIT`）与 `applied: {depth, limit}`（回显实际生效 cap，静默截断，非错误）。
- **分页**：UI 每页 ≤ `GRAPH_QUERY_LIMIT/10`；`graph_node_get` 缺失返回 `null`（非错误）。
- **无界防护**：命令内**不**写字面量（`5000`/`256`/`1000`），全部引用常量；遍历为 BFS、`VecDeque`、访问集去重，杜绝指数爆炸。

### 2.3 Cancellation（W10 新增维度）
> 图谱查询是**纯内存、有界、stateless** 操作，正常 <1ms；以下契约保证“可取消且无副作用”：

- **前端**：`graphQuery()` 包装 `invoke('graph_query', …)` 由 `AbortController` 包裹；新查询发起时 `abort()` 上一个（复用 W6 `searchGraphNodes` 的 300ms debounce + “cancel last query” 范式）。
- **后端**：命令标 `#[tauri::command]` + `async`；因无 mutation / 无 IO / 无子进程，JS 丢弃 Promise 即 Tauri 取消 Future，**零副作用、零残留状态**。
- **关联键**：入参可选 `request_id: String`；UI 以 `request_id` 为 key 维护 `AbortController` map，避免乱序回包。
- **取消语义**：命令要么返回完整结果、要么被丢弃；**不产生半截结果、不写审计、不泄露 props**。因有界原子，无需“部分返回”。
- **不引入取消信道开销**：因 bounded 查询瞬时完成，后端**不**需 `tokio` cancel channel；契约仅要求“drop = no-op”。这与 W10 硬停 L208（rmcp/tokio 必须 optional+feature-gated、default 构建不变）一致——图谱 live-query **不引入**任何 `tokio`/`rmcp` 依赖。

### 2.4 Privacy（W10 A4 复审口径对齐）
- **输出脱敏**：`graph_query`/`graph_node_get` 返回 `GraphNodeView`/`GraphEdgeView`（仅 `{id,kind,label}` / `{from,to,kind}`），**不含 `props`**（K7 双闸：前端 helper 不渲染 + 命令输出即删）。
- **错误串零 secret**：所有错误返回**稳定码串**（`GRAPH_ERR:REF_ID_NOT_HEX` 等），**绝不** echo `props` 值 / secret / body / 文件路径 / capability reason。对齐 W10 A4 关注点（tool result URLs / capability reasons / serialized errors 均不得含 secret）——图谱面无 URL/capability 概念，错误仅为稳定码。
- **载入双扫**：`from_json` → `validate_graph_node/edge` 复用 W5 `GRAPH_PRIVACY_DOUBLE_SCAN`（字段名黑名单 `token/password/secret/api_key` + 值模式 `sk-/AKIA/Bearer /eyJ/-----BEGIN`），命中即拒（`SecretInProps`）。
- **审计/日志**：命令不写任何含 props/secret 的审计；无 checkpoint 落盘 props。
- **A4 W10 图谱面结论（予 A4 参考）**：可见错误/日志无 secret echo → PASS。

### 2.5 原子同包清单（实施 wave 守“每条命令原子化”）
| 文件 | 改动 |
|---|---|
| `src-tauri/src/graph.rs` | 新增 3 个 `#[tauri::command] pub async fn`（复用 `bounded_subgraph`/`find`；输出 `GraphNodeView`/`GraphEdgeView`；移除 `#![allow(dead_code)]`） |
| `src-tauri/src/main.rs` | `invoke_handler!` 注册 3 命令 |
| `src-tauri/permissions/default-commands.toml` | `commands.allow` 在末条 `list_artifact_images` **之前**插入 3 条 `graph_*` |
| `src/bridge.ts` | 新增 `graphQuery(req, signal?)`/`graphNodeGet()`/`graphStats()` 包装 `invoke`（支持 `AbortController.signal`） |
| `src/types.ts` | 新增 `GraphNodeView`/`GraphEdgeView`（仅 `{id,kind,label}` / `{from,to,kind}`） |
| `scripts/check-graph-policy.py` | 加 `GRAPH_OUTPUT_NO_PROPS` 码（扫描命令输出不含 `props`）+ 自测样本（ACTIVE=8） |
| `scripts/pre-merge.sh` | 既有 `GRAPH_*` 门禁自动覆盖新码 |
| 测试 | `graph.rs` 单测：小 store→`graph_query` 断言 depth/limit 截断 + 输出 JSON 无 `props` key + `graph_node_get` 64-hex 校验；`cargo test graph` 全绿；`check-graph-policy.py --self-test` PASS(ACTIVE=8) |

### 2.6 容量 / 错误态映射（对照 `graph.rs` 13 `GraphError`）
`graph.rs` → `GRAPH_INVALID_ID`(`EmptyId`/`IdTooLong`) / `GRAPH_REF_ID_NOT_HEX`(`RefIdNotHex`) / `GRAPH_STORE_LOAD_FAILED`(`Serde`/`SecretInProps`/`Node|EdgeCapacityExceeded`/`Label|PropKey|PropValue TooLong`/`PropCountExceeded`)；UI 态 `EMPTY`/`NO_RESULT`/`ERROR_*`/`TRUNCATED`/`OVERSIZE`/`APPROACHING_CAPACITY`（与 A8 `emptyStateFor`/`errorStateFor` 对齐）。

## 3. 与 W10 调度的关系（关键）

- **W10 不开图谱 live-query**：W10 仅开 A3 的 MCP stdio-prep（feature-gated）；图谱 live-query 命令属“later runtime wave”，本波**仅产卡片、不实现**（守 W10 A7 “do not implement commands”）。
- **零 A3/MCP 依赖**：图谱 live-query 只读内存 store，**不经 `mcp.rs`、不需 A3 的 `mcp` feature gate、不引 `rmcp`/`tokio`** → 实施 wave 默认构建即包含，与 W10 硬停 L208（default 构建不变）相容。
- **非 W10 “locked runtime surface”**：W10 锁定的是 Plugin/Agent 执行与 MCP stdio runtime；图谱 live-query 是**安全只读 runtime 增补**，可在任一不含 unsafe execution 的后续 wave 落地，**无需等 A3**，亦不触发 W10 硬停 L206/L207。
- **store 来源**：命令读 `GraphStore`（快照/AppState，由 `from_json` 载入）；命令**不构建** store（守“no graph rebuild worker”）；store 填充（extractor）属后续独立 scope，非本卡片阻塞项。

## 4. Build metrics / 硬停（实施 wave 守）

- **read-only**：无插入/删除/更新、无 skill/plugin/agent 执行、无 model call、无网络、无第二执行路径（命令复用 `graph.rs` helper，禁 `std::process`/`Command::new`/`tokio`/网络）。
- **ACL 同步**：新命令须 source check + ACL + bridge/types + 策略 + 测试同包（W10 硬停 L209）。
- **build metrics ≤22%**：实施 wave 引入少量 Rust 字节 + 前端类型；届时由 A11 复核（W10 阈值 22%，cargo warnings 不增）；本卡片不增字节。
- **无 secret echo**（L208 隐含 + W9/W10 隐私纪律）：错误串仅稳定码。
- 仅 A0 push（L211）。

## 5. 给各 Lane 的 NEXT

- **给 A0**：本卡片即后续 runtime wave 的图谱 live-query 实施蓝图；开启时由 A7/A8 按 §2 原子落地，**无需等 A3**、不引 `rmcp`/`tokio`、默认构建安全。
- **给 A3**：图谱 live-query 与你的 `mcp` feature 无关；请勿在 `mcp.rs` 内耦合图谱命令（守 core/bin 边界，A2 W10 复审）。
- **给 A8（W10）**：继续 Graph UI polish（no-backend 态/确定性/有界渲染）；W10 不加后端命令；`GraphNodeView`/`GraphEdgeView` 可于其后 UI wave 加（runtime-free），或随 §2.5 实施 wave 一起加。
- **给 A4（W10）**：图谱面隐私结论见 §2.4（稳定码错误、输出删 props、双扫）→ PASS。
- **给 A10（W10）**：图谱 live-query 卡片零 listener/网络/side-effect，与 A3 MCP prep 隔离 → 不阻塞。
- **给 A11（W10 收口）**：核对 `graph.rs` 0 命令（live 延后）、`src/types.ts` 无 `GraphNodeView`（待实施 wave）、`check-graph-policy.py` ACTIVE 仍 7（W10 不新增）、build metrics 21.07≤22%。

## 6. LANE 输出模板

```
LANE=A7
STATUS=PASS（GRAPH DOCS ONLY，无产品代码）
WAVE=M5-W10 Controlled Runtime Prep
BASE=3792115
HEAD=logs/assist/A7-M5-W10-graph-live-query-card-20260907-0927.md
FILES=logs/assist/A7-M5-W10-graph-live-query-card-20260907-0927.md
VERIFY=docs-only；产出图谱 live-query 实现卡片(后续 wave 原子实施)覆盖四维:① command names=graph_query/graph_node_get/graph_stats(锁定);② result limits=depth≤4/limit≤1000 有界+GRAPH_MAX_NODES=5000/EDGES=20000+truncated/applied 信号+无字面量全引常量;③ cancellation(新增)=前端 AbortController+debounce 取消+request_id 关联键,后端 async stateless 有界 drop=no-op 零副作用,不引 tokio/rmcp;④ privacy=输出删 props(GraphNodeView/EdgeView)+错误稳定码零 secret echo+GRAPH_PRIVACY_DOUBLE_SCAN+审计无 props(K7 双闸,对齐 W10 A4);原子包(graph.rs 命令+main.rs+ACL 插 list_artifact_images 前+bridge.ts 支持 signal+types.ts View 类型+check-graph-policy.py GRAPH_OUTPUT_NO_PROPS ACTIVE=8+测试)+13 GraphError→稳定码映射;W10 不开(仅 A3 MCP stdio-prep),零 A3/MCP 依赖不需 mcp feature gate 默认构建安全,非 locked runtime surface;A7 core 零改动(graph.rs 仍 0 命令,types.ts 仍无 View);未改任何产品代码;build metrics 保护(文档不增字节)
CHECKPOINT=logs/assist/A7-M5-W10-graph-live-query-card-20260907-0927.md
MERGE_NOTES=A7 W10 GRAPH DOCS ONLY 图谱 live-query 实现卡片:① 四维覆盖 command names(锁定 graph_query/graph_node_get/graph_stats)/result limits(depth≤4 limit≤1000 有界,GRAPH_MAX_NODES=5000/EDGES=20000,truncated+applied 信号,全引 domain.rs 常量无字面量)/cancellation(新增:前端 AbortController+300ms debounce 取消+request_id 关联;后端 async stateless 有界 drop=no-op 零副作用;不引 tokio/rmcp)/privacy(输出 GraphNodeView/EdgeView 删 props,K7 双闸;错误稳定码零 secret echo,对齐 W10 A4 对 tool result URL/capability reason/serialized error 的关注;GRAPH_PRIVACY_DOUBLE_SCAN 载入双扫;审计无 props);② 原子同包清单(graph.rs 3 async 命令+main.rs+default-commands.toml 插 list_artifact_images 前+bridge.ts 支持 AbortSignal+types.ts View 类型+check-graph-policy.py 新增 GRAPH_OUTPUT_NO_PROPS ACTIVE=8+测试)+13 GraphError→稳定码映射;③ W10 调度关系:图谱 live-query 属 later wave,W10 仅开 A3 MCP stdio-prep 故本波只产卡片不实现;零 A3/MCP 依赖不需 mcp feature gate(默认构建即含,与 W10 硬停 default 构建不变相容);非 W10 locked runtime surface(Plugin/Agent 执行/MCP stdio),属安全只读 runtime 增补,后续 wave 可落地无需等 A3;④ build metrics/硬停:read-only 无执行/网络/第二路径,ACL 同步同包,实施 wave 引少量字节届时 A11 复核 ≤22%;⑤ A7 core 零改动确认(graph.rs/domain.rs W9→W10 未动,graph.rs 仍 0 命令,types.ts 仍无 GraphNodeView,graph.rs 无 mcp/feature 引用);W10 事实基线 Graph UI 逻辑 43 断言/W9 全绿
NEXT=A0 后续 runtime wave 开启 graph_query 时按 §2 原子落地(由 A7/A8 实施,无需等 A3,不引 rmcp/tokio,默认构建安全);A8 W10 继续 Graph UI polish 不加后端命令;A3 勿在 mcp.rs 内耦合图谱命令
```
