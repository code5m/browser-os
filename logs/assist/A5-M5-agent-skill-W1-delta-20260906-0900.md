# A5 · M5-W1 增量说明（Delta Note · Agent/Skill runtime+commands 支持文档）

> LANE=A5　WAVE=M5-W1 Implementation Dispatch　STATUS=PASS_WITH_DOCS（SUPPORT DOCS ONLY，无产品代码）
> BASE=5ca8f9f（本地 `master`，领先 `origin/master` 1）
> SCOPE=`logs/assist/A5-M5-agent-skill-*.md` only（本文件 + 前作 `A5-M5-agent-skill-20260906-0800.md`）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W1 Implementation Dispatch → A5：**SUPPORT DOCS ONLY**；W1 Hard Stops（仅 A2 可动产品代码）
> 目标产物：`logs/assist/A5-M5-agent-skill-W1-delta-20260906-0900.md`（本文件）
> 配套前作：`logs/assist/A5-M5-agent-skill-20260906-0800.md`（M5-W0 预研，仍有效，本文件为其增量修订）

---

## 0. 增量摘要（给 A0 / A16 / 后续 Lane）

W1 相对 W0 的关键变化，本增量说明逐条对齐：

1. **A5 在 W1 的角色 = 支持文档，不是实现者**。M5-4/M5-5 的实现责任 Lane 候选为 **A16**（见 `logs/checkpoints/M5-20260906/M5-4-agent-skill-runtime.md` / `M5-5-agent-skill-commands.md` 顶部「责任 Lane 候选：A16，A0 签发时定」）。W0 文档 §2/§13 中「A5 实施」的措辞**作废**，改为「A0 签发后由 A16 实施」。
2. **W1 硬停：仅 A2 可动产品代码**。A5 不得写 `src-*`/`Cargo.toml`/ACL/新 Tauri 命令/`pre-merge.sh`。本文件零产品代码。
3. **指挥板对 A5 的唯一硬性要求 = 「keep Skill execution reuse of M2-4 explicit」**。本文件 §2 将其提升为最高红线并绑定 A2 W1 边界门 `check-core-boundary.py` 的「no second execution path」断言。
4. **类型模型对齐 A1 展开卡**：`SkillExec::{ScriptRef,CommandRef,Sequence}`（禁 Inline，K6）、`AclLevel::{Safe,Confirm,Dangerous}`、流式走 `app.emit("agent://<id>/stream")` + ≥50ms 节流（与 M3-4.b 同款）、`skill-runs.json`/`agent-runs.json` 独立 500 上限。
5. **命令数从 W0 的 4 条修订为 A1 卡的 15 条**（7 skill + 8 agent），全部进 ACL 且末条恒为 `list_artifact_images`。

---

## 1. W1 角色与依赖闸门（修正 W0 的「A5 实施」假设）

| 项 | W0 文档旧措辞 | W1 修订 |
|---|---|---|
| M5-4/5 实现责任 | 「A5 实施 runtime+command 批」 | **A0 签发后由 A16 实施**（候选见 M5-4/5 卡） |
| A5 在 W1 的状态 | DOCS ONLY（被动） | **SUPPORT DOCS ONLY（主动增量）** |
| 产品代码 | 全禁 | 全禁（仅 A2 可动） |
| 实施解锁条件 | §10.2 四条件 | 追加：**A2 M5-1 core boundary 落地 + A3 M5-2 capability.rs 落地**（见 §4） |

> A5 的「整包交付」在 W1 = 本增量文档；不产 patch（W1 硬停禁止产品代码，assist 文档由 A0 直接拣入，同 W0 口径）。

---

## 2. 红线重申：Skill 执行必须复用 M2-4（指挥板对 A5 的硬性要求）

**这是本增量文件的核心交付点 —— 把「Skill/Agent 复用 M2-4 执行通道」讲死、讲可机检。**

### 2.1 事实锚点（已实测，见 W0 §4.1）

