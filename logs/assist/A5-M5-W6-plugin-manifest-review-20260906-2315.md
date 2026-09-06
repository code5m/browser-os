# A5 · M5-W6 契约评审：A9 插件 manifest / lifecycle 对照 Agent/Skill 域与 permission preview 假设

> LANE=**A5**（M5-W6 Parallel Dispatch，指挥板 §M5-W6 行 156）：**SUPPORT/REVIEW ONLY** — *Review A9 plugin manifest against Agent/Skill domain and permission preview assumptions; no product code unless fixing docs only*。
> WAVE=M5-W6（A0 于 `4b438ef` 派发；`git pull --ff-only` 已最新，工作树除 A1/A9 并发文档改动外干净）
> 性质：**只读评审 note + 对 M5-10 W6 验收标准的纠错**，零产品代码、零前端改动、不 push。
> 评审对象：A9 的插件 manifest/lifecycle 设计（`logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` 含 W6 验收标准 + `M5-11-plugin-commands-isolation.md` + `logs/assist/A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md`）。
> 权威契约源：A5 W4 落入 `src-tauri/src/domain.rs`（`AgentDef`/`SkillDef`/`SkillExec`/`AclLevel`/`CapabilityRef`/`PermissionPreview`）+ `agent.rs`/`skills.rs` 的 `permission_preview()` + `security_policy.rs` 能力白名单/凭据扫描。已随 `1610939` 集成，W6 期间未变。
> 本文件结论可直接供 A9 W6 落地 M5-10 与 A0 裁定 W4 §4 单一真源时采用。

---

## 1. 评审形态说明

- `grep -rln "PluginManifest|PluginCapability|PLUGIN_CAPABILITY_V1|capability.rs" src-tauri/src/` → **0 命中**：A9 的 W6 实际产品码（`plugin_manifest.rs`/`plugin_lifecycle.rs`/`check-plugin-policy.py`/在 `domain.rs` 追加的 `PluginManifest` 等）**尚未进树**。
- 故本评审对照 **A9 已写的插件 manifest 设计 + M5-10 W6 验收标准**（文档级），属 A5 W6 任务明文范围（"Review A9 plugin manifest against Agent/Skill domain and permission preview assumptions"）。
- **concrete code review 的 unblock 条件**（见 §7）：A9 W6 实际代码落地后，A5 再做逐行评审。

---

## 2. 权威契约锚点（grep 实证，W4 已集成、W6 未变）

`src-tauri/src/domain.rs`（`serde(rename_all="snake_case")`）：

| 类型 | 实际形状 | 行 |
|---|---|---|
| `AclLevel` | `"safe"` / `"confirm"` / `"dangerous"` | 1913-1919 |
| `SkillExec`(tag=`kind`) | `script_ref`/`command_ref`/`sequence` | 1925-1939 |
| `CapabilityRef` | `{ id: String }` | 1942-1945 |
| `SkillDef` | `{id,version,display_name,description,acl:AclLevel,exec,inputs[],capabilities:Vec<CapabilityRef>,tests[],metadata}` | 1965-1983 |
| `AgentDef` | **无 `acl` 字段**；`{...,dialect,system_prompt,default_capabilities:Vec<CapabilityRef>,a2a,metadata}` | 2003-2019 |
| `PermissionPreview` | `{ gate: AclLevel, capabilities: Vec<String> }`（`capabilities` = 已解析能力 id 列表） | 2021-2027 |

`permission_preview()` 语义（**A9 复用假设的真相**）：
- `SkillDef::permission_preview()` → `gate = self.acl`（skill 自带 acl），`capabilities = capabilities[].id`（`skills.rs:45-50`）。
- `AgentDef::permission_preview()` → `gate = AclLevel::Confirm`（**硬编码**，因 AgentDef 无 acl 字段），`capabilities = default_capabilities[].id`（`agent.rs:40-49`）。
- **关键事实**：W4 模型中**不存在 capability→AclLevel 的风险分级**。`SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1` 均为扁平 `&[&str]`，无任何 risk tier。

`src-tauri/src/security_policy.rs` 能力单一真源现状（**W4 §4 碎片化仍 open**）：
- `SKILL_CAPABILITY_V1: &[&str] = &[]`（2067）— 空
- `AGENT_CAPABILITY_V1: &[&str] = &[]`（2070）— 空
- `contains_credential_leak(s:&str)->bool`（2073）
- `check_skill_capabilities(ids)`（2087）/ `check_agent_capabilities(ids)`（2097）：查各自白名单，不在则 `UnknownCapability`。

