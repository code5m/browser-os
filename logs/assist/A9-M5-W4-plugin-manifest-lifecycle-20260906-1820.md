# A9 · M5-W4 Delta — 插件 manifest/lifecycle 对齐 A5 Agent/Skill 策略（已对照 A5 实际产物校正）

> 生成：2026-09-06 18:20 CST · 校正于 18:35（对照 A5 实际落地的 `agent.rs`/`skills.rs`/`security_policy.rs`/`check-agent-skill-policy.py`）
> Lane A9（M5-W4 · SUPPORT DOCS ONLY · 整包交付）
> BASE：`f7ad35a`（HEAD：`docs(M5): dispatch W4 agent memory and skill lanes`；本地已与 `origin/master` 同步，`git pull --ff-only` 无新）
> 性质：**对齐 delta**，零产品代码。仅 `logs/assist/A9-M5-W4-*.md`。
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W4 Parallel Dispatch → A9：**SUPPORT DOCS ONLY**；「Align plugin manifest/lifecycle plans to A5 Agent/Skill policy; no plugin product code. Plugin delta note.」
> 配套前序（已随 A0 拣入）：W1 `A9-M5-plugin-W1-delta-20260906-1530.md`；W1 第二切片 `A9-M5-plugin-impl-seam-20260906-1630.md`（已随 `712a14c` 集成）；W3 `A9-M5-plugin-W3-delta-20260906-1730.md`（已 tracked/clean）。
> 权威输入（本次对齐基线）：`M5-10-plugin-manifest-lifecycle.md` + `M5-4-agent-skill-runtime.md` + `A5-M5-agent-skill-W1-delta-20260906-0900.md`；**以及已写入本工作树的 A5 实际产物** `src-tauri/src/agent.rs`、`src-tauri/src/skills.rs`、`src-tauri/src/domain.rs`(L1892-2001)、`src-tauri/src/security_policy.rs`(L2067/2070)、`scripts/check-agent-skill-policy.py`（均 untracked WIP，尚未经 A0 集成）。

---

## 0. 一句话结论

把插件 **manifest schema（M5-10 §4.2）+ 生命周期状态机（§4.3/4.4/4.5）** 对齐到 **A5 Agent/Skill 实际产物**，逐字段同构、确认 `PermissionPreview` 已落地下沉"危险 capability⇒Dangerous 闸门"、并**新发现一处必须 A0 裁决的交叉 lane 不一致**：能力白名单单一真源已碎成 `domain.rs`（MCP）与 `security_policy.rs`（Skill/Agent）两个文件，插件 `PLUGIN_CAPABILITY_V1` 落点因此被阻塞。保持 Ed25519/form③ 显式。无产品代码。

---

## 1. 对齐基线事实（来源 file:line，已实测）

