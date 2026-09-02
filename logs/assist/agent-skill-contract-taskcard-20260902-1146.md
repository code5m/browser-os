# agent-skill-contract-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §7.1 **M5-4 ~ M5-6**（需求 #12 Agent 与 Skill 生态）
> 状态：⏸ **任务卡 / 未实现运行时 / 未执行验收 / 不宣称 PASS**
> 关联：`M5-12.a-prework-20260902-1055.md`（SkillDef 草案）· `M5-7.a-prework-20260902-1055.md`（能力白名单，**两处复用同一份**）· `script-execution-safety-taskcard-20260902-1146.md`（危险参数必须复用）

---

## 1. 目标

整理 M5 Agent-Skill 契约卡：**SkillMeta、输入输出、权限声明、安装/启用/禁用、版本、失败态、审计**。

| 概念 | 定位 |
|---|---|
| **Skill** | 一个可被调用的能力单元；**只能引用已登记的 `ScriptMeta` 或纯计算逻辑**，不得自带可执行代码 |
| **Agent** | 编排多个 Skill 完成目标的实体（外部进程，走 `A2P-A2A-protocol-taskcard`） |

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 现状 |
|---|---|
| Skill 运行时 | ❌ 无 |
| `ScriptMeta` 领域模型 | ❌ `domain.rs` 中无（见 `M2-3.a`） |
| `run_script` 执行通道 | ❌ 未实现（见 `script-execution-safety-taskcard`） |
| Skill 命令 | ❌ 59 个白名单命令中无 `skill_*` |
| 前端 AI 参照 | ✅ `src/components/browser/AINavPanel.vue` |
| 能力白名单 | ⚠️ 仅存在于 `M5-7.a` 文档草案，**无代码** |
| 退出收口 | ❌ 无 `ShutdownCoordinator`（依赖 TASK-10） |
| 持久化 | ✅ `workspace.rs` 范式（非原子写 ⚠️） |

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| `src-tauri/src/domain.rs` | 新增 `SkillMeta` / `SkillInstall` / `SkillRunRecord` | 必须 |
| **新增** `src-tauri/src/skill.rs` | Skill 注册表：安装/启用/禁用/版本/校验 | 必须 |
| `src-tauri/src/bridge.rs` | 新增 `skill_list` / `skill_install` / `skill_enable` / `skill_disable` / `skill_invoke` / `skill_runs` | 必须 |
| `src-tauri/permissions/default-commands.toml` | 新增命令（**K1**） | 必须 |
| **新增/改** `src-tauri/src/workspace.rs` | `skills.json`（**原子写**） | 必须 |
| `src-tauri/src/shutdown.rs` | 注册 `SkillShutdown`（取消 in-flight） | 必须（依赖 TASK-10） |
| `src-tauri/src/capability.rs`（与协议共用） | 能力白名单与权限判定 | 必须 |

---

## 4. 契约 / 数据结构

### 4.1 `SkillMeta`

```rust
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    /// 稳定标识：反向域名风格，如 "com.example.git.batch-status"
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,                    // semver，如 "1.2.0"

    /// 作者/来源（用于审计与 UI 展示）
    pub author: Option<String>,
    /// 来源类型
    pub source: SkillSource,                // Builtin | Local | Registry

    /// 输入契约
    pub inputs: Vec<SkillParam>,
    /// 输出契约（仅声明形状，不做运行时强校验）
    pub outputs: SkillOutput,

    /// 声明的能力（必须与 inputs 的实际用途一致）
    pub capabilities: Vec<String>,          // 如 ["fs:read", "shell:execute"]

    /// 实现方式：只能二选一
    pub implementation: SkillImpl,

    pub timeout_sec: u64,                   // 默认 60
    pub dangerous: bool,                    // 是否高危（UI 标红 + 需二次确认）
    pub tags: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource { Builtin, Local, Registry }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillImpl {
    /// 引用已登记的脚本（推荐；复用 run_script 全部安全校验）
    Script { script_id: String },
    /// 宿主内建纯计算（不执行外部进程）
    Builtin { handler: String },
    // ❌ 明确不提供 Inline { code } —— 禁止 Skill 自带可执行代码
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkillParam {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: SkillParamType,               // string|number|bool|path|enum|password
    pub required: bool,
    pub default: Option<String>,
    pub options: Option<Vec<SkillParamOption>>,
    pub pattern: Option<String>,
    /// 危险参数（复用 script-execution-safety 的同一套判定）
    pub dangerous: bool,
    pub danger_reason: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillParamType { String, Number, Bool, Path, Enum, Password }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct SkillParamOption { pub value: String, pub label: String }

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkillOutput {
    #[serde(rename = "type")]
    pub kind: SkillOutputType,              // text|json|markdown|file
    pub schema: Option<serde_json::Value>,  // kind=json 时的 JSON Schema（可选）
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillOutputType { Text, Json, Markdown, File }
```

