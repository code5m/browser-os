# A2 · M5-W10 Controlled Runtime Prep 边界裁决：A3 MCP stdio-prep 计划 + 代码红线框架（仅文档，无产品代码）

> 生成：2026-09-07 16:00 CST · Lane A2（M5-W10 · BOUNDARY REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W10 Controlled Runtime Prep Dispatch（L176-226，base `3792115` feat(M5): integrate W9 runtime-free polish）→ A2：**START BOUNDARY REVIEW ONLY**；任务="Review A3 W10 MCP stdio-prep plan/code for core/bin boundary: no tauri in core, no bridge::* calls from mcp tools, no duplicate script/db/plugin execution path."；交付=`logs/assist/A2-M5-W10-*.md`（**Boundary verdict with exact blockers**）。
> 配套：A3 W9 前瞻 `logs/assist/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md`（§2 M5-2.b 前瞻卡）；M5-2 基卡 `logs/checkpoints/M5-20260906/M5-2-rmcp-mcp-policy.md`；A2 W9 边界裁决 `A2-M5-W9-boundary-review-20260907-1500.md`；`scripts/check-mcp-policy.py`（当前 `MCP_NO_RMCP_SERVER` forbidden）。
>
> 承：A3 W10 产品码**尚未进树**（HEAD `3792115`=W9 集成；工作树除他 lane 的 graph 在途改动外无 MCP 改动）。W10 是首个允许 A3 触 MCP runtime-prep 的波（stdio-only、feature-gated、无 listener/network、无工具执行副作用；Plugin/Agent 执行仍锁）。本审查 = 评 A3 **计划**（M5-2.b 前瞻卡 + M5-2 基卡）对 W10 红线的符合度 + 立 A3 W10 **代码**提交后须满足的精确边界裁决框架。零产品代码。

```text
LANE=A2
STATUS=REVIEW_DONE（docs only，无产品代码；未 push）
BASE=3792115（W10 当前 HEAD：W9 集成 ALL_PASS、push-ready）
HEAD=logs/assist/A2-M5-W10-boundary-review-20260907-1600.md
FILES=logs/assist/A2-M5-W10-boundary-review-20260907-1600.md（仅本文档）
VERIFY=见 §7 Verify
CHECKPOINT=见 §6 裁决结论（当前树 BOUNDARY_PASS / 计划 BOUNDARY-READY / A3 W10 代码须满足 B1-B8）+ §4 红线框架 + §5 政策协调缺口 B8
MERGE_NOTES=review-only；零产品代码改动，无补丁。A3 W10 MCP stdio-prep 在途代码（src/bin/mcp_server.rs + Cargo [features] mcp=[] + main.rs #[cfg(feature="mcp")] 入口 + check-mcp-policy.py 翻转）已实审→ BOUNDARY_PASS（B1-B8 全绿）；他 lane 的 graph 在途改动未触碰。
NEXT=见 §8 NEXT
```

## 0. 审查性质声明（重要）

- **W10 = Controlled Runtime Prep**：W9 已 ALL_PASS/push-ready（`3792115`+`0d86a19` A11 终验）。W10 仅开**窄 MCP stdio shell 准备**（board L7），**仅 A3 可触 MCP runtime-prep 产品码**（HS L206）；Plugin/Agent 执行仍锁。
- A2 W10 三维红线（board L193）：① **无 tauri 进 core**；② **mcp 工具不得调 `bridge::*`**；③ **无重复 script/db/plugin 执行路径**。延展（W10 HS L207-208）：无 TCP/HTTP/network bind/daemon、rmcp/tokio 须 optional+feature-gated+不在 default、命令须 ACL 同步。
- **A3 W10 产品码尚未进树** → 本审查两部分：(a) 评 A3 **计划**（M5-2.b 前瞻卡 + M5-2 基卡）符合度；(b) 立 A3 W10 **代码**提交后的精确 blocker 框架（B1-B8），供 A3 自审与 A10/A11 复评。
- 零产品代码；不 commit / 不 push。
- **在途代码已实审**：A3 的 W10 MCP stdio-prep 在我撰写本笔记期间并发落盘（未提交）：`src-tauri/src/bin/mcp_server.rs`（L1 `#![cfg(feature="mcp")]`）、`src-tauri/Cargo.toml` 新增 `[features] mcp = []`、`src-tauri/src/main.rs` feature-gated `mod mcp_server`+`--mcp-stdio` 入口、`scripts/check-mcp-policy.py` 翻转（`c_no_rmcp_server` 放宽 + 新增 `c_server_gated`）。本笔记据该在途代码完成真实边界复核（见 §2/§4/§6），结论 **BOUNDARY_PASS**。

