# M5-4 Agent/Skill runtime（`AgentDef`/`SkillDef` + 流式）

> 子卡 ID：**M5-4** · 需求 #12 · `[S3|LEVERAGE:3|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A16**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L566（`M5-4 Agent/Skill runtime`）
> 主预研：`logs/assist/M5-12.a-prework-20260902-1055.md`（SkillDef / AclLevel / 执行体禁 Inline）
> 配套：`M5-5-agent-skill-commands.md`（命令与权限）· `M5-2-rmcp-mcp-policy.md`（capability.rs 共用）

---

## 0. 编号与锚定

- 批次任务号 `M5-4`；需求号 #12；WBS L566 一致。
- 依赖：M5-1 ✅（domain 落 core）+ M5-2 ✅（capability.rs 落地）+ M2-4 `run_script` ✅
- 与 M5-2 关键共用：能力白名单（`capability.rs`）只此一份（防漂移）

---

## 1. GOAL

冻结 `AgentDef` / `SkillDef` / `AgentRunRecord` 等核心类型；实现 `agent_runtime.rs`（Agent 宿主 + 多方言归一 + 流式回传）与 `skill_runtime.rs`（Skill 解析 + 安装 + 执行）；流式回传首期走 `app.emit` + 节流（≥ 50ms 批量，与 M3-4.b 同款静默思路）。**Skill 执行体只引 Script/Command 引用，禁内联 shell 字符串**（K6）。

---

## 2. READ