**红线**：`SkillImpl` **不提供** `Inline { code }` 变体。Skill 携带可执行代码 = 绕过 `script-execution-safety` 的全部防线。

### 4.2 安装记录

```rust
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SkillInstall {
    pub meta: SkillMeta,
    pub enabled: bool,
    pub installed_at: String,               // UTC ISO8601
    pub updated_at: String,
    /// 用户授予的能力（⊆ meta.capabilities）
    pub granted_capabilities: Vec<String>,
    /// manifest 文件 sha256，防篡改
    pub checksum: String,
    /// 安装来源路径（Local 时为绝对路径）
    pub origin: Option<String>,
}
```

### 4.3 输入输出契约

| 阶段 | 规则 |
|---|---|
| **输入校验** | 复用 `script-execution-safety-taskcard` §4.4 的 `validate_param`：**类型 → 正则 → 路径 canonicalize → 长度 → NUL**，fail-closed |
| **参数传递** | 数组传递，键名不进命令行；`password` 类型走 keyring 引用 |
| **输出** | 流式事件 `skill-output`（同 `script-output` 契约：8 KB/片，30 ms 合并，5000 行/4 MB 上限 + `truncated`） |
| **终态** | `skill-finished`：`{ runId, status, exitCode, durationMs, truncated, error? }` |
| **输出类型** | `SkillOutput.kind` 仅影响**前端渲染方式**（text/markdown/json 高亮/file 下载），**不影响**运行时校验 |

### 4.4 权限声明与判定

```
判定链（与 A2P/A2A 共用同一份 capability.rs）：
  1) Skill 声明 capabilities ⊆ 已知白名单？        否 → 安装时拒绝
  2) 用户是否授予（granted_capabilities）？         否 → fail-closed 拒绝执行
  3) 本次参数是否在 scope 内？                      否 → 拒绝
  4) dangerous / capabilities 含 exec|net？         是 → 每次调用前二次确认
```

| 能力 | 默认授予 | 说明 |
|---|---|---|
| `fs:read` | 安装时询问 | 需路径 scope |
| `fs:write` | 安装时询问 | 需路径 scope |
| `shell:execute` | **默认不授予** | 需用户显式开启 + 每次调用二次确认 |
| `net:fetch` | **默认不授予** | 需域名白名单 |
| `db:query` | 安装时询问 | 只读优先 |
| `ui:notify` | 自动 | 低危 |

**fail-closed**：声明未授予 → 拒绝；**不降级**、不「只给只读权限凑合」。

### 4.5 安装 / 启用 / 禁用 / 卸载

| 操作 | 行为 | 审计 |
|---|---|---|
| **安装** | 校验 manifest（schema + checksum + capabilities ⊆ 白名单）→ 写 `skills.json`（原子写）→ 默认 `enabled = false`（**安装后不自动启用**） | ✅ |
| **启用** | 展示声明的能力清单 → 用户勾选授予 → `granted_capabilities` 落盘 | ✅ |
| **禁用** | `enabled = false`；**in-flight 执行不受影响**（跑完） | ✅ |
| **卸载** | 若有 in-flight → 先取消；删除记录 + 撤销授予 | ✅ |
| **升级** | 校验新版本 → 若 `capabilities` **扩大** → 必须重新征求授权；缩小则自动 | ✅ |
| **降级** | 允许；同样按能力变化处理 | ✅ |

**红线**：安装后**不得自动启用**（避免「装上就被 Agent 调用」）。

### 4.6 版本