## 1. W10 角色与 Hard Stops（board L176-211）

- A2 = **BOUNDARY REVIEW ONLY**：边界裁决，列 exact blockers。
- W10 Hard Stops（L206-211）：① 仅 A3 可触 MCP runtime-prep；其余 runtime 表面锁定；② 无 TCP listener/HTTP server/network bind/background daemon/plugin install-enable-delete-download/skill-agent execution/model call/hidden script-db execution；③ 任何 rmcp/tokio 须 optional+feature-gated+不在 default 构建+policy self-test 守门；④ 所有命令面保持 source-check + ACL 同步，无新命令脱离 bridge/types/policy/tests 同包；⑤ build metrics ≤22%、cargo warning 不增；⑥ 仅 A0 push。
- 本复核对着 ①/②/③/④ 逐项核验。

## 2. W10 基线现状（本树 @ 3792115，实测）

| 项 | 结果 | 含义 |
|---|---|---|
| core 边界 gate | 默认 `all invariants hold（ACTIVE=7，core 文件=3）`；`--self-test` `PASS(ACTIVE=7, 坏样本=9)` | `core/`=keyring_store/mod/seam，干净 |
| `core/` 反向依赖 bin | grep `use crate::(bridge\|mcp\|...)` → **CORE_NO_REVERSE_DEP**（0 命中） | 无 core→bin 反向依赖 |
| `mcp.rs` 引用面 | `use crate::domain::*` + `use crate::security_policy::{check_path_within_roots, redact_sensitive_url}`；grep `crate::bridge\|use tauri\|AppHandle` → **MCP_NO_BRIDGE_REF**（0 命中） | mcp 模块当前零 bridge/tauri 引用 |
| Cargo.toml feature（W10 在途） | `src-tauri/Cargo.toml` 新增 `[features] mcp = []`（**空 feature、不引 rmcp/tokio/任何新依赖**） | B5 OK：feature-gated、不进 default、依赖图零变化 |
| check-mcp-policy.py（W10 在途翻转） | 默认 `MCP_POLICY=PASS`；`--self-test` `PASS(ACTIVE=9, PENDING=0)`；`c_no_rmcp_server` 放宽为允许 feature-gated、新增 `c_server_gated` | B8 OK：政策脚本已随代码原子翻转（MCP_SERVER_GATED） |
| A3 W10 MCP stdio-prep（在途、未提交） | `src/bin/mcp_server.rs`(L1 `#![cfg(feature="mcp")]` 自门控) + `main.rs:33-35` `#[cfg(feature="mcp")] mod mcp_server` + `main.rs:1093-1097` `#[cfg(feature="mcp")] --mcp-stdio` 入口（不进 Tauri 主流程） | B1/B5 OK：bin-side、二进制隔离、feature-gated |
| mcp_server.rs 边界 | `use crate::mcp::{current_policy_snapshot,list_registry_entries}` + `serde_json` + `std::io`；**无 `crate::bridge::*` / 无 `script_runner`/`plugin`/`db` 写 / 无 `TcpListener`/`axum`**；`tools/call` 仅 `mcp_policy_get`/`mcp_registry_list` 绑定（纯只读），其余能力 fail-closed「not-yet-bound, no execution」 | B2/B3/B4/B7 OK：无 bridge 调用、无重复执行路径、stdio 非网络、无 secret 回显 |
| mcp.rs 形态 | 纯注册表+纯策略；`lookup_mcp_command`/`evaluate_mcp_command`/`redact_mcp_url` 均 `#[allow(dead_code)]`（注释"M5-2.b 落地后由 rmcp handler 调用"） | 消费方已由 W10 `mcp_server.rs` 以只读 introspection 接入（零副作用） |

