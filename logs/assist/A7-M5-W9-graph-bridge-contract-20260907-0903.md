# Lane A7 · M5-W9 图谱桥契约定稿（GRAPH CONTRACT DOCS ONLY · runtime-free 分离）

> 续 W5（`4b438ef` 图谱 core）+ W6（契约评审 `A7-M5-W6-graph-contract-review-20260906-2239.md`）+ W7（桥卡片 `A7-M5-W7-graph-bridge-card-20260907-0725.md`）+ W8（桥卡片 `A7-M5-W8-graph-bridge-card-20260907-0756.md`）。
> 本文件是 **M5-W9 Runtime-Free Polish Dispatch（board `97118d6`，A0 2026-09-07 14:30 添加）** 下 Lane A7 的 docs-only 交付。
> 依据 board §M5-W9（line 197）：`A7 = START GRAPH CONTRACT DOCS ONLY`，任务 = *“Finalize next graph bridge contract for later live graph query commands, separating runtime-free UI/store items from blocked backend runtime items.”* 交付物 = 本**契约定稿**笔记（runtime-free 项 与 blocked backend runtime 项**显式分离**）。
> **本波 W9 不写任何图谱命令/前端代码**，仅定稿契约面。

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git pull --ff-only` | 已是最新（HEAD=`97118d6`，W9 dispatch 已含；W8 验证后 A0 privacy/fmt/metrics 修复已合） | ✅ |
| NEXT/M5 | board §M5-W9：W9 = runtime-free polish + 收口验证；A7 为 **GRAPH CONTRACT DOCS ONLY** | ✅ 本包合规（仅笔记） |
| A3 排除遵守（沿用 W8） | 不触碰 `mcp.rs`/A3 文件；图谱桥零 MCP 依赖 | ✅ |
| 是否越界 | 仅产出本笔记（`logs/assist/`）；**未**改 `graph.rs`/`domain.rs`/`main.rs`/`bridge.ts`/`default-commands.toml`/`src/types.ts`/`src/components/graph/**` | ✅ |
| 工作树脏文件 | W9 其他 lane 并发产物非本 Lane，不纳入、不触碰 | — |

## 1. W8→W9 事实基线（来自 board L178 + 仓库核查）

- A3 MCP 策略 current-phase：**ACTIVE=8 / PENDING=0**（W8 `94e763e` 结清 W1 pending 语义债，加 `MCP_NO_RMCP_SERVER` + `--expect-current-gaps`）。
- A5 Agent/Skill 策略在 A0 红acted `CredentialLeak` Display 后转 **green**。
- **Graph UI 逻辑 41 断言**（A8 W8 `4d7be97` 仅动 A8 的 `scripts/check-graph-ui-logic.mjs` + `src/components/graph/*`，**未**动 A7 `graph.rs`/`domain.rs`）。
- `npm run build` PASS；build metrics **21.07 ≤ 22%**（cargo warnings 未增）。
- `graph.rs` 仍 **0 命令**；`src/types.ts` 仍**无 `GraphNodeView`** → 后端 live 命令确属「延后」，与 W9「for later live graph query commands」一致。

## 2. 契约定稿（锁定，供后续 backend-runtime wave 实施）

> 沿用 W8 卡片 §3.1–§3.5，此处**定稿**（签名/错误码/类型/策略/原子包不再变更）；实施方按此精确落地。

### 2.1 只读命令（延后到 backend-runtime wave，非 W9）

| 命令 | 入参 | 行为 | 出参 | 错误码（稳定串，绝不 echo props/secret） |
|---|---|---|---|---|
| `graph_query` | `{start_id, depth=1, limit=1000}` | `depth=min(depth,4)`；`limit=min(limit,1000)` → `bounded_subgraph` 有界子图 | `{nodes:GraphNodeView[], edges:GraphEdgeView[], truncated, applied:{depth,limit}}` | `GRAPH_INVALID_ID` / `GRAPH_REF_ID_NOT_HEX` / `GRAPH_STORE_LOAD_FAILED` |
| `graph_node_get` | `{id}` | Skill/Agent id 走 `is_hex(64)` 校验 | `GraphNodeView \| null`（缺失返 `null`） | `GRAPH_INVALID_ID` / `GRAPH_REF_ID_NOT_HEX` |
| `graph_stats` | 无 | 容量概览 | `{node_count, edge_count, max_nodes, max_edges, approaching_capacity}` | `GRAPH_STORE_LOAD_FAILED` |

### 2.2 脱敏视图类型（frontend，runtime-free）
```ts
export interface GraphNodeView { id: string; kind: GraphNodeKind; label: string; }
export interface GraphEdgeView { from: string; to: string; kind: GraphEdgeKind; }
```

### 2.3 容量 / 错误态映射（对照 `graph.rs` 13 `GraphError`）
`graph.rs` → `GRAPH_INVALID_ID`(`EmptyId`/`IdTooLong`) / `GRAPH_REF_ID_NOT_HEX`(`RefIdNotHex`) / `GRAPH_STORE_LOAD_FAILED`(`Serde`/`SecretInProps`/`Node|EdgeCapacityExceeded`/`Label|PropKey|PropValue TooLong`/`PropCountExceeded`)；UI 态 `EMPTY`/`NO_RESULT`/`ERROR_*`/`TRUNCATED`/`OVERSIZE`/`APPROACHING_CAPACITY`（与 A8 W6 `emptyStateFor`/`errorStateFor` 对齐）。

### 2.4 原子同包清单（实施 wave 守「每条命令原子化」）
`graph.rs` 命令 + `main.rs` `invoke_handler!` + `default-commands.toml` ACL（插末条 `list_artifact_images` **前**）+ `bridge.ts` 包装 + `types.ts` `GraphNodeView`/`GraphEdgeView` + `check-graph-policy.py` 新增 `GRAPH_OUTPUT_NO_PROPS`(ACTIVE=8) + `cargo test graph` 全绿 + `check-graph-policy.py --self-test` PASS(ACTIVE=8)。

### 2.5 策略对齐
复用 W5 7 `GRAPH_*` 码 + 新增 `GRAPH_OUTPUT_NO_PROPS`（W8 已规划）。

---

## 3. RUNTIME-FREE UI/STORE 项（现在可收尾 · 不含后端 runtime）

> 这些项**不**需要 live 命令，可于 W9 runtime-free polish wave 由 A8 收尾；属本波“可交付”范围（A7 仅定稿，A8 实施）。

| 项 | 当前状态 | W9 处理方 | 后端依赖 |
|---|---|---|---|
| `GraphPanel` 静态消费（`useGraphStore.ts` fixture，5 节点 + 7 边） | 已落（W6/W8，41 断言） | A8 W9 polish（a11y/确定性/empty/error/oversize/no-backend 态） | 无 |
| `GraphNodeView`/`GraphEdgeView` TS 类型 | **未加** | 可于 W9 由 A8 加（纯 frontend，runtime-free）；或随 backend wave 一起加 | 无（仅类型） |
| 纯 helper `filterGraphNodes`/`searchGraphNodes`/`summarizeNode`（4 字段白名单） | 已落（A1 W7 按 W6 F1 改 8→4） | A8 W9 保持确定性 | 无 |
| 7 容量常量（`types.ts`，与 `domain.rs` 对齐） | 已落 | 仅展示/守卫用 | 无 |
| 策略文档 + `GRAPH_OUTPUT_NO_PROPS` 码 | W8 已规划 | A7 定稿于此；实施 wave 落地脚本 | 无 |
| 确定性 filter/search/layout + bounded arrays + no-backend/read-only 态 | A8 W8 已落 | A8 W9 打磨 | 无 |

**结论**：runtime-free 项目前**无阻塞**，W9 可由 A8 完成收尾；不引入 backend 命令即可闭环视觉/逻辑验收。

---

## 4. BLOCKED BACKEND RUNTIME 项（延后到 backend-runtime wave · 非 W9）

> W9 明确 live graph query 命令属「later」；这些项**不在 W9 实施**，待 A0 开启「backend-runtime wave」（且在任何 MCP server / plugin runtime / skill execution wave **之前**）。

### 4.1 A3 无关但延后（可独立交付，待 A0 开 backend wave）
- **`graph_query` / `graph_node_get` / `graph_stats` 三条 Tauri 命令**（§2.1）—— 仅需本仓库 Tauri ACL + 既有的 `graph.rs` helper；**零 A3/MCP 依赖**（沿用 W8 判定）。
- **store 来源接线**：命令读取 `GraphStore`（快照/AppState）；store 构建仍由 `from_json` 载入（无 live extractor，属后续 scope 决策，非 A3 依赖）。命令**不**构建 store（守“no graph rebuild worker”）。

### 4.2 A3/MCP-gated（待 A3 恢复后才可实施）
- 经 MCP/rmcp 暴露 `graph_query`（`mcp_graph_query` capability）。
- 图谱 DTO 引用 MCP registry tool id（`GraphNodeKind::McpTool` / `GraphEdgeKind::UsesMcpTool`）。
- 图谱↔MCP 策略交叉校验（命令前调 `mcp_capability_preview`）。

**结论**：§4.1 是 W9 之后首个可落地的「图谱能力」wave，且与 A3 解耦；§4.2 才是真·A3 阻塞项。W9 本波二者均**不**实施。

---

## 5. 隐私 / 安全对齐（W8 A0 `CredentialLeak` 红action 后）

- 图谱命令错误串仅含**稳定码**（`GRAPH_ERR:REF_ID_NOT_HEX` 等），**绝不** echo `props` 值/secret/body（守 W8 硬停 L207 + W9 硬停 L207）。
- `GRAPH_PRIVACY_DOUBLE_SCAN`（载入 `from_json`→`validate_*` 双扫）+ 命令输出删 `props`（§2.1 `GraphNodeView`/`GraphEdgeView` 无 props）= 双重保险。
- **A4 W9 privacy 复审图谱面结论（予 A4 参考）**：图谱可见错误/审计文本无 secret echo → PASS；无政策样例需补。

## 6. Build metrics 保护

- 本 W9 契约笔记为 docs-only，**不增** bundle/Rust 字节 → 维持 21.07 ≤ 22%。
- 实际 backend 命令（§4.1）会在未来实施 wave 引入少量 Rust 字节；届时由 A11 重新核 metrics，不属 W9 范畴。

## 7. 硬停 / 禁止（W9）

- 无 MCP server/listener/rmcp、无 plugin install/enable/delete/download、无 skill/agent 执行、无 model call、无网络（W9 硬停 L205）。
- 新/改命令须只读 + source check + ACL + bridge/types + 策略 + 测试同包（L206）—— 由 §4.1 实施 wave 守。
- 无 token/cookie/Authorization/body/prompt-secret 于日志/审计/前端态/checkpoint/错误串（L207）。
- build metrics 阈值 22%；超出或 cargo warnings 增 → 阻塞 A0 push（L208）。
- 仅 A0 push（L209）。

## 8. 给各 Lane 的 NEXT

- **给 A0**：本笔记即 W9 后「backend-runtime wave」图谱落地蓝图；开启时由 A7/A8 按 §2+§4.1 原子落地即可，**无需等 A3**（§4.2 才需 A3）。
- **给 A8（W9）**：runtime-free 项（§3）现可收尾——`GraphPanel` polish、`GraphNodeView`/`GraphEdgeView` 类型（可选加）、保持确定性/no-backend 态；W9 不碰后端命令。
- **给 A2/A4/A10（W9）**：复审重点 = 图谱面零 A3/MCP 泄漏（§4.2 未混入）、只读、输出零 props、无第二路径、错误串无 secret echo（§5）。
- **给 A11（W9 收口）**：核对 `graph.rs` 0 命令（live 延后）、`src/types.ts` 无 `GraphNodeView`（待 backend wave）、`check-graph-policy.py` ACTIVE 仍 7（W9 不新增，待实施 wave 升 8）、build metrics 21.07≤22%。

## 9. LANE 输出模板

```
LANE=A7
STATUS=PASS（GRAPH CONTRACT DOCS ONLY，无产品代码）
WAVE=M5-W9 Runtime-Free Polish
BASE=97118d6
HEAD=logs/assist/A7-M5-W9-graph-bridge-contract-20260907-0903.md
FILES=logs/assist/A7-M5-W9-graph-bridge-contract-20260907-0903.md
VERIFY=docs-only；定稿 next 图谱桥契约(锁定 graph_query/graph_node_get/graph_stats 签名+13 GraphError→稳定码+GraphNodeView/EdgeView+原子包+GRAPH_OUTPUT_NO_PROPS)；显式分离 runtime-free UI/store 项(可 W9 收尾,无后端依赖)与 blocked backend runtime 项(live 命令+store 来源延后,其中 A3/MCP-gated 子项待 A3 恢复)；A3 排除遵守(0 触碰 mcp.rs)；W8→W9 A7 core 零改动(graph.rs/domain.rs 未动,graph.rs 仍 0 命令,types.ts 仍无 View 类型)；未改任何产品代码；build metrics 保护(文档不增字节,21.07≤22%)
CHECKPOINT=logs/assist/A7-M5-W9-graph-bridge-contract-20260907-0903.md
MERGE_NOTES=A7 W9 GRAPH CONTRACT DOCS ONLY 定稿笔记:① 锁定 next 图谱桥契约(签名/错误码/类型/策略/原子包,源自 W7+W8 卡片,不再变更)供 backend-runtime wave 实施;② 显式分离两桶:RUNTIME-FREE UI/STORE 项(可 W9 收尾,无后端依赖=GraphPanel 静态消费/fixture/41断言+GraphNodeView/EdgeView 类型可选加+纯 helper 4字段白名单+7容量常量+策略文档+确定性/no-backend态,由 A8 W9 polish);BLOCKED BACKEND RUNTIME 项(live graph_query/graph_node_get/graph_stats 三条命令+store 来源接线,延后到 backend-runtime wave;其中 A3/MCP-gated 子项=经 MCP 暴露 graph_query/MCP tool 节点 kind/图谱↔MCP 交叉校验 待 A3 恢复);③ 沿用 W8 判定:live 命令零 A3/MCP 依赖(可独立于 A3 交付);④ 隐私对齐 W8 A0 CredentialLeak 红action:错误串仅稳定码不 echo props/secret,GRAPH_PRIVACY_DOUBLE_SCAN+输出删 props 双保险,A4 W9 图谱面 PASS;⑤ build metrics 保护(文档不增字节,维持 21.07≤22%);⑥ W8→W9 A7 core 零改动确认(graph.rs/domain.rs 未动,graph.rs 仍 0 命令,types.ts 仍无 GraphNodeView)
NEXT=A0 后续 backend-runtime wave 开启 live graph_query 时按 §2+§4.1 原子落地(由 A7/A8 实施,无需等 A3);A8 W9 收尾 runtime-free 项;§4.2 A3-gated 项待 A3 恢复
```
