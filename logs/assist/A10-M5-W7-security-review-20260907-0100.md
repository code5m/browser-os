# A10 · M5-W7 安全复审门禁说明（predecessor 未产出 → BLOCKED）

> Lane: A10（M4/M5 独立安全复审）
> Dispatch: `PARALLEL_COMMAND_BOARD.md` → "M5-W7 Integration Dispatch"（A0 于 2026-09-07 00:50 经 `5f92ece`/`a26fbaf` 加入；board 头标 master=`5f92ece`）
> A10 W7 任务：Security review A3/A5 command bridges: **source check / ACL / redaction / no execution·install·network / no sensitive audit**
> 交付形态（board）：`logs/assist/A10-M5-W7-*.md`；仅当修复具体失败时可加 policy 夹具
> 复审时间：2026-09-07 ~01:00 CST
> 交付物：本说明（仅 docs，未改动任何产品代码，未 push）

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=BLOCKED
BASE=5f92ece
HEAD=logs/assist/A10-M5-W7-security-review-20260907-0100.md（docs only）
FILES=logs/assist/A10-M5-W7-security-review-20260907-0100.md
VERIFY=实证：A3/A5 W7 命令桥在产品副本缺席（见 §2）；复审尚不可运行
CHECKPOINT=logs/assist/A10-M5-W7-security-review-20260907-0100.md
MERGE_NOTES=A3/A5 W7 产出尚未由 A0 集成进 master；A10 无法复审不存在的产品代码。依 Startup Gate（board 第64行）产出只读 assist 说明并给精确解锁条件，不编产品代码
NEXT=A0 集成 A3 W7（mcp.rs 只读 MCP 命令 + bridge.rs/main.rs/ACL/bridge.ts/types.ts + check-mcp-policy.py）+ A5 W7（agent.rs/skills.rs 只读 Agent/Skill 命令 + 同套接线 + check-agent-skill-policy.py）后，A10 跑 §4 验证矩阵并写 A10-M5-W7-security-review-<ts>.md（PASS / PASS_WITH_DEBT / BLOCKED）
```

## 1. 范围与方法

按 board W7，A10 复审两块（W7 唯一可写产品代码者），且本波是**窄只读命令桥波、非运行时激活**：
- **A3（M5-2）**：只读 MCP 注册表/策略命令桥——列出注册表项、预览能力裁定、返回脱敏 DTO。**禁 rmcp/server/listener/网络**。须含 source check + ACL + 前端 bridge/types（若加命令）。
- **A5（M5-4/5）**：只读 Agent/Skill 命令桥——解析/校验 `AgentDef`/`SkillDef` + 权限预览。**禁 skill 执行、禁安装、禁网络、禁持久化写**。须含 source check + ACL + 前端 bridge/types（若加命令）。

6 类焦点（W7 特化）：source check / ACL / redaction / no execution·install·network / no sensitive audit。
方法：产出后**实证**跑门 + `git diff` + `grep`（非仅凭读码）。

## 2. 实证发现：A3/A5 W7 命令桥尚未落库（阻塞项）

证据（2026-09-07 ~01:00 在 `WORKDIR`）：
- **A3**：`grep -nE 'mcp_list|mcp_registry|mcp_get_policy|mcp_command|fn mcp_|McpCommand' src-tauri/src/bridge.rs src-tauri/src/main.rs` → 空；`grep -nE 'mcp_' src-tauri/permissions/default-commands.toml` → 空；`grep -nE 'mcp_|Mcp' src/bridge.ts src/types.ts` → 空。
- **A5**：`grep -nE 'agent_parse|skill_validate|agent_permission_preview|agent_list|skill_list|fn agent_|fn skill_' src-tauri/src/bridge.rs src-tauri/src/main.rs` → 空；`grep -nE 'agent_|skill_' src-tauri/permissions/default-commands.toml` → 空。注：`src/bridge.ts:352-372` **已存在** `skillList/agentList/skill_install/agent_install/confirm_skill_install/confirm_agent_install` 前端封装，但 `src/bridge.ts:63` 注释明示「`skill_*/agent_*` 后端命令尚未落地；故置[占位]」——即后端 handler 未落地；且 `skill_install/agent_install/confirm_*` 属 **W7 禁止项**（执行/安装），A5 W7 只落只读子集。
- `git log --oneline --all | grep -iE 'M5-2|M5-4|M5-5|mcp-bridge|agent-bridge|read-only|command bridge'` → 仅 `a26fbaf`（W7 调度文档）、A3 各波 MCP 兼容复审文档（`412d0eb`/`d3f11cd`/`d71f558`）、A3 W3 MCP 注册表+全局策略（`12f1cff`/`a654f0c`）。**无 A3/A5 W7 产品提交**。
- `git status`：仅 `M5-0-overview.md` 被改（A1 对账类 checkpoint 文档，非实现）。**无 A3/A5 W7 产品/未跟踪文件**。

结论：A3/A5 W7 命令桥不在主副本。二者大概率在各自 worktree 开发，A0 尚未集成。

## 3. 解锁条件（精确）

满足任一即触发 A10 实质复审：
- **(a)** A0 将 A3 W7（`feat(M5-2 …)` / `mcp.rs` 只读命令 + bridge.rs/main.rs/ACL/bridge.ts/types.ts + `check-mcp-policy.py`）+ A5 W7（`feat(M5-4 …)`/`feat(M5-5 …)` / `agent.rs`/`skills.rs` 只读命令 + 同套接线 + `check-agent-skill-policy.py`）提交进 master（文件在场、`git log` 可见）；或
- **(b)** A3/A5 在本主副本暂存/落盘其 W7 命令桥（`git status`/`ls`/`grep` 可见）。

解锁前 A10 不得改产品代码（Startup Gate 第64行）。本说明即当期交付。

## 4. 预飞验证矩阵（解锁后执行）

### A3（M5-2 只读 MCP 命令桥）
- Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml mcp`（须 PASS）。
- 策略门三模式：`python3 scripts/check-mcp-policy.py --self-test` / 默认 / `--expect-pending`（pending 守门须 NONE）。
- 命令原子性（关键）：新命令（建议 `mcp_list_registry` / `mcp_preview_capability` / `mcp_get_policy` 之类只读名）须 5 处一致——Rust handler（`bridge.rs`/`mcp.rs`）+ `main.rs` 注册 + ACL 位于 `list_artifact_images` **之前** + `src/bridge.ts` 封装 + `src/types.ts` DTO；且 handler 内调用 `check_invocation_source`（source check）。
- 只读不越界：`grep -rnE 'TcpListener|\.bind\(|rmcp|tokio::net|reqwest|ureq|execute|spawn|std::process' src-tauri/src/mcp.rs` → 空（W7 硬停：无 MCP server/listener、无网络、无执行）。
- 脱敏返回：`grep -rnE 'token|secret|password|Authorization|url|endpoint' src-tauri/src/mcp.rs` → 返回的注册表/策略 DTO 不得含 server token/带凭据的 URL/Authorization；只返回脱敏视图（`McpCommandDef` 等已冻结结构，不得附 secret）。
- 无敏感审计：命令审计 detail 不得含 token/cookie/Authorization/body/prompt-secret。

