# A2 · M5-W9 Runtime-Free Polish 边界裁决：A3/A5 W8 修复后再审（仅文档，无产品代码）

> 生成：2026-09-07 15:00 CST · Lane A2（M5-W9 · REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W9 Runtime-Free Polish Dispatch（L175-224，base `97118d6` chore(M5): normalize W8 patch evidence whitespace）→ A2：**START REVIEW ONLY**；任务="Re-review command boundary after A3/A5 W8 fixes: no runtime server/listener, no core tauri leak, read-only bridge only, no duplicate execution path."；交付=`logs/assist/A2-M5-W9-*.md`（**Boundary verdict with concrete blockers only**）。
> 配套：W8 已集成 `4d7be97 feat(M5): integrate W8 command bridge polish`（bridge.rs +181 行等）；`94e763e fix(M5-W8,A3): close MCP policy phase debt — retire W1 pending, add MCP_NO_RMCP_SERVER + --expect-current-gaps`；A2 W8 笔记 `A2-M5-W8-boundary-review-20260907-1100.md`；A3 W8 笔记 `A3-M5-W8-mcp-policy-phase-debt-20260907-0807.md`。
>
> 承：本树 HEAD `97118d6`，已含 A3 W7 只读 MCP 桥（`6c1f30e`）+ A5 W4/W8 Agent/Skill 只读桥 + W8 集成加固与 A3 MCP 策略债收口。本次为**对已落 W8 修复的再边界复核**，零产品代码。

```text
LANE=A2
STATUS=REVIEW_DONE（docs only，无产品代码；未 push）
BASE=97118d6（W9 当前 HEAD：W8 集成 + A3 MCP 债收口 + W8 patch 证据规范化）
HEAD=logs/assist/A2-M5-W9-boundary-review-20260907-1500.md
FILES=logs/assist/A2-M5-W9-boundary-review-20260907-1500.md（仅本文档）
VERIFY=见 §7 Verify
CHECKPOINT=见 §6 裁决结论（BOUNDARY_PASS，concrete blockers=NONE）+ §5 残留债（F2 non-blocking）+ §4 W8 fix 确认
MERGE_NOTES=本笔记为 review-only；零产品代码改动，无补丁（git diff 为空）。他 lane 的 W8/W9 暂存/修改不在本任务范围，未触碰。
NEXT=见 §8 NEXT
```

## 0. 审查性质声明（重要）

- **W9 = Runtime-Free Polish**：W8 focused validation 已 PASS（board L7：经 A0 privacy/fmt/metrics 修复后）；W9 各 lane 做 runtime-free 打磨与最终验证，在**任何 MCP server / plugin runtime / skill execution wave 之前**。
- A2 本次为**对 A3/A5 W8 修复的再边界复核**，聚焦四问（board L192）：① 无 runtime server/listener；② 无 core tauri 泄漏；③ 仅只读桥；④ 无重复执行路径。
- **零产品代码**；工作树中他 lane 的 W8/W9 暂存与修改不在本任务范围，未触碰。不 commit / 不 push。
- 关键基线变化（vs W8 笔记）：W8 集成 `4d7be97` 改 bridge.rs +181 行（A5 加固=内联 `reject_oversized_def` 有界输入 + 测试）；`94e763e` 收口 MCP 策略债（`MCP_NO_RMCP_SERVER` 单一 ACTIVE 守门 + `--expect-current-gaps`，PENDING 归零）。本次逐一核验这些变化未引入边界泄漏。

## 1. W9 角色与 Hard Stops（board L175-210）

- A2 = **REVIEW ONLY**：再边界复核，只列 concrete blockers 的裁决。
- W9 Hard Stops（L205-209）：① 无 MCP server/listener/rmcp runtime、无 plugin install/enable/delete/download runtime、无 skill/agent execution、无 model call、无 network access；② 新/改命令须保持只读 + source check + ACL + bridge/types + policy/tests 同包原子；③ 无 token/cookie/Authorization/body/prompt-secret 于日志/审计/前端状态/checkpoint/错误串；④ build metrics 阈值 22%、cargo warning 不得增加（超则阻 A0 push）；⑤ 仅 A0 push。
- 本复核对着 Hard Stop ①/② 逐项核验边界。

