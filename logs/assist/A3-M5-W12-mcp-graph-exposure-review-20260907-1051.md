# Lane A3 — M5-W12 Graph Live-Query Readonly Dispatch（MCP 评审 note）

- 时间：2026-09-07 10:51（本地）
- 基线：`master` @ `269269a`（W11 已由 A0 集成，工作树干净）
- 路线：**START MCP REVIEW ONLY**（不扩展 MCP runtime；仅 `logs/assist/` 评审 + `scripts/check-mcp-policy.py` 必要夹具）
- 交付物：本 note + 补丁 `logs/checkpoints/Lane-A3-M5-W12-mcp-graph-exposure-20260907-1051.patch`（仅 `scripts/check-mcp-policy.py` 一个文件，**未 push**）

## 1. 评审问题

> W12 的图谱只读命令（`graph_query` / `graph_node_get` / `graph_stats`）会不会被错误地通过 MCP stdio 暴露成可执行工具？

结论（当前事实基线）：**不会。当前零暴露。** 我把 MCP 完整暴露面做了逐跳走查，确认图谱命令与 MCP stdio 之间存在多重天然隔离。

## 2. 暴露面逐跳走查（证据）

| 暴露层 | 当前事实 | 图谱命令能否进入 | 来源 |
|---|---|---|---|
| `MCP_CAPABILITY_V1`（能力白名单，domain.rs:1540） | 静态冻结 7 项（`file_read/file_list/tab_query/history_query/bookmarks_query/downloads_query/console_query`），**无 `graph_*`** | 否（白名单硬编码） | `grep -n "MCP_CAPABILITY_V1" -A 9 domain.rs` |
| `MCP_COMMAND_REGISTRY`（mcp.rs:23） | 静态 7 项，与白名单双向覆盖 | 否（静态常量） | `mcp.rs` |
| `tools/list`（mcp_server.rs:211） | 由 `list_registry_entries()` → 静态注册表；**不**动态枚举 Tauri 命令 / ACL | 否（只读静态注册表） | `mcp_server.rs::tools_list` |
| `tools/call`（mcp_server.rs:249） | 仅 `BOUND_READONLY_TOOLS = {mcp_policy_get, mcp_registry_list}` 真绑定；其余一律 `isError=true` fail-closed | 即便白名单被改，也**无法执行**（allowlist 不含 graph） | `mcp_server.rs::tools_call` |
| `mcp_capability_preview`（bridge.rs:6583） | 走 `is_known_mcp_capability` → `MCP_CAPABILITY_V1.contains`；未知能力 `Deny`（非执行路径） | 否（判红而非执行） | `bridge.rs` / `domain.rs:1575` |
| `mcp_server.rs` 自身 | feature-gated（`#![cfg(feature="mcp")]`），默认构建不编译；其中无任何 `crate::graph` / `graph_query` 引用 | 否 | `grep` 当前命中为空 |

补充确认：`mcp_server_start` 在链路中**不存在**（仅历史测试夹具字符串），不会产生额外监听面。

## 3. 残留风险（A7 落地 W12 后需守的点）

即便 A7 按卡片在 `bridge.rs` 新增 `graph_query/graph_node_get/graph_stats` Tauri 命令，只要遵守卡片「零 MCP 耦合」裁定，MCP 面不受影响。**唯一需守的人为失误路径**是：有人为「图方便」把图谱命令塞进 MCP 白名单/注册表，导致：

- `tools/list` 会**元数据暴露**图谱能力存在（信息泄漏，且 dry-run 下误导调用方以为可执行）；
- 若进一步误把图谱命令加入 `mcp_server.rs` 的 `BOUND_READONLY_TOOLS` → MCP full runtime 被打开，直接违反 W12 Hard Stop 行 209（MCP full runtime LOCKED）。

该风险属「契约未解锁前的误接」，需要静态守门固化。

## 4. 固化措施（concrete fixture）