能力白名单实际分布（grep 实证，**无 `capability.rs` 文件**）：
- `domain.rs:1540` → `MCP_CAPABILITY_V1`（被 `mcp.rs:113/145/150` 使用）
- `security_policy.rs:2067/2070` → `SKILL_CAPABILITY_V1` / `AGENT_CAPABILITY_V1`
- **`src-tauri/src/capability.rs` → 不存在**（`search_file` 0 命中）

验证（本波实跑）：`cargo test --manifest-path src-tauri/Cargo.toml` → **368 passed / 0 failed**（A5 模块 13/13 通过；W6 期间 A7 graph 测试并入，契约稳定）；`grep` 证实树中无插件代码、无 `capability.rs`。

---

## 3. A9 插件 manifest 设计摘要（待评审）

来自 `M5-10-plugin-manifest-lifecycle.md`（含 W6 验收标准，行 13-53）与 `M5-11`：

- `PluginManifest { id, version, display_name, description, min_app_version, entry: PluginEntry, capabilities: Vec<PluginCapability>, hash, signature: PluginSignature, metadata }`
- `PluginEntry { entry_url, icon }`（形态③ 声明式）
- `PluginCapability { capability: String, reason: String }`（W6 AC-1/AC-3 强制 `capability` 字段名 + 非空 `reason`、去重、单插件 ≤5 via `MAX_PLUGIN_CAPABILITIES=5`）
- `PluginSignature { algorithm:"Ed25519", key_id, value, signed_at }`（W6 仅声明，不实装校验）
- `PluginState`：Discovered→Validating→Signed(OK|Failed)→Loaded→Enabled→Disabled→Uninstalled
- `validate_plugin_manifest(m) -> ValidationResult`：纯函数（不调 fs/crypto/network），AC-1 仅列 schema/长度/semver/hash 字节(64-hex) 校验
- `PermissionManifestRule(caps, &CapabilityRegistry) -> PermissionVerdict { allowed, denied }`：AC-3
- 权限预览假设（A9 W5 §2）：插件 `permission_preview() -> PermissionPreview`，由 A6 `PermissionPreviewModal` 复用渲染

**W6 验收标准的关键断言（行 17/40）**：
> "A9 W6 PluginManifest schema / PluginCapability 命名 / permission manifest rules **复用** 同一 **capability.rs** 真源"
> "W6-HS4：A2P/A2A/Skill/Plugin/Agent 五类共用 **capability.rs** 单一真源；A9 W6 加 `PLUGIN_CAPABILITY_V*` 必须**走同一文件**"

---

## 4. 关键发现（F1 为阻断级）