- `src-tauri/src/script_runner.rs`：
  - `start_run(table, meta, script_abs_path, values, roots, home_dir, app, records_file) -> Result<String, RunError>`（行 843）
  - `start_command(table, snippet, values, roots, home_dir, app, records_file) -> Result<String, RunError>`（行 882）
  - 二者经 `start_argv_run`，复用同一进程表 / supervisor / 输出 reader / 取消 / 超时 / 运行记录落盘链路。
- M4 scheduler（F6）已证明正确范式：`scheduler.rs` 只做触发，执行唯一入口是 `script_runner::start_run` / `start_command`（行 651/681）。

### 2.2 强制约束（对 A16 实施期的机器可守门条款）

| 调用方 | 必须调用 | 禁止 |
|---|---|---|
| `skill_runtime::run_skill(SkillExec::ScriptRef{script_id}, params)` | `script_runner::start_run`（按 id 取 `ScriptMeta`） | `std::process::Command`、`sh -c`、自拼 argv、`InlineScript` |
| `skill_runtime::run_skill(SkillExec::CommandRef{command_id}, params)` | `script_runner::start_command` | 直接 spawn shell |
| `skill_runtime::run_skill(SkillExec::Sequence{steps})` | 每步递归走上述二者 | 任何步骤直起进程 |
| `agent_runtime` 调工具（含 `ExternalCli`） | 经 Skill 或 `script_runner::start_command` | 为 Agent 引入第二条进程路径；禁 `tokio::spawn` stdio 进程 |

> **AGSK_1（W1 升级为硬门）**：`skill_runtime.rs` / `agent_runtime.rs` 中 `grep -nE 'std::process|Command::new|"-c"|tokio::spawn'`（针对直起进程）必须为 0；否则视为「第二执行路径」P0 缺陷，与 scheduler F6 / A2 `check-core-boundary.py` 的「creates a second execution path」断言同源。
> M5-4 卡 §4.4 / §5 同口径：「禁为 Agent 引入第二执行路径；禁 Agent 直接 `tokio::spawn` stdio 进程」。

### 2.3 与 A2 W1 边界门的对齐

A2 在 W1 落 `scripts/check-core-boundary.py`，其失败条件含：
- core 导入 `tauri` / 引用 `AppHandle` / 引用 `crate::bridge` → FAIL
- core **创建第二执行路径** → FAIL

因此 `skill_runtime.rs` / `agent_runtime.rs` 必须落在 `mvp_core`（A2 M5-1 boundary），且：
- 仅依赖 `script_runner`（core 内）与注入 seam（`ProgressSink`/`PathResolver`/`RootsProvider`，见 W0 §2.2），**绝不 `use crate::bridge`**。
- 经 `script_runner` 执行，天然满足「无第二执行路径」。
→ 这样 AGSK_1 既被 `check-core-boundary.py` 守门，也被 A5 的 `check-agent-skill-policy.py` 守门（W0 §11.1），双保险。

---

## 3. 类型模型对齐 A1 展开卡（M5-4 §4 / M5-5 §4）

### 3.1 `SkillExec`（取代 W0 的 `SkillImpl`，语义一致、命名与 K6 对齐）

```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillExec {
    ScriptRef  { script_id: String, params: serde_json::Value },  // 复用 M2-3 已存脚本
    CommandRef { command_id: String, params: serde_json::Value }, // 复用 M2-3 已存命令
    Sequence   { steps: Vec<SkillExec> },                         // 串联
    // 【禁】InlineScript / RawShell / SystemCommand —— K6 红线
}
```

> W0 文档用 `SkillImpl::{Script, Builtin}`；A1 卡用 `SkillExec::{ScriptRef,CommandRef,Sequence}`。两者均「禁内联代码」。推荐 A16 采用 A1 卡的 `SkillExec` 命名（更显式表达「引用而非内嵌」），W0 §3.1 的 `SkillImpl` 视为历史别名。

### 3.2 `AclLevel`（取代 W0 的 `Low/Medium/High`）

```rust
pub enum AclLevel { Safe, Confirm, Dangerous }
```

- `Safe`：装入 `skills_dir()` 即可生效，不弹闸门。
- `Confirm`：必须前端弹窗两段式确认。
- `Dangerous`：必须 keyring 二次确认 + UI 风险提示。

