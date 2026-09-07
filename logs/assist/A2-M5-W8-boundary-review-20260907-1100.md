# A2 · M5-W8 Excluding-A3 边界审查笔记：Agent/Skill 桥 / graph runtime / plugin runtime / MCP runtime 是否泄漏 core 或重复脚本执行（仅文档，无产品代码）

> 生成：2026-09-07 11:00 CST · Lane A2（M5-W8 · REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W8 Excluding-A3 Dispatch（L174-223，base `6c1f30e` docs/feat(M5-W7,A3) 已集成）→ A2：**START REVIEW ONLY**；任务="Boundary review of non-A3 W7/W8: confirm Agent/Skill bridge does not leak into `mvp_core`, duplicate script execution, graph runtime, plugin runtime, or MCP runtime. Identify exact files/commands that must remain bin-side."；交付=`logs/assist/A2-M5-W8-*.md`（PASS/BLOCKED + 可操作行/文件引用）。
> 配套：A5 W7 集成 Agent/Skill 只读桥（bridge.rs L6144-6224 + ACL L118-123）；A3 W7 已集成 MCP 只读桥（`6c1f30e` L6316-6345 + ACL L124-126，W8 HOLD）；A7 W5 graph.rs（bounded store）；A9 W6 plugin.rs（纯状态机）；`core/seam.rs` 已物化；既有 policy 脚本（check-agent-skill-policy.py / check-graph-policy.py / check-plugin-policy.py）；A2 已集成之 W7 笔记 `A2-M5-W7-boundary-review-20260907-0115.md`。
>
> 承：本树 `6c1f30e`（A3 W7 MCP 只读桥已集成）、`5f92ece`（A8/A9 W6）、`1610939`（A5 W4 Agent/Skill 域 + 桥壳）。A3 在 W8 被 A0 显式 HOLD（board L177/L192/L204），其 W7 代码已进树但 W8 不得扩展。

```text
LANE=A2
STATUS=REVIEW_DONE（docs only，无产品代码；未 push）
BASE=6c1f30e（W8 当前 HEAD：A3 W7 MCP 只读桥已集成）
HEAD=logs/assist/A2-M5-W8-boundary-review-20260907-1100.md
FILES=logs/assist/A2-M5-W8-boundary-review-20260907-1100.md（仅本文档）
VERIFY=见 §10 Verify
CHECKPOINT=见 §9 Findings（F1 PASS / F2=W8 动作项 / F3-F9）+ §8 bin-side 清单
MERGE_NOTES=本笔记为 review-only；零产品代码改动，无补丁（git diff 为空）。他 lane 的 W7/W8 暂存/修改不在本任务范围，未触碰。
NEXT=见 §11 NEXT
```

## 0. 审查性质声明（重要）

- **W8 = Excluding-A3**：A3 被 A0 显式 HOLD（board L192/L204）——其 W7 本地产品码已集成于 `6c1f30e`，但 W8 任何 lane 不得扩展/依赖 A3 的产品码改动。A2 本次只审查 **non-A3** 面：A5 Agent/Skill 只读桥（已集成）、graph runtime（A7 W5）、plugin runtime（A9 W6），并以 A3 已集成的 MCP 只读桥作**参考基线**（确认其不泄漏、不扩展）。
- A5 的 Agent/Skill 只读桥**已进 canonical 树**（bridge.rs L6144-6224，ACL L118-123，源自 `1610939` W4 域壳 + W7 加固）。本次是**对已成代码的边界复核**（非前瞻），重点验证：不泄漏 `mvp_core`、不重复脚本执行、不触及 graph/plugin/MCP runtime、命令须留 bin 侧。
- A2 零产品代码；工作树中他 lane 的 W7/W8 暂存与修改（含本 A2 的 W7 笔记已被 staged）不在本任务范围，未触碰。不 commit / 不 push。

## 1. W8 角色与 Hard Stops（board L174-209）

