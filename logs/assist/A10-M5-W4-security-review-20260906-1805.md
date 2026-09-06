# A10 · M5-W4 安全复审门禁说明（predecessor 未产出 → BLOCKED）

> Lane: A10（M4/M5 独立安全复审）
> Dispatch: `PARALLEL_COMMAND_BOARD.md` → "M5-W4 Parallel Dispatch"（A0 于 2026-09-06 17:55 经 `f8f1f49` 加入）
> A10 W4 任务：Security review A4/A5 for **credential leakage / unbounded maps / command exposure / source-check·ACL drift / duplicate execution path**
> 交付形态（board）：`logs/assist/A10-M5-W4-*.md`；仅当修复具体失败时方可改 policy 脚本
> 复审时间：2026-09-06 ~18:05 CST
> 交付物：本说明（仅 docs，未改动任何产品代码，未 push）

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=BLOCKED
BASE=f7ad35a
HEAD=logs/assist/A10-M5-W4-security-review-20260906-1805.md（docs only）
FILES=logs/assist/A10-M5-W4-security-review-20260906-1805.md
VERIFY=实证：A4/A5 W4 产品代码在主副本缺席（见 §2 证据）；复审尚不可运行
CHECKPOINT=logs/assist/A10-M5-W4-security-review-20260906-1805.md
MERGE_NOTES=A4/A5 W4 产出尚未由 A0 集成进 master；A10 无法复审不存在的产品代码。依 Startup Gate（board 第64行附近）产出只读 assist 说明并给出精确解锁条件，不编产品代码
NEXT=A0 集成 A4 W4（agent_memory/a2a + check-agent-memory-policy）+ A5 W4（agent/skills + check-agent-skill-policy）后，A10 跑 §4 验证矩阵并写 A10-M5-W4-security-review-<ts>.md（PASS / PASS_WITH_DEBT / BLOCKED）
```

## 1. 范围与方法

按 board W4，A10 复审两块：
- **A4（M5-3）**：A2A / agent memory KV 契约切片——DTOs、校验、容量/隐私策略、纯 store 助手或 JSON 持久化壳；无网络协议、无后台运行时。
- **A5（M5-4/5）**：Agent/Skill 域 + 命令策略壳——`AgentDef`/`SkillDef` DTOs、校验、权限预览、策略脚本；暂不执行 skill；除非 source-check+ACL+types+bridge+tests 同包完成，否则不加命令运行时。

5 类焦点：credential leakage / unbounded maps / command exposure / source-check·ACL drift / duplicate execution path。
方法：产出后**实证**跑门 + `git diff` + `grep`（非仅凭读码）。

## 2. 实证发现：A4/A5 W4 产出尚未落库（阻塞项）

证据（2026-09-06 ~18:05 在 `WORKDIR` 运行）：
- `git log --oneline -15`：头 `f7ad35a docs(M5): dispatch W4 agent memory and skill lanes`；`f8f1f49 feat(M5): add core seam abstractions`（A2 W3 seam 收口，已提交）。W4 打开后**无 A4/A5 产品提交**。
- 缺失文件（全部）：`src-tauri/src/agent_memory.rs`、`src-tauri/src/a2a.rs`、`src-tauri/src/agent.rs`、`src-tauri/src/skills.rs`、`scripts/check-agent-memory-policy.py`、`scripts/check-agent-skill-policy.py`。
- `domain.rs` grep `AgentDef|SkillDef|AgentMemory|AgentKV|a2a` → 仅有 W3 的 `McpCommandDef`；无 A4/A5 W4 DTO。
- `pre-merge.sh` grep `check-agent-memory-policy|check-agent-skill-policy` → 未接线。
- `git status`：仅 4 个 M5 checkpoint md 被改（A1 W4 对账）+ 2 个未跟踪 assist（`A2-M5-W4-seam-review`、`A8-M5-W4-graph-ui-delta`）；无 A4/A5 产物。

结论：A4/A5 W4 产品代码不在主副本。二者大概率在各自 worktree 开发，A0 尚未集成。

## 3. 解锁条件（精确）

满足任一即触发 A10 实质复审：
- **(a)** A0 将 A4 W4（`feat(M5-3 …)` / agent_memory / a2a / check-agent-memory-policy）+ A5 W4（`feat(M5-4 …)` / agent / skills / check-agent-skill-policy）提交进 master（文件在场、`git log` 可见）；或
- **(b)** A4/A5 在本主副本暂存/落盘其 W4 产品代码（`git status`/`ls` 可见）。

在解锁前，A10 不得改产品代码（Startup Gate 第64行）。本说明即当期交付。

## 4. 预飞验证矩阵（解锁后执行）

### A4（M5-3 agent memory / A2A KV）
- 策略门三模式：`python3 scripts/check-agent-memory-policy.py --self-test` / 默认 / `--expect-pending`（pending 守门须 NONE）。
- 无凭据/body 泄漏：`grep -rnE 'token|cookie|Authorization|password|secret|body|prompt' src-tauri/src/agent_memory.rs src-tauri/src/a2a.rs` → DTO/审计/日志/前端态须脱敏或零出现。
- 有界记录：确认 `domain.rs` + `agent_memory.rs` 含容量/隐私策略；无无界 `HashMap`/`Vec`；测试断言容量强制。
- 无新依赖：比对 `Cargo.toml`，W4 硬停止禁止 npm、禁止新 rust 依赖（除非已存在且合理）。
- 无网络：`grep -rnE 'TcpListener|\.bind\(|reqwest|ureq|tokio::net|rmcp' src-tauri/src/agent_memory.rs src-tauri/src/a2a.rs` → 须为空。
- core 侧复用 seam 而非 tauri：`cargo test mvp_core` + `python3 scripts/check-core-boundary.py`（A4 core 侧 KV 不得 import tauri/bridge/AppHandle）。

### A5（M5-4/5 Agent/Skill）
- 策略门三模式：`python3 scripts/check-agent-skill-policy.py --self-test` / 默认 / `--expect-pending`。
- 无第二执行路径：`grep -rnE 'std::process::Command|spawn|shell' src-tauri/src/agent.rs src-tauri/src/skills.rs` → 不得新增执行；skill 执行须复用 M2-4 `script_runner`（board：暂不执行 skill）。
- 无安装器/网络/下载：`grep -rnE 'download|install|curl|wget|npm i|fetch\(' src-tauri/src/agent.rs src-tauri/src/skills.rs` → 空。
- 命令暴露：若新增 Tauri 命令，须 5 处原子一致（Rust handler + `main.rs` invoke + ACL 位于 `list_artifact_images` 之前 + `src/bridge.ts` + `src/types.ts` 同包）+ source check（`check_invocation_source`）+ 测试。
- Agent/Skill DTO 有界且隐私过滤；权限预览在场。

### 跨切（两块皆查）
- `cargo test --manifest-path src-tauri/Cargo.toml`（全量，基线 335 + 新增）。
- `git diff HEAD -- src-tauri/src/domain.rs src-tauri/src/bridge.rs src-tauri/src/main.rs src/tauri/permissions/default-commands.toml src/bridge.ts src/types.ts` → 查 ACL/命令/前端包漂移（高冲突文件清单）。
- `bash scripts/pre-merge.sh`（A4/A5 接好各自策略脚本后须全过）。
- `git diff --check`。

## 5. 预飞风险假设（基于已落地契约）

- **A4 复用 `mvp_core::seam`**：`lib.rs` 已 `pub mod core; pub use crate::core::*`。KV 侧若需路径/根/进度，须用 `ProgressSink`/`PathResolver`/`RootsProvider`，**不得**在 core 侧重引 tauri/bridge（core 边界门强制）。
- **单一真源模式**：A4 KV 须镜像 MCP 注册表——能力/白名单/上限在 `domain.rs` 定义一处；`check-agent-memory-policy.py` 守漂移（类 `MCP_CAPABILITY_DRIFT`）。
- **A5 复用与 fail-closed**：Agent/Skill 须镜像 MCP 注册表 fail-closed，复用 `security_policy::{check_path_within_roots, redact_sensitive_url}` 单一真源；**不得**引入第二执行路径（复用 M2-4）。
- **有界 + 隐私过滤**：两块 store 须有界且隐私过滤，DTO/审计/文件/日志不得含 token/cookie/Authorization/body/已记 prompt 秘文（M4 guardrails）。
- **W4 硬停止断言**：无网络监听 / rmcp server / 插件安装器 / 模型提供商集成 / 后台 agent 运行时 / npm 依赖 / GUI 面板。A10 逐条 grep 断言。
- **新命令 5 路一致**：任一新命令须 source-check + ACL + bridge/types + 策略覆盖 + 测试同包；A10 验 5 路 parity。

## 6. 延续 / 引用

- A10 M5-W3 复审：`logs/assist/A10-M5-W3-security-review-20260906-1730.md`（A2 seam PASS、A3 MCP 注册表 PASS；W2 findings D1/#1 已关闭）。
- A4 契约基础：`logs/assist/A4-M5-a2a-memory-20260906-{0755,0825-w1-delta,1336-w2-slice,1410-w3-delta}.md`
- A5 契约基础：`logs/assist/A5-M5-agent-skill-20260906-{0800,W1-delta-0900,W3-next-card-1715}.md`
- board W4 段：第 134-168 行（Assignments + Hard Stops）。

## 7. 声明

- 仅 docs；未改产品代码；未 push。
- 工作树其余未提交物（A1 checkpoint md、A2/A8 assist）属他 lane，未触碰。
- HEAD = `f7ad35a`，本地与 `origin/master` 一致（领先 0）。
