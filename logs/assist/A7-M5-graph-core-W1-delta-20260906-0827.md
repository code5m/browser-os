# A7 · M5-W1 增量说明（Delta Note · 图模型/存储对齐 A2 core 边界，支持文档）

> LANE=A7　WAVE=M5-W1 Implementation Dispatch　STATUS=PASS_WITH_DOCS（SUPPORT DOCS ONLY，无产品代码）
> BASE=`a1a2061`（本地 `master`，领先 `origin/master` 1；M4 已集成推送：cargo test 329/329、npm run build PASS、pre-merge ALL_PASS）
> SCOPE=`logs/assist/A7-M5-graph-core-*.md` only（本文件 + 前作 `A7-M5-graph-core-20260906-0755.md`）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → A7：**SUPPORT DOCS ONLY**（行 154）；W1 Hard Stops（仅 A2 可动产品代码）
> 目标产物：`logs/assist/A7-M5-graph-core-W1-delta-20260906-0827.md`（本文件）
> 配套前作：`logs/assist/A7-M5-graph-core-20260906-0755.md`（M5-W0 契约预研，仍有效，本文件为其增量修订）
> 衔接输入：`A2-M5-core-20260906-0749.md`（§8 行 420 给 A7 的边界钩子、§3.3 `COREBOUND_*`）、`M5-20260906/M5-7-graph-model-extract.md`、`M5-20260906/M5-8-graph-store-query.md`（责任 Lane 候选 A17）

---

## 0. 增量摘要（给 A0 / A17 / 后续 Lane）

W1 相对 W0 的关键变化，本增量说明逐条对齐：

1. **A7 在 W1 的角色 = 支持文档，不是实现者**。M5-7/M5-8 的实现责任 Lane 候选为 **A17**（见 `M5-7`/`M5-8` 卡顶部「责任 Lane 候选：A17，A0 签发时定」；A9 提案）。W0 文档的「A7 实施」措辞**作废**，改为「A0 签发后由 A17 实施」。
2. **W1 硬停：仅 A2 可动产品代码**。A7 不得写 `src-*`/`Cargo.toml`/ACL/新 Tauri 命令/`pre-merge.sh`。本文件零产品代码。
3. **指挥板对 A7 的唯一硬性要求 = 「图模型/存储对齐 A2 边界，识别哪些纯函数落在 core」**（行 154）。本文件 §2/§3 把 M5-7/M5-8 卡里**平铺在 `src-tauri/src/` 的 graph 模块重新落位到 `src-tauri/src/core/`**，并逐函数分类纯逻辑 vs I/O vs 命令面。
4. **修正 W0 的 G-D1**：W0 称「graph.db 独立 SQLite 文件，不经 DbPool/Keyring」。A2 §8 行 420 要求「不得新起 SQLite 连接路径（复用 `database.rs`）」。二者**不矛盾、已收敛**：graph 复用 `database.rs` 的 SQLite 访问原语（open/query/row-map 单一代码路径，零新依赖），graph.db 是**独立内部文件**（无用户凭据、不经 DbPool 凭据路径、不经 Keyring）——见 §4.1。
5. **修正 W0 的 G-D2**：W0 称「A3 提升 `QueryLimiter`（M4-1.c 结果上限）供 graph 复用」。M4-1.c 仍 STOPPED，A3 在 W1 为 SUPPORT DOCS ONLY；`database.rs` 在 M4-2 已有行/字节上限逻辑，graph 查询结果上限**直接复用 `database.rs` 既有上限**，不再另造 limiter——见 §4.2。
6. **类型落点修正**：M5-7 卡把 `GraphNode/GraphEdge/GraphSchema` 写进 `domain.rs`。A2 切片 0a 把 `domain` 迁 `core`，故这些类型应落地 `src-tauri/src/core/graph_model.rs`（core 内），二进制侧经 `pub use mvp_browser_os_core::graph_model;` shim 免改写——见 §3。

---

## 1. W1 角色与依赖闸门（修正 W0 的「A7 实施」假设）