| 事实 | 来源 | 对插件 manifest/lifecycle 的影响 |
|---|---|---|
| A5 类型已落 `domain.rs`：`AclLevel`(L1892)、`SkillExec`(L1904)、`CapabilityRef`(L1920)、`SkillDef`(L1945)、`AgentDef`(L1983)、`PermissionPreview`(L2001) | `domain.rs` grep | 插件 `PluginManifest`/`PluginState`/`PluginSignature` 须落 `domain.rs`（与 A5 同文件），保持契约根一致 |
| `CapabilityRef { id: String }`（仅一个 `id` 字段） | `skills.rs:97` / `agent.rs:78`：`CapabilityRef { id: "file_read".into() }` | 插件 `PluginCapability{capability, reason}` 是超集；建议复用 `CapabilityRef` 作为引用、把 `reason` 作为插件 manifest 独立字段（见 §2） |
| `PermissionPreview { gate: AclLevel, capabilities: Vec<String> }` + `permission_preview()` 返回 `gate: AclLevel::Confirm` | `domain.rs:2001` / `agent.rs:38-41` | **确认**本 delta §3 的"危险 capability⇒Dangerous"建议已被 A5 实装为共享类型；插件应复用 `PermissionPreview`（见 §3） |
| A5 `SkillDef`/`AgentDef` schema 与 M5-4 卡一致：`id`(反向域名)/`version`(semver)/`acl`/`exec`(K6 禁内联)/`capabilities:Vec<CapabilityRef>`/无凭据 | `domain.rs:1945/1983` + `M5-4` §4.1/4.2 | 插件 `PluginManifest` 同构（见 §2） |
| A5 策略脚本 `check-agent-skill-policy.py` 已落地，同族 A3 `check-mcp-policy.py`：`_agent_skill_present()` 探针 + `AGSK_SECOND_PATH`/`AGSK_INLINE_SHELL` + `c_acl_tail`(K1) + `c_capability_drift` | 本工作树 `scripts/check-agent-skill-policy.py` | 插件 `check-plugin-policy.py` 须镜像 A5 实际脚本（见 §5） |
| **能力单一真源碎片化**：`MCP_CAPABILITY_V1` 在 `domain.rs:1540`；`SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1` 在 `security_policy.rs:2067/2070`（当前均 `&[]`） | grep 双文件 | ⚠️ 违反 D46"一份文件"本意；插件 `PLUGIN_CAPABILITY_V1` 落点被阻塞（见 §4） |
| A3 漂移门 `MCP_CAPABILITY_DRIFT`（查 `domain.rs` 的 `MCP_CAPABILITY_V1`）与 A5 `c_capability_drift`（查 `security_policy.rs` 的 `SKILL/AGENT_CAPABILITY_V1`）**均按名硬编码** | `check-mcp-policy.py:245` / `check-agent-skill-policy.py:62,129` | 插件常量无论放哪都不会被任一门捕获 → 须统一（见 §4） |
| 插件 `PluginManifest`/`PluginCapability`/`PLUGIN_CAPABILITY_V1` **不在** `domain.rs`（grep 0） | grep `domain.rs` | 插件尚未实现，符合本 Lane SUPPORT DOCS 范围 |

---

## 2. manifest schema 对齐（PluginManifest ↔ AgentDef/SkillDef）

| 字段 | A5 Agent/Skill（实际） | 插件 PluginManifest（M5-10 §4.2） | 对齐 |
|---|---|---|---|
| `id` / `version` / `display_name` / `description` | 有（反向域名 / semver） | 有 | ✅ 一致 |
| 凭据 | `system_prompt` 禁含凭据；凭据走 Keyring | 无凭据字段（签名 key 走 `trusted-pubkeys.json`+Keyring） | ✅ 一致 |
| 能力引用 | `CapabilityRef { id: String }`（实际仅 `id`） | `PluginCapability { capability, reason }` | ⚠️ 插件是超集。建议：插件 `capabilities` 复用 `CapabilityRef { id }` 作引用，把 `reason` 作为 manifest 顶层/独立字段（如 `capability_justifications: Map<id,reason>`），既兼容 A5 类型又保留"逐项说明用途" |
| 执行体 | `SkillExec::{ScriptRef,CommandRef,Sequence}`（K6 禁内联） | form③ 声明式（入口 URL + 权限 + JS 钩子，禁独立 webview/stdio） | ✅ 同向（均禁内联/第二执行路径）；插件以"形态③"表达，等价于 K6+AGSK_1 |
| `metadata` | 自由扩展 | 自由扩展 | ✅ 一致 |
| 版本/兼容 | — | `min_app_version`（过期拒绝，N3） | ➕ 插件特有，保留 |
| 完整性 | — | `hash`(sha256) + `signature`(Ed25519) | ➕ 插件特有（分发完整性），保留且 fail-closed |

**结论**：manifest schema 与 A5 同构；仅"能力引用"类型需对齐到 `CapabilityRef`（保留 `reason` 为独立字段）。

---

## 3. 生命周期对齐（Plugin lifecycle ↔ A5 安装/清理/审计范式）

