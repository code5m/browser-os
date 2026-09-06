# Lane A3 · M5-W7 MCP 只读注册表/策略桥 · 交付 note（START PRODUCT CODE）

> LANE=A3　WAVE=M5-W7 Integration Dispatch（board `5f92ece`，mainline 已同步）　ROLE=**START PRODUCT CODE**
> 依据 board §M5-W7（行 115-195）→ A3 = **START PRODUCT CODE**；Task="Implement M5-2 read-only MCP registry/policy bridge commands: list registry entries, preview capability verdicts, return redacted DTOs. Must include source check, ACL, frontend bridge/types only if commands are added. No rmcp/server/listener."
> Allowed Scope = `src-tauri/src/mcp.rs`, `src-tauri/src/bridge.rs`, `src-tauri/src/main.rs`, `src-tauri/permissions/default-commands.toml`, `src/bridge.ts`, `src/types.ts`, `scripts/check-mcp-policy.py`, focused tests/checkpoint
> Must Deliver = **Commands + ACL/source check + policy/tests PASS，no runtime server，no network**
> BASE=`5f92ece`　STATUS=PASS（编译/单测/策略全绿；仅 1 处 pre-existing fmt 差异与本次无关）

---

## 0. 调度自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git fetch && pull --ff-only` | 同步到 `5f92ece`（W7 dispatch） | ✅ |
| 仅 A3/A5 可写产品代码 | ✅；A5 同波交付 Agent/Skill 只读桥（`logs/assist/A5-M5-W7-agent-skill-readonly-bridge-*.md`，ACL 已含 `agent_*`/`skill_*`） | ✅ 互补不冲突 |
| 是否触碰 `domain.rs` / board / identity | 否（W7 A3 允许范围不含 `domain.rs`） | ✅ 严守 |
| 红线（rmcp/tokio/监听/网络/第二执行路径） | 本波无任何 `rmcp`/`tokio`/`TcpListener`/`Command::new` 引入 | ✅ |

---

## 1. 交付物（3 个只读命令 + 视图类型 + 单测 + 策略门禁）

### 1.1 后端：纯函数（mcp.rs，未动 domain.rs）
- 新增可序列化视图类型（避开 domain.rs 编辑红线）：
  - `McpDecisionView`（`#[serde(rename_all="snake_case")]`→`"allow"/"deny"`，与 `domain.rs::McpDecision` 对应）
  - `McpRegistryEntryView { capability, core_api, touches_fs, returns_url }`（与 `McpCommandDef` 字段对齐，显式构造以控暴露面）
  - `pub fn list_registry_entries() -> Vec<McpRegistryEntryView>`
- `current_policy_snapshot()`（W4 已有，本波解除 `#[allow(dead_code)]` 正式被命令调用）、`evaluate_mcp_command`、`lookup_mcp_command` 均被消费，死代码告警消除。

### 1.2 命令（bridge.rs，含来源校验 = "source check"）
| 命令 | 行为 | 来源校验 | 复用既有原语 |
|---|---|---|---|
| `mcp_policy_get` | 返回 `McpPolicySnapshot`（能力白名单 + 策略版本，红线性无凭据） | `check_invocation_source(&webview,"mcp_policy_get",None,&app)`（main 免令牌；tab-*/grid-* 无令牌拒绝） | `crate::mcp::current_policy_snapshot()` |
| `mcp_registry_list` | 返回 `Vec<McpRegistryEntryView>`（注册表只读 introspection） | 同上 | `crate::mcp::list_registry_entries()` |
| `mcp_capability_preview(capability, raw_path?)` | 返回 `McpDecisionView`（Allow/Deny 预览） | 同上 | `allowed_roots(&app)` + `crate::mcp::evaluate_mcp_command`（内部 `check_path_within_roots` + `redact_sensitive_url`） |

> "source check" 实现：复用 `bridge.rs::check_invocation_source`（report_resources 同款）——仅受信任 `main` 主窗口可调用，外部 webview 无令牌一律拒绝。这正是 board W7「source check」要求。