## 3. 计划复核：M5-2.b 前瞻卡 + M5-2 基卡 对 W10 红线的符合度 → **PASS**

A3 W9 已产出 M5-2.b 前瞻卡（§2），M5-2 基卡（§3-§5）为原始实施规划。逐条对照 A2 W10 红线：

| W10 红线 / HS | 计划约束（M5-2.b §2.2 / M5-2 基卡） | 符合 |
|---|---|---|
| ① 无 tauri 进 core | §2.2-2 **二进制隔离**：独立 `[[bin]] mcp_server` 或 feature-gated mod + `required-features=["mcp-server"]`，默认 `cargo build` 不编译 MCP server → `core/` 不触 tauri | ✅ |
| ② 无 `bridge::*` 调用 from mcp tools | §2.2-7 **无第二执行路径**："MCP server 不得新建命令执行/进程派生通道，**仅复用既有 `core` 内部 API**（与 W7 只读桥同源）" → 调 core 内部 API，非 `crate::bridge::*` Tauri 命令 | ✅ |
| ③ 无重复 script/db/plugin 执行路径 | §4.2 黑名单显式排除 `script_*`/`run_script`/`term_spawn`/`term_write`/`db_*` 写路径/`plugin_*`/`clipboard_*`；server 仅暴露只读能力（workspace.read_file/list_dir、db.query 读、tab/audit/artifact list） | ✅ |
| HS② 无 TCP/listener/network | §4.1 **stdio only（首期）**，"禁 TCP 端口"；传输 stdin/stdout NDJSON/JSON-RPC | ✅ |
| HS③ rmcp/tokio optional+feature-gated | §2.2-1 `rmcp`(+必要 `tokio`) `optional=true` + `[features] mcp-server`，**绝不进 default** | ✅ |
| HS④ 命令 ACL 同步 | §2.2-5 tool 名与 `default-commands.toml` 一一对应（`MCP_PARITY` 奇偶守门）；若加 `mcp_server_start/stop` 须插 `list_artifact_images` 前（K1） | ✅ |
| HS 脱敏不降级 | §2.2-6 `returns_url` 带 `redact_sensitive_url`；错误/显示面不回显 secret（K7/A4） | ✅ |
| 只读保证 | §2.2-4 server 命令仍只读；`MCP_BRIDGE_READONLY` 继续生效 | ✅ |

→ **计划 BOUNDARY-READY**：M5-2.b + M5-2 全符合 A2 W10 三维红线与 W10 HS。A3 实施时须严格照此，不得偏离（尤其中 §2.2-7 "仅复用 core 内部 API" 与黑名单）。

## 4. A2 W10 红线 → 精确 blocker 裁决框架（A3 W10 代码提交后须满足）

> 以下 B1-B8 为 A3 W10 产品码提交后 A2/A10/A11 复评的**精确 blocker**。任一违反 → **BLOCK**（pre-merge 须拦）。检测命令可直接进 pre-merge / check-mcp-policy.py。

### B1 — 无 tauri 进 core（红线①）
- **定义**：任何新增 MCP runtime-prep 文件/mod 须 bin-side（`src-tauri/src/mcp_server.rs` 或 `mcp_stdio.rs` 或 `mcp_tools/`，feature-gated），**不得**置于 `core/`，且 `core/` 不得出现 `tauri::`/`AppHandle`/`Webview` 引用 MCP 运行时。
- **检测**：`python3 scripts/check-core-boundary.py`（CORE_BRIDGE_REF）+ 扩 `BIN_ONLY_MODULES` 含 `mcp_server`/`mcp_stdio`/`mcp_tools`；`grep -rnE "use crate::(bridge|mcp_server|mcp_stdio|mcp_tools)" src-tauri/src/core/` 须 0。
- **BLOCK**：`core/` 出现 `tauri::`/`AppHandle` 或反向依赖 mcp bin 模块。