| 维度 | A5 Agent/Skill（实际） | 插件生命周期（M5-10 §4.3/4.4/4.5） | 对齐 |
|---|---|---|---|
| 安装闸门 | `AclLevel::{Safe,Confirm,Dangerous}`；`PermissionPreview{gate,capabilities}` 已落 `domain.rs:2001`，`permission_preview()` 映射能力→闸门（`agent.rs:38`） | `plugin_install`/`plugin_enable` 两段式确认，逐项列 capabilities+来源+风险（§4.5） | ✅ **已确认**：插件应复用 `PermissionPreview` 类型 + 同款 `permission_preview()` 逻辑，"声明危险 capability ⇒ `Dangerous`（keyring 二次确认）"由共享类型统一 |
| 卸载清理 | 删除 ⇒ 必须删目录（N7） | `Uninstalled` = 删 `plugins_dir()/<id>/` | ✅ 一致 |
| 停用保留 | `Disabled` 保留资源但移除 capability，可重启用 | `Disabled` 保留资源但移除 capability（§4.3） | ✅ 完全一致 |
| 审计/持久化 | `skill-runs.json`/`agent-runs.json` 独立 500 FIFO；审计仅 `id/version/acl/capabilities/result`，禁 body | 建议 `plugin-runs.json` 独立 500 FIFO、同审计纪律 | ✅ 同形（M5-10 未明写，建议补入） |
| 凭据 | KeyringStore（键 `llm:<provider>`/`agent:<id>`） | `trusted-pubkeys.json` 经 KeyringStore 保护（SB-11） | ✅ 同形 |
| 路径/URL 守门 | skill_runtime 复用 `security_policy` | 插件触文件系统/URL ⇒ 复用 `security_policy::{check_path_within_roots, redact_sensitive_url}`（A3 `mcp.rs:16` 先例） | ✅ 建议（与 A3 MCP 同单一真源） |
| 无第二执行路径 | AGSK_1：禁 `std::process::Command`/`tokio::spawn` 直起进程（`check-agent-skill-policy.py:AGSK_SECOND_PATH`） | form③：禁独立 stdio/webview；JS 钩子走受控 `bridge.ts` 子集 | ✅ 一致 |

**结论**：生命周期与 A5 范式完全同构；建议把"危险 capability ⇒ Dangerous 闸门（复用 `PermissionPreview`）"与"`plugin-runs.json` 500 审计"两处显式写进 M5-10（见 §7 修订建议）。

---

## 4. ⚠️ 能力单一真源碎片化（domain.rs vs security_policy.rs）—— 需 A0 裁决

D46 本意是四类能力白名单**单一真源、一份文件**。但实测：

| 常量 | 落点 | 现状 |
|---|---|---|
| `MCP_CAPABILITY_V1` | `domain.rs:1540` | A3 M5-2 |
| `SKILL_CAPABILITY_V1` | `security_policy.rs:2067` | A5 M5-4（当前 `&[]`） |
| `AGENT_CAPABILITY_V1` | `security_policy.rs:2070` | A5 M5-5（当前 `&[]`） |
| `PLUGIN_CAPABILITY_V1` | **待定** | 插件（A18 实现期） |

**问题**：
1. **两份文件 = 碎片化**，直接违反 D46"单一真源"。
2. **漂移门各自按名硬编码**：A3 `MCP_CAPABILITY_DRIFT` 只查 `domain.rs` 的 `MCP_CAPABILITY_V1`；A5 `c_capability_drift` 只查 `security_policy.rs` 的 `SKILL/AGENT_CAPABILITY_V1`。插件 `PLUGIN_CAPABILITY_V1` 无论放哪，都不会被任一门捕获。
3. **插件落点被阻塞**：A18 无处可放 `PLUGIN_CAPABILITY_V1` 而不加剧碎片化或被漏检。

**推荐方案（三选一，交 A0 裁决）**：
- **(a) 收口到 `domain.rs`**（推荐，最小 churn）：把 `security_policy.rs:2067/2070` 的 `SKILL/AGENT_CAPABILITY_V1` 迁到 `domain.rs`，与 `MCP_CAPABILITY_V1` 并列；A5 的 `c_capability_drift` 改查 `domain.rs`。符合"契约根在 domain.rs"既有约定。
- **(b) 收口到新建 `capability.rs`**（最干净单一真源）：把四类全迁入新建 `capability.rs`（A3 当初为避依赖争议未建此文件，但 M5 已落地、可建）；A3/A5 两门都改查 `capability.rs`。
- **(c) 维持双文件分工**（MCP↔domain.rs，Skill/Agent/Plugin↔security_policy.rs）：需把插件常量放 `security_policy.rs`，并让两门各自只管自己的文件——**背离 D46，不推荐**。

