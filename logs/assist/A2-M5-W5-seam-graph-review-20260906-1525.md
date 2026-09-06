# A2 · M5-W5 Graph Store vs Core Seam 边界审查笔记（仅文档，无产品代码）

> 生成：2026-09-06 15:25 CST · Lane A2（M5-W5 · SUPPORT/REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W5 Parallel Dispatch（L135-161）→ A2 = **SUPPORT/REVIEW ONLY**；任务="Review whether A7 graph store should use existing core seam or stay bin-side; no product code"；交付=`logs/assist/A2-M5-W5-*.md` 边界笔记。
> 配套：`M5-1.b-seam-trait-injection-and-b-extract.md`（A1 权威卡）、`M5-7-graph-model-extract.md` / `M5-8-graph-store-query.md`（A1 权威卡）、`logs/assist/A7-M5-W4-graph-core-delta-20260906-1455.md`（A7 W4 delta）、`scripts/check-core-boundary.py`。

```text
LANE=A2
STATUS=REVIEW_DONE（docs only，无产品代码；未 push）
BASE=0e76a89（本地已 ff 到 W5 dispatch；seam 已由 A0 集成于 f8f1f49）
HEAD=logs/assist/A2-M5-W5-seam-graph-review-20260906-1525.md
FILES=logs/assist/A2-M5-W5-seam-graph-review-20260906-1525.md（仅本文档）
VERIFY=见 §Verify
CHECKPOINT=本文件
MERGE_NOTES=见 §5（纠正 A7 W4 delta §3.6 的 "graph_store 是 core 模块" 矛盾 + 建议加固 gate 的 BIN_ONLY_MODULES）
NEXT=见 §6
```

## 1. W5 范围确认
- board §M5-W5：`A2 = SUPPORT/REVIEW ONLY`；Allowed Scope 仅 `logs/assist/A2-M5-W5-*.md`；Must Deliver = Boundary note；**no product code**。
- 本笔记**零行** `src-tauri/**`、`scripts/**`、ACL/Capability/前端改动；工作树在 `git pull --ff-only` 后干净（HEAD `0e76a89`），无 graph 代码进树。

## 2. 核心问题
> A7 的 graph store 应**复用现有 core seam**（`mvp_core::seam` 的 `ProgressSink`/`PathResolver`/`RootsProvider`）还是**留在 bin-side**？

## 3. 实证（架构锚点）

| # | 断言 | 实测 | 结论 |
|---|---|---|---|
| E1 | `core/`（`mvp_core` lib）仅由 `lib.rs` 声明 `pub mod core;`，其余模块全在 bin 侧 | `lib.rs` 仅 31 行，仅声明 `core`；`main.rs` 声明 **23 个** `mod`（agent/agent_memory/bridge/database/domain/grid_ipc/grid_process/images/mcp/scheduler/script_runner/scripts/security_policy/session/shutdown/skills/snippets/sync/tasks/terminal/tools/workspace） | `database`/`domain`/… 全是 **bin-only 模块**，core 无法 `use crate::database`（不在 lib crate，编译失败 + 违反 R-B2/R-B3） |
| E2 | `database.rs` 是否 Tauri 自由、如何解析路径 | grep `use tauri/AppHandle/app.path/data_dir/PathResolver` 全 0 命中；路径经 `validate_sqlite_path(db, roots: &[PathBuf])`（L333）与 `connect(..., roots: &[PathBuf])`（L426）**注入式**解析（非 `AppHandle`） | `database.rs` 本身是干净 bin 模块；graph_store 复用其单连接属 bin→bin，合法 |
| E3 | seam 是否双端可达 | `main.rs:32 pub use mvp_core::core::seam;` + `lib.rs:30 pub use crate::core::*;` | core 与 bin 模块均可用 `crate::seam::{ProgressSink,PathResolver,RootsProvider}`，注入抽象成立 |
| E4 | graph 代码当前是否存在 | `ls src-tauri/src/graph*.rs` → no match；`grep GraphNode/graph_store` → 空 | ✅ W5 刚开，A7 产品码未进树 |
| E5 | 边界 gate 现状 | 默认 `all invariants hold（ACTIVE=7，core 文件=3）`；`--self-test` `CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9）` | 守门有效，core 仍 3 文件（mod/keyring_store/seam） |

## 4. 结论（Verdict）：分层（hybrid），非二选一

**现有 core seam 是正确的依赖注入点，但 `graph_store` 本身必须留在 bin-side。** 具体切分：

1. **纯逻辑 → 进 `core/`，消费 seam（推荐）**
   - `GraphNode` / `GraphEdge` / `GraphQuery` / `GraphSchema` / `ExtractResult` 类型 + 序列化/校验 + `normalize`/`merge` 纯函数 + schema 注册表：零 IO、零 DB，天然 Tauri 自由，**应落 `core/graph_model.rs`（或 `core/graph/`）**。
   - `graph_extract_html.rs` 扫描 `allowed_roots` → 注入 `&dyn RootsProvider`（复用 `core/seam.rs::RootsProvider`，已集成）。✅ 与 A7 W4 delta §3.6 一致。
   - `graph_query.rs` 多跳/深度/路径遍历算法：纯函数，**进 `core/`**。
   - 这些模块不引用 `crate::database`，故可安全置于 core，且被 `check-core-boundary.py` 守门（零 `tauri`/`AppHandle`/`crate::bridge`/第二执行路径）。

