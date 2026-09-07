# A3 — M5-W15 MCP 隔离回归复核（Verdict）

- 日期：2026-09-07
- Lane：A3（MCP 隔离回归复核，W15 Dispatch：Review only → Verdict）
- 基线：`886ea29 feat(M5): integrate W14 plugin manager UI`（`git fetch` + `git pull --ff-only` 后已是最新，工作树 clean）
- 结论：**PASS —— 无 MCP 隔离回归**。本次为纯复核，**零产品代码改动、无补丁**。

---

## 1. 复核问题

W15 硬停止要求确认：W13（插件 manifest 生命周期）与 W14（插件管理器 UI）引入的
8 个插件命令，以及既有运行时能力，是否经 MCP stdio 变得可达。
MCP 必须维持「dry-run / 只读 introspection」形态，不得成为第二执行路径。

## 2. 逐面审查与证据

| # | 审查面 | 证据 | 判定 |
|---|---|---|---|
| 1 | 能力白名单单一真源 | `domain.rs:1540` `MCP_CAPABILITY_V1` = 7 项只读能力（`file_read`/`file_list`/`tab_query`/`history_query`/`bookmarks_query`/`downloads_query`/`console_query`），**无插件项** | 干净 |
| 2 | 注册表 | `mcp.rs:23` `MCP_COMMAND_REGISTRY` 7 项，与白名单**双向覆盖**（`mcp_policy_tests::registry_entries_are_subset_of_capability_whitelist` 断言），无 `plugin_*` | 干净 |
| 3 | stdio 骨架 | `mcp_server.rs:24` 仅 import `crate::mcp::{current_policy_snapshot, list_registry_entries}`；全文 grep `plugin` **零命中** | 干净 |
| 4 | `tools/call` 绑定 | 仅 `mcp_policy_get` / `mcp_registry_list` 两个只读 introspection 工具返回；其余一律 fail-closed（`isError=true`，仅回显能力名、绝不回显参数） | 干净 |
| 5 | 构建门控 | `Cargo.toml:23` `mcp = []` **非默认 feature**；`main.rs:1093-1097` `--mcp-stdio` 分支在 `run_stdio()` 后 **return**，不进入 Tauri 启动流程 → 插件 Tauri 命令在 stdio 模式**结构上不可达** | 干净 |
| 6 | ACL 命名空间 | `default-commands.toml`：`mcp_*` 3 项（124-126）与 `plugin_*` 8 项（130-137）**无交集** | 干净 |
| 7 | 前端 | `src/components/plugin/**` 中 `mcp` **零引用**；`bridge.ts` 仅 3 个 `mcp_*` 只读 wrapper（713-716），无 plugin→mcp 路径 | 干净 |
| 8 | 机器门禁 | `MCP_PLUGIN_NOT_EXPOSED`（**ACTIVE**）扫描 `mcp.rs`（全文件）/ `domain.rs`（仅 `MCP_CAPABILITY_V1` 块，避免 `PluginManifest` 等类型误报）/ `mcp_server.rs`（全文件），命中 `plugin_install|enable|disable|list|get|plugin_key|plugin_uninstall|crate::plugin` 即判红；自检含对应坏样本 | 干净 |

### 2.1 W14 对 MCP 门禁的改动（重点复核）
`886ea29` 仅改 `scripts/check-mcp-policy.py` 4 行（4+/4-）：从图谱与插件两个检查的
扫描目标元组中**删除不存在的** `src-tauri/src/bin/mcp_server.rs`，并同步注释。

- 已核实 `src-tauri/src/bin` 与 `src-tauri/bin` **均不存在**（`ls` 报 No such file or directory）。
- 因此该删除是**清理陈旧路径**，对实际守门面**零削弱**：真实文件 `src-tauri/src/mcp_server.rs` 仍在扫描列表内。

### 2.2 MCP Rust 面自 W11 起未变
`mcp.rs` / `mcp_server.rs` 自 `269269a`（W11 硬化）后**无改动**；W12/W13/W14 三次提交
只增改 `check-mcp-policy.py`（W12 +37、W13 +40、W14 ±4），均为门禁侧加固/清理。

## 3. 门禁与测试证据（实跑）

| 项 | 结果 |
|---|---|
| `check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS（ACTIVE=13，PENDING=0）` |
| `check-mcp-policy.py`（默认） | `MCP_POLICY=PASS（无违规）` |
| `check-mcp-policy.py --expect-current-gaps` | `MCP_CURRENT_GAPS_RESULT=PASS` |
| `cargo test --features mcp mcp` | 30 passed / 0 failed |
| `cargo test`（默认构建，无 mcp feature） | 433 passed / 0 failed |
| `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=ALL_PASS` |

## 4. 非阻塞观察（不在 W15 修改，W15 禁止 MCP 扩张）

**`tools/list` 与 `tools/call` 不对称**：`mcp_server.rs:34`
`BOUND_READONLY_TOOLS = ["mcp_policy_get", "mcp_registry_list"]` 这两个名字**不是**
注册表能力名（注册表 7 项是 `file_read` 等），因此 `tools_list` 中
`BOUND_READONLY_TOOLS.contains(&e.capability)` 对全部 7 项恒为 `false`——
`tools/list` 把 7 个能力**全部**标注为 `not-yet-bound`，而实际可调用的两个只读工具
又**不在** `tools/list` 里。

- 方向是 **fail-closed（过度收紧）**，不是隔离缺陷，不违反任何硬停止；
- 严格按 `tools/list` 发现工具的客户端看不到两个可用工具；
- 现有测试只断言「数量一致」，未覆盖该不对称，故未暴露。

建议 A0 排入 **M5-2.b**（与「绑定 core API」一并收口）：要么把两个 introspection 工具
并入注册表，要么让 `tools_list` 额外列出已绑定工具。本次不改。

## 5. Verdict

**PASS —— MCP 隔离无回归。** W13/W14 的插件生命周期能力**未**经 MCP 暴露：
白名单、注册表、stdio 骨架、构建门控、ACL、前端六面均无插件可达路径，
且 `MCP_PLUGIN_NOT_EXPOSED` 已固化为 ACTIVE 门禁并含坏样本。
本 lane 无产品代码改动、无需补丁；可进入 A11 的 W15 放行矩阵。
