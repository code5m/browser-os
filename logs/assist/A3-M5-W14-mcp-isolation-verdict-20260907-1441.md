# A3 M5-W14 MCP Isolation Verdict

**Lane:** A3 · **Wave:** M5-W14 Plugin Manager UI Dispatch · **Scope:** START MCP REVIEW ONLY
**Verdict:** **MCP_ISOLATION = PASS** · 0 blocker · concrete fixture 修正已落
**Timestamp:** 2026-09-07 14:41 CST · **HEAD:** `a7eefbbe` · **Working tree base:** master (干净 fast-forward 后；本交付零对其他 lane 文件的改动)

---

## 1. Verdict (TL;DR)

W13 由 A9 落地、经 A0 整合的 **八个 plugin 生命周期命令**（`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_keys_add` / `plugin_keys_list` / `plugin_keys_remove`）**均不**经 MCP stdio 可达。W14 仅 A6 消费这些命令的 UI（`src/bridge.ts` 包装），未新增生命周期命令、未触碰 MCP 面；MCP 隔离由 W13 护栏 `MCP_PLUGIN_NOT_EXPOSED` 与 W14 复验共同成立。本轮发现并修正一处 W13 护栏中的悬空路径（concrete fixture），MC 行为与覆盖范围不变。

---

## 2. Evidence

### 2.1 plugin 生命周期命令落点（**仅 Tauri 侧**）

| 位置 | 范围 | 说明 |
|---|---|---|
| `src-tauri/src/bridge.rs:6729-6892` | 8 个 `#[tauri::command] pub fn plugin_X` 定义 | 每个命令首行 `check_invocation_source(&webview, "plugin_X", None, &app)?` 门控（Tauri 来源校验，与 MCP stdio 路径无关） |
| `src-tauri/src/main.rs:1478-1485` | `generate_handler!` 宏 | 8 个 `bridge::plugin_X` 注册到 Tauri command handler |
| `src/bridge.ts:722-739` | 8 个 `pluginXxx(...)` 包装 | `invoke<...>("plugin_X", ...)`，由 A6 W14 UI 消费 |

**结论**：plugin 命令的「定义 + 注册 + 前端包装」三段全部落在 Tauri/前端层，**未进入** MCP 模块的任何文件。

### 2.2 MCP 面文件对 plugin 标识**零命中**

对护栏关键字集 `plugin_install|plugin_enable|plugin_disable|plugin_list|plugin_get|plugin_key|plugin_uninstall|crate::plugin` 在三处 MCP 面文件直接 grep：

| 文件 | 命中数 |
|---|---|
| `src-tauri/src/mcp.rs` | **0** |
| `src-tauri/src/domain.rs` | **0** |
| `src-tauri/src/mcp_server.rs` | **0** |

### 2.3 MCP 能力白名单（单一真源，**7 只读，无 plugin**）

- `MCP_COMMAND_REGISTRY`（`src-tauri/src/mcp.rs:23-66`）：`file_read` / `file_list` / `tab_query` / `history_query` / `bookmarks_query` / `downloads_query` / `console_query`。
- `MCP_CAPABILITY_V1`（`src-tauri/src/domain.rs:1540-1548`）：同上 7 项，**单一真源**（`check-mcp-policy.py` 的 `MCP_CAPABILITY_DRIFT` 码守门）。
- **无任何 `plugin_*` 能力项。**

### 2.4 MCP stdio `tools/call` 派发面（**fail-closed，无 plugin 分支**）

`src-tauri/src/mcp_server.rs:255-293` `tools_call` 的 `match name` 仅四个分支：

| 输入 | 行为 | 是否副作用 |
|---|---|---|
| `"mcp_policy_get"` | `current_policy_snapshot()`（只读内省） | 零 |
| `"mcp_registry_list"` | `list_registry_entries()`（只读内省） | 零 |
| `""` | `Invalid params: missing tool name`（确定性错误码 -32602） | 零 |
| `other` | 在 `list_registry_entries()` 中存在 → `not-yet-bound (M5-2.b pending); fail-closed, no execution`；不存在 → `unknown tool; fail-closed, no execution`（`isError=true`） | 零 |

整个 `tools_call` **无任何指向 `bridge::plugin_*` 或 `crate::plugin` 的代码路径**；参数永不回显。

### 2.5 隔离机制（防御纵深）

1. **第一道闸（白名单层）**：plugin 命令不在 `MCP_COMMAND_REGISTRY` / `MCP_CAPABILITY_V1` → MCP 工具/能力层不可见。
2. **第二道闸（stdio 派发层）**：`mcp_server.rs::tools_call` 仅硬编码绑定两个只读内省工具，其余一律 fail-closed → 即使白名单被错误扩展，stdio 也不会执行 plugin 命令。
3. **第三道闸（运行时隔离）**：plugin 命令由 Tauri command dispatch 处理（`main.rs` `generate_handler!` + `bridge.rs` `check_invocation_source`）；MCP stdio 通道（feature-gated `mcp_server` 模块）为独立只读内省通道，不进 Tauri command dispatch，因此 plugin 命令不会被 MCP 接收的请求触发。