## 2. Re-review 证据锚点（本树 @ 97118d6）

| 项 | 结果 | 对审查含义 |
|---|---|---|
| core 边界 gate | 默认 `all invariants hold（ACTIVE=7，core 文件=3）`；`--self-test` `PASS(ACTIVE=7, 坏样本=9)` | `core/`=keyring_store.rs/mod.rs/seam.rs，现状干净 |
| `core/` 反向依赖 bin 模块 | grep `use crate::(bridge\|mcp\|skills\|agent\|graph\|plugin\|database\|script_runner\|...)` → **CORE_NO_REVERSE_DEP**（0 命中） | 无 core→bin 反向依赖，core tauri 泄漏=否 |
| bridge.rs 运行时/执行类命令 | grep `skill_run\|agent_exec\|agent_chat\|plugin_install\|plugin_enable\|plugin_delete\|plugin_download\|mcp_serve\|mcp_start\|start_mcp` → **NO_RUNTIME_CMD** | 无 W9 禁止的运行时命令 |
| M5 模块网络/rmcp/tokio server | grep `use rmcp\|rmcp::\|tokio::net\|reqwest\|ureq\|hyper\|std::net::Tcp\|UdpSocket\|TcpListener\|TcpStream` over mcp/skills/agent/plugin/graph → 仅 `mcp.rs:7` 注释（硬停止文档），**无实代码命中** | 无网络/rmcp/tokio server（NETWORK_FOUND 为注释误报） |
| A3 MCP 命令位置 + server-free | `mcp_policy_get`(bridge.rs:6479)/`mcp_registry_list`(6488)/`mcp_capability_preview`(6497)；体仅调 `crate::mcp::{current_policy_snapshot,list_registry_entries,evaluate_mcp_command}`+`allowed_roots`；mcp.rs 顶部明示"纯注册表+纯策略、不 spawn/不起监听/不引 rmcp"；`mcp_server_start`/`rmcp` 仅现于策略脚本**拒绝样例** | MCP 桥 server-free，W9 ① PASS |
| check-mcp-policy.py | 默认 `MCP_POLICY=PASS（无违规）`；`--self-test` `PASS（ACTIVE=8，PENDING=0）` | A3 W8 已收口 W1 pending 债；`MCP_NO_RMCP_SERVER` 单一 ACTIVE 守门 |
| A5 命令体（W8 加固后） | `agent_parse`(6176)/`agent_validate`(6186)/`agent_permission_preview`(6196)/`skill_parse`(6244)/`skill_validate`(6254)/`skill_permission_preview`(6264)；均 `check_invocation_source` + 内联 `*_inner` 纯函数；新增 `reject_oversized_def(text, AGENT_DEF_MAX_BYTES/SKILL_DEF_MAX_BYTES, kind)` 有界输入守卫（L6206/6222/6227/6232/6238/6240） | 仅调 `crate::domain::AgentDef/SkillDef::{parse,validate,permission_preview}`；无 `script_runner`/`skill_run`/`agent_chat`/`SkillExec`/`crate::mcp`/`crate::graph`/`crate::plugin` 引用 |
| ACL 执行类命令 | L103 `script_runs_list`、L114 `task_run_now`（合法 list/now）；无 `skill_run`/`agent_chat`/`plugin_install`/`mcp_server_start` | ACL 无 W9 禁止的运行时命令 |
| security_policy.rs:1140 `IpAddr` | `host_is_loopback_or_private_ip(host)`(L1135-) 用 `use std::net::IpAddr` **纯函数**解析 IP 字符串判回环/私网（fail-closed，DB 连接信号）；非 server/listener | 前次 RUNTIME_FOUND 为误报 |
| grid_process.rs:140 `UnixListener` | 既有 grid 内部 Unix 套接字 IPC（M2/M3 grid worker，非 M5 MCP/plugin/skill runtime） | 非 W9 ① 所禁之 MCP/plugin/skill 运行时；**out-of-scope**，不计入 M5 边界泄漏 |
| 前端 parity（F2 缺口） | `src/bridge.ts:361/363` 仍包装 `skill_list`/`agent_list`，但 `bridge.rs` 无对应 handler（`BACKEND_MISSING_PARITY_GAP`） | 完整性债，**非边界泄漏/非执行路径**（见 §5） |