| 项 | W0 文档旧措辞 | W1 修订 |
|---|---|---|
| M5-7/8 实现责任 | 「A7 实施 graph 批」 | **A0 签发后由 A17 实施**（候选见 M5-7/8 卡） |
| A7 在 W1 的状态 | DOCS ONLY（被动） | **SUPPORT DOCS ONLY（主动增量：对齐 A2 边界）** |
| 产品代码 | 全禁 | 全禁（仅 A2 可动） |
| 实施解锁条件 | W0 §「实施闸门」 | 追加：**A2 M5-1 core boundary 落地 + A2 M5-1.b（B 类 seam，含 `PathResolver`）落地**（见 §4） |

> A7 的「整包交付」在 W1 = 本增量文档；不产 patch（W1 硬停禁止产品代码，assist 文档由 A0 直接拣入，同 W0 口径）。

---

## 2. 边界对齐 A2 `mvp_core`：哪些 graph 纯函数落在 core

A2 §8（行 420）给 A7 的钩子原文：
> 「A7（M5-7/8 图谱）| 节点/边 schema 落 core，复用 `session`/`keyring_store` | core 可脱离 Tauri 单测后，图谱重建/导出测试**不再需要起 App**；不得新起 SQLite 连接路径（复用 `database.rs`）」

A2 §3.3 的 `COREBOUND_*` 断言是分水岭。下面逐模块判定（graph 全部模块在 core 内必须 `grep` 零命中 `use tauri`/`AppHandle`/`crate::bridge`/`std::process::Command`）：

### 2.1 纯逻辑（→ `src-tauri/src/core/`，脱 Tauri 可单测，满足 `COREBOUND_TAURI_IN_CORE`/`COREBOUND_CORE_TO_BIN`/`COREBOUND_EXEC_PATH`）

| 模块（core 内路径） | 纯函数 | 不依赖 Tauri 的理由 |
|---|---|---|
| `core/graph_model.rs` | `GraphNode`/`GraphEdge`/`GraphSchema`/`GraphSource`/`ExtractResult` 类型 + `id` 哈希（`hash(source_ref+kind+stable_props_hash)`） | 仅 `serde`/`sha2`（在 A2 §3.3 白名单）；零 tauri |
| `core/graph_schema.rs` | 节点类型/关系类型注册表；`props` 严格 schema 校验 | 纯判定；`serde_json` 白名单内 |
| `core/graph_extract_html.rs` | 阶段① HTML 扫描（结构化标签 + link/script 资源） | 纯解析；输入为 `&str`/`PathBuf`，输出候选实体 |
| `core/graph_extract_doc.rs` | 阶段① 文档扫描（md/txt/json frontmatter + `## outputs:`） | 纯解析；**禁**二进制（pdf/docx），防漏洞 |
| `core/graph_normalize.rs` | 阶段② 实体识别 + 关系归一 + 同 hash 精确合并 | 纯函数；fs 只读（core 允许，同 `session`/`database`） |
| `core/graph_query.rs` | `GraphQuery` 解析/校验（`max_depth≤4`、`limit≤1000` 截断）；BFS 多跳**计划**（不含实际 SQL） | 纯计算；`chrono` 白名单内 |
| `core/graph_export.rs` | `graph_export("graphml")` → 首期返回「暂不支持」stub | 纯 stub；无副作用 |

### 2.2 I/O 层（→ `src-tauri/src/core/`，但依赖 `database.rs` 访问原语，仍脱 Tauri）

| 模块（core 内路径） | 职责 | 边界合规要点 |
|---|---|---|
| `core/graph_store.rs` | `graph.db` 邻接表读写 + 索引 + 备份 + 损坏恢复 | **复用 `database.rs` 的 SQLite open/query/row-map 原语**（单一连接代码路径，零新依赖）；graph.db 是独立内部文件（无用户凭据、不经 DbPool 凭据路径、不经 Keyring）；fs/`rusqlite` 在 A2 白名单（`rusqlite` 已 M4 引入） |
| `core/graph_maintenance.rs` | `graph-vacuum`/`graph-backup`/`graph-fts-rebuild` 任务体 | **不**自起进程；经 `script_runner` 通道（复用 `TaskKind::{Script,Command}`），满足 `COREBOUND_EXEC_PATH` |

### 2.3 命令面 / 前端（→ **不进 core**，留 bin/web，受 `COREBOUND_CORE_TO_BIN` 保护）