### 3.3 `AgentDef`（对齐 M5-4 §4.1）

```rust
pub struct AgentDef {
    pub id: String,                  // 反向域名规范
    pub version: String,             // semver
    pub display_name: String,
    pub description: String,
    pub dialect: AgentDialect,       // M5-3 §4.2
    pub system_prompt: String,       // 不含凭据
    pub default_capabilities: Vec<CapabilityRef>, // 引用 capability.rs 常量
    pub a2a: A2aConfig,              // delegate_to / delegated_from 开关
    pub metadata: serde_json::Value,
}
```

### 3.4 流式回传（对齐 M5-4 §4.3，修订 W0 的 `Channel<Token>`）

- 通道：`app.emit("agent://<agent_id>/stream", payload)`（Tauri event）。
- 节流：≥50ms 批量 + 累计 ≤16KB 强制 flush；心跳每 5s；终止 `.../done`/`.../error`/`.../canceled`。
- 前端：`bridge.ts` 收 event → `useAIStore.addStream`（复用既有 store）。
- 与 W0 引用的设计文档 `agent_chat(msg) -> Channel<Token>` 不矛盾（Channel 是 Tauri2 另一直输范式，如 `term_spawn_channel`）；**A0/A16 二选一**，A1 卡已定 `app.emit` 方案。无论哪种，Token 流**不得序列化凭据/完整 SQL/私密参数**（AGSK_4）。

### 3.5 持久化与审计容量（对齐 M5-4 §4.5 / M5-5 §4.3）

- `skill-runs.json` / `agent-runs.json`：**独立文件，500 上限 FIFO**（不复用 `audit.json` 的 1000 上限，避免刷爆）。
- 审计只记 `id`/`version`/`acl`/`capabilities`/`result`；**禁记输入/响应正文**（K3 + 体量防爆，对齐 A4 的 `git_write_audit_detail` 范式）。
- `skills_dir()` = `app_data_dir()/mvp-browser-os/skills/<id>/<version>/`；`agents_dir()` 同理；`agent_kv_file()` 复用 `PathResolver`（M5-1 注入）。
- 凭据经 `KeyringStore`（键 `llm:<provider>` / `agent:<id>`），**绝不进 `agent.json`/`skill.json`**。

---

## 4. 实施解锁闸门（A0 何时可签 `M5-4.a`/`M5-5.a` 给 A16）

W0 §10.2 的四条件 + W1 新增两条，全满足方可签发实现 dispatch：

1. **A2 M5-1 core boundary 落地**（`check-core-boundary.py` PASS + 最小 `mvp_core` 可编译、零行为变更、零新依赖）—— 提供 runtime 模块的落点。
2. **A3 M5-2 capability.rs 落地**（全局护栏 + 确认闸门 + `classify_sql_risk` 接入点）—— 提供 `SkillExec`/`AgentDef` 引用的能力白名单单源。
3. A1 M5-4/M5-5 卡已展开（✅ 已存在 `logs/checkpoints/M5-20260906/`）。
4. A0 签署首张实现 dispatch，并指定实施 Lane（候选 A16）。

> A5 的文档工作已完成，不阻塞上述 1/2；A5 在 W1 仅持续提供本增量说明与后续 review 支持。

---

## 5. W1 硬停合规确认

| W1 Hard Stop | A5 是否触碰 | 结论 |
|---|---|---|
| 仅 A2 可动产品代码 | 否（A5 零 `src-*` 改动） | ✅ |
| 禁 `rmcp`/`tokio`/npm/MCP server/Agent runtime/graph/plugin runtime/新 Tauri 命令/ACL 条目 | 否 | ✅ |
| `check-core-boundary.py` 不得被 A5 触发 false positive | 不适用（A5 未写 core） | ✅ |
| 不得移动 NEXT / 不得 push | 否 / 未 push | ✅ |

`git diff --name-only` 在 A5 文件外无任何产品代码改动（见 §7 自检）。

---

## 6. 未来文件清单（实施期由 A16 落，A5 不写）

来源：A1 `M5-4-agent-skill-runtime.md` §3 / `M5-5-agent-skill-commands.md` §3。