**无论 (a)/(b)**：漂移门应统一为"任一 `*_CAPABILITY_V1` 全仓仅一处定义"，即把 A3/A5 两门的正则泛化为 `\b[A-Z_]*CAPABILITY_V1\s*[=:]`（一处覆盖四类），避免插件漏检。

> 本 Lane 不直改产品码（见 §7）；上述裁决建议交 A0，插件 `PLUGIN_CAPABILITY_V1` 落点待 (a)/(b) 裁定后由 A18 实施。

---

## 5. 策略脚本对齐（check-plugin-policy.py 镜像 A5 实际脚本）

A5 的 `check-agent-skill-policy.py` 已落地，是 `check-plugin-policy.py`（1630 已设计）的**实体模板**：
- `_agent_skill_present()` 探针 → PENDING 码位"产物存在才守门"；默认扫描 EXIT 0，`--self-test` 转 DEFAULT 进 pre-merge。
- `_CODE_REX`：`AGSK_SECOND_PATH`（`std::process`/`Command::new`/`"-c"`/`tokio::spawn`/`tokio::net`）、`AGSK_INLINE_SHELL`（`InlineScript|RawShell|…`）。
- `c_acl_tail`（K1）：ACL 末条恒 `list_artifact_images`。
- `c_capability_drift`：单一真源（见 §4，当前按名查 `security_policy.rs`）。

**对齐要求**：`check-plugin-policy.py` 必须同族（同样的探针二分 + `_present()` 触发 + 三模式 + pre-merge 挂载），码位命名对齐 A5（如 `PLUGIN_SECOND_PATH`/`PLUGIN_INLINE_SHELL`/`PLUGIN_SIG_BYPASS`/`PLUGIN_CAP_SINGLE_DEF`），并在 §4 裁决后让 `PLUGIN_CAP_SINGLE_DEF` 复用统一后的 `*_CAPABILITY_V1` 单定义守门。

---

## 6. Ed25519 / form③ 显式保留（未变假设）

- **form③**（声明式资源包：入口 URL 走既有 webview、JS 钩子走受控 `bridge.ts` 子集、禁独立 webview/独立 stdio 进程）与 A5 的 K6（禁内联）+ AGSK_1（无第二执行路径）**同向**，不被 W4 任何产物推翻。
- **Ed25519 插件签名 + fail-closed**（`verify_plugin_signature` 失败拒绝加载，不得 `cfg`/`feature`/fallback 绕过，见 1630 `PLUGIN_SIG_BYPASS`）：A5/A3 无重叠主张，保持显式。
- **信任根**（SB-11）：`trusted-pubkeys.json` 经 KeyringStore；首装录 key、公钥不可改、签名绕过=红线。与 A5 的 `KeyringStore` 凭据范式一致。

---

## 7. FORBID（W4 切片）与冲突规避

- 仅写 `logs/assist/A9-M5-W4-*.md`；未碰 `src/`/`src-tauri/`/`scripts/`（含 `pre-merge.sh`）/ACL/三主文档。
- **不编辑 M5-10/11/12 卡**：W4 中 A1 正在 reconcile 卡片（工作树已见 `M5-0/1/1.b/2/3/4` 修改态），且 A5 实际产品码（agent.rs/skills.rs/security_policy.rs/check-agent-skill-policy.py）已在本树，为避免同文件 lane 冲突，本 Lane 仅以 assist 文档表达对齐；§4 的碎片化裁决建议交 A0。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。
- 所有"已落地"陈述以来源 file:line 标注（§1）。

---