### A5（M5-4/5 只读 Agent/Skill 命令桥）
- Rust 测试：`cargo test --manifest-path src-tauri/Cargo.toml agent` + `… skills`（须 PASS）。
- 策略门三模式：`python3 scripts/check-agent-skill-policy.py --self-test` / 默认 / `--expect-pending`（pending 守门须 NONE）。
- 命令原子性：新只读命令（建议 `agent_parse` / `skill_validate` / `agent_permission_preview` / `skill_permission_preview` / `agent_list` / `skill_list`）须 5 处一致（handler + `main.rs` + ACL 在 `list_artifact_images` 前 + `bridge.ts` + `types.ts`）+ `check_invocation_source`。
- **W7 禁止项断言（关键）**：`grep -rnE 'skill_install|agent_install|confirm_skill_install|confirm_agent_install|skill_execute|agent_execute|skill_run|agent_run' src-tauri/src/agent.rs src-tauri/src/skills.rs src-tauri/src/bridge.rs src-tauri/src/main.rs` → 空（W7 硬停：无 skill 执行/安装/enable/disable；`src/bridge.ts` 现有 `skill_install/agent_install/confirm_*` 仅前端占位，本波不计后端 handler）。
- 无执行/网络/持久化写：`grep -rnE 'std::process|spawn|execute|install|reqwest|ureq|fs::write|sqlx|insert|update|delete' src-tauri/src/agent.rs src-tauri/src/skills.rs` → 空（解析/校验/权限预览须纯函数，不写库、不联网、不执行）。
- 脱敏/不泄露 secret：`grep -rnE 'token|secret|password|Authorization|body|prompt' src-tauri/src/agent.rs src-tauri/src/skills.rs` → 权限预览/校验结果不得回显 secret/body/prompt 秘文（DTO 真源为 `domain.rs` 的 `SkillDef`@1968/`AgentDef`@2006，已不含凭据字段）。
- 无敏感审计。