| 项 | 规则 |
|---|---|
| 格式 | semver `MAJOR.MINOR.PATCH` |
| 兼容判定 | `MAJOR` 相同视为兼容；`MAJOR` 变化视为破坏性（需重新安装授权） |
| 同 id 多版本 | ❌ **不支持并存**（避免歧义）；升级即替换 |
| checksum | manifest 内容 sha256；变化 → 提示「清单已变更，请重新确认」 |
| 追溯 | 审计记录含 `skillId + version` |

### 4.7 失败态

| 场景 | 表现 | 用户可见 |
|---|---|---|
| manifest schema 非法 | 安装拒绝 + 逐字段错误 | ✅ |
| checksum 不符 | 拒绝安装，提示「清单已损坏或被篡改」 | ✅ |
| 声明未知能力 | 安装拒绝，列出未知能力名 | ✅ |
| 引用的 `script_id` 不存在 | **安装时**校验并拒绝；若脚本后被删除 → 调用时报可读错误并自动禁用 | ✅ |
| 参数校验失败 | 调用前拒绝，逐参数给出原因 | ✅ |
| 能力未授予 | 调用前拒绝 + 审计 + 显示「已阻止」页（同 `plugin-permission-taskcard` §4.4） | ✅ |
| 执行超时 | `status = Timeout`，输出保留 | ✅ |
| 执行被取消 | `status = Cancelled`，**不重试** | ✅ |
| 输出超限 | `truncated = true`，顶部提示 | ✅ |
| Skill 进程崩溃 | `status = Failed`，`error` 含退出码 | ✅ |

**统一原则**：任何失败都返回**结构化错误**（`{ code, message, retriable, details }`），不抛裸字符串、不 `unwrap`。

### 4.8 审计

| 事件 | 落哪里 | 内容 |
|---|---|---|
| 安装 / 启用 / 禁用 / 卸载 / 升级 | `audit.json` | skillId、version、能力变化 |
| 能力被拒 | `audit.json` | skillId、capability、traceId |
| **每次调用开始/结束** | ⚠️ **独立 `skill-runs.json`**（**K5**） | runId、skillId、version、**脱敏参数**、耗时、状态 |
| 危险参数二次确认 | `audit.json` | skillId、参数名（**不回显值**） |
| 输出正文 | ❌ 不记 | 隐私 + 体积 |

**红线（K3）**：`password` 类型参数在审计与历史中一律 `***`。

---

## 5. 实现要点（步骤化）

1. **先完成依赖**：`ShutdownCoordinator`（TASK-10）+ `run_script`（TASK-12）+ 能力白名单（与 A2P/A2A 共用）。
2. `domain.rs` 加 §4.1 / §4.2 结构。
3. 新 `skill.rs`：注册表（安装/启用/禁用/卸载/升级）+ manifest 校验 + checksum。
4. **能力白名单抽成 `src-tauri/src/capability.rs`**，供 Skill 与 A2P/A2A **共用**（避免两份漂移）。
5. `bridge.rs` 加 6 个命令 → `default-commands.toml`（**K1**）。
6. 持久化用**原子写** + 损坏备份（**K6**）。
7. 注册 `SkillShutdown`。
8. 前端：Skill 列表 / 详情 / 参数弹窗（复用 `M2-script-library-ui-static-shell` 的组件形态）。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ `SkillImpl::Inline { code }` | 绕过全部执行安全防线 |
| 2 | ❌ Skill 自己拼命令行 | 必须走 `run_script` |
| 3 | ❌ 安装后自动启用 | 用户未授权即被调用 |
| 4 | ❌ 能力未授予时降级放行 | fail-closed |
| 5 | ❌ 审计/历史中出现 password 明文 | K3 |
| 6 | ❌ 每次调用写 `audit.json` | 刷爆 1000 条上限（K5） |
| 7 | ❌ 非原子写 `skills.json` | 崩溃 = 全丢（K6） |
| 8 | ❌ 能力白名单在 Skill 与 A2A 各写一份 | 必然漂移；必须共用 `capability.rs` |
| 9 | ❌ 新增命令忘进 ACL | K1 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | 三个上游依赖（`ShutdownCoordinator` / `run_script` / 能力白名单）未完成 | 未完成时不得开工 |
| 高 | Skill 变成「绕过脚本安全校验的后门」 | `SkillImpl` 只允许 `Script` / `Builtin`；参数校验复用同一函数 |
| 中 | manifest 被篡改（本地文件可写） | checksum + 变更提示重新确认 |
| 中 | 能力白名单两处漂移 | 抽公共 `capability.rs`（§5.4） |
| 中 | 危险 Skill 被 Agent 静默调用 | 每次调用二次确认（可「本次会话记住」） |
| 中 | 同 id 升级导致旧调用记录语义变化 | 审计记录带 version |
| 低 | Skill 数量增长导致列表慢 | 静态壳阶段不优化；>50 个再评估 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 安装 manifest 缺 `id` | 拒绝安装 + 逐字段错误 |
| R2 | 安装声明未知能力 `foo:bar` | 拒绝安装，列出未知能力 |
| R3 | 安装后未启用就调用 | 拒绝（未启用） |
| R4 | 启用时只授予 `fs:read`，Skill 调 `shell:execute` | fail-closed 拒绝 + 审计 |
| R5 | 修改 manifest 文件后重启 | checksum 不符 → 提示重新确认，不静默加载 |
| R6 | 引用的脚本被删除后调用 | 可读错误 + 自动禁用 |
| R7 | 参数填 `; rm -rf /` | 作为字面量传入（复用 `run_script` 校验） |
| R8 | `password` 参数调用后查审计 | 显示 `***` |
| R9 | 取消一次调用 | `Cancelled`，**不重试** |
| R10 | 超时调用 | `Timeout`，输出保留，进程组回收 |
| R11 | 卸载时有 in-flight 调用 | 先取消再删；无残留进程 |
| R12 | 高频调用 1000 次 | `audit.json` 不爆（明细在 `skill-runs.json`） |
| R13 | 宿主退出时有 in-flight | 走 `SkillShutdown`，无残留 |
| R14 | `skills.json` 半截损坏 | 备份为 `.corrupt`，不静默清空，应用可启动 |
| R15 | Skill 输出 100 MB | 截断（`truncated = true`），不 OOM |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 结构存在
grep -n "SkillMeta\|SkillInstall\|SkillImpl" src-tauri/src/domain.rs     # 期望命中

