# M5-W13 A3 MCP Isolation Review — Plugin Lifecycle Not Exposed via MCP stdio

- **LANE** = A3
- **STATUS** = DONE（REVIEW ONLY；未触碰任何产品代码，仅补策略脚本 concrete fixture）
- **WAVE** = M5-W13 Plugin Runtime Stage-I Manifest Lifecycle Dispatch
- **ASSIGNMENT**（指挥板行 200）= `START MCP REVIEW ONLY`：确认插件生命周期命令**不**经 MCP stdio 暴露；MCP 仍为 dry-run/只读内省。Must Deliver：`logs/assist/A3-M5-W13-*.md` + `scripts/check-mcp-policy.py` 仅作 concrete fixture；产出 = **MCP 隔离判定**。

## 1. 判定结论：**PASS（隔离成立）+ 新增预防性护栏**

当前 MCP 面与插件生命周期**结构性隔离**；并补 `MCP_PLUGIN_NOT_EXPOSED` 静态护栏（与 W12 `MCP_GRAPH_NOT_EXPOSED` 同源范式），使 A9 后续并发落地插件命令时，任何误接进 MCP 的行为都会被 pre-merge 门禁捕获。

## 2. 实测证据（file:line 锚定）

| 检查项 | 结果 | 锚点 |
|---|---|---|
| MCP 命令注册表无 plugin 项 | 7 项全为只读（file_read/file_list/tab_query/history_query/bookmarks_query/downloads_query/console_query） | `src-tauri/src/mcp.rs:23-66` |
| MCP 能力白名单（`MCP_CAPABILITY_V1` 单一真源）无 plugin 能力 | 同 7 项只读，无 `plugin_*` | `src-tauri/src/domain.rs:1540-1569` |
| MCP stdio 骨架无 plugin 引用 | `grep "plugin" mcp_server.rs` = **0 命中**；仅复用 `crate::mcp` 视图、fail-closed、零执行 | `src-tauri/src/mcp_server.rs:1-555`（注释 8-22 明确不碰插件运行时） |
| 插件生命周期命令尚未落入本树 | `grep "plugin_install\|plugin_enable\|plugin_disable\|plugin_list\|plugin_get\|plugin_key"` 全仓 = **0 命中**（A9 W13 产品代码仍 START，未提交） | 全仓 `src-tauri/src` |
| MCP feature 测试 | `cargo test --features mcp mcp` → **30 passed / 0 failed** |（mcp.rs + mcp_server.rs 模块）|
| MCP 策略脚本 | `check-mcp-policy.py` self-test **PASS(ACTIVE=13)** / default **PASS** / expect-current-gaps **PASS** | `scripts/check-mcp-policy.py` |

## 3. 隔离机制（为何 plugin 命令无法经 MCP 到达）

1. **MCP 面是固定白名单**：能力来源仅 `MCP_COMMAND_REGISTRY`（`mcp.rs`）+ `MCP_CAPABILITY_V1`（`domain.rs`，策略脚本 `MCP_CAPABILITY_DRIFT` 守单一真源）。插件生命周期命令（`plugin_install`/`plugin_enable`/`plugin_disable`/`plugin_list`/`plugin_get`/受信密钥增删）一旦由 A9 实现，是 **Tauri 侧命令**（注册于 `bridge.rs` + `main.rs` `generate_handler`），自带 ACL 条目与审计——它们**不**进入 MCP 注册表/白名单，故在 MCP stdio 路径上不可达。
2. **stdio 骨架只做只读内省**：`mcp_server.rs` 的 `tools/list`/`tools/call` 仅经 `list_registry_entries()` / `current_policy_snapshot()` 返回视图；其余 registry 能力一律 fail-closed（`not-yet-bound` / `unknown tool` → `isError=true`），**不**分发到 `crate::plugin`，不执行、不调用 fs/db/脚本/插件。
3. 因此即便 A9 的插件命令落地，只要不主动把它们写进 `MCP_COMMAND_REGISTRY` / `MCP_CAPABILITY_V1` / `mcp_server.rs` 分发，MCP 面天然不暴露——本护栏（第 4 节）正是守住这个「不主动写」的边界。

## 4. Concrete fixture（A3 在 `check-mcp-policy.py` 的交付）

新增 `MCP_PLUGIN_NOT_EXPOSED`（`ACTIVE`，与 W12 `MCP_GRAPH_NOT_EXPOSED` 同构）：

- 守门文件：`mcp.rs`（注册表）、`domain.rs`（`MCP_CAPABILITY_V1` 白名单块）、`mcp_server.rs` / `bin/mcp_server.rs`（stdio 骨架）。
- 命中标识：`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key` / `plugin_uninstall` / `crate::plugin`。
- `domain.rs` 仅扫描能力白名单块（避免误报既有 `PluginManifest`/`PluginState` 等插件领域类型）。
- 新增 self-test 坏样本（`mcp_server.rs` 调 `crate::plugin::plugin_install`）→ 验证可检出；`ACTIVE` 由 12 → **13**。
- 改动范围：`scripts/check-mcp-policy.py` **+40 行**，零产品代码、零 Rust 改动（符合 W13 Hard Stops 与 A3 仅 REVIEW/fixture 的权限）。

## 5. 跨车道与升级项

- **A9（W13 START，并发）**：拥有插件产品代码（manifest 注册表/store、`plugin_install` 等、受信密钥、ACL、审计脱敏）。本判定要求其命令**不得**写入 MCP 注册表/白名单/stdio 分发；本护栏已机器守护，A0 整合时把 `scripts/check-mcp-policy.py` 补丁并入即可。
- **A0 整合**：Apply patch `logs/checkpoints/A3-M5-W13-mcp-plugin-isolation.patch`（仅 `scripts/check-mcp-policy.py`）；本 assist 文档为新文件，随整批提交。**不得 push**，仅 A0 push。
- **W13 Hard Stops 遵守**：未实现 plugin_invoke / 命令执行 / 动态加载 / 网络下载监听 / daemon / 模型调用 / Agent/Skill 执行 / MCP full runtime / graph 构建写导出 / 后台 worker；未存凭据/原始签名/请求响应体/插件 stdout/stderr/未脱敏路径于审计或 checkpoint。

## 6. Lane Output Template

```
LANE=A3
STATUS=DONE (REVIEW ONLY)
WAVE=M5-W13
BASE=3c3f460 (origin/master, ff-only 已同步, 工作树干净)
FILES=scripts/check-mcp-policy.py (+40 行, 护栏 fixture); logs/assist/A3-M5-W13-mcp-plugin-isolation-verdict-20260907-1346.md (本判定)
VERIFY=python3 scripts/check-mcp-policy.py --self-test → PASS(ACTIVE=13); --expect-current-gaps → PASS; cargo test --features mcp mcp → 30/30
CHECKPOINT=logs/checkpoints/A3-M5-W13-mcp-plugin-isolation.patch
MERGE_NOTES=A0 并入 check-mcp-policy.py 补丁即可；插件产品代码归 A9，本车道仅守 MCP 隔离
NEXT=A9 落地插件生命周期命令时，须保持不写入 MCP 注册表/白名单/stdio 分发（本护栏守门）；A11 在 W13 验证矩阵纳入 MCP_POLICY 全绿
```
