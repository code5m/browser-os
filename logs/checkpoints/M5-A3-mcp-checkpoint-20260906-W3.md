# A3 · M5-2 MCP 命令注册表 / 全局策略 · W3 整包交付 checkpoint

> LANE=A3　WAVE=M5-W3（Parallel Dispatch，板 §M5-W3）　STATUS=PASS_WITH_CODE+GATE
> BASE=98a3b01（master 已 pull --ff-only 至 origin/master，含 A0 的 W3 dispatch）
> DELIVERABLES=`src-tauri/src/mcp.rs`(新增) + `domain.rs`/`main.rs`(改动) + `check-mcp-policy.py`(扩) + `pre-merge.sh`(接线) + 本 checkpoint
> 生成：2026-09-06 14:00 CST

---

## 0. 本切片是什么 / 不是什么（对照 W3 Hard Stops）

**是**：M5-2「首期 MCP 命令注册表 + 全局策略」的**纯产品代码切片**——
- 冻结 DTO（`McpCommandDef` / `McpDecision` / `McpPolicySnapshot` / `MCP_CAPABILITY_V1` 能力白名单，单一真源在 `domain.rs`）；
- 纯注册表（`MCP_COMMAND_REGISTRY`）+ 纯策略（`evaluate_mcp_command` fail-closed，复用 `security_policy::check_path_within_roots` / `redact_sensitive_url`）；
- 把门禁 `check-mcp-policy.py` 从「已落地未接线」推进为「已接线进 `pre-merge.sh`」，并新增 `MCP_FS_TOOL_PATH_POLICY` 码位。

**不是**（W3 Hard Stops 逐条对账）：
- ❌ 无 `rmcp` 依赖（Cargo.toml 未加，仓库零 `tokio::`）；
- ❌ 无网络 server / 后台 listener / `TcpListener` / `axum` / `hyper`（mcp.rs 零网络形态）；
- ❌ 无 npm 分包 / Node 运行时；
- ❌ 无新 Tauri 命令（未动 `bridge.rs`/`ACL`/`bridge.ts`/`types.ts`；A1 卡里的 `mcp_server_start/stop`/`mcp_policy_get` 仍按 B1/B7 挂账，未落地）；
- ❌ 无 Agent 运行时 / plugin 运行时。

> 机器守门：`check-mcp-policy.py` 的 ACTIVE 红线（`MCP_NPM_SDK_PRESENT` / `MCP_NPM_IN_CARGO` / `MCP_NODE_RUNTIME_PRESENT`）对全仓常扫；PENDING 红线（`MCP_LISTEN_PORT` / `MCP_RUNTIME_LEAK` / `MCP_OPTIONAL_DEP` / `MCP_BIN_GATED` 等）在 `MCP_CAPABILITY_V1` 出现后置真、现零命中。

---

## 1. 交付物清单与改动点

| 文件 | 改动 | 说明 |
|---|---|---|
| `src-tauri/src/mcp.rs` | 新增 | 纯注册表 + 纯策略 + 6 个 Rust 单测；`#[allow(dead_code)]` 标记首期未被消费的 fn（零新增编译警告） |
| `src-tauri/src/domain.rs` | 增 ~60 行 | 冻结 `MCP_CAPABILITY_V1` / `McpCommandDef` / `McpDecision` / `McpPolicySnapshot` / `is_known_mcp_capability`（全部 `#[allow(dead_code)]`，与 `DbConnectionConfig` 同口径） |
| `src-tauri/src/main.rs` | +1 行 | `mod mcp;`（bin 专属产品模块，非 `mvp_core` core 边界，不触发 `check-core-boundary.py`） |
| `scripts/check-mcp-policy.py` | 扩 | 新增 ACTIVE 码位 `MCP_FS_TOOL_PATH_POLICY`（注册表 `touches_fs`/`returns_url` 项必须复用路径根 / URL 脱敏守门）+ 自测坏样本；现在 `ACTIVE=5, PENDING=9` |
| `scripts/pre-merge.sh` | 接线 | `run_pre_merge` 增 M5-2 段（`--self-test` + 默认门禁）；`run_self_test` 增文件存在性 + `--self-test`；**不挂 `--expect-pending`**（M5-2 切片已存在，该守卫已完成历史使命） |
| `logs/checkpoints/M5-A3-mcp-checkpoint-20260906-W3.md` | 新增 | 本 checkpoint |

### 1.1 能力白名单（单一真源，`MCP_CAPABILITY_V1`）
`file_read` · `file_list` · `tab_query` · `history_query` · `bookmarks_query` · `downloads_query` · `console_query`
（首期：只读 + 路径根 / URL 脱敏约束；无 write/exec/navigation/close——承接 A1 M5-2 卡 §4 与 A3 主篇 §3）

### 1.2 策略不变量（fail-closed）
1. 能力不在白名单 ⇒ Deny；
2. 触碰文件系统但未过 `check_path_within_roots` ⇒ Deny；
3. URL 回传前必须经 `redact_sensitive_url`（与 M1-8 同源）。

---

## 2. 验证证据（2026-09-06 14:00，本机）