1. `logs/assist/M5-12.a-prework-20260902-1055.md`（**全读**）
2. `M5-2-rmcp-mcp-policy.md` §4.2-4.3（capability.rs + 判定顺序，**共用**）
3. `src-tauri/src/script_runner.rs`（执行体复用 `run_script`）
4. `src-tauri/src/bridge.rs:688-790`（两段式确认闸门）
5. `src-tauri/src/domain.rs`（契约根，**全读**）
6. `src/components/browser/AINavPanel.vue`（前端 AI 面板既有实现，**全读**）
7. `src-tauri/src/workspace.rs`（路径解析 + `skills_dir()` / `agent_kv_file()` 新增）
8. `M5-3-a2a-bidir-agent-kv.md`（agent_kv 与本卡共用）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/agent_runtime.rs` | **新增** | Agent 宿主 + 多方言归一 + 流式 |
| `src-tauri/src/skill_runtime.rs` | **新增** | Skill 解析 + 安装 + 执行 |
| `src-tauri/src/skill_parser.rs` | **新增** | SkillDef YAML/JSON 解析 + 校验 |
| `src-tauri/src/skills/` | **新增目录** | `skills_dir()` 默认 `~/.local/share/.../skills/` |
| `src-tauri/src/agents/` | **新增目录** | 同上，agents_dir() |
| `src-tauri/src/domain.rs` | 新增类型 | `AgentDef` / `SkillDef` / `AgentRunRecord` / `SkillInstallState` / `AclLevel` / `SkillExec` |
| `src-tauri/src/capability.rs` | 扩展 | 加 `SKILL_CAPABILITY_V1` 列表（首期空，按需逐个评估添加） |
| `src-tauri/src/workspace.rs` | 修改 | 加 `skills_dir()` / `agents_dir()` / `agent_kv_file()`（复用 `PathResolver` trait 来自 M5-1） |
| `src/types.ts` | 新增类型 | TS 镜像 `AgentDef`/`SkillDef`/`AgentRunRecord` |

---

## 4. 关键契约

### 4.1 `AgentDef`（核心 schema）

```rust
pub struct AgentDef {
    pub id: String,                  // 唯一，反向域名规范（与"反向域名"目录结构对应）
    pub version: String,             // semver
    pub display_name: String,
    pub description: String,
    pub dialect: AgentDialect,       // M5-3 §4.2
    pub system_prompt: String,       // 不含凭据
    pub default_capabilities: Vec<CapabilityRef>,  // 引用 capability.rs 常量
    pub a2a: A2aConfig,              // 可被 A2A 委派 + 可委派他人的开关
    pub metadata: serde_json::Value, // 自由扩展（schema_version 走 atomic_write 写入）
}
pub struct A2aConfig {
    pub delegate_to: bool,           // 允许本 Agent 委派给其他 Agent
    pub delegated_from: bool,        // 允许其他 Agent 委派给本 Agent（影响 inbound A2A 校验）
}
```

**Agent 存盘**：`app_data_dir()/mvp-browser-os/agents/<id>/agent.json`（每 Agent 独立目录，便于多版本并存）。

### 4.2 `SkillDef`（核心 schema）

```rust
pub struct SkillDef {
    pub id: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub acl: AclLevel,               // Safe / Confirm / Dangerous
    pub exec: SkillExec,             // 仅 ScriptRef/CommandRef/Sequence
    pub inputs: Vec<SkillInput>,     // 必填字段
    pub capabilities: Vec<CapabilityRef>,
    pub tests: Vec<SkillTest>,       // 自检用例（可选）
    pub metadata: serde_json::Value,
}
pub enum AclLevel { Safe, Confirm, Dangerous }
pub enum SkillExec {
    ScriptRef { script_id: String, params: serde_json::Value },     // M2-3 已存
    CommandRef { command_id: String, params: serde_json::Value },  // M2-3 已存
    Sequence { steps: Vec<SkillExec> },                              // 串联
    // 【禁】InlineScript、RawShell、SystemCommand 等"内联"形态——K6
}
```

**K6 红线**：禁止任何"内联 shell 字符串"形态。Skill 只是一层"脚本/命令 + 输入/权限/UI 包装"，**不可承载新执行体**。

**Skill 存盘**：`app_data_dir()/mvp-browser-os/skills/<id>/skill.json`（多版本并存于 `<id>/<version>/`）。

### 4.3 流式回传契约

- 通道：`app.emit("agent://<agent_id>/stream", payload)`（Tauri event）
- 节流：≥ 50ms 批量 + 累计 ≤ 16 KB 强制 flush
- 心跳：每 5s 一次（前端连接断时显示"打字中"）
- 终止：`app.emit("agent://<agent_id>/done", result)` 或 `/error` 或 `/canceled`
- 与终端同款：前端 `bridge.ts` 收 `event`，转 `useAIStore.addStream`（既有 store 加方法）

### 4.4 多方言归一（agent_runtime.rs）

| 方言 | 适配 | 流式 |
|---|---|---|
| `OpenAiCompatible { base_url, model }` | 直连 HTTP/SSE（reqwest + eventsource） | SSE chunked 解析 |
| `ExternalCli { program }` | 落 Script/Command 引用，**走 script_runner** | 用 `run_script` 既有的 stdout 行式回传 |

**禁**为 Agent 引入第二执行路径；**禁**Agent 直接 `tokio::spawn` stdio 进程。

### 4.5 安装与 ACL

- Skill 安装分三态：
  - `Safe`：装入 `skills_dir()` 即可生效
  - `Confirm`：必须用户在前端弹窗确认（两段式闸门）
  - `Dangerous`：必须 keyring 二次确认 + UI 风险提示
- 安装后写 `skill-runs.json`（独立文件，500 上限）
- 删除 / 升级 / 重装都触发同一闸门

---

## 5. FORBID

- **不**让 Skill `exec` 含任何"内联 shell 字符串"形态（K6）
- **不**为 Agent 引入第二执行路径（ExternalCli 必须走 `script_runner`）
- **不**让流式回传不节流（防事件刷爆；与终端 M3-4.b 同款）
- **不**让 `capability.rs` 漂移（A2P / A2A / Skill / Plugin / Agent 共用一份）
- **不**让凭据进入 `agent.json` / `skill.json`（keyring 存 token）
- **不**破 K1（ACL 末条恒为 `list_artifact_images`）/K3/K5/K6
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
ls src-tauri/src/skills src-tauri/src/agents 2>&1
grep -rniE "AgentDef|SkillDef|skill_runtime" src-tauri/src | head

# B. capability.rs 共用
grep -n "MCP_CAPABILITY_V1\|SKILL_CAPABILITY_V1\|AGENT_CAPABILITY_V1" src-tauri/src/capability.rs

# C. 反向用例
# N1: SkillDef.exec 写 InlineScript → 解析拒
# N2: SkillDef.exec 写 RawShell → 解析拒
# N3: AgentDef.system_prompt 含 "我的 token 是 sk-..." → 安装前 keyring 检查拒
# N4: 装一个 Safe Skill → 不弹闸门；装 Confirm Skill → 必弹；装 Dangerous → 必弹 + keyring
# N5: ExternalCli 试图直起子进程 → 编译失败（被 refactor 拒）
# N6: 流式回传每秒 1 万 chunk → 至少 50ms 批量（事件数 ≤ 20）
# N7: Agent 删除后目录残留 → 卸载必须删目录

# D. 流式节流验证
python3 - <<'PY'
# 模拟 1s 10000 chunk；期望窗口内 emit ≤ 20
PY

# E. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# F. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | `AgentDef` / `SkillDef` / `SkillExec` / `AclLevel` 落 `domain.rs` | `grep` 命中 |
| 2 | K6 红线：解析器拒 InlineScript / RawShell 等 | 单测 N1/N2 |
| 3 | `capability.rs` 唯一一份（MCP / Skill / Agent / Plugin 共用） | 命令 B |
| 4 | ExternalCli 走 `script_runner`，编译期阻断直起子进程 | 单测 N5 |
| 5 | 三档 ACL 安装闸门行为正确 | 单测 N4 |
| 6 | 流式回传节流 ≥ 50ms / 16 KB flush | 单测 N6 |
| 7 | Agent/Skill 卸载清目录 | 单测 N7 |
| 8 | 凭据不入 Agent/Skill 配置文件 | 单测 N3 |
| 9 | `skill-runs.json` 独立上限 500 | 命令 D |
| 10 | `cargo test` 全绿 | 命令 E |
| 11 | `pre-merge.sh` ALL_PASS | 命令 F |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| Skill `exec` 出现 InlineScript | **K6 红线失守**：回 design 重写 |
| Agent 绕过 `script_runner` | **第二执行路径**：与 A6 冻结冲突，阻断 |
| `capability.rs` 在两处定义 | 阻断：必须收口 |
| 流式不节流 | 阻断：M3-4.b 静默事件先例 |
| ACL 闸门不弹 | 阻断：危险操作必须有确认 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L566 `[ ]` → `[x]`
2. `后续需求TODO.md` §12 状态 `PARTIAL`（留 `M5-5/6`）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-5`
4. `logs/checkpoints/M5-4.a-2026MMDD-HHMM.md`
5. `M5-14-debt-ledger.md` 增项：LLM 用量配额；流式断线重连

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A16 实施填
- **NEXT**：M5-5（命令与权限），同 A16 拆卡

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
- 与 A9 既有契约文档无文件冲突