在 `scripts/check-mcp-policy.py` 新增 **ACTIVE 码位 `MCP_GRAPH_NOT_EXPOSED`**（ACTIVE 11 → 12，PENDING 恒 0）：

- 守门文件：`mcp.rs`（注册表）、`domain.rs`（`MCP_CAPABILITY_V1` 白名单块，用正则截取该块避免误伤既有 `GRAPH_*` 容量常量与 `GraphNode` 领域类型）、`mcp_server.rs` / `bin/mcp_server.rs`（stdio 骨架）。
- 命中令牌：`graph_query` / `graph_node_get` / `graph_stats` / `GraphState` / `crate::graph` 任一出现即判「图谱能力被接进 MCP 面」。
- 配套：基线好样本 `_baseline_repo()` 不含上述 token（已在改前确认 `grep` 当前零命中，故好样本不误报）；新增 1 条坏样本（在 `mcp_server.rs` 直接 `crate::graph::graph_query(...)`），变异防呆生效。
- 语义：这是个**预防性收口**——当前 PASS（零误报），未来任何人把图谱命令接进 MCP 面都会被本码位拦下。若后续确有合法解锁需求（A0 显式裁决 M5-2.b 或 graph-via-MCP wave），应先改/删本码位并在提交说明里留裁决记录。

## 5. 验证结果（本机实跑）

```
python3 scripts/check-mcp-policy.py --self-test           -> PASS（ACTIVE=12，PENDING=0）
python3 scripts/check-mcp-policy.py                      -> PASS（无违规）
python3 scripts/check-mcp-policy.py --expect-current-gaps -> PASS（W10 相位）
cargo test --features mcp mcp_server                     -> 21 passed（无 Rust 改动，基线维持）
git diff --check                                         -> 干净
```

`cargo test`（默认，无 mcp）与 `cargo fmt --check` 因本波未改 Rust 而零影响；MCP 策略 ACTIVE 由 11 升至 12。**未改任何 MCP 产品代码**（mcp.rs / mcp_server.rs 仅作为被扫描对象，源码未变动）。

## 6. 给其他 lane 的接口结论

- **A7（W12 图谱命令）**：请保持卡片「零 MCP 耦合」裁定——命令在 `bridge.rs`、View 在 `domain.rs`、`GraphState` 在 `graph.rs`，**不要**把命令名加进 `mcp.rs` 的 `MCP_COMMAND_REGISTRY` / `domain.rs` 的 `MCP_CAPABILITY_V1`，也**不要**在 `mcp_server.rs` 的 `BOUND_READONLY_TOOLS` 里引用。否则 `MCP_GRAPH_NOT_EXPOSED` 会判红。
- **A4（隐私评审）**：与 A3 面正交——本评审只管「图谱命令不进 MCP 执行面」，图谱输出删 `props` / 稳定错误码 / 审计无 secret 仍由 A4 在 `graph.rs`/`domain.rs` 侧复核。
- **A10（安全评审）**：新增码位为收紧型（只增不放），未弱化任何既有 MCP 守门；如需在审查里引用，结论为「MCP-Graph 隔离已静态固化」。
- **A11（验证矩阵）**：MCP 策略 `ACTIVE=12`（含 `MCP_GRAPH_NOT_EXPOSED`），需纳入 W12 验证矩阵。
- **A0（集成）**：补丁仅 `scripts/check-mcp-policy.py`，与其他 lane 无文件重叠，可直接 apply；如认为预防性码位过早，可整段丢弃，不影响任何运行时行为。

## 7. 未做 / 不建议在本波做

- 不实现任何图谱命令、不扩展 MCP server runtime（W12 锁死 MCP full runtime）。
- 未引入新依赖、未改 Cargo.toml、未改 `mcp.rs` / `mcp_server.rs` 源码（仅被策略扫描）。
- 未处理 `check-security-policy.py` 的 3 条既有 known gap（与图谱/MCP 无关，交 A10/A11）。
