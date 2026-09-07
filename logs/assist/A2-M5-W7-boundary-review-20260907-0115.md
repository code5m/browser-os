# A2 · M5-W7 边界审查笔记：A3 MCP 只读命令桥 / A5 Agent·Skill 只读命令桥 的 core boundary 与「重复执行路径」方向（仅文档，无产品代码）

> 生成：2026-09-07 01:15 CST · Lane A2（M5-W7 · SUPPORT/REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W7 Integration Dispatch（L137-171，base `a26fbaf` docs(M5): dispatch W7 read-only command bridge lanes）→ A2：**SUPPORT/REVIEW ONLY**；任务="Review A3/A5 command bridges for core boundary leaks and duplicate execution paths."；交付=`logs/assist/A2-M5-W7-*.md` 边界笔记。
> 配套：A1 W7 reconcile；A3 M5-2（mcp.rs/bridge.rs）；A5 M5-4/5（agent.rs/skills.rs）；A9 W6（plugin.rs/security_policy.rs）；`core/seam.rs`；`check-mcp-policy.py`/`check-agent-skill-policy.py`；A2 已集成之 W6 笔记 `A2-M5-W6-boundary-review-20260906-2030.md`。
>
> 承：本树 `5f92ece feat(M5): add graph UI and plugin policy slices`（A8/A9 W6 产品码已由 A0 集成）；`a26fbaf` 为 W7 dispatch。

```text
LANE=A2
STATUS=REVIEW_DONE（docs only，无产品代码；未 push）
BASE=a26fbaf（W7 dispatch）
HEAD=logs/assist/A2-M5-W7-boundary-review-20260907-0115.md
FILES=logs/assist/A2-M5-W7-boundary-review-20260907-0115.md（仅本文档）
VERIFY=见 §9 Verify
CHECKPOINT=本文件
MERGE_NOTES=见 §6 Findings（F1–F9）+ §7 gate 建议 + §8 seam 结论
NEXT=见 §10 NEXT
```

## 0. 审查性质声明（重要）

- A3/A5 的 W7 **产品代码尚未进入 canonical 树**（HEAD `a26fbaf` 仅是 W7 dispatch 文档提交；只有 A0 可 push 集成，A3/A5 在各自 worktree，由 A0 拣入）。故本笔记是**实施前（pre-implementation）边界审查**，基于：
  - W7 dispatch 任务定义与硬约束（board §M5-W7，L137-171）
  - A3 既有 W3 已落地 MCP registry/policy（`mcp.rs` `12f1cff`→`MCP_COMMAND_REGISTRY`+`evaluate_mcp_command`+`redact_mcp_url`+`current_policy_snapshot`，server-free）
  - A5 既有 W4 已落地 Agent/Skill domain（`agent.rs`/`skills.rs` `1610939`→`AgentDef/SkillDef::parse/validate/permission_preview` + `SkillExec` 纯结构）
  - `core/seam.rs` 已物化（`ProgressSink`/`PathResolver`/`RootsProvider`，`agent_memory.rs:27,583` 已采用 `&dyn PathResolver`）
  - 既有策略脚本 `check-mcp-policy.py`/`check-agent-skill-policy.py` 已含 `MCP_RUNTIME_LEAK`/`MCP_CAPABILITY_DRIFT`/`AGSK_SECOND_PATH`/`AGSK_CAPABILITY_DRIFT`/`AGSK_ACL_TAIL` 等码
  - A9 W6 已加 `PLUGIN_CAPABILITY_V1`+`PermissionManifestRule`（security_policy.rs/plugin.rs，承本 A2 W6 §F4 裁定）
- 目的：为 A3/A5 划清 **core boundary 泄漏** 与 **重复执行路径** 红线，并给出 **seam 方向**（路径解析注入 `&dyn PathResolver`/`&dyn RootsProvider`）。当 A3/A5 代码经 A0 集成后，A0/A10 可凭本笔记 §6 红线清单做二次复核。
- 性质同本 A2 W6 笔记（已集成于 `5f92ece`）：本波仍是 review-only，零产品代码改动。

## 1. W7 角色与硬约束（来自 board §M5-W7）

