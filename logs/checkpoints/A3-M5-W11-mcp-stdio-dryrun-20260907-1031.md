# Lane A3 — M5-W11 MCP Stdio Dry-Run Hardening（交付检查点）

- 时间：2026-09-07 10:31（本地）
- 基线：`master` @ `5226aad`（W10 已 push，A3 stdio-prep 骨架已集成）
- 路线：START PRODUCT CODE NARROW（仅 `src-tauri/src/mcp_server.rs` + `scripts/check-mcp-policy.py`）
- 交付物：补丁 `logs/checkpoints/Lane-A3-M5-W11-mcp-stdio-dryrun-20260907-1031.patch`（仅本 lane 两文件，**未 push**）

## 1. 本波做了什么（W11 硬化项逐条）

| W11 要求 | 落地方式 | 位置 |
|---|---|---|
| 确定性 JSON-RPC 错误 | `-32700` Parse error / `-32600` Invalid Request / `-32600 Request too large` / `-32601` Method not found / `-32602` Invalid params / `-32603` 响应超界·内部错误；非对象请求（数组/标量）一律 `-32600` | `mcp_server.rs` `parse_error/invalid_request/request_too_large/handle_request` |
| 有界输入 | `MAX_INPUT_BYTES = 1 MiB`；`read_line_bounded` 超界后继续 drain 到换行/EOF 并标记 `truncated`，**内存有上限且不脱位** | `read_line_bounded` |
| 有界响应 | `MAX_RESPONSE_BYTES = 4 MiB`；超界/序列化失败回确定性错误，**绝不截断业务数据、绝不静默丢响应** | `serialize_bounded` |
| 稳定 `tools/list` 模式 | 每个工具恒定含 `name` / `description` / `inputSchema`；`inputSchema` 为合法 JSON Schema（`type=object` + `properties` + `additionalProperties=false`），不随运行时变化 | `stable_input_schema` / `tools_list` |
| 未绑定/未知能力显式 fail-closed | 仅 `mcp_policy_get` / `mcp_registry_list` 两个只读 introspection 工具真绑定（复用 `crate::mcp` 视图，零副作用）；其余 registry 能力与完全未知工具均 `isError=true`，消息区分 `not yet bound` vs `unknown tool` | `BOUND_READONLY_TOOLS` / `tools_call` / `fail_closed_text` |
| 零参数回显（W11 Hard Stop 行 208） | fail-closed 文本只回显能力名（协议标识符），`params/arguments` 从不进响应；无 `println/eprintln` 输出 params | 全文件 + 新码位 `MCP_STDIO_NO_ARG_ECHO` |
| 流级冒烟 | stdio 循环抽出可注入读写的 `serve<R: BufRead, W: Write>`，`run_stdio` 仅做 stdin/stdout 接线；据此新增 3 条端到端流测试 | `serve` / `serve_bytes` + 3 tests |

新增/变更测试（21 条，W10 为 7 条，+14）：

- 单测：`initialize_returns_server_info`、`tools_list_uses_registry_source_of_truth`、`tools_list_schema_is_stable`、`policy_get_is_readonly_no_error`、`registry_list_is_readonly_no_error`、`known_unbound_capability_is_fail_closed`、`unknown_tool_is_fail_closed`、`arguments_are_never_echoed_in_fail_closed`、`missing_tool_name_is_invalid_params`、`non_object_request_is_invalid_request`、`unknown_method_returns_method_not_found`、`notifications_have_no_response`、`handle_line_*`、`read_line_bounded_*`、`response_size_guard_caps_oversized`
- 流级冒烟（本波新增）：`stdio_smoke_invalid_json_unknown_tool_and_large_line`（坏 JSON → 未绑定能力+机密参数 → 通知 → 超大行 → ping，断言 4 条输出、通知无响应、超大行 drain 后循环不脱位、全流零回显）、`stdio_smoke_oversized_line_never_echoes_payload`、`stdio_smoke_empty_input_and_eof_only`

## 2. 策略守门（scripts/check-mcp-policy.py）

新增 2 个 ACTIVE 码位（ACTIVE 9 → 11，PENDING 恒为 0）：

1. `MCP_STDIO_NO_ARG_ECHO`：stdio 传输层不得把 `params/arguments` 原始内容序列化/插值进响应文本，也不得 `println/eprintln` 输出 params（W11 Hard Stop 行 208 的静态化）。
2. `MCP_STDIO_BOUNDED`（本波新增）：stdio 必须声明 `MAX_INPUT_BYTES` / `MAX_RESPONSE_BYTES` 双上限、必须存在有界行读取 `read_line_bounded`、禁止退回无界 `stdin().lock().lines()`。防止后续重构把有界读/有界响应改回无限分配。