## 3. 四维边界裁决（W9 四问）

### ① 无 runtime server/listener → **PASS**
- A3 MCP 桥：`mcp.rs` 纯注册表+纯策略，零 rmcp/tokio/listener；`check-mcp-policy.py` PASS(ACTIVE=8, PENDING=0) 含 `MCP_NO_RMCP_SERVER`。
- M5 模块（mcp/skills/agent/plugin/graph）grep 网络/server 仅命中 mcp.rs:7 注释；无 `reqwest`/`ureq`/`hyper`/`TcpListener`/`TcpStream` 实代码。
- `security_policy.rs` 的 `IpAddr` 为纯解析函数；`grid_process` 的 `UnixListener` 为既有内部 IPC，非 M5 运行时。
- **无 MCP server/plugin runtime/skill execution/model call/network access**。

### ② 无 core tauri 泄漏 → **PASS**
- core gate PASS(ACTIVE=7)；`core/` 0 反向依赖 bin 模块（CORE_NO_REVERSE_DEP）。`seam.rs`/`keyring_store.rs`/`mod.rs` 均已物化且 seam-clean。

### ③ 仅只读桥 → **PASS**
- 9 个 M5 命令（agent×3、skill×3、mcp×3）全部 `check_invocation_source` + 纯只读调用（domain parse/validate/permission_preview；mcp current_policy_snapshot/list_registry_entries/evaluate_mcp_command）。无状态变更、无持久化写入、无执行。
- ACL 无执行类命令；新增/改命令保持 source check+ACL+bridge/types+policy/tests 同包（W8 集成已含）。

### ④ 无重复执行路径 → **PASS**
- 9 个只读命令 0 调用 `script_runner`/`skill_run`/`agent_chat`/`SkillExec`。脚本执行唯一入口仍在既有 `script_runner`（start_command/start_run，L3250/3300），与只读桥物理隔离。
- capability 按域单源（MCP↔evaluate_mcp_command；Skill/Agent↔permission_preview；Plugin↔PermissionManifestRule），无跨域 verdict 重复、无第二执行路径。

**四维全 PASS → 边界无 concrete blocker。**

## 4. W8 修复确认（positive，收口 W7/W8 历史发现）

- **A5 W8 有界输入加固（收口 W7 F2「parse 输入无界」）**：W8 集成将 handler 重构为 `*_inner` 纯函数，并新增 `reject_oversized_def(text, AGENT_DEF_MAX_BYTES/SKILL_DEF_MAX_BYTES, kind)` 守卫，覆 `agent_parse`/`skill_parse`/`agent_permission_preview`/`skill_permission_preview`/`skill_validate`（L6206/6222/6227/6232/6238/6240）。`AGENT_DEF_MAX_BYTES`/`SKILL_DEF_MAX_BYTES` 现已强制 → 输入无界风险闭合。
- **A3 W8 MCP 策略债收口（收口 W1 pending 语义）**：`94e763e` 将 W1 的 7 个 PENDING 坏样本收口为单一 ACTIVE 守门 `MCP_NO_RMCP_SERVER` + `--expect-current-gaps`；`check-mcp-policy.py` 现 `PASS(ACTIVE=8, PENDING=0)`。MCP server 红线机器守门更硬。

## 5. 残留债（NON-BLOCKING for boundary，供 A0/A5/A6 跟踪）

- **§F2（仍开，非边界阻塞）skill_list/agent_list parity 缺口**：`src/bridge.ts:361/363` 已包装 `skill_list`/`agent_list`，但 `bridge.rs` 无对应 handler（W8 集成未加，A5 W8 任务为"加固"非"加 list"）。属 completeness 债——不影响"无 server/无 core 泄漏/只读桥/无重复执行"任一维度，故**非边界 blocker**。建议 W9 收尾或后续 wave 二选一闭环：① 实现 `skill_list`/`agent_list` 后端（只读、单一加载器、`allowed_roots` 取路径、atomic ACL/bridge/types/测试、bin-side，且 `AGSK_RO_COMMAND_PARITY` 须过）；或 ② 从 `bridge.ts` 移除悬空 wrapper。
- **§F3（沿用 W7/W8）BIN_ONLY_MODULES 加固建议**：`check-core-boundary.py` 的 `BIN_ONLY_MODULES` 仅 6 项；建议 W9/W10 扩为全部 bin-side 模块（保留 `domain`/`seam`/`keyring_store` 为 core 可见），使 core→bin 反向依赖机器守门。属产品代码改动，由 A0 落地，A2 不实现。