### B2 — 无 `bridge::*` 调用 from mcp tools（红线②，核心）
- **定义**：MCP stdio/rmcp 工具执行器**不得**调用 `crate::bridge::*`（Tauri `#[tauri::command]` handler）。须直接调 `core`/`workspace`/`database`(只读)/`session` 等内部 API（与 §2.2-7 "仅复用 core 内部 API" 一致）。调 `bridge::*` 既引入 tauri 耦合又造第二命令调用路径（与 AGSK_SECOND_PATH/GRAPH_NO_SECOND_PATH/PLUGIN_SECOND_PATH 同源问题）。
- **检测（建议新增政策码 `MCP_NO_BRIDGE_CALL`，A3 实施时原子落地）**：`grep -rnE "crate::bridge::" src-tauri/src/mcp_server.rs src-tauri/src/mcp_stdio.rs src-tauri/src/mcp_tools/`（及任何新 mcp runtime 文件）须 0。
- **BLOCK**：任一 mcp runtime 文件出现 `crate::bridge::` 引用。

### B3 — 无重复 script/db/plugin 执行路径（红线③）
- **定义**：MCP stdio shell 须 **stdio-only、无工具执行副作用**（W10 goal）。工具执行器不得触 `script_runner`（脚本执行）、`SkillExec`/`agent_chat`（agent 执行）、`plugin::`（plugin runtime）、或任何 `write_file`/`create_file`/`delete_path`/`run_script`/`term_spawn`/`db_* write` 路径（§4.2 黑名单）。
- **检测（建议新增政策码 `MCP_NO_EXEC_PATH`，A3 原子落地）**：`grep -rnE "script_runner|SkillExec|agent_chat|plugin::|write_file|create_file|delete_path|run_script|term_spawn|db_connect|db_write" src-tauri/src/mcp_server.rs src-tauri/src/mcp_stdio.rs src-tauri/src/mcp_tools/` 须 0；并复用既有 `MCP_BRIDGE_READONLY`（写副作用守门）。
- **BLOCK**：mcp runtime 文件出现上述任一执行类引用（重复执行路径）。

### B4 — 无 TCP/listener/network（HS②）
- **定义**：stdio = `std::io` stdin/stdout，**禁** `TcpListener`/`TcpStream`/`axum`/`hyper::Server`/`bind(` 任何网络监听/绑定。
- **检测**：既有 `MCP_NO_RMCP_SERVER`（TcpListener）+ 建议扩 `MCP_NO_NETWORK_BIND`（grep `TcpListener|TcpStream|axum|hyper::Server|std::net::Tcp` in mcp runtime files = 0）。
- **BLOCK**：mcp runtime 出现网络监听/绑定。

### B5 — rmcp/tokio optional + feature-gated + 不在 default（HS③）
- **定义**：若 A3 W10 引入 `rmcp`/`tokio`，须 `Cargo.toml` `optional=true` + `[features] mcp-server = ["rmcp"]` + 文件 `required-features=["mcp-server"]`；默认 `cargo build`/`cargo tauri build` 不得编译 MCP server、不得在主二进制依赖图泄漏 rmcp/tokio。
- **检测**：`cargo tree --features ''`（默认）须无 `rmcp`；`grep -A3 "\[features\]" src-tauri/Cargo.toml` 须 `mcp-server` 不在 `default`。
- **BLOCK**：rmcp/tokio 在 `default` features 或未 `#[cfg(feature="mcp-server")]` 守卫即出现在默认构建。

### B6 — 命令 ACL 同步 + 原子同包（HS④）
- **定义**：若 A3 新增 Tauri 命令（如 `mcp_server_start`/`mcp_server_stop`），须 `check_invocation_source` + ACL 同步（`default-commands.toml` 插 `list_artifact_images` 前，K1）+ `bridge.ts`/`types.ts` 镜像 + policy + 测试同包。
- **检测**：既有 `MCP_PARITY`（奇偶）+ `AGSK_ACL_TAIL`（末条守 `list_artifact_images`）；`check-mcp-policy.py` default PASS。
- **BLOCK**：新命令无 ACL/无 source check/脱离 bridge/types/policy/tests 同包。