| # | 项 | 发现 | 严重度 | A5 处置 |
|---|---|---|---|---|
| **F1** | **W6 验收标准引用不存在的 `capability.rs`** | M5-10 W6 标准反复要求"A9 加 `PLUGIN_CAPABILITY_V1` 到 `capability.rs`、五类共用 `capability.rs` 单一真源"，并称"`capability.rs` 既有 `MCP_CAPABILITY_V1`"。但 **`src-tauri/src/capability.rs` 不存在**；`MCP_CAPABILITY_V1` 实际在 `domain.rs:1540`，Skill/Agent 在 `security_policy.rs`。W4 §4 碎片化（MCP@domain.rs、Skill/Agent@security_policy.rs）**从未经 A0 收口**，W6 标准却虚构了一个从未创建的 `capability.rs` 作单一真源。 | ❌ **阻断**（标准自身事实错误） | A9 若按字面执行，要么引用幽灵文件、要么新建第 4 个能力片段（`plugin.rs` 内联或新建 `capability.rs`），均违反"单一真源"。**须 A0 先裁 W4 §4**：选定 ONE 文件后 A9 才落 `PLUGIN_CAPABILITY_V1`。详见 §5。 |
| **F2** | `PluginCapability` 形状 vs `CapabilityRef` | W6 AC 强制 `PluginCapability { capability: String, reason: String }`；我的 Agent/Skill 用 `CapabilityRef { id: String }`。能力 id 子字段命名不同（`capability` vs `id`）。 | ⚠ 中 | 形状差异本身可接受（插件附加 `reason` 合理），但**能力 id 子类型必须与 `CapabilityRef.id` 同一语义**，且 `PermissionManifestRule` 必须经**统一** `CapabilityRegistry` 校验，不得再 fork 第三套校验器。推荐 `PluginCapability { cap: CapabilityRef, reason: String }`（编译期复用），否则至少保证 validator 复用统一白名单（见 §5）。 |
| **F3** | 插件 `permission_preview()` 的 `gate` 语义 | A9 假设复用我的 `PermissionPreview` 类型（✅ 对齐）。但 W6/M5-11 §4.3 提"风险等级与 policy 比对""按 capability risk"——**我的 W4 模型无 capability→AclLevel 风险分级**（AgentDef gate 硬编码 Confirm；SkillDef gate=自身 acl）。 | ❌ 高 | 插件（同 Agent，无自有 acl）的 `permission_preview().gate` 必须 = `AclLevel::Confirm`（**镜像 `AgentDef::permission_preview`**），`capabilities` = `PluginCapability[].capability` 解析为 `Vec<String>`。W6 **不得**自创 capability→risk 映射；若需 "Dangerous" 档，属 W4 §4 统一能力分级裁决（A0），非 A9 单边引入。 |
| **F4** | 凭据泄漏扫描缺失 | `validate_plugin_manifest`（AC-1）仅列 schema/长度/semver/hash 校验，**未要求对 `description` / `metadata`(JSON) / `entry_url` 跑 `contains_credential_leak`**；而我的 `SkillDef`/`AgentDef` `validate()` 必跑该扫描（K3 隐私）。 | ⚠ 中 | A9 W6 `validate_plugin_manifest` 必须调用 `security_policy::contains_credential_leak` 于 `description` 与序列化后的 `metadata` 字符串（及 `entry_url` 的 token/Authorization 查询参数）。该扫描是纯函数，符合"不调 fs/crypto/network"约束。 |
| **F5** | 空白名单 fail-closed 一致性 | `PLUGIN_CAPABILITY_V1` 首期空（M5-10 "首期空"），故 `PermissionManifestRule` 默认**拒绝所有**能力（fail-closed），与我的 SKILL/AGENT 空名单一致。 | ⚠ 中 | 须显式：未知能力 → deny；UI（`PluginPermissionPanel`）须渲染"暂无可授权能力"。`check-plugin-policy.py` 须断言 fail-closed。能力 id 比对必须走统一白名单，禁止 `plugin.rs` 内嵌能力字面量（W6-HS4 已要求）。 |
| **F6** | W6 纯策略范围守门 | W6 Hard Stop：A9 **不得** install/uninstall/delete/download/execute 真实插件。AC-1 的 `validate_plugin_manifest` 纯函数 ✅；但 M5-10 §4.3 生命周期含真实 fs 变更（删 `plugins_dir()/<id>/`、注册到 capability.rs）。 | ⚠ 中 | A9 W6 只实现**状态机迁移逻辑**（Discovered→…→Uninstalled）为纯转移，**不执行**真实目录删除/签名验真/注册写。真实 fs 变更属 A18 运行时（post-W6）。A9 须明确在代码注释/checkpoint 标注哪些转移是"策略判定"、哪些 fs 动作被 W6 禁止。 |
| **F7** | `check-plugin-policy.py` 守门 | A9 W5 §5 计划镜像我的 `check-agent-skill-policy.py` 加 `PLUGIN_*` 码 + UI/graph 红线。 | ⚠ 中 | 脚本须断言：① 无内联能力字面量（强制走统一 registry）；② `PermissionPreview`/`AclLevel` 复用（禁第二套权限类型）；③ 无第二执行路径/内联 shell（K6 类比）；④ Ed25519 W6 仅声明不强制；⑤ `metadata`/`description` 凭据扫描存在；⑥ 能力名单不硬编码进脚本本身（复用 registry 位置）。策略脚本三模式（self-test/default/pending）+ pre-merge 挂载。 |
| **F8** | 前端 TS 镜像（A19 后续） | M5-10 §3 列 `src/types.ts` 新增 `PluginManifest` 镜像。 | ◽ 低 | 落地时须复用我的 `PermissionPreview`/`CapabilityRef` 形状；`usePluginStore.pendingConfirms` 复用我两段式确认范式（A9 W5 §2 已对齐）。本 W6 不实现。 |