2. **`graph_store.rs`（SQLite 邻接存储）必须留 bin-side**
   - 它复用 `database.rs` 的 SQLite **单连接**（A7 W4 delta §3.3 + M5-8 §4.1 "图谱=唯一 SQLite 通道"），而 `database` 是 **bin-only 模块**（E1）。core 无法 `use crate::database` → **graph_store 不能搬进 `core/`**。
   - 它是 bin→bin 依赖，合法；可仍注入 `&dyn PathResolver`（解析 `graph.db` 路径）与 `&dyn ProgressSink`（维护任务进度）以保持可测、与 seam 设计同构，但**位置在 bin**。
   - `graph_maintenance.rs`、`graph.rs`（编排 + 取消/进度）同理 bin-side（`graph.rs` 用 `&dyn ProgressSink` 与 A7 §3.6 一致）。

3. **纠正 A7 W4 delta §3.6 的矛盾表述**
   - 该 delta 写 "graph_store.rs 取 &dyn PathResolver … 以上均在 `check-core-boundary.py` COREBOUND_* 守门范围内（core 模块零 tauri/AppHandle/crate::bridge）" —— **"graph_store 是 core 模块" 这一前提错误**：graph_store 依赖 `database`（bin-only），若真放进 `core/` 会编译失败并违反 R-B2/R-B3。
   - 建议 A7/A17 修正该 delta：仅 **`graph_extract_html`、纯 model/normalize/query 辅助** 是 core 模块；**`graph_store` / `graph_maintenance` / `graph.rs` 仍 bin-side**。这不影响 W5 实施（seam 已集成，A7 从 bin 与 core 两侧均可 `use crate::seam`），仅修正放置断言。

## 5. 下一步建议（交 A0 排期，W5 A2 不执行）

### 5.1 加固 `check-core-boundary.py` 的 `BIN_ONLY_MODULES`（强烈建议）
- 现状 `BIN_ONLY_MODULES = ("bridge","main","terminal","grid_process","tools","shutdown")` 仅 6 个，**未覆盖** `database`/`domain`/`workspace`/`script_runner`/`scheduler`/`tasks`/`sync`/`session`/`snippets`/`scripts`/`images`/`agent`/`agent_memory`/`skills`/`mcp`/`security_policy`/`grid_ipc`/`crashlog` 等全部 bin 模块。
- 风险：若有人把 `graph_store.rs` 放进 `core/` 并写 `use crate::database`，gate **不会报**（false negative）；虽然后续会因 `database` 不在 lib crate 而编译失败，但 gate 在 phase-1 未能机器化守住 R-B2/R-B3 的"core 不反向依赖任意 bin 模块"全貌。
- 建议：将 `main.rs` 声明的全部 23 个 bin 模块加入 `BIN_ONLY_MODULES`（或改为"core 内禁止任何 `crate::<非 core 模块>::` 引用"的通用判定），使 R-B2/R-B3 在 phase-1 被完整守门。属产品代码改动，由 A0/A2 在后续产品代码波落地（A2 W5 review-only，不在此实现）。

### 5.2 M5-1.b 切片 2 仍是"下一个大核心抽取"
- 将 `script_runner`/`workspace`/`sync`/`tasks`/`scheduler`（及潜在的 `database`）搬入 core 并注入 seam，闭合 8 处反向边，仍是 M5-1.b 设计上的下一步。届时 `graph_store` 若随 `database` 一起入 core，方可成为 core 模块。该切片体量大、属产品代码，不在 W5 A2 范围。

## 6. 边界守门结论
- `core/seam.rs` 零 Tauri；gate 默认 + self-test 全 PASS（ACTIVE=7，坏样本=9）；本期未新增任何 core 文件、未引入 core→bin 新反向边。
- 本笔记仅审查，给出"分层放置 + 纠正 delta 表述 + 建议加固 gate"三项结论；**不阻塞 A7 W5 实施**（seam 已就绪，A7 可在 bin/core 两侧按需注入）。

## 7. Verify（本笔记证据）
```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) 边界 gate 三模式
python3 scripts/check-core-boundary.py            # all invariants hold（ACTIVE=7，core 文件=3）
python3 scripts/check-core-boundary.py --self-test  # CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9）
# 2) 模块归属：lib 仅 core；bin 有 23 模块
grep -nE "pub mod" src-tauri/src/lib.rs            # 仅 core
grep -cE "^\s*mod " src-tauri/src/main.rs          # 23
# 3) database.rs 零 Tauri、注入式路径解析
grep -nE "use tauri|AppHandle|app\.path|data_dir|PathResolver" src-tauri/src/database.rs  # 0 命中
grep -nE "validate_sqlite_path|roots: &\[PathBuf\]" src-tauri/src/database.rs            # L333/L426
# 4) seam 双端可达
grep -n "pub use mvp_core::core::seam" src-tauri/src/main.rs   # 32
grep -n "pub use crate::core::\*" src-tauri/src/lib.rs        # 30
# 5) 无 graph 代码进树
ls src-tauri/src/graph*.rs 2>&1   # no matches
git status --short   # 空（仅文档态交付）
```

## 8. NEXT
- 交 A0：① 采纳本笔记对 A7 W4 delta §3.6 的纠正（graph_store 为 bin-side，非 core 模块）；② 排期加固 `check-core-boundary.py` 的 `BIN_ONLY_MODULES`（覆盖全部 23 个 bin 模块，完整守住 R-B2/R-B3）；③ M5-1.b 切片 2（database 等搬入 core）作为后续大抽取。
- A7（W5 产品码）：可立即开工——纯 model/extractor/query 进 `core/`（消费 seam），`graph_store`/`graph_maintenance`/`graph.rs` 留 bin-side（复用 `database.rs` 单连接 + 注入 `PathResolver`/`ProgressSink`）。无需等本 Lane 任何改动。