- **只有 A3 与 A5 可在 W7 写产品代码**；其余 lane（A1/A2/A4/A6/A7/A8/A9/A10/A11）为 docs/review/support。A2 仅审查。
- W7 Hard Stops（L165-171）：
  - W7 命令**只读**：no skill execution / no plugin install·enable·disable·uninstall / no MCP server·listener / no graph rebuild worker。
  - 每个新命令须 **atomic**：source check + ACL + 前端 bridge/types + policy 覆盖 + tests 同包落地。
  - 无 token/cookie/Authorization/body/prompt-secret 日志或持久化。
  - 各 lane 先 `git pull` 自 `origin/master` 且**不得 push**。
- A3 允许范围（L155）：`mcp.rs`/`bridge.rs`/`main.rs`/`default-commands.toml`/`bridge.ts`/`types.ts`/`scripts/check-mcp-policy.py` + 测试/checkpoint。可暴露只读 MCP registry/policy 命令（list registry、preview capability verdict、return redacted DTO）。**无 rmcp/server/listener**。
- A5 允许范围（L157）：`agent.rs`/`skills.rs`/`bridge.rs`/`main.rs`/`default-commands.toml`/`bridge.ts`/`types.ts`/`scripts/check-agent-skill-policy.py` + 测试/checkpoint。可暴露只读 Agent/Skill 命令（parse/validate/permission preview）。**无 skill 执行 / 无 install / 无网络 / 无持久化写**。

## 2. 实测证据锚点（本树）

| 项 | 结果 | 对审查的含义 |
|---|---|---|
| core 边界 gate | 默认 `all invariants hold（ACTIVE=7，core 文件=3）`；`--self-test` `PASS(ACTIVE=7, 坏样本=9)` | 现状干净（`core/`=keyring_store.rs/mod.rs/seam.rs）。但 `BIN_ONLY_MODULES` 仅 6 项（bridge/main/terminal/grid_process/tools/shutdown），未覆盖 mcp/agent/skills/plugin/graph 等 → core→bin 反向依赖**漏检**（见 §F6） |
| `bridge.rs` 是否已注册 skill/agent/mcp 命令 | grep `skill_list\|agent_list\|mcp_` **0 命中**（仅 `#[tauri::command]` 行号） | A6 W5 `bridge.ts` 已包装 `skill_list`/`agent_list`/`skill_install`/`skill_run` 等，但**后端命令尚未落地** → A5 W7 正是补这些只读后端命令（A3 补 mcp 只读命令） |
| ACL 现有 skill/agent/mcp 条目 | `default-commands.toml` grep `skill\|agent\|mcp` **0 命中** | A3/A5 加命令须 **atomic 扩 ACL**（当前无任何 skill/agent/mcp 条目）；末条仍 `list_artifact_images`（AGSK_ACL_TAIL 守） |
| mcp.rs 状态 | `MCP_COMMAND_REGISTRY`(L22) / `lookup_mcp_command`(L70) / `evaluate_mcp_command`(L84) / `redact_mcp_url`(L105) / `current_policy_snapshot`(L111)；注释明示 server-free（无 rmcp/tokio/TcpListener） | A3 W7 只读命令须**暴露既有函数**，不得重实现 registry/verdict/redactor；不得起 server（MCP_RUNTIME_LEAK/MCP_LISTEN_PORT 守） |
| skills.rs / agent.rs | `SkillDef/AgentDef::parse`(L17/L12) / `validate`(L22/L17) / `permission_preview`(L45/L40，返回 `PermissionPreview{gate,capabilities}`)；`SkillExec`(L9/L53) 为纯结构，`validate`(L56) 仅递归校验无内联 shell | A5 W7 只读命令只调 **parse/validate/permission_preview（纯）**；**禁触 `SkillExec` 执行体 / `skill_run` / `script_runner` / `agent_chat`**（重复执行路径红线） |
| seam 已物化 | `core/seam.rs` 含 `ProgressSink`/`PathResolver`/`RootsProvider` trait；`agent_memory.rs:27,583` 已 `use mvp_core::core::seam::PathResolver` + `agent_kv_default_path(resolver: &dyn PathResolver)`；`bridge.rs:1155/1170` 定义 `TauriPathResolver`/`TauriRootsProvider` | A3/A5 路径解析**标准模式**：纯函数吃 `&dyn PathResolver`/`&dyn RootsProvider`，bridge handler 注入 `TauriPathResolver`/`TauriRootsProvider`（见 §F8） |
| 无既有 skill/agent 磁盘加载器 | grep `load_skills\|SkillStore\|agents_dir\|read_skill` **0 命中** | A5 W7 需建 `skill_list`/`agent_list` 加载器；**须为单一加载器**，被未来 `skill_run`/`agent_chat` 复用，否则成重复执行路径（§F2） |
| 策略脚本已就位 | `check-mcp-policy.py`(ACTIVE: MCP_BIN_GATED/MCP_CAPABILITY_DRIFT/MCP_COMMAND_REGISTRY/MCP_FS_TOOL_PATH_POLICY/MCP_LISTEN_PORT/MCP_NODE_RUNTIME_PRESENT/MCP_NPM_IN_CARGO/MCP_NPM_SDK_PRESENT/MCP_OPTIONAL_DEP/MCP_PARITY/MCP_PATH_POLICY_MISSING/MCP_RUNTIME_LEAK/MCP_TOOL_CALLS_COMMAND/MCP_TREE_TAURI/MCP_URL_NOT_REDACTED) / `check-agent-skill-policy.py`(ACTIVE: AGSK_ACL_TAIL/AGSK_CAPABILITY_DRIFT/AGSK_COMMAND_PARITY/AGSK_INLINE_SHELL/AGSK_INLINE_SHELL_ENUM/AGSK_INLINE_SHELL_REF/AGSK_SECOND_PATH) | A3/A5 W7 须**扩展**既有脚本（非新建），加只读桥码（§F1/§F7） |