### 1.3 注册 / ACL / 前端
- `main.rs` `generate_handler!` 注册 3 条 `bridge::mcp_*`。
- `src-tauri/permissions/default-commands.toml` 在末条 `list_artifact_images` **之前**插入 3 条 ACL 允许（遵守「ACL 末条恒为 list_artifact_images」约定）。
- `src/bridge.ts` 新增 3 个 `invoke<...>()` 封装：`mcpPolicyGet` / `mcpRegistryList` / `mcpCapabilityPreview`。
- `src/types.ts` 新增 3 个 DTO（与后端对齐，红线性无凭据）：`McpDecision` / `McpPolicySnapshot` / `McpRegistryEntry`。

### 1.4 单测（mcp.rs `mcp_policy_tests`）
- `registry_view_mirrors_registry`（视图与 `MCP_COMMAND_REGISTRY` 逐字段镜像）
- `decision_view_maps_both_variants`（Allow/Deny 双向映射）
- `unknown_capability_preview_is_denied`（未知能力→Deny）
- 既有 6 项保留；本波 `cargo test mcp_policy_tests` **9 passed**。

### 1.5 策略门禁（check-mcp-policy.py）—— 泛化 + 新增
- **`MCP_PARITY` 泛化**：由「硬编码 `mcp_policy_get|mcp_policy_set|mcp_server_*`」改为扫描 `main.rs` 中 `bridge::(mcp_[a-z_]+)` 注册项，要求每条在 `default-commands.toml` 与 `bridge.ts` 同时存在（奇偶守门 B5）。后续任何 `mcp_*` 命令自动受守。
- **新增 `MCP_BRIDGE_READONLY`（ACTIVE）**：扫描 `bridge.rs`/`mcp.rs` 中 `#[tauri::command] pub fn mcp_*` 的函数体，禁止写/执行类副作用（`std::fs::write|fs::write|write_file|create_*|delete_path|rename_path|script_runner|run_command|db_*|spawn|Command::new|.store(|insert(|emit(`）。守 W7 Hard Stop「只读」。
- self-test 同步更新：`MCP_PARITY` 坏样本改为在 `main.rs` 注册（否则不触发）；新增 `MCP_BRIDGE_READONLY` 坏样本（`mcp_policy_set` 含 `std::fs::write`）。

---

## 2. 红线 / Hard Stop 合规

| W7 Hard Stop | 本波合规 |
|---|---|
| 仅 A3/A5 可写产品代码 | ✅ 仅 A3 编辑 mcp/bridge/main/toml/bridge.ts/types.ts/check-mcp-policy.py |
| 命令只读：无 skill 执行 / 无 plugin 安装 / 无 MCP server/监听 / 无 graph rebuild worker | ✅ 3 命令纯返回 `current_policy_snapshot`/`list_registry_entries`/`evaluate_mcp_command` 结果，零副作用、零 rmcp、零监听 |
| 每新命令须 source check + ACL + 前端 bridge/types + 策略覆盖 + 测试 | ✅ 见 §1.2-1.5 逐项齐备 |
| 无 token/cookie/Authorization/body/prompt-secret 日志或持久化 | ✅ 返回 DTO 仅含能力名/策略版本/注册表映射/Allow-Deny，零凭据零 URL 明文 |

> 能力单源仍是 `domain.rs:1540` 的 `MCP_CAPABILITY_V1`（W6 已记录碎片化；本波只读桥不改单源，单源收敛仍属 A3 M5-2.b + A0 裁决的 R1 挂账）。`mcp_capability_preview` 直接复用 `evaluate_mcp_command`，确保 **plugin/agent 未来经 MCP 暴露时裁决与 MCP 完全一致**（承接 W6 G3）。

---

## 3. 验证（全绿）

```bash
# 策略自测 + 默认扫描
python3 scripts/check-mcp-policy.py --self-test   # MCP_POLICY_SELF_TEST=PASS (ACTIVE=6, PENDING=9)
python3 scripts/check-mcp-policy.py                # MCP_POLICY=PASS（3 命令满足奇偶 + 只读）
# Rust 编译 + 单测
cd src-tauri && cargo check                        # 0 新增告警（仅 3 条 pre-existing 无关告警：plugin.rs/grid_process.rs）
cd src-tauri && cargo test mcp_policy_tests        # 9 passed
# 前端语法
npx esbuild src/types.ts --format=esm --outfile=/dev/null   # Done（无语法错）
npx esbuild src/bridge.ts --format=esm --outfile=/dev/null  # Done（无语法错）
# 格式
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
#   → 仅 1 处 diff，位于 bridge.rs:6276 的 A4/A5 W7 `agent_validate_inner` 既有测试
#     （非本次新增；本波 mcp_* 命令区域 0 diff，fmt 干净）
```

