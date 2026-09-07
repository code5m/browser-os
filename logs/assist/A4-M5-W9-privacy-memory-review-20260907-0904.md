# Lane A4 · M5-W9 Review Note — A0 `CredentialLeak` 脱敏修复后 全表面隐私/memory 复核（Re-Review）

> Lane=A4 · WAVE=M5-W9 Runtime-Free Polish Dispatch · Status=**REVIEW ONLY**（看板 §M5-W9 行 194：隐私评审，可提 policy 夹具但**不编辑产品代码**）
> W9 镜头：credential redaction、bounded preview payloads、audit contents、no prompt/body persistence、no local secret capture（与 W8 同）。
> W9 焦点：确认 A0 的 `CredentialLeak` Display 修复**生效且覆盖全表面**（Agent/Skill/MCP/Plugin/Graph 可见错误 + 审计文本），并复核 W8 残项 F-W8-2。
> 交付：`logs/assist/A4-M5-W9-privacy-memory-review-20260907-0904.md`（仅文档）。
> 边界真相源：① A4 W4 agent memory KV 契约（5 ACTIVE 码）；② A7 W5 图谱契约（7 ACTIVE 码）；③ A5 W4/W7 Agent/Skill 契约（`domain.rs`/`agent.rs`/`skills.rs`/`security_policy.rs`/`check-agent-skill-policy.py`）；④ A3 W7 MCP 契约（`mcp.rs`/`check-mcp-policy.py`）；⑤ A9 W6 插件契约（`plugin.rs`/`check-plugin-policy.py`）。

## 0. 仓库实况（@ `97118d6`，已 `git fetch origin && git pull --ff-only` 已是最新，工作树 clean）

| 项 | 状态 | 证据 |
|---|---|---|
| HEAD | `97118d6` | git rev-parse |
| 工作树 | clean | git status --short 空 |
| 5 政策脚本自测/默认 | **全 PASS** | agent-memory ACTIVE=5 / graph ACTIVE=7 / plugin ACTIVE=1 PENDING=5 / agent-skill ACTIVE=3 PENDING=5 / mcp ACTIVE=8 PENDING=0 |
| graph UI logic | PASS | 通过 41 / 失败 0（W9 41 = W8 34 + 7，W9 A8 加） |
| A0 修复：CredentialLeak Display 脱敏 | **已落地** | `security_policy.rs:118-120` `CredentialLeak(_s) => write!(f, "...：<redacted>")`（用 `_s` 丢弃密文） |
| A10 修复：AGSK_CREDENTIAL_NOT_ECHOED 守门 | **已落地** | `check-agent-skill-policy.py:250-277`（PENDING 桶，_agent_skill_present=true 时守门）+ 446-448 自测坏样本 |
| A5 既有测试 | PASS（已通过 W8） | `bridge.rs:6322 validate_inner_rejects_credential_leak` / `6349 validate_inner_rejects_skill_credential`；**仅断言 `r.valid==false`，未断言 errors 不含密文（建议 A5 W9 强化）** |
| ACL 末条 K1 | **保真** | `default-commands.toml` 末条 = `list_artifact_images`；9 个 W7+W8 read-only 命令（6 Agent/Skill + 3 MCP）齐 |
| Read-only 桥审计/日志 | **零调用** | bridge.rs:mcp_/agent_/skill_ 命令体 + mcp.rs + plugin.rs 全文无 `log_audit`/`eprintln`/`println`/`info!`/`warn!`/`error!`/`tracing::` |

→ 本评审为**对 A0 修复后 mainline 的具体复核**。W8 F-W8-1 已闭环；F-W8-2 仍 OPEN（见 §3）。

## 1. F-W8-1 闭环复核 — `CredentialLeak` 脱敏

### 1.1 修复证据（mainline `97118d6`）
```rust
// security_policy.rs:118-120
PolicyError::CredentialLeak(_s) => {
    write!(f, "凭据/密钥泄露（禁止进入 Agent/Skill 定义）：<redacted>")
}
```
- **关键点**：模式用 `_s` 而非 `s`（Rust 编译器会警告未使用捕获，但 `_s` 显式丢弃；这是 A0 的脱敏手法，让 Display 文本**永远不接触密文**）。
- 4 处 `Err(PolicyError::CredentialLeak(...))` 调用点（`agent.rs:31/34`、`skills.rs:32/38`、`plugin.rs:237`）仍 `clone` 原文进错误——**这是有意保留**，便于上层 `match` 区分错误来源（仅 pattern 匹配，不读密文）。Display 层不输出 → 密文不出 frontend/审计/日志。

