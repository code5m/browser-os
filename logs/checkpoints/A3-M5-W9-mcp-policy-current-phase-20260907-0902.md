# Checkpoint · Lane A3 · M5-W9 Runtime-Free Polish — MCP 策略相位保持

- **LANE**: A3
- **WAVE**: M5-W9 Runtime-Free Polish Dispatch（BASE `97118d6`，A0 已集成 W8）
- **ROLE**: START POLICY/REVIEW ONLY
- **TASK**: Keep MCP policy current-phase green；prepare later M5-2.b card for actual rmcp/server work. Do NOT implement rmcp/server/listener/network.
- **STATUS**: PASS（三模式全绿，零回归，零产品代码改动）
- **NOTE**: `logs/assist/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md`

## 相位保持验证（W9 复跑，W8 收口后）

| 模式 | 结果 |
|---|---|
| `check-mcp-policy.py --self-test` | PASS（ACTIVE=8，PENDING=0） |
| `check-mcp-policy.py` 默认 | 无违规 PASS |
| `check-mcp-policy.py --expect-current-gaps` | MCP_CURRENT_GAPS_RESULT=PASS（W8 相位仍成立） |
| `cargo test mcp_policy_tests` | 9 passed（产品代码零改动） |

→ **无回归** → 按 scope（`scripts/check-mcp-policy.py` only if a regression is found）**未修改策略脚本**。`PENDING_CODES` 仍为空。

## 关键守门（当前相位，仍全绿）

- `MCP_NO_RMCP_SERVER`（ACTIVE，永久 forbidden）：禁 `rmcp`/`tokio` 依赖、`mcp_server` bin、`mcp_tools/` 目录、`axum`/`hyper::Server`/`TcpListener`。
- `MCP_PARITY`（ACTIVE）：命令↔ACL↔bridge.ts 奇偶。
- `MCP_BRIDGE_READONLY`（ACTIVE）：MCP 命令无写副作用。
- `MCP_FS_TOOL_PATH_POLICY` / `MCP_CAPABILITY_DRIFT` / `MCP_NPM_*` / `MCP_NODE_RUNTIME_PRESENT`（ACTIVE）：路径根 / 能力漂移 / npm SDK / npm in Cargo / node runtime。

## M5-2.b 前瞻卡（摘要，详见 note §2）

当前被 **W7 Hard Stop 永久否决**：禁止 rmcp/server/listener。仅当产品确需对外暴露 MCP server（如对接外部 MCP 客户端）时，方可由 A0 显式解锁，并满足：rmcp/tokio 须 `optional`+`required-features` 隔离、不进 default、复用 `MCP_CAPABILITY_V1` 同源校验、只读保证与脱敏不降级、无第二执行路径。策略脚本届时将 `MCP_NO_RMCP_SERVER`（forbidden）收口为 `MCP_SERVER_GATED`（gated-allowed），保留其余 7 个 ACTIVE 守门。

## 范围隔离

- 零产品代码改动（`mcp.rs`/`domain.rs`/`main.rs`/`ACL`/`bridge.ts`/`types.ts` 均无 diff）。
- 未修改 `scripts/check-mcp-policy.py`。
- 仅新增 note + checkpoint；无 `.patch`（无代码 diff）。
- 未 push（仅 A0 可 push master）。

## 挂账（W9 镜头，不阻塞当前只读桥）

- **R1/G2**：能力单源碎片化（MCP/Skill/Agent/未来 Plugin/Graph）→ A3 M5-2.b 收口 `core/capability.rs` + 泛化 `CAPABILITY_SINGLE_SOURCE` 门。
- `bridge.rs:6276` 既有测试 fmt 差异（A4/A5 W7/W8 产物）→ 集成时 A0 统一 fmt。