## 3. A5（Agent/Skill 只读命令桥）core boundary & seam 方向

**边界定位**：后端（`agent.rs`/`skills.rs` 增纯函数或薄封装、`bridge.rs` 增 `#[tauri::command]` handler、`default-commands.toml` 增 ACL、`bridge.ts`/`types.ts` 增前端包装）。均 bin-side，**不经 `core/`**。

**seam 方向（后端层）**：
- `skill_list`/`agent_list` 的磁盘读取必须注入 `&dyn PathResolver`/`&dyn RootsProvider`（来自 `core/seam.rs`），**复用 `TauriPathResolver`/`TauriRootsProvider`**（`bridge.rs:1155/1170`）——与 `agent_memory.rs:583 agent_kv_default_path(resolver: &dyn PathResolver)` 同款。纯解析/校验/preview 函数（`SkillDef::parse`/`validate`/`permission_preview`）吃 `&str`/DTO，本身 seam-free，无需改动。
- 新建的 **skill/agent 加载器**应设计为"单一加载器"：`load_skill_defs(resolver: &dyn PathResolver) -> Vec<Result<SkillDef,String>>`（agent 同构），放在 `skills.rs`/`agent.rs`（bin-side，可后续随切片搬 core，届时保持 `&dyn PathResolver` 签名即可无缝搬入）。**该加载器即未来 `skill_run`/`agent_chat` 的唯一来源**，W7 只读与 W8+ 执行共用，杜绝分叉。

**红线（A5 必须遵守）**：见 §6（F2/F3/F5/F7/F8/F9）。

## 4. A3（MCP 只读命令桥）core boundary & seam 方向

**边界定位**：后端（`mcp.rs` 增薄封装或复用既有函数、`bridge.rs` 增 `#[tauri::command]` handler、`default-commands.toml` 增 ACL、`bridge.ts`/`types.ts` 增前端包装）。均 bin-side。

**seam 方向（后端层）**：
- `mcp_capability_preview(capability, raw_path)` handler 须调既有 `evaluate_mcp_command(capability, raw_path, roots)`——该函数 L84 **已吃 `roots: &[PathBuf]`**，即 seam 注入点。handler 从 `TauriRootsProvider` 取 roots 传入即可，无需内联 `app.path()`。
- `mcp_policy_get` 返回既有 `current_policy_snapshot()`（L111，含 `MCP_CAPABILITY_V1` + version）；`mcp_registry_list` 返回 `MCP_COMMAND_REGISTRY`（L22）；URL 字段经 `redact_mcp_url`（L105，单一脱敏真源）后再出参。