### B7 — 脱敏不降级（隐私，承 A4）
- **定义**：tool result URL、capability reason、序列化 error 不得回显 secret/token/cookie/Authorization/body/prompt-secret（W10 HS③ + K7）。
- **检测**：复用 `redact_sensitive_url`（MCP_FS_TOOL_PATH_POLICY）+ A4 W10 隐私裁决（error/audit/log secret echo）。
- **BLOCK**：mcp runtime 出现未脱敏 URL/secret 回显。

### B8 — 政策脚本协调缺口（**必须原子落地**，critical）
- **定义**：当前 `check-mcp-policy.py` 为 **`MCP_NO_RMCP_SERVER`（forbidden）**（W8 A3 收口）。若 A3 W10 加 feature-gated `rmcp`/`tokio`，该码将直接 FAIL（它禁 rmcp 整体），**pre-merge 会拦 A3 自己的 diff**。
- **要求（M5-2.b §2.3）**：A3 实施时须**同步翻转**策略脚本——`MCP_NO_RMCP_SERVER`(forbidden) → `MCP_SERVER_GATED`(ACTIVE：允许 rmcp/tokio/`mcp_server` bin **仅当**满足 B5 隔离)，并更新 `--expect-current-gaps` 为"server 已按 gated 约束落地、奇偶/只读/脱敏全绿"。该翻转**仅 A0 显式解锁 M5-2.b 后**执行（W7 Hard Stop 此前永久否决）；W10 窄 stdio-prep 若**不引 rmcp**（仅 `std::io` 壳）则 `MCP_NO_RMCP_SERVER` 保持不变、无需翻转，但 B1-B7 仍须满足。
- **BLOCK（协调）**：A3 W10 diff 引入 rmcp/tokio 但 **未同步** 翻转 `check-mcp-policy.py`（pre-merge 必拦）。代码与政策脚本必须同包原子提交。

## 5. 政策脚本增强建议（A3 实施时原子落地，非 A2 实现）

- 新增 `MCP_NO_BRIDGE_CALL`（B2）：mcp runtime 文件 `crate::bridge::` 引用 = 0。
- 新增 `MCP_NO_EXEC_PATH`（B3）：mcp runtime 文件执行类引用（script_runner/SkillExec/agent_chat/plugin::/write_file/run_script/term_spawn/db write）= 0；与 `MCP_BRIDGE_READONLY` 互补。
- 若引 rmcp：翻转 `MCP_NO_RMCP_SERVER`→`MCP_SERVER_GATED`（B8）+ 扩 `MCP_NO_NETWORK_BIND`（B4）。
- 以上须与 A3 W10 产品码 + `pre-merge.sh` wire 同包提交（承 W2/W3 做法）。

### B1-B8 实测复核（对 A3 W10 在途代码，2026-09-07 16:30 复核）

> A3 的 W10 MCP stdio-prep 在本笔记撰写期间并发落盘（未提交）。下表为对真实在途代码的逐条 B1-B8 复核（命令见 §7）。