- A2 = **REVIEW ONLY**：边界复核 non-A3 W7/W8，列 PASS/BLOCKED + 可操作行/文件引用。
- W8 Hard Stops（L202-209）：① A3 被排除，任何 lane 不得扩展/依赖 A3 产品码；② 无 MCP 产品码/rmcp/server/listener、无 plugin install/enable/delete/download、无 skill 执行/模型调用/网络；③ A5 触及的每个命令须保持只读且 source check/ACL/前端 bridge/types/policy/测试同包原子；④ 无 token/cookie/Authorization/body/prompt-secret 日志/审计/持久化/checkpoint/前端状态；⑤ 只有 A0 可 push。
- 本复核对着 Hard Stop ②/③/④ 逐项核验。

## 2. 实测证据锚点（本树 @ 6c1f30e）

| 项 | 结果 | 对审查含义 |
|---|---|---|
| core 边界 gate | 默认 `all invariants hold（ACTIVE=7，core 文件=3）`；`--self-test` `PASS(ACTIVE=7, 坏样本=9)` | 现状干净（`core/`=keyring_store.rs/mod.rs/seam.rs） |
| `core/` 是否反向依赖 bin 模块 | grep `use crate::(bridge\|mcp\|skills\|agent\|graph\|plugin\|database\|script_runner\|...)` → **CORE_NO_REVERSE_DEP**（0 命中） | 无 core→bin 反向依赖，core 边界未泄漏 |
| A5 Agent/Skill 只读命令 | bridge.rs：`agent_parse`(L6155)/`agent_validate`(L6165)/`agent_permission_preview`(L6175)/`skill_parse`(L6196)/`skill_validate`(L6206)/`skill_permission_preview`(L6216)，各仅 1 次注册（无 dup） | 6 命令均 bin-side `#[tauri::command]` |
| A5 命令内部调用 | 仅 `crate::domain::AgentDef/SkillDef::{parse,validate,permission_preview}`（L6145/6161/6182/6186/6202/6223） | **纯函数，无执行、无 graph/plugin/mcp runtime 引用** |
| bridge.rs 是否引用 graph/plugin | grep `crate::graph\|crate::plugin` → **BRIDGE_NEVER_REFS_GRAPH_OR_PLUGIN**（0 命中） | A5/A3 handler 不泄漏进 graph/plugin runtime |
| bridge.rs 是否含执行入口 | 仅既有 `script_runner::{start_command,start_run,kill_all_running,load_run_records}`（L893/3250/3300/3391）；**无 `skill_run`/`agent_chat`** | 脚本执行唯一入口仍是 `script_runner`（L911 注释 F6 守约）；A5 只读命令不触执行 |
| ACL 新命令 | L118-126：`agent_parse/validate/permission_preview`、`skill_parse/validate/permission_preview`、`mcp_*`；末条恒 `list_artifact_images`(L130) | A5/A3 只读命令 atomic 入 ACL；无 `skill_run`/`agent_chat`/`plugin_install`/`mcp_server_start`；末条守约（AGSK_ACL_TAIL） |
| A3 MCP 只读命令 | bridge.rs：`mcp_policy_get`(L6316)/`mcp_registry_list`(L6325)/`mcp_capability_preview`(L6334)，均 `check_invocation_source` + 调 `crate::mcp::{current_policy_snapshot,list_registry_entries,evaluate_mcp_command}` | 纯、server-free；W8 HOLD 不扩展 |
| 前端 parity | `src/bridge.ts:361` `skillList→invoke("skill_list")`；`:363` `agentList→invoke("agent_list")` | **后端无 `skill_list`/`agent_list` handler → parity 缺口**（见 §F2） |
| 政策脚本 ACTIVE 码 | check-agent-skill-policy.py：`AGSK_ACL_TAIL/AGSK_CAPABILITY_DRIFT/AGSK_COMMAND_PARITY/AGSK_RO_COMMAND_PARITY/AGSK_INLINE_SHELL*/AGSK_SECOND_PATH`；check-graph-policy.py：GRAPH_*（含 GRAPH_NO_SECOND_PATH/GRAPH_BOUNDED_STORE/GRAPH_PRIVACY_DOUBLE_SCAN）；check-plugin-policy.py：PLUGIN_*（含 PLUGIN_NO_SECRETS/PLUGIN_SECOND_PATH/PLUGIN_CAP_SINGLE_DEF） | 各 runtime 已有守门码；`AGSK_RO_COMMAND_PARITY` 正守 §F2 缺口 |
| roots 来源 | `allowed_roots(app)`(bridge.rs L1124) 用 `app.path().home_dir()`+workspace/notes dirs；`TauriRootsProvider`(L1170) 已将其包进 seam `RootsProvider`（当前 `#[allow(dead_code)]` 待切片 2） | A5 W8 新路径读取应复用 `allowed_roots`（单一 roots 源），避免第二解析路径 |