### 1.2 触发链 end-to-end 验证
- 链 `agent_validate(text)` → `agent_validate_inner`（`bridge.rs:6148-6172`）→ `AgentDef::parse(text).validate()` → `Err(PolicyError::CredentialLeak(self.system_prompt.clone()))` → `format!("{e}")` = `"凭据/密钥泄露（禁止进入 Agent/Skill 定义）：<redacted>"` → `ValidationReport::with_errors(vec![...])` → 前端 Toast。
- **结论**：响应里**仅含 `<redacted>` 标记**，无 secret 原文。✓

### 1.3 守门回归 — AGSK_CREDENTIAL_NOT_ECHOED
- `check-agent-skill-policy.py:250-263` 坏样本：扫描 `security_policy.rs` 检测 `CredentialLeak(\w+) => { ...{s} }` 形态 → 若命中返违规。
- 当前 mainline 是 `CredentialLeak(_s) => { ...<redacted> }` → 坏样本不命中 → 通过。✓
- **反向自测**（`scripts/check-agent-skill-policy.py:446-448`）注入 `{s}` 模拟未脱敏形态，**应触发 FAIL**（已注入到 PENDING 桶 self-test 路径，A0/A10 已设）。
- 桶位：PENDING（第 5 项）。A4 建议**升 ACTIVE**（见 §4），因为：(1) 实现已合规且 A0 已加反向自测；(2) 升 ACTIVE 进一步阻止 PENDING→ACTIVE 漂移时漏检。

### 1.4 跨域 CredentialLeak 调用点（mainline 全文）
| 域 | 文件:行 | 文本源 | Display 是否脱敏 |
|---|---|---|---|
| Agent | `agent.rs:31` | `self.system_prompt.clone()` | ✓ 经 `_s` |
| Agent | `agent.rs:34` | `self.description.clone()` | ✓ 经 `_s` |
| Skill | `skills.rs:32` | `self.description.clone()` | ✓ 经 `_s` |
| Skill | `skills.rs:38` | `input.name.clone()` | ✓ 经 `_s` |
| Plugin | `plugin.rs:237` | `s.to_string()`（manifest 文本字段） | ✓ 经 `_s` |

**全部 5 处 CredentialLeak 出口均经 `CredentialLeak(_s) => ...<redacted>` Display**。✓

## 2. 全表面复核（W9 镜头）

### 2.1 Agent/Skill 桥（6 命令）
- 命令：`agent_parse`/`agent_validate`/`agent_permission_preview`/`skill_parse`/`skill_validate`/`skill_permission_preview`（`bridge.rs:6175-6271`）
- 错误：仅 `agent_validate`/`skill_validate` 走 `format!("{e}")`（`bridge.rs:6169`）；其他命令无错误 format 路径。
- 审计/日志：6 命令体**无 `log_audit`/`eprintln`/`println`/`tracing`** → 错误即便含密文（已不含）也不入审计/日志。✓
- 持久化：6 命令体**无 `agent_kv`/`fs::write`/DB** → 错误不入 agent_memory。✓
- **W9 结论**：Agent/Skill 桥全部脱敏。✓

### 2.2 MCP 桥（3 命令）
- 命令：`mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview`（`bridge.rs:6478-6507`）
- 错误：3 命令体**无 `format!`/`write!`/`to_string`**；`?` 传播 `PolicyError` → 经 Display（含 `<redacted>`）→ 出错时仅给脱敏分类文本。
- 审计/日志：`mcp.rs` 全文**无 `log_audit`/`eprintln`/`println`/`tracing`**。✓
- 持久化：MCP 桥纯返回快照/视图（`McpPolicySnapshot`/`McpRegistryEntryView`/`McpDecisionView`），无 I/O。✓
- URL 脱敏：`mcp_capability_preview` 走 `evaluate_mcp_command`（`mcp.rs:85-103`），`returns_url` 走 `redact_sensitive_url`（`security_policy.rs:446`）→ query/fragment/userinfo 均脱敏。✓
- **W9 结论**：MCP 桥全部脱敏。✓

### 2.3 Plugin 桥（**无 tauri::command 暴露**）
- W6/W7 决定：plugin.rs 是 `#[allow(dead_code)]` 纯函数切片，**无 `#[tauri::command]` 暴露到前端**。
- 错误：`InvalidPluginManifest(msg)`/`InvalidPluginSignature(msg)`/`PluginStateTransition(msg)` 经 Display → "插件 manifest 非法：{msg}"；`msg` 来自 `format!("plugin.id 非反向域名规范：{}", m.id)` 等 — **回显的是 id/entry_url/hash/capability 名，非 secret**。CredentialLeak 路径已 §1.4 覆盖。
- metadata 体量错 `"plugin.metadata 超 64KiB"` 固定字符串，**不插入 metadata 文本**（plugin.rs:142）。✓
- 审计/日志：`plugin.rs` 全文**无 `log_audit`/`eprintln`/`println`/`tracing`**。✓
- 持久化：plugin.rs 无 agent_kv/fs/DB。✓
- **W9 结论**：Plugin 桥（纯函数）脱敏合规。✓