---

## 4. 改动文件清单（本 lane 专属，7 文件）

```
src-tauri/src/mcp.rs                         +81  -0   视图类型 + list_registry_entries + 单测
src-tauri/src/bridge.rs                     +231 -0   import + 3 个 #[tauri::command] mcp_*
src-tauri/src/main.rs                        +9  -0   generate_handler! 注册 3 条
src-tauri/permissions/default-commands.toml  +9  -0   ACL 允许 3 条（末条 list_artifact_images 之前）
src/bridge.ts                               +24  -0   3 个 invoke 封装
src/types.ts                                +31  -0   3 个 DTO
scripts/check-mcp-policy.py                 +65 -17   MCP_PARITY 泛化 + MCP_BRIDGE_READONLY + self-test
```

> 工作树另有 A1/A2/A4/A5/A7/A8/A9/A10 的 WIP（未跟踪/未暂存），本 lane 仅暂存并交付上述 7 文件，不触碰他 lane 产物（见 `git status`）。

---

## 5. 补丁 / checkpoint

- 补丁：`logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.patch`（仅本 lane 7 文件，`git diff -- <7 files>` 生成）
- checkpoint：`logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.md`

---

## 6. NEXT / 合并须知

- 本 patch 仅新增 3 个**只读** MCP 桥命令 + 视图类型 + 策略门禁；无 `mcp_policy_set`/server，无副作用，无网络。
- 与 A5 同波 Agent/Skill 只读桥互补（bridge.rs 已共存 `agent_*`/`skill_*` 命令 + 本波 `mcp_*`），`generate_handler!` 与 ACL 各自独立增项，无冲突。
- 挂账（非本波范围）：① 能力单源碎片化（MCP/Skill/Agent/未来 Plugin/Graph）——仍待 A3 M5-2.b 收口 `core/capability.rs` + 泛化 `CAPABILITY_SINGLE_SOURCE` 门（W6 G2/R1）；② `bridge.rs:6276` 既有测试 fmt 差异（A4/A5 W7 产物，非本波引入，集成时由 A0 统一 fmt）。
- 未 push（仅 A0 可 push master）。

```
LANE=A3
STATUS=PASS
WAVE=M5-W7 (START PRODUCT CODE)
BASE=5f92ece
HEAD=logs/assist/A3-M5-W7-mcp-readonly-bridge-20260907-0739.md
FILES=src-tauri/src/mcp.rs,src-tauri/src/bridge.rs,src-tauri/src/main.rs,src-tauri/permissions/default-commands.toml,src/bridge.ts,src/types.ts,scripts/check-mcp-policy.py
VERIFY=check-mcp-policy.py --self-test PASS(ACTIVE=6,PENDING=9) / 默认 PASS；cargo check 0 新增告警；cargo test mcp_policy_tests 9 passed；esbuild 转译 types.ts/bridge.ts 无错；cargo fmt --check 仅 bridge.rs:6276 既有 A4/A5 测试 diff（非本波）；git diff -- <7 files> 干净
CHECKPOINT=logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.{patch,md}
MERGE_NOTES=仅增 3 只读 MCP 桥命令(mcp_policy_get/mcp_registry_list/mcp_capability_preview)+视图类型+单测+门禁；来源校验复用 check_invocation_source(main 受信/tab-grid 拒)；ACL 在末条前；未动 domain.rs(避红线)；无 rmcp/server/监听/网络/副作用；与 A5 Agent/Skill 只读桥互补；单源碎片化仍挂账 R1/G2
NEXT=A3 M5-2.b 收口 core/capability.rs + 泛化 CAPABILITY_SINGLE_SOURCE 门；plugin/agent 未来经 MCP 暴露时复用 evaluate_mcp_command(G3)
```