## 3. A5 Agent/Skill 只读桥边界审查 → **PASS**

- **不泄漏 core**：6 命令在 `bridge.rs`（bin 侧），仅用 `crate::domain::AgentDef/SkillDef`（共享类型模块，core/bin 共用，合法）。无 `core::` 引用、无 `AppHandle`/Tauri 落入 core。
- **不重复脚本执行**：handler 仅调 `parse`/`validate`/`permission_preview`（纯，零副作用）。`skill_permission_preview`(L6223) 调 `def.permission_preview()` 返回 `PermissionPreview{gate, capabilities: id 列表}`（skills.rs:45，无 secret）。**未调用** `script_runner`、`skill_run`、`agent_chat`、`SkillExec` 执行入口。脚本执行唯一入口仍是既有 `script_runner`（L911 F6 注释 + L3250/3300），与只读桥物理隔离。
- **不触及 graph/plugin/MCP runtime**：A5 handler 仅 `crate::domain`；`crate::mcp` 导入（bridge.rs L11）仅服务于 A3 的 MCP 命令（隔离）。`bridge.rs` 全程 0 引用 `crate::graph`/`crate::plugin`。
- **bin-side 落点正确**：命令为 `#[tauri::command]`，天生 bin 侧，不得搬 `core/`。
- **ACL 原子**：6 命令已在 ACL L118-123，且末条守 `list_artifact_images`。
- 结论：**PASS**（边界干净）。残留 1 项 W8 动作 = §F2 parity 缺口。

## 4. A3 MCP 只读桥（已集成，参考基线；W8 HOLD）→ **PASS（参考）**

- `mcp_policy_get`→`current_policy_snapshot()`、`mcp_registry_list`→`list_registry_entries()`(mcp.rs L149)、`mcp_capability_preview`→`evaluate_mcp_command(&capability, raw_path, &roots)`(L6343，`roots=allowed_roots(&app)`)。全纯、server-free、无 `rmcp`/`tokio`/`TcpListener`/`std::process`。
- W8 Hard Stop ①：A3 HOLD，任何 lane 不得扩展/依赖这些产品码。A2 仅确认其不泄漏（confirmed）作为 non-A3 审查的对照基线。
- 结论：**PASS（参考）**；W8 不动作。

## 5. graph runtime（A7 W5）边界 → **PASS**

- `graph.rs` 为内存 bounded store（`#![allow(dead_code)]`，W5 无消费方），无 server/网络/后台重建 worker。守门码 `GRAPH_NO_SECOND_PATH`（禁 `std::process`/`Command::new`/`tokio`/网络/后台重建）、`GRAPH_BOUNDED_STORE`、`GRAPH_PRIVACY_DOUBLE_SCAN`（复用 A4 隐私双扫）。
- A5/A3 命令未引用 `crate::graph`（§2 确认）。
- 结论：**PASS**（graph runtime 未泄漏、无第二执行路径）。

## 6. plugin runtime（A9 W6）边界 → **PASS**

- `plugin.rs` 纯状态机（manifest 生命周期），无 install/enable/delete/download/runtime。守门码 `PLUGIN_NO_SECRETS`、`PLUGIN_SECOND_PATH`、`PLUGIN_CAP_SINGLE_DEF`、`PLUGIN_INLINE_SHELL`、`PLUGIN_SIG_BYPASS`。
- A5/A3 命令未引用 `crate::plugin`（§2 确认）。
- 结论：**PASS**（plugin runtime 未泄漏、无第二执行路径）。

## 7. 重复执行路径复核（headline）→ **PASS**