**红线（A3 必须遵守）**：见 §6（F1/F4/F5/F7/F8/F9）。核心：**不得重实现 registry/verdict/redactor，不得起 rmcp server/listener**（MCP_RUNTIME_LEAK/MCP_LISTEN_PORT 守）。

## 5. 跨 lane（A3 ↔ A5）重复执行路径 / capability 单一真源

- **capability 裁决按域单一真源**（承本 A2 W6 §F4）：
  - MCP → `evaluate_mcp_command`（mcp.rs，查 `MCP_CAPABILITY_V1`）
  - Skill/Agent → `permission_preview`（skills.rs/agent.rs，查 `security_policy.rs` 的 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`）
  - Plugin → `PermissionManifestRule(&CapabilityRegistry)`（plugin.rs，查 `security_policy.rs` 的 `PLUGIN_CAPABILITY_V1`，A9 W6 加）
- **红线**：A3 的 "preview capability verdicts" **必须限定 MCP 域**（调 `evaluate_mcp_command`），**不得**实现跨域通用 capability checker（否则与 A5/A9 的 verdict 逻辑重复 = 重复执行路径/真源漂移）。A5 的 "permission preview" **必须限定 Skill/Agent 域**（调 `permission_preview`），**不得**碰 MCP/plugin。两 lane 各自守域，互不重造 verdict。
- 既有 `check-agent-skill-policy.py` 的 `AGSK_CAPABILITY_DRIFT` 与 `check-mcp-policy.py` 的 `MCP_CAPABILITY_DRIFT` 已分别守各自域的单源；W7 须确保新增命令不触发这两码（即不引入第二份 capability 列表/verdict）。

## 6. Findings

### §F1 —（Critical）core gate 不直接覆盖 bin 侧"重复执行路径"；须手动审查 + 策略脚本断言
`check-core-boundary.py` 的 `CORE_SECOND_EXEC_PATH` 仅查 `core/` 内 `std::process::Command`/`sh -c`/`bash -c`。A3/A5 的只读命令 handler 在 `bridge.rs`/`mcp.rs`/`agent.rs`/`skills.rs`（bin 侧），若某只读命令**内部调用 `skill_run`/`script_runner`/`agent_chat`/rmcp server**，core gate **漏检**。
→ **裁定**：① 该"重复执行路径"须在 **bridge 层手动审查**（见 §F3/F4）；② A3/A5 的 W7 策略脚本须加断言码：`check-agent-skill-policy.py` 增 `AGSK_READ_ONLY_BRIDGE`（新 `skill_*`/`agent_*` 命令源码不得出现 `skill_run`/`script_runner`/`agent_chat`/`SkillExec::execute` 引用），`check-mcp-policy.py` 增 `MCP_READ_ONLY_BRIDGE`（新 `mcp_*` 命令源码不得出现 `rmcp`/`TcpListener`/`tokio::spawn`/`std::process`）。既有 `AGSK_SECOND_PATH`/`MCP_RUNTIME_LEAK` 已部分覆盖，W7 补齐"只读桥"专属码。

### §F2 —（Critical）A5 必须建"单一" skill/agent 加载器，被未来执行命令复用
实测无既有 skill/agent 磁盘加载器。`skill_list`/`agent_list`（W7 只读）与未来 `skill_run`/`agent_chat`（W8+ 执行）若各自实现加载逻辑 → **两条分叉执行路径**，解析/校验漂移难发现。
→ **裁定**：A5 W7 须落地**唯一加载器** `load_skill_defs(resolver: &dyn PathResolver)`/`load_agent_defs(resolver: &dyn PathResolver)`（放 `skills.rs`/`agent.rs`），W7 只读命令与 W8+ 执行命令**共用**；不得为只读新建一套、为执行另建一套。该加载器签名吃 `&dyn PathResolver`，未来搬 `core/` 时无缝（seam 已物化）。

### §F3 — A5 只读命令禁触执行路径
`skills.rs` 有 `SkillExec`（L53，纯结构，`validate` L56 仅递归校验无内联 shell，无副作用），但**真实执行**在 `skill_run`/`script_runner`（bin 侧，W7 不实现）。
→ **裁定**：A5 W7 只读命令（`skill_parse`/`skill_validate`/`skill_permission_preview`/`skill_list`/`agent_*` 同类）**只调** `SkillDef/AgentDef::parse`/`validate`/`permission_preview`（纯，零副作用）。**禁止**调用 `skill_run`/`script_runner`/`agent_chat`/任何 `SkillExec` 执行入口。`SkillExec::validate` 仅作结构守卫可被 `skill_validate` 复用，但**绝不 execute**。

### §F4 — A3 只读命令必须复用既有单一真源，禁起 server
mcp.rs 已有 `MCP_COMMAND_REGISTRY`/`evaluate_mcp_command`/`redact_mcp_url`/`current_policy_snapshot`，且注释明示 server-free（无 rmcp/tokio/TcpListener）。
→ **裁定**：A3 W7：① `mcp_registry_list` 返 `MCP_COMMAND_REGISTRY`；② `mcp_capability_preview` 调 `evaluate_mcp_command`（传 `roots`）；③ `mcp_policy_get` 返 `current_policy_snapshot()`；④ URL 经 `redact_mcp_url` 脱敏。**禁止**重实现 registry/verdict/redactor（否则 `MCP_CAPABILITY_DRIFT`/`MCP_COMMAND_REGISTRY` 漂移）。**禁止**起 rmcp server / `TcpListener` / `tokio::spawn` / `std::process`（MCP_RUNTIME_LEAK/MCP_LISTEN_PORT/MCP_TREE_TAURI 守；mcp.rs 既有 `MCP_*_PENDING` 红线标记继续有效）。

### §F5 — capability 裁决按域单一真源，A3/A5 各守其域（承 W6 §F4）
见 §5。A3 不造跨域 capability checker；A5 不碰 MCP/plugin verdict。避免"第二份 capability verdict 路径"。

### §F6 —（gate 加固，沿用 W1/W6）扩展 BIN_ONLY_MODULES 覆盖全部 bin 模块
`check-core-boundary.py` 的 `BIN_ONLY_MODULES` 当前仅 `(bridge, main, terminal, grid_process, tools, shutdown)`。`core/` 已物化（含 `seam.rs`），若 A3/A5 误将共享逻辑搬 `core/` 并 `use crate::mcp`/`crate::agent`/`crate::skills`/`crate::plugin`/`crate::graph`/`crate::database`/`crate::script_runner`…，当前 gate **漏检**（这些模块不在 BIN_ONLY）。
→ **裁定**：建议 A0 在 W7/W8 将 `BIN_ONLY_MODULES` 扩为全部 bin-side 模块（mcp/agent/skills/plugin/graph/database/script_runner/sync/tasks/scheduler/workspace/images/session/resources/scripts 等；**保留** `domain`/`seam`/`keyring_store` 为 core 可见，不列入），使 core→bin 反向依赖在 phase-1 被机器守门。属产品代码改动，由 A0 落地，W7 A2 不实现。

### §F7 — ACL 必须 atomic 扩；末条守 list_artifact_images
实测 skill/agent/mcp 当前**无任何 ACL 条目**。W7 Hard Stop 要求每个新命令 atomic 带 ACL。
→ **裁定**：A3/A5 每加一个 `mcp_*`/`skill_*`/`agent_*` 命令，必须同步在 `default-commands.toml` 加对应 ACL 条目（插入末条 `list_artifact_images` **之前**）；`check-agent-skill-policy.py` 的 `AGSK_ACL_TAIL` + `check-mcp-policy.py` 相应断言须覆盖末条不变。frontend `bridge.ts`/`types.ts` 包装同步加，policy 脚本断言"命令↔ACL↔bridge/types↔test"同包原子（AGSK_COMMAND_PARITY / MCP_PARITY）。

### §F8 — seam 方向：路径解析注入 &dyn PathResolver/&dyn RootsProvider
`core/seam.rs` 已物化；`agent_memory.rs:583` 已示范 `agent_kv_default_path(resolver: &dyn PathResolver)`；`bridge.rs:1155/1170` 定义 `TauriPathResolver`/`TauriRootsProvider`。
→ **裁定**：A5 `skill_list`/`agent_list` 与 A3 `mcp_capability_preview`(传 roots) 的路径/roots 解析**必须**走该 seam（handler 注入 `TauriPathResolver`/`TauriRootsProvider`），**禁止**内联 `app.path()`/硬编码路径。纯 parse/validate/preview 函数保持 seam-free（吃 `&str`/DTO）。未来这些纯函数随切片搬 `core/` 时，`&dyn PathResolver` 签名使其零改即可注入。

### §F9 — 隐私：只读返回 DTO 须脱敏，禁 secret 日志
W7 Hard Stop：无 token/cookie/Authorization/body/prompt-secret 日志/持久化。
→ **裁定**：① MCP 出参 URL 经 `redact_mcp_url`（单一真源）；② Agent/Skill `permission_preview` 返回 `PermissionPreview{gate,capabilities}`，不含 secret/token（K7 不渲染 props，承 W6 A4/A7 评审）；③ 任何 W7 命令 handler **不得** log/persist token/cookie/Authorization/body/prompt-secret（承 A4 W4 隐私双扫 `SENSITIVE_KEY_NAMES`/`SENSITIVE_VALUE_PATTERNS`，graph.rs/agent_memory.rs 已复用同款常量）；④ `skill_list`/`agent_list` 返回 `SkillDef`/`AgentDef` 须已脱敏（不携 agent_kv 值、不携凭证）。

## 7. 策略脚本与 gate 建议

- **A3** 扩展 `scripts/check-mcp-policy.py`：保留既有 15 ACTIVE 码；新增 `MCP_READ_ONLY_BRIDGE`（断言新 `mcp_*` 命令 handler 源码不含 `rmcp`/`TcpListener`/`tokio::spawn`/`std::process`/`skill_run` 等执行形态，且调用 `evaluate_mcp_command`/`redact_mcp_url`/`current_policy_snapshot` 单一真源）；`MCP_PARITY` 须覆盖"命令↔ACL↔bridge.ts↔types.ts↔test"原子。可借 `check-graph-policy.py`/`check-agent-skill-policy.py` 同族范式。
- **A5** 扩展 `scripts/check-agent-skill-policy.py`：保留既有 7 ACTIVE 码；新增 `AGSK_READ_ONLY_BRIDGE`（断言新 `skill_*`/`agent_*` 命令 handler 不含 `skill_run`/`script_runner`/`agent_chat`/`SkillExec::execute` 引用）、`AGSK_LOADER_SINGLE`（断言 `skill_list`/`agent_list` 与（未来）`skill_run` 共用同一 `load_*_defs(resolver)` 加载器，无第二 read-loader）；`AGSK_SECOND_PATH` 已守"无第二执行路径"，W7 须确保只读命令不触发；`AGSK_CAPABILITY_DRIFT` 守 Skill/Agent 域 capability 单源；`AGSK_ACL_TAIL`/`AGSK_COMMAND_PARITY` 守 ACL 原子。
- 两脚本均接 `scripts/pre-merge.sh`（与 W5/W6 同款位置），且须含 `--self-test`/`--default`/`--expect-pending` 三模式。
- core 边界 gate 加固（§F6）沿用 W1/W6：扩展 `BIN_ONLY_MODULES`。

## 8. seam 方向结论（前瞻）

- **W7 不需改 `core/seam.rs`**：`ProgressSink`/`PathResolver`/`RootsProvider` 已物化且被 `agent_memory.rs` 采用。A3/A5 只读命令只需在 handler 层注入 `TauriPathResolver`/`TauriRootsProvider`，纯逻辑吃 `&dyn PathResolver`/`&dyn RootsProvider` 或 `&str`/DTO。
- **重复执行路径**是 W7 头号风险，且 core gate 不直接覆盖 bin 侧 → 靠 §F1/F2/F3/F4 手动审查 + 策略脚本 `AGSK_SECOND_PATH`/`MCP_RUNTIME_LEAK`/`*_READ_ONLY_BRIDGE` 断言兜底。
- 未来 W8+ 执行命令（`skill_run`/`agent_chat`/MCP server）启用时，**必须复用 W7 建立的单一加载器与既有 `evaluate_mcp_command`/`permission_preview` 真源**，不得另起炉灶——此即 W7 seam/单源方向对后续波次的价值。

## 9. Verify（本笔记证据）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) core 边界 gate（现状基线）
python3 scripts/check-core-boundary.py            # all invariants hold（ACTIVE=7，core 文件=3）
python3 scripts/check-core-boundary.py --self-test  # CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9）
# 2) 新命令尚未进树（W7 实施前）
grep -nE "skill_list|agent_list|mcp_registry_list|mcp_capability_preview" src-tauri/src/bridge.rs  # 0 命中
grep -nE "skill|agent|mcp" src-tauri/permissions/default-commands.toml  # 0 命中（ACL 待 A3/A5 扩）
# 3) 既有单一真源（A3/A5 须复用，不得重实现）
grep -nE "pub fn evaluate_mcp_command|pub fn redact_mcp_url|pub fn current_policy_snapshot|MCP_COMMAND_REGISTRY" src-tauri/src/mcp.rs  # L84/L105/L111/L22
grep -nE "pub fn parse|pub fn validate|pub fn permission_preview|pub struct SkillExec" src-tauri/src/skills.rs src-tauri/src/agent.rs  # 纯函数 + SkillExec 结构
# 4) seam 已物化 + 采用
ls src-tauri/src/core/                               # keyring_store.rs / mod.rs / seam.rs
grep -nE "use mvp_core::core::seam::PathResolver|agent_kv_default_path" src-tauri/src/agent_memory.rs  # L27/L583
# 5) 无既有 skill/agent 加载器（A5 须建单一加载器）
grep -rnE "fn load_skill_defs|fn load_agent_defs|SkillStore|AgentStore" src-tauri/src  # 0 命中
# 6) 策略脚本既存码（W7 扩展而非新建）
grep -oE '"MCP_[A-Z_]+"' scripts/check-mcp-policy.py | sort -u   # 15 码
grep -oE '"AGSK_[A-Z_]+"' scripts/check-agent-skill-policy.py | sort -u  # 7 码
# 7) 工作树状态（本笔记为 review-only，无产品码）
git status --short --branch
```