# 9.2 禁止 Inline 代码
grep -n "Inline" src-tauri/src/domain.rs src-tauri/src/skill.rs | wc -l  # 期望 0

# 9.3 只走 run_script
grep -rn '"\-c"' src-tauri/src/skill.rs | wc -l                          # 期望 0
grep -n "run_script\|script_runner" src-tauri/src/skill.rs               # 期望命中

# 9.4 能力白名单共用（防漂移）
ls src-tauri/src/capability.rs                                            # 期望存在
grep -rn "capability::" src-tauri/src/skill.rs src-tauri/src/protocol/    # 期望命中

# 9.5 安装后不自动启用
grep -n "enabled" src-tauri/src/skill.rs | head                           # 期望默认 false

# 9.6 密码脱敏（K3）
grep -n '"\*\*\*"' src-tauri/src/skill.rs                                 # 期望命中

# 9.7 原子写（K6）
grep -n "atomic_write\|rename\|corrupt" src-tauri/src/skill.rs            # 期望命中

# 9.8 命令已进 ACL（K1）
grep -c "skill_list\|skill_install\|skill_enable\|skill_disable\|skill_invoke\|skill_runs" \
  src-tauri/permissions/default-commands.toml                             # 期望 6

# 9.9 退出收口
grep -n "SkillShutdown" src-tauri/src/shutdown.rs                         # 期望命中

# 9.10 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 baseline 13 warning
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 出现 `SkillImpl::Inline` | 立即移除该变体；这是绕过执行安全的 P0 缺陷 |
| 能力白名单出现两份 | 合并为公共 `capability.rs`；不得「先上线后合并」 |
| 安装后自动启用 | 改为默认禁用；不得「方便起见」自动开 |
| 校验缺失（只有前端） | 补后端 fail-closed 校验 |
| 明文库/审计泄露 | 立即修复并清理已有记录（K3） |
| `skills.json` 静默清空 | 补 `load_or_backup`（K6） |
| clippy warning 增加 | 对照 baseline 回退 |

---

## 11. 推荐模型

`AI:DEEP`（契约 + 权限 + 生命周期 + 与 A2P/A2A 共用能力层）。
**人工验收必做**：R4（权限拒绝）、R5（篡改）、R8（脱敏）、R13（退出收口）。