| 落点 | 内容 | 说明 |
|---|---|---|
| `bridge.rs`（bin） | `graph_extract`/`graph_extract_cancel`/`graph_schema_*`/`graph_query`/`graph_node_*`/`graph_edge_*`/`graph_export` 共 ≤13 条命令 + 每段 `check_invocation_source` + 审计 | 命令面恒在 bin；core 不反向依赖 bin |
| `default-commands.toml` | 上述命令插 `list_artifact_images` 之前 | K1 末条恒定 |
| `src/bridge.ts` `src/types.ts` | TS 镜像 | web 层 |

> **结论**：M5-7/8 卡里平铺在 `src-tauri/src/` 的 8 个 graph 模块，W1 修订为 **7 纯逻辑 + 1 I/O 全部落 `core/`**，仅 `bridge.rs` 命令处理 + ACL + 前端留 bin/web。这与 A2 §2「core = 领域类型/策略/持久化/数据库逻辑」一致，且天然满足 `COREBOUND_TAURI_IN_CORE`/`COREBOUND_CORE_TO_BIN`/`COREBOUND_EXEC_PATH`。

---

## 3. 类型/模块映射（修订 M5-7/M5-8 卡）

| M5-7/8 卡原写法 | W1 修订落点 | 理由 |
|---|---|---|
| `domain.rs` 新增 `GraphNode`/`GraphEdge`/`GraphSchema`/`GraphSource`/`ExtractResult` | `core/graph_model.rs` | A2 切片 0a 把 `domain` 迁 `core`；新增图类型随迁，bin 经 shim 免改写 |
| `domain.rs` 新增 `GraphQuery`/`GraphResult`/`GraphSubgraph`/`GraphPath`/`GraphNodeUpsert`/`GraphEdgeUpsert` | `core/graph_query.rs` + `core/graph_model.rs` | 同上 |
| `graph.rs`/`graph_schema.rs`/`graph_extractor_*.rs`/`graph_normalize.rs` | `core/graph_*.rs` | 纯逻辑落 core（§2.1） |
| `graph_store.rs`/`graph_maintenance.rs` | `core/graph_store.rs`/`core/graph_maintenance.rs` | I/O 落 core，复用 `database.rs`（§2.2） |
| `bridge.rs`/`ACL`/`src/types.ts` | 不变（bin/web） | 命令面不进 core |

---

## 4. W0 决策（G-D1~G-D6）按 A2 边界修订

| ID | W0 决策 | W1 修订（按 A2 边界） |
|---|---|---|
| **G-D1** | graph.db 独立 SQLite 文件，不经 DbPool/Keyring，零新依赖 | **收敛**：graph 复用 `database.rs` 的 SQLite 访问原语（open/query/row-map 单一代码路径，零新 `Cargo` 依赖，`rusqlite` 已 M4 引入）；graph.db 是独立内部文件（无用户凭据、不经 DbPool 凭据路径、不经 Keyring）。即「复用 `database.rs`」= 复用访问层，非复用用户连接池。满足 A2 §8 行 420 |
| **G-D2** | A3 提升 `QueryLimiter`（M4-1.c）供 graph 复用 | **降级为直接复用**：M4-1.c 仍 STOPPED，A3 W1 为 SUPPORT DOCS ONLY；`database.rs`（M4-2）已有行/字节上限，graph 查询 `max_depth`/`limit` 上限直接在 `core/graph_query.rs` 内截断并复用 `database.rs` 既有结果上限逻辑，**不再另造 limiter** |
| **G-D3** | graph 不引新依赖 | **维持**：W1 Hard Stop 禁新 `Cargo`/`npm` 依赖；graph 仅用白名单内 `serde`/`serde_json`/`sha2`/`chrono`/`rusqlite` |
| **G-D5** | `check-graph-policy.py` 由实施 lane 补 | **维持**：实施期（A17）补 `scripts/check-graph-policy.py` 并挂 `pre-merge.sh`；W0 已给码位草案（`GRAPH_*`），本波不提前落地（仅 A2 可动 `scripts/`） |
| **G-D6** | flush 序 A7 冻结：graph-store-shutdown 在 stop-scheduler 之前 | **维持并获 M5-8 §4.5 确认**：`graph-store-shutdown` 注册在 `stop-scheduler` **之前**（scheduler 关 → 维护任务结束 → graph store flush → close db）。与 M4 冻结的 `stop-scheduler` 在 `stop-background-workers` 之后**不冲突**（graph-store-shutdown 插在更前）。实施期由 A17 在 `shutdown.rs` 注册，单测断言顺序（M5-8 N7） |
| **G-D4（W0 既有）** | graph 重建/导出测试脱 App | **获 A2 印证**：core 脱 Tauri 后，`core/graph_*` 单测**无需起 AppHandle**（A2 §8 行 420 同口径） |