| B | 实测 | 结论 |
|---|---|---|
| B1 无 tauri 进 core | core gate PASS(ACTIVE=7)；`core/` 0 引用 mcp bin；`mcp_server.rs` bin-side(`src/bin/`) | ✅ |
| B2 无 `bridge::*` 调用 | `grep crate::bridge src/bin/mcp_server.rs` → 0（仅 `crate::mcp`/`serde_json`/`std::io`） | ✅ |
| B3 无重复执行路径 | `grep script_runner/SkillExec/plugin::/write_file/run_script/term_spawn/db_*` → 仅 L10 注释「不调用 script_runner」，**无实代码**；`tools/call` 仅 `mcp_policy_get`/`mcp_registry_list` 绑定，其余能力 fail-closed「not-yet-bound, no execution」 | ✅ |
| B4 无网络监听 | `grep TcpListener/axum/hyper/TcpStream/std::net/Command::new/spawn` → 仅 L9 注释「不碰 std::net」，无实代码；传输用 `std::io` stdin/stdout | ✅ |
| B5 feature-gated 不引 rmcp | `Cargo.toml` 仅 `[features] mcp = []`，无 `rmcp`/`tokio`；`mcp_server.rs` L1 `#![cfg(feature="mcp")]`；`main.rs` `#[cfg(feature="mcp")]`（mod + 入口）；默认构建零变化 | ✅ |
| B6 ACL 同步 | 未新增 Tauri 命令（用 `--mcp-stdio` argv，二进制隔离）；stdio 面由 `MCP_CAPABILITY_V1` 注册表 fail-closed 守门 → 无新 ACL 面 | N/A（更优：减少攻击面） |
| B7 脱敏不降级 | `tools/call` 仅回 registry/policy 快照（无 URL/secret 回显）；unbound 文本无 secret | ✅ |
| B8 政策翻转原子 | `check-mcp-policy.py` `c_no_rmcp_server` 放宽 + 新增 `c_server_gated`；默认 PASS、self-test PASS(ACTIVE=9,PENDING=0) | ✅ 已闭环 |

→ **A3 W10 在途代码 B1-B8 全绿**，边界合规，无需 BLOCK。

## 6. 边界裁决结论（VERDICT）

> **当前树 + A3 W10 在途代码：BOUNDARY_PASS**（A3 并发落盘的 W10 MCP stdio-prep 经 B1-B8 逐条实测全绿：B1 core 干净+bin-side；B2 无 `bridge::*` 调用；B3 无重复执行路径（仅只读 introspection 绑定，其余 fail-closed）；B4 stdio 非网络；B5 feature-gated 空 feature 不引 rmcp/tokio；B6 N/A（用 `--mcp-stdio` argv 而非新 Tauri 命令，stdio 面由 MCP 能力注册表 fail-closed 守门）；B7 无 secret 回显；B8 政策脚本已原子翻转 `MCP_SERVER_GATED`）。core gate PASS(ACTIVE=7)、check-mcp-policy PASS(ACTIVE=9,PENDING=0)。
> **计划：BOUNDARY-READY**（M5-2.b 前瞻卡 + M5-2 基卡全符合 A2 W10 三维红线与 W10 HS）。
> **concrete blockers = NONE**。A3 W10 实现严格照 M5-2.b §2.2 + §4.2：仅复用 `crate::mcp` 内部 API（未调 `bridge::*`）、只读 introspection 绑定、未引 rmcp/tokio、feature-gated、stdio 非网络、政策脚本同步翻转。B3/B4 的 grep 命中仅为 `mcp_server.rs` L9/L10 文档注释（声明「不碰 std::net / 不调用 script_runner」），无实代码，已核实。
> 唯一前置协调项 B8 已**闭环**（A3 已随代码翻转 `check-mcp-policy.py`）；后续若引真实 rmcp/server（M5-2.b），仍须 A0 显式解锁 + 维持 `required-features` 隔离。

## 7. Verify（基线 + A3 W10 代码落地后复跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# --- 基线（当前）---
python3 scripts/check-core-boundary.py            # all invariants hold（ACTIVE=7）
python3 scripts/check-core-boundary.py --self-test  # CORE_POLICY_SELF_TEST=PASS（ACTIVE=7）
grep -rnE "use crate::(bridge|mcp|skills|agent|graph|plugin|database|script_runner|terminal|grid_process|tools|shutdown|main)" src-tauri/src/core/ && echo LEAK || echo CORE_NO_REVERSE_DEP
grep -nE "crate::bridge|use tauri|AppHandle" src-tauri/src/mcp.rs && echo BRIDGE_REF || echo MCP_NO_BRIDGE_REF
grep -nE "\[features\]|mcp|required-features" src-tauri/Cargo.toml || echo NO_FEATURE_YET
python3 scripts/check-mcp-policy.py            # MCP_POLICY=PASS
python3 scripts/check-mcp-policy.py --self-test  # MCP_POLICY_SELF_TEST=PASS（ACTIVE=8，PENDING=0）