- **脚本执行单入口**：只读桥（A5 6 命令 + A3 3 命令）全部纯函数，0 调用 `script_runner`/`skill_run`/`agent_chat`。执行仍仅在既有 `script_runner` 命令（L3250/3300），物理隔离 → **无重复执行路径**。
- **capability 裁决按域单源**（承 W7 §F5）：MCP→`evaluate_mcp_command`(mcp.rs)；Skill/Agent→`permission_preview`(skills.rs/agent.rs→security_policy.rs 的 `SKILL/AGENT_CAPABILITY_V1`)；Plugin→`PermissionManifestRule`(plugin.rs→security_policy.rs 的 `PLUGIN_CAPABILITY_V1`)。A5 的 `permission_preview` 不跨域（不碰 `MCP_CAPABILITY_V1`/`PLUGIN_CAPABILITY_V1`）→ 无跨域 verdict 重复。
- **roots 解析单源**：A3 `mcp_capability_preview` 用 `allowed_roots`；A5 W8 若加 `skill_list`/`agent_list` 须复用同一 `allowed_roots`（§F8），不另起路径解析。
- 结论：**PASS**。

## 8. 须留 bin-side 的精确命令/文件清单（可操作引用）

**必须留 bin 侧（不得搬 `core/`，core 无 Tauri/`AppHandle`）**：
- A5：`agent_parse`(bridge.rs:6155)、`agent_validate`(6165)、`agent_permission_preview`(6175)、`skill_parse`(6196)、`skill_validate`(6206)、`skill_permission_preview`(6216)。
- A3（W8 HOLD 不扩展）：`mcp_policy_get`(6316)、`mcp_registry_list`(6325)、`mcp_capability_preview`(6334)。
- A5 W8 待补（若落地）：`skill_list`/`agent_list` —— 须 bin 侧（`bridge.rs` handler + 加载器放 `skills.rs`/`agent.rs` 或 seam-clean 的 `core/` 模块），只读，复用单一加载器与 `allowed_roots`，**不得**引 `crate::bridge`/`AppHandle` 入 core。

**纯逻辑（bin 侧模块，合法；若未来抽 `core/` 须 seam-clean）**：
- `domain.rs`：`AgentDef`/`SkillDef`::{parse,validate,permission_preview}（共享类型，core/bin 共用）。
- `mcp.rs`：`evaluate_mcp_command`/`current_policy_snapshot`/`list_registry_entries`（bin 侧）。
- `graph.rs`：bounded store（bin 侧）。
- `plugin.rs`：纯状态机（bin 侧）。
- 若任一抽 `core/`：须保持 seam-clean（吃 `&dyn PathResolver`/`&dyn RootsProvider`、不 `use crate::bridge`/任何 bin 模块），否则触发 `CORE_BRIDGE_REF`/反向依赖红线。

**执行唯一入口（须保持隔离，只读桥不得引用）**：
- `script_runner::{start_command,start_run}`(bridge.rs:3250/3300) + `kill_all_running`(893) + `load_run_records`(3391)。

## 9. Findings

### §F1 —（PASS）A5 Agent/Skill 只读桥边界干净
6 命令全 bin-side、仅调纯 domain 函数、无 core 泄漏、无执行引用、无 graph/plugin/MCP runtime 泄漏、ACL 原子、末条守 `list_artifact_images`。core gate PASS(ACTIVE=7)。**结论 PASS。**

### §F2 —（W8 动作项 / 非 core 泄漏，属 completeness）前端 `skill_list`/`agent_list` 后端缺失 → parity 缺口
`src/bridge.ts:361/363` 已包装 `skill_list`/`agent_list`（`skillList`/`agentList`），但 `bridge.rs` 无对应 handler（A5 仅交付 parse/validate/preview，源自 W4 壳）。运行时 `invoke("skill_list")` 将无 handler。
→ **裁定**：A5/A6 W8 必须闭环——二选一：① 实现 `skill_list`/`agent_list` 后端（只读、单一加载器、复用 `parse`/`validate`/`permission_preview` 管线、`allowed_roots` 取路径、atomic ACL/bridge.ts/types.ts/测试、bin-side）；或 ② 若 W8 不做 list，从 `bridge.ts` 移除该 wrapper 以免悬空调用。`AGSK_RO_COMMAND_PARITY`（check-agent-skill-policy.py 已含）将守此 parity。此缺口不影响 core 边界/执行路径，但决定"Agent/Skill 桥"是否完整，A2 标记供 A0/A5/A6 决议。