### 跨切（两块皆查）
- `cargo test --manifest-path src-tauri/Cargo.toml`（全量，基线含 W4–W6 新增）。
- `git diff HEAD -- src-tauri/src/domain.rs src-tauri/src/security_policy.rs src-tauri/src/bridge.rs src-tauri/src/main.rs src/tauri/permissions/default-commands.toml src/components/** src/stores/** src/types.ts src/bridge.ts scripts/pre-merge.sh` → 查 ACL/命令/前端包漂移（高冲突文件清单），重点核对 ACL 顺序（`list_artifact_images` 前插入新命令）。
- `bash scripts/pre-merge.sh`（A3/A5 接好各自脚本后须全过）。
- `git diff --check`。
- 敏感常量单一真源（承接 W6 A4 F1）：`grep -rnE 'SENSITIVE_KEY_NAMES|SENSITIVE_VALUE_PATTERNS' src-tauri/src/*.rs scripts/check-*.py` → 须仅 `domain.rs` 一处定义（A4 agent_memory.rs:134 + A7 graph.rs:18 重复项应已抽离）。

## 5. 预飞风险假设（基于已落地契约）

- **W7 是"只读命令桥"，非运行时激活**：最高风险是某命令误暴露写/执行路径。A10 对两块逐命令断言「只读」——A3 禁 server/网络/执行；A5 禁执行/安装/网络/持久化写。
- **原子命令红线**：W7 硬停"Every new command must be atomic with source check, ACL, frontend bridge/types, policy coverage, and tests"。A10 验 5 路 parity + `check_invocation_source` 调用 + 测试存在。
- **脱敏 DTO**：MCP 注册表/策略返回不得含 server token/带凭据 URL；Agent/Skill 校验/权限预览不得泄露 secret/body/prompt（真源 DTO 已无凭据字段，但预览/解析输出须二次确认不回显）。
- **无第二执行路径**：A5 `skill_*`/`agent_*` 执行须仅在**未来**波通过复用 M2-4 `script_runner` 落地，W7 不得引入（grep 须空）。
- **ACL 顺序漂移**：新命令须插在 `list_artifact_images` 之前；A10 用 `git diff` 核对 `default-commands.toml` 与 `main.rs` invoke 注册，并验 `bridge.ts`/`types.ts` 命名一致。
- **前端占位一致性**：`src/bridge.ts:356-372` 已有 `skill_install/agent_install/confirm_*` 占位；A5 W7 只落只读子集，不得误把安装/确认命令实现进后端（W7 禁止）。
- **敏感审计**：所有新命令审计 detail 无 token/cookie/Authorization/body/prompt-secret（对齐 M4 护栏 + A4/A7 policy）。

## 6. 延续 / 引用

- **W4/W5/W6 复审积压（现已解锁，建议补做）**：W7 "Current facts" 明示 **graph UI shell（A8 W6）+ plugin manifest/lifecycle policy（A9 W6）已集成**；W6 facts 明示 A6 W5（UI shell）+ A7 W5（图模型/存储）已集成；W5 facts 明示 A4/A5 W4 已集成。故 A10 既有门禁 **全部具备复审条件**：
  - `A10-M5-W4-security-review-20260906-1805.md`（A4/A5，原 BLOCKED）→ 现可审。
  - `A10-M5-W5-security-review-20260906-1900.md`（A6/A7，原 BLOCKED）→ 现可审。
  - `A10-M5-W6-security-review-20260906-2300.md`（A8/A9，原 BLOCKED）→ 现可审。
  - 建议 A0/A10 在 W7 之后按 W4→W5→W6 顺序补做这三份实质复审（本说明不越位执行，待指令）。
- 同波 W7 支持文档（他 lane，本期未在主副本落地，待产出）：A2 core 边界/seam 方向、A4 agent/skill payload 隐私交互、A8 前端 bridge/types 对图 UI 影响、A9 插件命令面保持只读。
- A3 MCP 兼容复审（他 lane，已落地）：`412d0eb`(W6)/`d3f11cd`(W5)/`d71f558`(W4) —— 本波 A3 只读 MCP 命令桥须对齐其结论。
- board W7 段：第 137-172 行（Assignments + Hard Stops）。

## 7. 声明

- 仅 docs；未改产品代码；未 push。
- 工作树其余未提交物（`M5-0-overview.md` 属 A1 对账）未触碰。
- HEAD = `5f92ece`，本地与 `origin/master` 一致（领先 0）。