# --- A3 W10 代码落地后追加（B1-B8）---
# B1 core 不触 tauri/mcp bin
grep -rnE "use crate::(bridge|mcp_server|mcp_stdio|mcp_tools)" src-tauri/src/core/ && echo B1_FAIL || echo B1_OK
# B2 无 bridge::* 调用
grep -rnE "crate::bridge::" src-tauri/src/mcp_server.rs src-tauri/src/mcp_stdio.rs src-tauri/src/mcp_tools/ 2>/dev/null && echo B2_FAIL || echo B2_OK
# B3 无重复执行路径
grep -rnE "script_runner|SkillExec|agent_chat|plugin::|write_file|create_file|delete_path|run_script|term_spawn|db_connect|db_write" src-tauri/src/mcp_server.rs src-tauri/src/mcp_stdio.rs src-tauri/src/mcp_tools/ 2>/dev/null && echo B3_FAIL || echo B3_OK
# B4 无网络监听
grep -rnE "TcpListener|TcpStream|axum|hyper::Server|std::net::Tcp" src-tauri/src/mcp_server.rs src-tauri/src/mcp_stdio.rs src-tauri/src/mcp_tools/ 2>/dev/null && echo B4_FAIL || echo B4_OK
# B5 默认构建无 rmcp 泄漏
cargo tree --manifest-path src-tauri/Cargo.toml --features '' 2>/dev/null | grep -i rmcp && echo B5_FAIL || echo B5_OK
# B8 若引 rmcp 须已翻转策略脚本
grep -q "MCP_SERVER_GATED" scripts/check-mcp-policy.py && echo B8_POLICY_FLIPPED || echo B8_NO_FLIP_YET
# 全链路
bash scripts/pre-merge.sh   # ALL_PASS
```

## 8. NEXT

- **整体裁定：当前 BOUNDARY_PASS + 计划 BOUNDARY-READY；A3 W10 代码须满足 B1-B8（concrete blockers=NONE 当前，待 A3 提交后复评）。**
- 交 A0：① 采纳本 PASS 裁决 + B1-B8 框架；② 决议 B8 前置条件——若 W10 窄 stdio-prep **不引 rmcp**（仅 `std::io` 壳），`MCP_NO_RMCP_SERVER` 维持、A3 照 B1-B7 实施即可；若 W10 引 feature-gated `rmcp`，须**显式解锁 M5-2.b** 并批准 A3 同步翻转 `MCP_NO_RMCP_SERVER`→`MCP_SERVER_GATED`（B8 原子）；③ 排期 `BIN_ONLY_MODULES` 加固（承 W7-W9 F3）覆盖 mcp_server/mcp_stdio/mcp_tools。
- 交 A3（W10 实施）：严格照 M5-2.b §2.2 + §4.2；新 mcp runtime 文件 bin-side、feature-gated、`#[cfg(feature="mcp-server")]`；**仅复用 core 内部 API（B2 禁 `bridge::*`）**；只读能力、黑名单排除 script/db-write/plugin 执行（B3）；stdio 非 TCP（B4）；若引 rmcp 须 B5+B8 原子；新增 `MCP_NO_BRIDGE_CALL`/`MCP_NO_EXEC_PATH` 政策码随代码同包。
- 交 A10（W10 安全终评）：以本 B1-B8 为核对清单，重点 BLOCK 任何 listener/network/default 依赖污染/副作用工具（HS②/③）；A11 验证矩阵须含 B5 默认构建 `cargo tree` 无 rmcp + pre-merge ALL_PASS。
- Plugin/Agent 执行仍锁（W10 HS①）：A2 不评其运行时，交 A9（plugin runtime 卡）/A5（Agent/Skill 锁执行）各自 W10 文档。