---

## 5. 第二执行路径红线（与 M4 冻结 + A2 `check-core-boundary.py` 同源）

**这是本增量文件的核心交付点 —— 把「图谱不引入第二执行路径」讲死、讲可机检。**

- 抽取器（阶段①/②）是**纯进程内文件扫描/解析**，**零 `std::process::Command`**、零 shell——天然满足 `COREBOUND_EXEC_PATH`。
- 维护任务（`graph-vacuum`/`backup`/`fts-rebuild`）是**唯一**可能涉及外部执行的地方，M5-8 §4.4 已冻结：复用 `TaskKind::{Script,Command}` + `script_runner::start_run`/`start_command`，**禁**新增 `TaskKind::GraphMaintenance`、禁直起进程、禁 `tokio::spawn` stdio。
- `graph_extract` 长任务的取消复用 `script_runner.cancel`（M5-7 §4.4）。

> **AGRAPH_1（W1 升级为硬门）**：`core/graph_*.rs` 内 `grep -nE 'std::process|Command::new|"-c"|tokio::spawn'` 必须为 0；否则视为「第二执行路径」P0 缺陷，与 scheduler F6 / A2 `check-core-boundary.py` 的 `creates a second execution path` 断言同源。M5-7/8 卡 §5 FORBID 同口径。

→ 这样图谱既被 `check-core-boundary.py`（`COREBOUND_EXEC_PATH` + `COREBOUND_TAURI_IN_CORE` + `COREBOUND_CORE_TO_BIN`）守门，也被未来的 `check-graph-policy.py`（`GRAPH_EXEC_PATH`/`GRAPH_NO_SECOND_PATH`）守门，双保险。

---

## 6. W1 硬停合规确认

| W1 Hard Stop | A7 是否触碰 | 结论 |
|---|---|---|
| 仅 A2 可动产品代码 | 否（A7 零 `src-*`/`Cargo.toml`/ACL/`pre-merge.sh` 改动） | ✅ |
| 禁 `rmcp`/`tokio`/npm/MCP server/Agent runtime/**graph runtime**/plugin runtime/新 Tauri 命令/ACL 条目 | 否 | ✅ |
| `check-core-boundary.py` 不得被 A7 触发 false positive | 不适用（A7 未写 core） | ✅ |
| 不得移动 NEXT / 不得 push | 否 / 未 push | ✅ |

`git diff --name-only` 在 A7 文件外无任何产品代码改动（见 §8 自检）。

---

## 7. 未来文件清单（实施期由 A17 落，A7 不写）

来源：A1 `M5-7-graph-model-extract.md` §3 / `M5-8-graph-store-query.md` §3，经本文 §2/§3 修订落位。

- `src-tauri/src/core/graph_model.rs`（新增）— 图模型类型 + `id` 哈希（**落 core**）
- `src-tauri/src/core/graph_schema.rs`（新增）— Schema 注册表 + `props` 校验（**落 core**）
- `src-tauri/src/core/graph_extract_html.rs` / `graph_extract_doc.rs` / `graph_normalize.rs`（新增）— 两阶段抽取（**落 core，纯逻辑**）
- `src-tauri/src/core/graph_query.rs`（新增）— `GraphQuery` 解析/截断/BFS 计划（**落 core**）
- `src-tauri/src/core/graph_export.rs`（新增）— graphml stub（**落 core**）
- `src-tauri/src/core/graph_store.rs`（新增）— `graph.db` 邻接表读写 + 备份 + 恢复（**落 core，复用 `database.rs`**）
- `src-tauri/src/core/graph_maintenance.rs`（新增）— 维护任务体（**落 core，经 `script_runner`**）
- `src-tauri/src/bridge.rs`（修改）— ≤13 条 `graph_*` 命令 + `check_invocation_source` + 审计（bin，不进 core）
- `src-tauri/permissions/default-commands.toml`（修改）— 插 `graph_*` 于 `list_artifact_images` 之前
- `src-tauri/src/scheduler.rs`（修改）— 注册 `graph-maintenance` 周期任务（`enabled=false`）
- `src-tauri/src/shutdown.rs`（修改）— 注册 `graph-store-shutdown`（先于 `stop-scheduler`）
- `src/types.ts` `src/bridge.ts`（新增镜像）
- `scripts/check-graph-policy.py`（W0 码位草案，实施期补并挂 `pre-merge.sh`）