## 6. 裁决结论（VERDICT）

> **BOUNDARY_PASS** —— 四维边界（无 runtime server/listener / 无 core tauri 泄漏 / 仅只读桥 / 无重复执行路径）全部 PASS，**concrete blockers = NONE**。
> W8 A3/A5 修复（MCP 策略债收口 + Agent/Skill 有界输入加固）经复核**未引入任何边界泄漏**；core gate 全绿、core 无反向依赖、ACL 无执行命令、MCP 策略 PASS(ACTIVE=8,PENDING=0)、M5 模块零网络/rmcp/server 实代码。
> 仅剩 1 项 **NON-BLOCKING** 完整性债（§F2 skill_list/agent_list parity），不阻断边界裁决，交 A0/A5/A6 后续闭环。

## 7. Verify（可复跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) core 边界 gate
python3 scripts/check-core-boundary.py            # all invariants hold（ACTIVE=7）
python3 scripts/check-core-boundary.py --self-test  # CORE_POLICY_SELF_TEST=PASS（ACTIVE=7）
# 2) core 无反向依赖 bin 模块
grep -rnE "use crate::(bridge|mcp|skills|agent|graph|plugin|database|script_runner|terminal|grid_process|tools|shutdown|main)" src-tauri/src/core/ && echo LEAK || echo CORE_NO_REVERSE_DEP
# 3) bridge.rs 无运行时/执行命令
grep -nE "pub fn (skill_run|agent_exec|agent_chat|plugin_install|plugin_enable|plugin_delete|plugin_download|mcp_serve|mcp_start)\b" src-tauri/src/bridge.rs && echo RUNTIME_CMD || echo NO_RUNTIME_CMD
# 4) M5 模块无网络/server（忽略注释误报）
grep -rnE "use rmcp|rmcp::|tokio::net|reqwest|ureq|hyper|TcpListener|TcpStream" src-tauri/src/mcp.rs src-tauri/src/skills.rs src-tauri/src/agent.rs src-tauri/src/plugin.rs src-tauri/src/graph.rs | grep -v "//" && echo NETWORK || echo NO_NETWORK_IN_M5_MODULES
# 5) MCP 策略绿
python3 scripts/check-mcp-policy.py            # MCP_POLICY=PASS
python3 scripts/check-mcp-policy.py --self-test  # MCP_POLICY_SELF_TEST=PASS（ACTIVE=8，PENDING=0）
# 6) A5 命令有界输入守卫存在（W8 加固）
grep -nE "reject_oversized_def|AGENT_DEF_MAX_BYTES|SKILL_DEF_MAX_BYTES" src-tauri/src/bridge.rs | head
# 7) 工作树仅本笔记新增（不 commit/push）
git status --short --branch
```

## 8. NEXT

- **整体裁定：BOUNDARY_PASS（concrete blockers = NONE）**。W9 可推进 runtime-free polish 与最终验证（A10/A11），M5 命令边界已稳固，无 runtime server/plugin/skill 扩张。
- 交 A0：① 采纳本 PASS 裁决；② 排期 §F3 `BIN_ONLY_MODULES` 加固（core→bin 反向依赖机器守门）；③ 决议 §F2（`skill_list`/`agent_list` 落地 vs 移除 wrapper）；④ MCP 维持 server-free（`MCP_NO_RMCP_SERVER` 已锁），plugin/skill execution 仍 HOLD。
- A5（如续做）：§F2 若实现 `skill_list`/`agent_list`，须只读、复用 `allowed_roots`+单一加载器、atomic 同包、`AGSK_RO_COMMAND_PARITY` 须过，且不得触 `script_runner`/执行。
- A10/A11（W9 final）：本复核为其提供边界基线——命令边界 PASS、MCP 策略绿、M5 模块零网络/server；安全终评与验证矩阵可直接引用本裁决，无需重复边界核查。