## 8. 验收（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# A. 本 Lane 仅增 assist 文档，无产品码改动（A5/A4 的 WIP 不在本 Lane 范围）
git status --porcelain -- src/ src-tauri/ scripts/ | grep -E "A9|plugin" || echo "[无 A9 产品码改动]"
# B. 能力单一真源碎片化证据（MCP@domain.rs，Skill/Agent@security_policy.rs）
grep -rn "CAPABILITY_V1" src-tauri/src/domain.rs src-tauri/src/security_policy.rs
#    期望：domain.rs:1540 MCP_；security_policy.rs:2067 SKILL_；:2070 AGENT_。插件 PLUGIN_ 尚无。
# C. 插件类型尚未实现（grep 0）
grep -n "PluginManifest\|PLUGIN_CAPABILITY_V1" src-tauri/src/domain.rs || echo "[插件未实现，符合范围]"
# D. ACL 末条恒 list_artifact_images（K1）
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：118
# E. 本切片为纯文档
git status --porcelain -- logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md   # 期望：??
```

结果：产品代码工作树无 A9 改动；能力真源碎片化如前（§4）；插件类型未实现；ACL 末条（list_artifact_images:118）未变；本切片为纯文档。

---

## 9. Lane Output Template

```text
LANE=A9
STATUS=PASS
BASE=f7ad35a (HEAD: docs(M5): dispatch W4 agent memory and skill lanes; 已与 origin/master 同步)
HEAD=logs/checkpoints/Lane-A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.patch
FILES=logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md, logs/checkpoints/Lane-A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md
VERIFY=见 §8（docs-only；能力真源碎片化 domain.rs vs security_policy.rs；ACL 末条 list_artifact_images:118 未变；插件未实现）
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md
MERGE_NOTES=见 §10
NEXT=A18 (M5-10/11 runtime) / A19 (M5-12 UI)，在 A0 裁决 §4 能力真源收口后由 A0 签发
```

## 10. MERGE_NOTES

- **冲突**：无。本 Lane 仅写 `logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md` 与同名 checkpoint；与 A2/A3/A4/A5/A6/A7/A8/A10/A11 无文件交集（其余 Lane 的 W4 assist 文档并行存在，互不重叠）。
- **⚠️ 交叉 lane 不一致（新发现，需 A0 裁决）**：能力白名单单一真源已碎成 `domain.rs`(MCP) 与 `security_policy.rs`(Skill/Agent) 两文件，违反 D46；A3/A5 漂移门均按名硬编码，插件 `PLUGIN_CAPABILITY_V1` 落点被阻塞。详见 §4，推荐收口到 `domain.rs`(a) 或新建 `capability.rs`(b) 并泛化漂移门正则。
- **与 A1 的边界**：W4 中 A1 正在 reconcile 卡片（工作树已见 M5-0/1/1.b/2/3/4 修改态）；本 Lane 不编辑 M5-10/11/12 卡（§7 FORBID），对齐内容以 assist 文档表达。
- **对 A18 的实施契约**（不变，重申 + §4 纠正）：插件类型落 `domain.rs`（与 A5 同文件）；`capabilities` 复用 `CapabilityRef` + `reason` 独立字段；生命周期复用 A5 范式（Dangerous 闸门经 `PermissionPreview` + 卸载删目录 + `plugin-runs.json` 500 审计 + Keyring 信任根）；`check-plugin-policy.py` 同族 A5 实际脚本；Ed25519 fail-closed 保留；`PLUGIN_CAPABILITY_V1` 落点待 §4 裁决。
- **本 Lane 插件 docs 链完整就绪**：W0(1100)→W1 delta(1530)→W1 seam(1630, 已集成)→W3 delta(1730, 已拣入)→**W4 delta(1820)**。manifest/lifecycle 已对齐 A5 实际产物，待 §4 能力真源裁决后由 A18/A19 进入实现。

## 11. 交付索引

- `logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md`（本文件）—— 插件 manifest/lifecycle 对齐 A5 Agent/Skill 实际产物 + 能力真源碎片化发现。
- 上游：`A9-M5-plugin-impl-seam-20260906-1630.md`（seam 设计，已集成）· `A9-M5-plugin-W3-delta-20260906-1730.md`（已拣入）· `A9-M5-plugin-W1-delta-20260906-1530.md`。
- 权威基线：`M5-10-plugin-manifest-lifecycle.md` · `M5-4-agent-skill-runtime.md` · `A5-M5-agent-skill-W1-delta-20260906-0900.md` · 已落地 A5 产物 `src-tauri/src/{agent.rs,skills.rs,domain.rs(1892-2001),security_policy.rs(2067/2070)}` + `scripts/check-agent-skill-policy.py` · 已集成 A3 `src-tauri/src/mcp.rs` + `domain.rs:1540` + `scripts/check-mcp-policy.py`。