```text
# 格式化（IF-4：不新增 cargo fmt 差异）
$ cargo fmt --manifest-path src-tauri/Cargo.toml --all --check   → 0 (clean)

# 编译（IF-4：不新增 cargo warning）
$ cargo check --manifest-path src-tauri/Cargo.toml --locked
  → Finished；剩余 2 个 warning 在无关模块（fields `index`/`comms` unused、
    fn `new` unused），非本切片文件；本切片 0 新增 warning

# Rust 单测（整包交付含 focused tests）
$ cargo test --manifest-path src-tauri/Cargo.toml --locked mcp::
  running 6 tests
    capability_whitelist_is_closed_and_fail_closed ... ok
    registry_entries_are_subset_of_capability_whitelist ... ok
    fs_capability_outside_roots_is_denied ... ok
    unknown_capability_is_denied ... ok
    url_is_redacted_before_return ... ok
    fs_capability_inside_roots_is_allowed ... ok
  test result: ok. 6 passed

# 策略门禁（已接线 pre-merge）
$ python3 scripts/check-mcp-policy.py --self-test   → MCP_POLICY_SELF_TEST=PASS（ACTIVE=5，PENDING=9）
$ python3 scripts/check-mcp-policy.py               → MCP_POLICY=PASS（无违规，真实仓含 mcp.rs）
$ bash -n scripts/pre-merge.sh                       → SYNTAX_OK

# 红线复核（真实仓扫描确认 W3 Hard Stops 守约）
$ rg -n 'tokio::|rmcp|MCP_CAPABILITY_V1|MCP_COMMAND_REGISTRY|mod mcp|mcp_server|mcp_tools' src-tauri/src
  → 仅本切片新增项；无 tokio::/rmcp/mcp_server/mcp_tools 红迹
```

---

## 3. 与既有交付的衔接（已落实）

- **M5-2.a（W2，已随 a654f0c 集成）**：`check-mcp-policy.py` 守门脚本。本切片把它从「存在未接线」推进为「接线进 `pre-merge.sh`」，并补 `MCP_FS_TOOL_PATH_POLICY`——现在注册表一旦加入 `touches_fs`/`returns_url` 工具却漏路径/URL 守门，门禁必红。
- **M5-1.a（A2）**：`mcp` 模块放在 bin（`main.rs` 的 `mod mcp;`），**不进 `mvp_core` core 边界**，`check-core-boundary.py` 不受影响（已确认 core 纯洁性）。
- **M1-8 / security_policy（A3 主篇既定语）**：文件系统与 URL 守门**复用** `security_policy::{check_path_within_roots, redact_sensitive_url}`，单一真源，未各实现一份（堵 B8/B9）。

---

## 4. 剩余阻塞项（仍需 A0 裁决，未因本切片改变）

| ID | 阻塞项 | 与本切片关系 |
|---|---|---|
| B1 | `rmcp`→`tokio` 与 F-1 冲突 | 本切片 deliberately 不含 rmcp；真 server 由 M5-2.b 在 A0 裁决(a)豁免/(b)阶段二独立包 后落地 |
| B7 | stdio 独立进程 vs GUI 内 `mcp_server_start/stop` | 本切片未加该命令；建议首期砍掉 start/stop（纯 stdio subprocess） |
| B8 | `read_file`/`list_dir` 路径根策略 | 本切片 `evaluate_mcp_command` 已 fail-closed 调 `check_path_within_roots`；注册表 `file_read`/`file_list` 标记 `touches_fs` |
| B3 | MCP 无 Webview，`db_query` 首行 `check_invocation_source` | 本切片注册表 `core_api` 标注走核心内部 API（如 `workspace.read_file`），不调命令层 `bridge::*` |

> A1 的 M5-2 卡 (`logs/checkpoints/M5-20260906/M5-2-rmcp-mcp-policy.md`) 本 turn 被其他 lane 修改（不在本切片提交范围），其 `mcp_server_start/stop` 等命令仍待 A0 修订；本切片与之无冲突（本切片不新增任何 Tauri 命令）。

---

## 5. 输出模板回填（Lane Output Template）

```text
LANE=A3
STATUS=PASS_WITH_CODE+GATE（产品代码 + 门禁接线，双 PASS）
BASE=98a3b01
HEAD=src-tauri/src/mcp.rs（新增）+ domain.rs/main.rs（改）+ check-mcp-policy.py（扩）+ pre-merge.sh（接线）
FILES=src-tauri/src/mcp.rs
      src-tauri/src/domain.rs
      src-tauri/src/main.rs
      scripts/check-mcp-policy.py
      scripts/pre-merge.sh
      logs/checkpoints/M5-A3-mcp-checkpoint-20260906-W3.md
VERIFY=cargo fmt --all --check → 0；cargo check --locked → 0 新增 warning；
      cargo test mcp:: → 6 passed；
      python3 scripts/check-mcp-policy.py --self-test → PASS(ACTIVE=5,PENDING=9)；
      python3 scripts/check-mcp-policy.py → PASS；
      bash -n scripts/pre-merge.sh → OK
      rg 'tokio::|rmcp|mcp_server|mcp_tools' src-tauri/src → 仅本切片白名单项，零红迹
CHECKPOINT=logs/checkpoints/M5-A3-mcp-checkpoint-20260906-W3.md
MERGE_NOTES=①W3 Hard Stops 全守约（无 rmcp/tokio/listener/npm/新命令/Agent 运行时）；
            ②门禁已接线 pre-merge（self-test+默认，未挂 expect-pending——M5-2 切片已存在）；
            ③本切片不新增任何 Tauri 命令/ACL/bridge.ts，零前端改动，merge 原子、低冲突；
            ④B1/B7/B8/B3 仍待 A0 裁决；M5-2.b（rmcp server bin + capability.rs 真源 handler）待裁决后由本 Lane 续做
NEXT=A0 裁决 B1 → A3 据修订后 M5-2 卡落地 M5-2.b：rmcp stdio server bin（optional+required-features 隔离）
     + capability handler 调本切片注册表；届时门禁 PENDING 红线自然接管
```
