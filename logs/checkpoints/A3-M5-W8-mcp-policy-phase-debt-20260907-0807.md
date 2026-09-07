# Checkpoint · Lane A3 · M5-W8 MCP 策略相位债收口

- **LANE**: A3
- **WAVE**: M5-W8 Full-Lane Follow-up Dispatch（BASE `a26fbaf`）
- **ROLE**: START POLICY FIX ONLY
- **TASK**: 收口 W7 MCP 策略相位债 —— `--expect-pending` 因 MCP 产物已存在而失败；退役 W1 pending 语义，转 W7/W8 active 守门 + 当前相位断言。
- **STATUS**: PASS（三模式 + rust 测试全绿）
- **PATCH**: `logs/checkpoints/A3-M5-W8-mcp-policy-phase-debt-20260907-0807.patch`
- **NOTE**: `logs/assist/A3-M5-W8-mcp-policy-phase-debt-20260907-0807.md`

## 改动矩阵（策略脚本 W8 vs W7）

| 项 | W7 状态 | W8 状态 |
|---|---|---|
| `--expect-pending` 模式 | 存在（现 FAIL，因 MCP 产物已落地） | **退役** → 替换为 `--expect-current-gaps`（当前相位断言，PASS） |
| 7 个 server 架构 PENDING 码位 | PENDING（守「独立 rmcp server」架构） | **退役** → 收口为单一 ACTIVE `MCP_NO_RMCP_SERVER` |
| `MCP_TREE_TAURI` | PENDING no-op | **退役** |
| `MCP_PARITY` | PENDING | **升 ACTIVE**（桥已存在，奇偶始终守门） |
| `MCP_NO_RMCP_SERVER` | 不存在 | **新增 ACTIVE**（永久禁 rmcp/tokio/mcp_server bin/mcp_tools 目录/axum/hyper::Server/TcpListener） |
| `PENDING_CODES` | 9 条 | **空**（相位债关闭） |
| `pre-merge.sh` MCP 段 | `--self-test` + 默认 | 追加 `--expect-current-gaps` |

## 关键设计决策

1. **W7 Hard Stop 提升到策略层**：W7 以「只读 Tauri 命令桥」落地 MCP，否决了 W1 的「独立 rmcp server」架构。W8 把该否决**永久化**为 `MCP_NO_RMCP_SERVER` ACTIVE 守门——任何未来误引入 `rmcp`/`tokio`/`mcp_server`/`mcp_tools` 监听形态都会立即被 `cargo` 之外的策略夹具拦下（编译层守不住的红线只能靠夹具，与 `check-core-boundary.py` 同理）。
2. **regex 避开裸 `TcpListener` 字样**：`mcp.rs:7` 注释含「无 `TcpListener`」，故只匹配 `tokio::net::TcpListener`/`std::net::TcpListener`/`axum`/`hyper::Server` 等真实监听形态，避免注释误命中。
3. **相位债显式关闭**：`PENDING_CODES=[]` 且 self-test 打印 `PENDING=0`，使「W1 pending 假设已退役」可被 `--self-test` 自身证明。
4. **零产品代码改动**：本波仅 `scripts/check-mcp-policy.py` + `scripts/pre-merge.sh`，Rust/TS/ACL/domain 均无 diff → 「no product runtime expansion」天然满足；MCP 桥行为仍由 W7 `6c1f30e` 决定。

## 验证结果

| 检查 | 结果 |
|---|---|
| `check-mcp-policy.py --self-test` | PASS（ACTIVE=8，PENDING=0） |
| `check-mcp-policy.py` 默认 | 无违规 PASS |
| `check-mcp-policy.py --expect-current-gaps` | MCP_CURRENT_GAPS_RESULT=PASS（W8 相位） |
| `cargo test mcp_policy_tests`（src-tauri） | 9 passed |
| `bash -n scripts/pre-merge.sh` | 语法 OK |
| `read_lints scripts/check-mcp-policy.py` | 0 diagnostics |

## 范围隔离

- 本 lane 仅暂存并提交 2 文件（见 PATCH）。工作树其余 A1/A2/A4/A5/A6/A7/A8/A9/A11 WIP 均保持未跟踪/未暂存，不纳入本 commit。
- 未 push（仅 A0 可 push master）。

## 挂账（非本波）

- **R1/G2**：能力单源碎片化（MCP/Skill/Agent/未来 Plugin/Graph）仍待 A3 M5-2.b 收口 `core/capability.rs` + 泛化 `CAPABILITY_SINGLE_SOURCE` 门。
- **G3**：plugin/agent 未来经 MCP 暴露时复用 `evaluate_mcp_command`（`mcp_capability_preview` 已演示同函数路径）。
- `bridge.rs:6276` 既有测试 fmt 差异（A4/A5 W7 产物），集成时由 A0 统一 fmt。
