# Checkpoint · Lane A3 · M5-W7 MCP 只读注册表/策略桥

- **LANE**: A3
- **WAVE**: M5-W7 Integration Dispatch（BASE `5f92ece`）
- **ROLE**: START PRODUCT CODE
- **TASK**: M5-2 只读 MCP 注册表/策略桥命令（list registry / preview capability verdicts / return redacted DTOs），含 source check + ACL + 前端 bridge/types + 策略覆盖 + 测试，无 rmcp/server/监听/网络。
- **STATUS**: PASS（编译/单测/策略全绿）
- **PATCH**: `logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.patch`
- **NOTE**: `logs/assist/A3-M5-W7-mcp-readonly-bridge-20260907-0739.md`

## 改动矩阵（命令 ↔ 注册 ↔ ACL ↔ 前端 ↔ 测试）

| 命令 | 后端实现 | 来源校验 | `main.rs` 注册 | ACL | `bridge.ts` | `types.ts` | 单测 |
|---|---|---|---|---|---|---|---|
| `mcp_policy_get` | `mcp.rs::current_policy_snapshot()` | `check_invocation_source` | ✅ | ✅ | `mcpPolicyGet` | `McpPolicySnapshot` | （pure fn 经既有 `current_policy_snapshot` 路径） |
| `mcp_registry_list` | `mcp.rs::list_registry_entries()` | `check_invocation_source` | ✅ | ✅ | `mcpRegistryList` | `McpRegistryEntry[]` | `registry_view_mirrors_registry` |
| `mcp_capability_preview` | `mcp.rs::evaluate_mcp_command` + `allowed_roots` | `check_invocation_source` | ✅ | ✅ | `mcpCapabilityPreview` | `McpDecision` | `unknown_capability_preview_is_denied` + `decision_view_maps_both_variants` |

## 关键设计决策

1. **不编辑 `domain.rs`**（W7 A3 允许范围不含它）：在 `mcp.rs` 新增可序列化 `McpDecisionView` / `McpRegistryEntryView`，而非给 `domain.rs` 的 `McpDecision`/`McpCommandDef` 加 `Serialize`。`McpPolicySnapshot` 本就 `Serialize`（domain.rs 未动）。
2. **source check = `check_invocation_source`**：复用 `bridge.rs` 既有私有 fn（report_resources 同款）。`main` 受信任免令牌；`tab-*`/`grid-*` 无令牌拒绝。满足 board「source check」要求且零新增机制。
3. **只读铁证**：3 命令仅返回既有的纯函数结果（`current_policy_snapshot` / `list_registry_entries` / `evaluate_mcp_command`），无 `&mut`、无 `fs::write`、无 `spawn`、无 `Command::new`、无 `emit`、无 DB 写。新增 `MCP_BRIDGE_READONLY` 门禁在 CI 层机械守此红线（编译器守不住，见 pre-merge 注释）。
4. **`MCP_PARITY` 泛化**：由硬编码白名单改为扫描 `main.rs` 中 `bridge::mcp_[a-z_]+`，未来任何 `mcp_*` 命令自动要求 ACL + `bridge.ts` 奇偶。

## 验证结果

| 检查 | 结果 |
|---|---|
| `check-mcp-policy.py --self-test` | PASS（ACTIVE=6，PENDING=9） |
| `check-mcp-policy.py` 默认 | PASS |
| `cargo check`（src-tauri） | 0 新增告警（3 pre-existing 无关告警在 plugin.rs/grid_process.rs） |
| `cargo test mcp_policy_tests` | 9 passed |
| `npx esbuild` 转译 `types.ts`/`bridge.ts` | 无语法错 |
| `cargo fmt --check` | 仅 `bridge.rs:6276` 既有 A4/A5 W7 测试 diff（非本波；本波 mcp_* 区域 0 diff） |

## 范围隔离

- 本 lane 仅暂存并提交 7 文件（见 PATCH）。工作树其余 A1/A2/A4/A5/A7/A8/A9/A10 WIP 均保持未跟踪/未暂存，不纳入本 commit。
- 未 push（仅 A0 可 push master）。

## 挂账（非本波）

- **R1/G2**：能力单源碎片化（MCP/Skill/Agent/未来 Plugin/Graph）待 A3 M5-2.b 收口 `core/capability.rs` + 泛化 `CAPABILITY_SINGLE_SOURCE` 门。
- **G3**：plugin/agent 未来经 MCP 暴露时复用 `evaluate_mcp_command`（本波 `mcp_capability_preview` 已演示同函数路径）。
- `bridge.rs:6276` 既有测试 fmt 差异（A4/A5 W7 产物），集成时由 A0 统一 fmt。