### 2.6 策略脚本复验（W14 当前实测）

```
$ python3 scripts/check-mcp-policy.py --self-test
MCP_POLICY_SELF_TEST=PASS（ACTIVE=13，PENDING=0）

$ python3 scripts/check-mcp-policy.py
MCP_POLICY=PASS（无违规）

$ python3 scripts/check-mcp-policy.py --expect-current-gaps
MCP_CURRENT_GAPS_RESULT=PASS（W10 相位：只读桥在 + gated stdio 骨架已落地，无未门控 rmcp/server/listener/tokio，奇偶/只读/红线性门禁全绿）
```

`MCP_PLUGIN_NOT_EXPOSED` 与 `MCP_GRAPH_NOT_EXPOSED` 的合成坏样本 / 好样本自检均通过；默认扫描无违规。

### 2.7 Cargo MCP feature 测试

```
$ cd src-tauri && cargo test --features mcp mcp
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.66s
```

W13 plugin 整合后 MCP 面编译与测试全绿；`mcp_server::tests` 覆盖 `unknown_tool_is_fail_closed` / `tools_list_uses_registry_source_of_truth` / `stdio_smoke_*` / `response_size_guard_caps_oversized` 等关键不变量。

---

## 3. W14 Concrete Fixture Correction

### 3.1 问题
W13 落地的护栏 `c_plugin_not_exposed`（W13）与 `c_graph_not_exposed`（W12）元组中均包含 `src-tauri/src/bin/mcp_server.rs` 这一**悬空路径**：
- 真实模块位于 `src-tauri/src/mcp_server.rs`（`main.rs` 下 `#[cfg(feature="mcp")]` 调用 `mcp_server::run_stdio()`）。
- `src-tauri/Cargo.toml` 无 `[[bin]]` 条目；`src-tauri/src/bin/` 目录**不存在**。
- 悬空项因真实文件已在元组内而**不致漏报**，但属不准确，可能误导未来读者。

### 3.2 修正
在 `scripts/check-mcp-policy.py` 中，从两个 `if rel not in (...)` 元组各移除字符串 `"src-tauri/src/bin/mcp_server.rs"`：
- `c_graph_not_exposed`（W12 遗留）→ 移除悬空路径。
- `c_plugin_not_exposed`（W13 落地）→ 移除悬空路径。

### 3.3 影响
- **覆盖范围不变**：真实文件 `src-tauri/src/mcp_server.rs` 仍在两个元组内。
- **行为不变**：自检 / 默认 / expect-gaps 三模式复跑仍全绿（`ACTIVE=13`，`PASS（无违规）`，`PASS`）。
- **改动量**：`scripts/check-mcp-policy.py`，2 处元组（`c_graph_not_exposed` + `c_plugin_not_exposed`）各移除 `"src-tauri/src/bin/mcp_server.rs"`，并同步对齐 2 处对应注释（移除 ` / bin/mcp_server.rs` 提及）；合计 `4 insertions, 4 deletions`（8 行 diff，2 个护栏各 2 hunks：`git diff --stat` = `scripts/check-mcp-policy.py | 8 ++++----`）。
- **范围严格限定** MCP 策略脚本，不触其他 lane 共享文件 / 不改产品代码 / 不改 MCP 面文件。

---

## 4. W14 Scope Compliance

| 板面 W14 Hard Stop | 状态 |
|---|---|
| Work only in canonical V3 main on master; only A0 pushes | ✓ 本轮不 push；改动文件属 lane 允许范围 |
| No `plugin_invoke` / code execution / dynamic loading / network / daemon / model call / Agent-Skill / MCP runtime expansion / graph write-export / background worker | ✓ 本轮零产品代码 |
| UI must call only `src/bridge.ts`; no raw Tauri `invoke` | ✓ A6 范围（不在 A3 评审内） |
| Do not display or persist raw signature / public-key / resource path / manifest metadata / credentials / request-response bodies / stdout / stderr | ✓ A4/A10 范围（不在 A3 评审内） |
| Existing command names and DTOs are frozen. Do not add lifecycle commands in W14. | ✓ W14 唯一产品代码 lane = A6，仅消费冻结的 `bridge.ts` 方法，未新增 lifecycle 命令 → MCP 面零变化 |

A3 W14 允许文件：`logs/assist/A3-M5-W14-*.md`、`scripts/check-mcp-policy.py`（concrete fixture only）。本轮交付：
- 新增：`logs/assist/A3-M5-W14-mcp-isolation-verdict-20260907-1441.md`（本文件）。
- 修改：`scripts/check-mcp-policy.py`（concrete fixture 修正：2 处元组 -1 字符串 + 2 处注释对齐，合计 4 insertions / 4 deletions）。
- 补丁：`logs/checkpoints/Lane-A3-M5-W14-mcp-isolation-20260907-1441.patch`。

---

## 5. Verdict

**MCP_ISOLATION = PASS · 0 BLOCKER · fixture 已 W14 修正（悬空路径移除，三模式复验全绿）· MCP 面产品代码零修改 · 不 push · 可交付 A0 集成。**