### 2.4 Graph 错误（`graph.rs:42-71`）
- `GraphError::SecretInProps` → `"props 含凭据/正文敏感字段（AGRAPH-9）"`（固定分类文本，**不回显 prop key/value**）。✓
- 其他 graph 错误回显 `id`/`label`/`prop key` 等结构字段（无 secret）。
- 图谱桥无 `tauri::command` 暴露（W7 决定）：Graph 当前仅经 UI 渲染（`useGraphStore.ts` + `NodeDetail.vue` + `summarizeNode` 剥离 props 守门）。
- **W9 结论**：Graph 错误全部脱敏。✓

### 2.5 审计文本（read-only 桥）
- **bridge.rs:mcp_/agent_/skill_ 命令体 + mcp.rs + plugin.rs 全文** 实扫无 `log_audit`/`eprintln`/`println`/`info!`/`warn!`/`error!`/`tracing::`。
- bridge.rs 53 处 `log_audit` 全在 terminal/scheduler/git 类运行时面（`git_write_audit_detail_contains_no_credentials_or_path_lists` 等），**与 M5-W9 read-only 桥无关**。
- 审计文本不可能含 secret（read-only 桥零调用）。✓

### 2.6 Frontend 错误捕获面
- `useAgentStore.ts:59` `error.value = e instanceof Error ? e.message : String(e ?? "未知错误")` —— 仅取 message。
- `useDatabaseStore.ts:24/89` 同形态（message 字符串）。
- 关键：前端只展示后端返的错误 message。后端 `CredentialLeak` 经 `format!("{e}")` → message = `"凭据/密钥泄露...：<redacted>"` → 前端仅拿到 `<redacted>`。
- **`useAgentStore.ts:9` 注释**："secret 值不进 store、不落盘、不进审计" → 边界守门已明示。
- **W9 结论**：前端错误捕获面仅展示后端 message（已脱敏），无额外 secret 暴露面。✓

## 3. 跨域扫描：其他可能含密文点（W8 残项 F-W8-2 复核）

### 3.1 F-W8-2：parse 输入无字节上限
- `agent_parse`/`skill_parse` 走 `agent_parse_inner`/`skill_parse_inner`（`bridge.rs`）；`skill_permission_preview_inner` 走 `reject_oversized_def`（`bridge.rs:6238`）。
- 实测：5 个 inner 函数（`agent_parse_inner`/`skill_parse_inner`/`agent_permission_preview_inner`/`skill_permission_preview_inner`/`agent_validate_inner`/`skill_validate_inner`）**全部经 `reject_oversized_def(text, 256*1024, "Agent"/"Skill")?`**（`bridge.rs:6149-6157`）。
- 上限：`AGENT_DEF_MAX_BYTES = SKILL_DEF_MAX_BYTES = 256 * 1024`（256 KiB，bridge.rs:6146-6147）。
- 测试：`reject_oversized_def_guard`（bridge.rs:6379-6387）断言 "大小上限" 关键字。
- **结论**：F-W8-2 **已闭环**（A5 在 W8→W9 之间修复）。✓

## 4. W9 残项与建议

### 4.1 F-W8-1 / F-W8-2 闭环状态
| ID | 描述 | 状态 | 修复点 |
|---|---|---|---|
| F-W8-1 | `CredentialLeak` Display 回显 secret | **CLOSED** | `security_policy.rs:118-120` `_s => ...<redacted>` + `check-agent-skill-policy.py:250` AGSK_CREDENTIAL_NOT_ECHOED 守门 |
| F-W8-2 | parse 输入无字节上限 | **CLOSED** | `bridge.rs:6146-6157` `reject_oversized_def(256 KiB)` + 6379 测试 |

### 4.2 W9 评审新增建议（建议项，非阻断）
- **R-W9-1（建议 A5 升 PENDING→ACTIVE）**：`AGSK_CREDENTIAL_NOT_ECHOED` 当前在 PENDING 桶。Display 已脱敏 + 反向自测已设，建议 A5 升 ACTIVE，进一步阻断回归。**风险**：`PENDING` 桶在 `_agent_skill_present` 为真时**已守门**（=已 ACTIVE 行为），升 ACTIVE 主要是命名/分级对齐，**实质不增加保护**。A5 自行决定。
- **R-W9-2（建议 A5 强化 Rust 回归）**：`bridge.rs:6322 validate_inner_rejects_credential_leak` / `:6349 validate_inner_rejects_skill_credential` 仅断言 `r.valid==false`。建议补：`assert!(!r.errors.join("\n").contains(&secret), "CredentialLeak 错误回显 secret：{:?}", r.errors)`，与 AGSK_CREDENTIAL_NOT_ECHOED 对齐（belt-and-suspenders）。
- **R-W9-3（承 W6/W7 F1 DRY）**：`SENSITIVE_*` 双份（`agent_memory.rs:134`/`graph.rs:18`）+ 图谱容量常量三处拷贝（`domain.rs`/`useGraphStore.ts:24-25`/`graphUi.ts:26-29`）。仍 OPEN。W9 范围内 A0 已对齐 `check-agent-skill-policy.py:446-448` 反向自测常量，但**未统一源代码常量**。建议 W10 或独立卡。