### §F3 —（gate 加固，沿用 W7 §F6/W1）扩展 `BIN_ONLY_MODULES` 覆盖全部 bin 模块
`check-core-boundary.py` 的 `BIN_ONLY_MODULES` 当前仅 6 项（bridge/main/terminal/grid_process/tools/shutdown）。`core/` 已物化（`seam.rs`），本次实测 `core/` 无反向依赖（CORE_NO_REVERSE_DEP），但**若未来** core 误引 `mcp`/`agent`/`skills`/`plugin`/`graph`/`database`/`script_runner` 等，当前 gate 漏检。
→ **裁定**：建议 A0 在 W8/W9 将 `BIN_ONLY_MODULES` 扩为全部 bin-side 模块（保留 `domain`/`seam`/`keyring_store` 为 core 可见）。属产品代码改动，由 A0 落地，A2 不实现。

### §F4 —（PASS，参考）A3 MCP 桥已集成且 server-free；W8 HOLD
`6c1f30e` 集成 `mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview`，全纯、无 server；W8 Hard Stop ① 排除 A3，任何 lane 不得扩展/依赖。A2 仅确认不泄漏作基线。**结论 PASS（参考）。**

### §F5 —（PASS）重复执行路径 = PASS
A5/A3 只读命令 0 调用 `script_runner`/`skill_run`/`agent_chat`；脚本执行唯一入口仍在既有 `script_runner`（L911 F6 + L3250/3300）。capability 按域单源（MCP↔mcp.rs、Skill/Agent↔permission_preview、Plugin↔PermissionManifestRule），无跨域 verdict 重复。**结论 PASS。**

### §F6 —（PASS）capability 裁决按域单源
`skill_permission_preview`(L6223)→`permission_preview`→`SKILL/AGENT_CAPABILITY_V1`（security_policy.rs）；`mcp_capability_preview`(L6343)→`evaluate_mcp_command`→`MCP_CAPABILITY_V1`。A5 不触 `MCP_CAPABILITY_V1`/`PLUGIN_CAPABILITY_V1`。无第二 verdict 路径。**结论 PASS。**

### §F7 —（隐私，A4 域，简记）只读返回不泄 secret
`permission_preview` 仅回 `gate`+capability `id`（skills.rs:45，无 token/secret）。`skill_list`/`agent_list`（若 W8 加）返回 `SkillDef`/`AgentDef` 须脱敏（不携 `agent_kv` 值、不携凭证）。无 token/cookie/Authorization/body/prompt-secret 日志/审计/持久化（Hard Stop ④）。`redact_mcp_url` 仍为 MCP URL 唯一脱敏真源。**结论 PASS（建议 A4 详审 W8 payload 边界）。**

### §F8 —（seam/roots 方向）A5 W8 新路径读取复用 `allowed_roots` 单一源
`allowed_roots`(L1124) 是 bin 侧 roots 单一来源（用 `app.path()`），`TauriRootsProvider`(L1170) 已将其包进 seam `RootsProvider`（待切片 2 消费）。A3 `mcp_capability_preview` 已用 `allowed_roots`(L6341)。
→ **裁定**：A5 W8 若加 `skill_list`/`agent_list`（或任何路径读取），**必须复用 `allowed_roots`**（不得内联 `app.path()` 另写路径逻辑），保持 roots 解析单源；纯加载器吃 `&dyn PathResolver`/`&dyn RootsProvider` 或 `&str`/DTO，seam-free。避免"第二 roots 解析路径"。