---

## 8. 验证锚点（实施期由 A17 跑，A7 仅记录口径）

取自 M5-7 §7 / M5-8 §7 PASS_CRITERIA + 本文边界约束：

| # | 判据 | 验证命令（节选） |
|---|---|---|
| 边界 | `grep -rnE 'use tauri|tauri::|AppHandle|AppState' src-tauri/src/core/graph_*.rs` 为空 | `grep` |
| 边界 | `grep -rnE 'crate::bridge' src-tauri/src/core/graph_*.rs` 为空 | `grep` |
| AGRAPH_1 | `core/graph_*.rs` 内 `std::process`/`Command::new`/`"-c"`/`tokio::spawn` 为 0 | `grep` |
| 复用 | `graph_store.rs` 调用 `database.rs` SQLite 访问原语，无第二 sqlite 连接原语 | `grep` 引用 |
| K7 | `props` 不存主数据正文（>1KB 拒） | 单测 N1（M5-7） |
| 上限 | `max_depth≤4`/`limit≤1000` 截断 | 单测 N1/N2（M5-8） |
| Manual | `source=Manual` 节点不被 upsert 覆盖 | 单测 N3（M5-8） |
| 恢复 | `graph.db` 损坏自动从 `.bak` 恢复 | 单测 N6（M5-8） |
| 顺序 | `graph-store-shutdown` 注册在 `stop-scheduler` 之前 | 单测 N7（M5-8） |
| ACL | 末条仍为 `list_artifact_images` | `grep -n list_artifact_images` |
| 门禁 | `bash scripts/pre-merge.sh` ALL_PASS | 含 `check-core-boundary.py` + `check-graph-policy.py` |

---

## 9. 输出模板回填

```text
LANE=A7
STATUS=PASS_WITH_DOCS
WAVE=M5-W1 (SUPPORT DOCS ONLY)
BASE=a1a2061
HEAD=docs only (logs/assist/A7-M5-graph-core-W1-delta-20260906-0827.md + A7-M5-graph-core-20260906-0755.md)
FILES=logs/assist/A7-M5-graph-core-W1-delta-20260906-0827.md
VERIFY=无产品代码改动；git diff --name-only 在 A7 文件外为空；事实锚点：A2 §8 行 420（图谱落 core、复用 database.rs）、A2 §3.3 COREBOUND_*、M5-7/M5-8 卡（责任 Lane A17）；git diff --check 干净；
       本文件为 M5-W0 契约的 W1 增量，逐函数落位 core/bin/web，修正 G-D1/G-D2，新增 AGRAPH_1 第二执行路径硬门
CHECKPOINT=logs/assist/A7-M5-graph-core-W1-delta-20260906-0827.md
MERGE_NOTES=W1 SUPPORT DOCS ONLY；无 patch；M5-7/8 实现责任候选 A17（A0 签发）；
            A7 红线=图谱纯函数落 core（复用 database.rs、零第二执行路径、AGRAPH_1 绑定 A2 check-core-boundary.py）；
            依赖 A2 M5-1 core boundary（切片0a）+ A2 M5-1.b（B类 seam/PathResolver）；
            与 A2 §8 行 420、M5-7/M5-8 卡一致；与他 lane W1 产物无文件交集
NEXT=A2 落 M5-1 core boundary（含 check-core-boundary.py）→ A2 M5-1.b（B类 seam）→ A0 签 M5-7.a/M5-8.a 交 A17 实施
```