### 4.3 W9 范围内**未发现新漏洞**。所有 read-only 桥（Agent/Skill/MCP/Plugin/Graph）的错误/审计/响应均脱敏；无 prompt/body 持久化；无本地 secret 捕获。

## 5. 解锁/二次复核清单

- [x] A0 `CredentialLeak` Display 修复生效（F-W8-1 闭环）
- [x] A10 AGSK_CREDENTIAL_NOT_ECHOED 守门落地
- [x] 5 政策脚本自测/默认全 PASS
- [x] graph UI logic 41/41 PASS
- [x] Read-only 桥命令体零审计/零日志/零持久化
- [x] Agent/Skill/MCP/Plugin/Graph 错误/响应无 secret 暴露
- [x] F-W8-2 字节上限已加（256 KiB + 测试）
- [x] ACL 末条 K1 保真（list_artifact_images 仍末条）
- [ ] R-W9-1 AGSK_CREDENTIAL_NOT_ECHOED 升 ACTIVE（建议）
- [ ] R-W9-2 Rust 回归补"errors 不含密文"断言（建议）
- [ ] R-W9-3 F1 DRY 常量化（建议）

## 6. 与 W9 Hard Stops 对齐
- "No token/cookie/Authorization/body/prompt-secret in logs, audit, frontend state, checkpoints, or error strings" → **已满足**（read-only 桥零审计/零日志，错误仅含 `<redacted>` 标记）。
- "Build metrics threshold 22%" / "cargo warnings unchanged" → 由 A11 复核（A4 不审）。
- "No MCP server/listener/runtime, no plugin runtime, no skill/agent execution" → read-only 桥仍 6+3 命令，零执行/安装/网络。
- 本 Lane **零产品代码**（仅文档），符合 REVIEW ONLY。
- "May propose policy fixtures but do not edit product code" → R-W9-1/R-W9-2 是建议，未改 `check-agent-skill-policy.py`。
- 未 push。

## 7. LANE 输出模板

```text
LANE=A4
STATUS=PASS（REVIEW ONLY，无产品代码）
BASE=97118d6
HEAD=logs/assist/A4-M5-W9-privacy-memory-review-20260907-0904.md
FILES=logs/assist/A4-M5-W9-privacy-memory-review-20260907-0904.md
VERIFY=git pull --ff-only @97118d6；5 政策脚本自测/默认全 PASS（agent-memory ACTIVE=5 / graph ACTIVE=7 / plugin ACTIVE=1 PENDING=5 / agent-skill ACTIVE=3 PENDING=5 含 AGSK_CREDENTIAL_NOT_ECHOED / mcp ACTIVE=8 PENDING=0）；graph UI logic 41/41；CredentialLeak Display 已脱敏(security_policy.rs:118-120 _s => <redacted>)；5 处 CredentialLeak 调用点(agent.rs:31/34, skills.rs:32/38, plugin.rs:237)均经 _s；reject_oversized_def(256KiB) 已覆盖 6 read-only 命令入口(bridge.rs:6146-6157)且单测 6379-6387；ACL 9 read-only + 1 list_artifact_images 末条 K1 保真；bridge.rs mcp_/agent_/skill_ 命令体 + mcp.rs + plugin.rs 零 log_audit/eprintln/println/tracing
CHECKPOINT=logs/assist/A4-M5-W9-privacy-memory-review-20260907-0904.md
MERGE_NOTES=W9 A4 复核 A0 CredentialLeak 脱敏修复 + 全表面扫描：F-W8-1 闭环（Display _s=<redacted> + AGSK_CREDENTIAL_NOT_ECHOED 守门）、F-W8-2 闭环（reject_oversized_def 256KiB + 单测）；Agent/Skill 6 + MCP 3 + Plugin 纯函数 + Graph 错误 + audit + frontend 错误捕获面 全部脱敏合规；5 政策脚本 + 41 UI 断言全 PASS；W9 范围内未发现新漏洞。残项 R-W9-1（升 AGSK_CREDENTIAL_NOT_ECHOED PENDING→ACTIVE，建议非阻断）/R-W9-2（Rust 补"errors 不含密文"断言，建议）/R-W9-3（F1 DRY 常量化，仍 OPEN）
NEXT=A5 决定 R-W9-1/R-W9-2；A11 最终验证；R-W9-3 留 W10 或独立卡
```