- `src-tauri/src/agent_runtime.rs`（新增）— Agent 宿主 + 多方言归一 + 流式（**落 core，禁 `use crate::bridge`**）
- `src-tauri/src/skill_runtime.rs`（新增）— 解析 + 安装 + 执行（**经 `script_runner`**）
- `src-tauri/src/skill_parser.rs`（新增）— `SkillDef` YAML/JSON 解析 + 校验（拒 Inline，K6）
- `src-tauri/src/skills/` `src-tauri/src/agents/`（新增目录）
- `src-tauri/src/domain.rs`（增量）— `AgentDef`/`SkillDef`/`SkillExec`/`AclLevel`/`AgentRunRecord`/`SkillInstallState`
- `src-tauri/src/capability.rs`（扩展）— `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`（与 MCP/A2A/Plugin 共用一份）
- `src-tauri/src/workspace.rs`（修改）— `skills_dir()`/`agents_dir()`/`agent_kv_file()`（复用 `PathResolver`）
- `src-tauri/src/bridge.rs`（修改）— 15 条命令 + 每段 `check_invocation_source`
- `src-tauri/permissions/default-commands.toml`（修改）— 15 条插 `list_artifact_images` 之前
- `src/bridge.ts` `src/types.ts`（新增镜像）— 15 条命令 + 类型
- `scripts/check-agent-skill-policy.py` + `scripts/check-agent-skill-ui-logic.mjs`（W0 §11，实施期补）

---

## 7. 验证锚点（实施期由 A16 跑，A5 仅记录口径）

取自 M5-4 §7 / M5-5 §7 PASS_CRITERIA，供 A0 验收时对照：

| # | 判据 | 验证命令（节选） |
|---|---|---|
| K6 | 解析器拒 `InlineScript`/`RawShell` | `cargo test` 单测 N1/N2 |
| 无第二路径 | `ExternalCli` 走 `script_runner`，编译期阻断直起子进程 | 单测 N5 |
| 能力单源 | `capability.rs` 仅一份（MCP/Skill/Agent/Plugin 共用） | `grep` 多定义检查 |
| 流式节流 | ≥50ms / 16KB flush | 单测 N6（1s 10000 chunk → emit ≤20） |
| 卸载清目录 | Agent/Skill 删除删目录 | 单测 N7 |
| 凭据不入配置 | `system_prompt`/`skill.json` 含 token 串 → 拒 | 单测 N3 |
| 运行明细上限 | `skill-runs.json`/`agent-runs.json` 500 | python 计数脚本 |
| ACL 末条 | 仍为 `list_artifact_images` | `grep -n list_artifact_images` |
| 门禁 | `bash scripts/pre-merge.sh` ALL_PASS | 含 `check-core-boundary.py` + `check-agent-skill-policy.py` |

---

## 8. 输出模板回填

```text
LANE=A5
STATUS=PASS_WITH_DOCS
WAVE=M5-W1 (SUPPORT DOCS ONLY)
BASE=5ca8f9f
HEAD=docs only (logs/assist/A5-M5-agent-skill-W1-delta-20260906-0900.md + A5-M5-agent-skill-20260906-0800.md)
FILES=logs/assist/A5-M5-agent-skill-W1-delta-20260906-0900.md
VERIFY=无产品代码改动；git diff --name-only 在 A5 文件外为空；事实锚点 script_runner.rs:843/882、scheduler.rs:651/681、M5-4/M5-5 卡（logs/checkpoints/M5-20260906/）
CHECKPOINT=logs/assist/A5-M5-agent-skill-W1-delta-20260906-0900.md
MERGE_NOTES=W1 SUPPORT DOCS ONLY；无 patch；M5-4/5 实现责任候选 A16（A0 签发）；A5 红线=Skill 复用 M2-4 执行（AGSK_1），绑定 A2 check-core-boundary.py；依赖 M5-1(A2)+M5-2(A3)
NEXT=A2 落 M5-1 core boundary → A3 落 M5-2 capability.rs → A0 签 M5-4.a/M5-5.a 交 A16 实施
```