---

## 5. 修正建议（A9 W6 可直接采用；F1 须 A0 先裁）

### 5.1 F1 — 能力单一真源裁决（最高优先，A0 责任）
W6 验收标准引用的 `capability.rs` 不存在，且 W4 §4 碎片化（MCP@domain.rs、Skill/Agent@security_policy.rs）未收口。二选一，但必须 **ONE 文件**：

- **选项 A（推荐，最低漂移，契合我 W4 注释）**：`PLUGIN_CAPABILITY_V1` 落 `security_policy.rs`（与 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1` 同处，符合 `domain.rs:1908/1941` "单一真源在 security_policy.rs" 的明文）；并将 `check_skill_capabilities`/`check_agent_capabilities`/`check_plugin_capabilities` 收敛为统一 `fn check_capabilities(ids:&[String], whitelist:&[&str]) -> Result<(),PolicyError>`。MCP 仍在 `domain.rs`（M5-2 已落地，暂不迁移，但登记于同一 `CapabilityRegistry` 视图）。A9 W6 不动 MCP。
- **选项 B（若 A0 已决 `capability.rs` 为统一家）**：A0 须**先**创建 `src-tauri/src/capability.rs` 并把 `MCP_CAPABILITY_V1`（从 domain.rs 迁出）+ `SKILL`/`AGENT`/`PLUGIN` 全部迁入，再让 A9 加 `PLUGIN_CAPABILITY_V1`。否则 A9 无法"五类共用 capability.rs"。

> **A5 立场**：无论选 A 或 B，A9 W6 绝不能新建/引用幽灵 `capability.rs`、也不能在 `plugin_manifest.rs` 内嵌能力字面量。W6 验收标准行 17/40/194 的 "capability.rs 单一真源" 表述**须由 A1/A0 修正为实际文件名**（建议改为 `security_policy.rs`，或先补建 `capability.rs` 并迁移），否则 A9 无法达成 "0 drift" 判据。

### 5.2 F2 — `PluginCapability` 形状
推荐：
```rust
#[derive(...)] pub struct PluginCapability {
    #[serde(flatten)] pub cap: CapabilityRef, // { id: String }，与 Agent/Skill 同语义
    pub reason: String,                       // 非空（AC-3 ReasonEmpty）
}
```
若 A9 坚持 W6 字面 `capability: String`，则须保证 `PermissionManifestRule` 调用统一 `CapabilityRegistry::contains(id)`，且 `id` 命名空间与 `CapabilityRef.id` 完全一致（如 `"fs:read"`），不得另起一套字符串常量。

### 5.3 F3 — 插件 `permission_preview()` 最小正确实现
```rust
impl PluginManifest {
    pub fn permission_preview(&self) -> PermissionPreview {
        PermissionPreview {
            gate: crate::domain::AclLevel::Confirm, // 同 AgentDef；无自有 acl
            capabilities: self.capabilities.iter().map(|c| c.cap.id.clone()).collect(),
        }
    }
}
```
`check-plugin-policy.py` 须断言插件 `gate` 不出现 W4 未定义的 "dangerous-by-capability" 推导（除非 A0 已统一能力 risk tier）。

### 5.4 F4 — 凭据扫描补入 `validate_plugin_manifest`
在 AC-1 的纯函数内追加：
```rust
if security_policy::contains_credential_leak(&m.description) { return Err(...) }
if security_policy::contains_credential_leak(&serde_json::to_string(&m.metadata).unwrap_or_default()) { return Err(...) }
// 可选：entry_url 的 token/Authorization 查询参数
```

### 5.5 F6 — 显式隔离运行时动作
W6 代码注释/checkpoint 须标注：`delete plugins_dir()/<id>/`、`register to capability.rs`、`verify_plugin_signature` 为 **A18 运行时**，W6 仅做状态机迁移判定（如 `Signed(Failed)` 不加载、仅置状态 + 写审计）。

---

## 6. 校验（read-only）

```text
git status --short --branch   → ## master...origin/master（干净；A1/A9 并发文档改动不在本评审范围）
grep -rln "PluginManifest|PluginCapability|PLUGIN_CAPABILITY_V1|capability.rs" src-tauri/src/ → 0 命中（A9 W6 产品码未进树）
grep -n "MCP_CAPABILITY_V1" src-tauri/src/domain.rs → 1540（在 domain.rs，非 capability.rs）
grep -n "SKILL_CAPABILITY_V1\|AGENT_CAPABILITY_V1" src-tauri/src/security_policy.rs → 2067 / 2070（空）
grep -n "pub struct PermissionPreview" src-tauri/src/domain.rs → 2024
cargo test --manifest-path src-tauri/Cargo.toml → 368 passed / 0 failed（A5 13/13；域契约稳定）
git diff --check → 无输出（本文件为新增 untracked，仅文档）
# W6 Hard Stop 全遵守：仅写 logs/assist/，未改 src/ / src-tauri/ / scripts/ / 主文档 / ACL
```

---

## 7. 阻塞态 / Unblock 条件

1. **A9 W6 产品码未进树** → 本评审为文档/标准级；concrete code review 待 A9 落地 `plugin_manifest.rs`/`plugin_lifecycle.rs`/`check-plugin-policy.py` + 在 `domain.rs` 追加 `PluginManifest` 等。
2. **F1 须 A0 先裁 W4 §4 单一真源**（选 §5.1 A 或 B）→ 否则 A9 W6 无法锁定 `PLUGIN_CAPABILITY_V1` 落点，W6 验收标准 "capability.rs 单一真源" 系虚构前提。**此点须 A1 修正 M5-10 W6 标准的措辞**（将 "capability.rs" 改为真实文件名或先补建该文件并迁移）。
3. **能力白名单为空** → 插件/agent/skill 当前均 fail-closed（无任何能力可授）；UI 须渲染空态。

→ A9 W6 开工前，A0 应：(a) 裁定 §5.1 单一真源落点；(b) 指令 A1 修正 M5-10 W6 标准中的 `capability.rs` 表述。A9 据此落地 DTOs + `validate_plugin_manifest` + 状态机 + `PermissionManifestRule` + `check-plugin-policy.py`，A5 再做逐行 code review。

---

## 8. Lane 输出模板（A5 · M5-W6）

```text
LANE=A5
STATUS=PASS_WITH_DEBT          # M5-W6：SUPPORT/REVIEW ONLY；评审 note + 对 M5-10 W6 标准的纠错（F1 阻断级：标准引用不存在的 capability.rs）；零产品代码
BASE=4b438ef                  # A0 派发 W6；git pull --ff-only 已最新
HEAD=logs/assist/A5-M5-W6-plugin-manifest-review-20260906-2315.md
FILES=logs/assist/A5-M5-W6-plugin-manifest-review-20260906-2315.md
VERIFY=cargo test 368 passed/0 failed（A5 13/13）；grep 证实无 A9 插件码、无 capability.rs（MCP@domain.rs:1540、SKILL/AGENT@security_policy.rs:2067/2070）；git diff --check 干净；W6 Hard Stop 全遵守
CHECKPOINT=logs/assist/A5-M5-W6-plugin-manifest-review-20260906-2315.md
MERGE_NOTES=评审拦截 3 处关键错配：(F1 阻断) M5-10 W6 验收标准引用不存在的 capability.rs 作"五类单一真源"，且误称 MCP_CAPABILITY_V1 在 capability.rs（实际在 domain.rs:1540）；W4 §4 碎片化(MCP@domain.rs / Skill·Agent@security_policy.rs)未收口，A0 须先裁落点，A1 须修正标准措辞。(F2) PluginCapability{capability:String,reason} vs 我的 CapabilityRef{id} 形状差异，须保证能力 id 语义一致 + 统一 validator。(F3) 插件 permission_preview().gate 必须=Confirm（镜像 AgentDef），W6 不得自创 capability→risk 分级。另 (F4) validate_plugin_manifest 须补 contains_credential_leak 扫描 (F5) 空名单 fail-closed (F6) W6 禁真实 fs/签名动作 (F7) check-plugin-policy.py 守门。A9 W6 代码未进树，concrete review 待其落地（unblock 见 §7）。
NEXT=A0 裁 W4 §4 单一真源（§5.1）+ A1 修正 M5-10 W6 标准 "capability.rs" 措辞；A9 落地 M5-10 W6 产品码 → A5 转 concrete code review。不移动主文档 NEXT、不 push。
```