## 10. NEXT

- 交 A0：① 采纳本笔记对 **W7 "重复执行路径"** 的裁定（§F1–§F5）——A3/A5 只读命令须复用既有单一真源（`evaluate_mcp_command`/`permission_preview`/`redact_mcp_url`/`MCP_COMMAND_REGISTRY`/`current_policy_snapshot`），不得重实现 registry/verdict/redactor，不得起 server，不得触执行路径；② 采纳 **单一加载器** 要求（§F2）：A5 建 `load_skill_defs`/`load_agent_defs(resolver)`，被 W8+ 执行命令复用；③ 排期 **`BIN_ONLY_MODULES` 加固**（§F6）使 core→bin 反向依赖被机器守门；④ 推动 W4 §4 capability 真源统一收口（§F5，跨 MCP/Skill/Agent/Plugin 四域）。
- A3（W7 产品码）：按 §4 红线交付 mcp 只读桥（复用 mcp.rs 既有函数，无 server）+ 扩 `check-mcp-policy.py`（加 `MCP_READ_ONLY_BRIDGE`）+ ACL atomic。
- A5（W7 产品码）：按 §3 红线交付 Agent/Skill 只读桥（建单一加载器、只调 parse/validate/permission_preview、不触执行路径）+ 扩 `check-agent-skill-policy.py`（加 `AGSK_READ_ONLY_BRIDGE`/`AGSK_LOADER_SINGLE`）+ ACL atomic。
- A0/A10 在 A3/A5 代码集成后，凭 §6 红线清单 + 策略脚本新码做二次复核（重点：① 新命令 handler 是否引用执行函数；② 是否重实现 capability verdict；③ ACL 是否 atomic；④ 加载器是否单一）。