配套改动：基线好样本 `_baseline_repo()` 的 `mcp_server.rs` 夹具升级为有界形态（否则新码位对好样本误报），并为两个新码位各补 1 条坏样本（变异防呆生效）。

## 3. 验证结果（本机实跑，全部复现）

```
cargo test --features mcp mcp_server    -> 21 passed; 0 failed（W10 为 7）
cargo test --features mcp               -> 431 passed（lib 2 + bin 429）
cargo test（默认，无 mcp feature）        -> 410 passed（lib 2 + bin 408）
cargo fmt --check                        -> 干净
cargo check / test 告警                  -> 2 条，均为既有 grid_process.rs（未新增）
python3 scripts/check-mcp-policy.py --self-test         -> PASS（ACTIVE=11，PENDING=0）
python3 scripts/check-mcp-policy.py                     -> PASS（无违规）
python3 scripts/check-mcp-policy.py --expect-current-gaps -> PASS（W10 相位）
python3 scripts/check-core-boundary.py                  -> all invariants hold（ACTIVE=7）
git diff --check                                        -> 干净
```

其余 `scripts/check-*-policy.py` 默认模式批量复跑：除 `check-security-policy.py` 报 **3 条既有 known gap**（`READ_ONLY_BROWSE_WITHOUT_PATH_POLICY` / `REMOTE_WILDCARD_IPC` / `EVAL_WITHOUT_SOURCE_CHECK`，与 MCP stdio 无关，属 W11 之前既有挂账，交 A10/A11 跟踪）外，全部 PASS。

## 4. Hard Stop 符合性证据

- 无网络/监听/进程/执行副作用：`mcp_server.rs` 中 `std::net|TcpListener|TcpStream|UdpSocket|tokio|rmcp|Command::new|std::process|std::fs|thread::spawn` **零命中**（仅文档注释里出现否定式描述字符串）。
- 依赖零新增：`src-tauri/Cargo.toml` 未改动，`mcp = []` 空 feature 保持；骨架纯 `std::io` + `serde_json`，默认构建仍不编译该文件（`#![cfg(feature = "mcp")]`）。
- 无真实执行：`tools/call` 仅两个只读 introspection 工具返回 `crate::mcp` 视图快照；其余一律 fail-closed 文本，无 fs/db/script/plugin/agent/skill/model 路径。
- 无原始参数回显：单测 + 流级冒烟 + 新策略码位三重兜底。
- 告警不增加：本波一度引入 `unused import: Read`（泛型 `R: BufRead` 的 supertrait 方法无需导入），已移除，告警回落到既有 2 条。

## 5. 交接给 A0 / 其他 lane

- **A0 集成**：补丁仅 `src-tauri/src/mcp_server.rs`、`scripts/check-mcp-policy.py` 两个文件，与其他 lane 无文件重叠，可直接 apply。
- **A2（边界评审）**：`serve` 是唯一新增抽象，`run_stdio` 只做 stdin/stdout 接线；无 `tauri::`、无 `bridge::*` 调用、无 workspace 写。
- **A4（隐私评审）**：需重点复核的两个面 —— `tools/call` fail-closed 文本（仅含能力名）与 `mcp_policy_get` / `mcp_registry_list` 的只读视图输出（`crate::mcp` 快照，不含 URL/凭据；若快照未来含 URL 需走既有 URL 脱敏）。
- **A10（安全评审）**：新增码位 `MCP_STDIO_BOUNDED` / `MCP_STDIO_NO_ARG_ECHO` 均为收紧型（只增不放），无既有码位被弱化或降级。
- **A11（验证矩阵）**：`cargo test --features mcp` 已从 417 → 431（+14，全部为 mcp_server）；MCP 策略 ACTIVE 9 → 11。

## 6. 残项 / 不建议在本波做

1. **批处理（batch）未实现**：JSON-RPC batch 数组请求当前按 `-32600 Invalid Request` 处理（与「非对象请求」同路径，确定性）。MCP 2024-11-05 允许 batch；是否支持属契约决策，交 A0 在 M5-2.b 前裁定，本波不做（避免引入批量执行的越权面）。
2. **`inputSchema` 仍为空 object**：两个绑定工具本就零参数；registry 其余能力未绑定，故 `additionalProperties=false` 恒成立。待 M5-2.b 绑定真实 core API 时按能力逐个补 schema，届时 `tools_list_schema_is_stable` 仍可作回归基线。
3. **真实 rmcp server / 工具执行**：仍锁定，等 A0 显式解锁 M5-2.b。
4. `check-security-policy.py` 的 3 条 known gap 非本 lane 引入，未在本波处理。