### §F9 —（PASS）graph/plugin runtime 边界复核
graph.rs bounded store（check-graph-policy GRAPH_NO_SECOND_PATH/GRAPH_BOUNDED_STORE/GRAPH_PRIVACY_DOUBLE_SCAN）；plugin.rs 纯状态机（check-plugin-policy PLUGIN_NO_SECRETS/PLUGIN_SECOND_PATH/PLUGIN_CAP_SINGLE_DEF）。A5/A3 命令 0 引用 `crate::graph`/`crate::plugin`。**结论 PASS。**

## 10. Verify（本笔记证据，可复跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) core 边界 gate（基线）
python3 scripts/check-core-boundary.py            # all invariants hold（ACTIVE=7，core 文件=3）
python3 scripts/check-core-boundary.py --self-test  # CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9）
# 2) core 无反向依赖 bin 模块
grep -rnE "use crate::(bridge|mcp|skills|agent|graph|plugin|database|script_runner|terminal|grid_process|tools|shutdown|main)" src-tauri/src/core/ \
  && echo "LEAK" || echo "CORE_NO_REVERSE_DEP"
# 3) A5 命令仅纯 domain 调用（无执行）
sed -n '6144,6224p' src-tauri/src/bridge.rs | grep -nE "script_runner|skill_run|agent_chat|SkillExec::exec|crate::mcp|crate::graph|crate::plugin" \
  && echo "LEAK" || echo "A5_PURE_DOMAIN_ONLY"
# 4) A3 MCP 命令 server-free
sed -n '6316,6345p' src-tauri/src/bridge.rs | grep -nE "rmcp|tokio|TcpListener|std::process|Command::new" \
  && echo "LEAK" || echo "A3_MCP_READONLY"
# 5) bridge 永不引用 graph/plugin
grep -nE "crate::graph|crate::plugin" src-tauri/src/bridge.rs && echo "LEAK" || echo "BRIDGE_NEVER_REFS_GRAPH_OR_PLUGIN"
# 6) 前端 parity 缺口确认
grep -nE "skill_list|agent_list" src/bridge.ts   # 应命中 361/363（wrapper 存在）
grep -nE "pub fn (skill_list|agent_list)\b" src-tauri/src/bridge.rs && echo "BACKEND_EXISTS" || echo "BACKEND_MISSING_PARITY_GAP"
# 7) 政策脚本 ACTIVE 码（守门覆盖）
grep -oE '"AGSK_[A-Z_]+"|"GRAPH_[A-Z_]+"|"PLUGIN_[A-Z_]+"' scripts/check-agent-skill-policy.py scripts/check-graph-policy.py scripts/check-plugin-policy.py | sort -u
# 8) 工作树仅本笔记新增（不 commit/push）
git status --short --branch
```

## 11. NEXT

- **整体裁定：PASS**（Agent/Skill 桥 / graph runtime / plugin runtime / MCP runtime 参考 四处边界均干净，无 core 泄漏、无重复脚本执行）。**1 项 W8 动作 = §F2 parity 缺口**（`skill_list`/`agent_list` 前端已包装、后端缺失），交 A0/A5/A6 在 W8 闭环。
- 交 A0：① 采纳本复核 PASS 结论；② 排期 §F3 `BIN_ONLY_MODULES` 加固（core→bin 反向依赖机器守门）；③ 决议 §F2（`skill_list`/`agent_list` 落地 vs 移除 wrapper）；④ A3 维持 HOLD，W8 不扩展 MCP。
- A5（W8 START PRODUCT CODE）：按 §F2/§F8 闭环 Agent/Skill 桥——如需 `skill_list`/`agent_list` 则只读实现、复用 `allowed_roots`+单一加载器、atomic ACL/bridge.ts/types.ts/测试、`AGSK_RO_COMMAND_PARITY` 须过；任何命令保持只读、不触 `script_runner`/执行。
- A6（W8 UI）：Agent/Skill 面板消费计划须对齐已落地的 6 个只读命令 + §F2 决议（list 是否可用）；不引未存在后端命令的实时执行。
- A4/A9/A8/A7/A10/A11（W8）：本复核为其提供边界基线——Agent/Skill 桥 PASS、graph/plugin runtime PASS、MCP 参考 PASS；各自详审隐私/策略/UI/安全/验证时，勿引入执行/install/网络/secret/第二路径。
