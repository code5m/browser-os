# Lane A4 · M5-W8 Review Note — Agent/Skill 只读桥 + graph/plugin 表面 隐私/memory 复核（Excluding-A3）

> Lane=A4 · WAVE=M5-W8 Excluding-A3 Dispatch · Status=**REVIEW ONLY**（看板 §M5-W8 行 193：隐私/memory 评审，可提 policy 夹具但**不编辑产品代码**）
> W8 镜头：credential redaction、bounded preview payloads、audit contents、no prompt/body persistence、no local secret capture。
> 交付：`logs/assist/A4-M5-W8-privacy-memory-review-20260907-0756.md` + 具体坏样本建议（不落地代码）。
> 边界真相源：① A4 W4 agent memory KV 契约（5 ACTIVE 码）；② A7 W5 图谱契约（7 ACTIVE 码）；③ A5 W4/W7 Agent/Skill 契约（`domain.rs`/`agent.rs`/`skills.rs`/`security_policy.rs`/`check-agent-skill-policy.py`）。

## 0. 仓库实况（@ `6c1f30e`，已 `git pull --ff-only`，工作树仅含他 lane staged 文档 + 本笔记）

| 项 | 状态 | 证据 |
|---|---|---|
| A5 W7 Agent/Skill 只读桥 | **已落地 mainline** | `bridge.rs:6155`(agent_parse)/`main.rs:1434`(注册)/`default-commands.toml:118-123`(ACL)；HEAD `a29b796 docs(A6)`、`6c1f30e` 含 A3 MCP 桥 |
| A3 MCP 只读桥 | 已落地（W8 排除 A3，仅边注） | `6c1f30e feat(M5-W7,A3): read-only MCP registry/policy bridge commands` |
| A8 图谱 UI / A9 插件 | 已落地（5f92ece） | 同 W7 跟踪复核 |
| check-agent-memory-policy | PASS | `--self-test`→`AGENT_KV_POLICY_SELF_TEST=PASS（ACTIVE=5）`；默认 PASS |
| check-graph-policy | PASS | `--self-test`→`GRAPH_POLICY_SELF_TEST=PASS（ACTIVE=7）`；默认 PASS |
| check-plugin-policy | PASS | `--self-test`→`ACTIVE=1 PENDING=5`；默认 `PLUGIN_POLICY=PASS` |
| check-agent-skill-policy | PASS | `--self-test`→`AGENT_SKILL_POLICY_SELF_TEST=PASS（ACTIVE=3，PENDING=4）`（A5 加 `AGSK_RO_COMMAND_PARITY`）；默认 PASS |
| check-graph-ui-logic.mjs | PASS | 通过 34 / 失败 0 |

→ 本评审为**对已落地代码的具体二次复核**（W7 时 A5 命令桥未进 mainline，当时为前瞻式；现可 concrete 核验）。

## 1. W8 A4 主审：A5 Agent/Skill 只读桥（6 命令，已落地）

### 1.1 credential redaction — **F-W8-1（CRITICAL，mainline 现行生效）**
- **现象**：`agent_validate` / `skill_validate` 在 `CredentialLeak` 错误里**明文回显 secret**。链路：
  1. `agent.rs:30-35` → `Err(PolicyError::CredentialLeak(self.system_prompt.clone()))` / `(self.description.clone())`
  2. `skills.rs:31-40` → `Err(PolicyError::CredentialLeak(self.description.clone()))` / `(input.name.clone())`
  3. `security_policy.rs:118-120` Display：`write!(f, "凭据/密钥泄露（禁止进入 Agent/Skill 定义）：{s}")` → **回显 secret 原文**
  4. A5 `agent_validate_inner`（`bridge.rs`）→ `ValidationReport::with_errors(vec![format!("{e}"])` → `errors` 含 secret 明文
- **例**：`system_prompt="use sk-abc123XYZ"` → `agent_validate` 返回 `errors=["凭据/密钥泄露（禁止进入 Agent/Skill 定义）：use sk-abc123XYZ"]` → 前端明文拿到 `sk-abc123XYZ`。
- **为何违规**：直接击穿 W8 镜头 `credential redaction` + `no prompt/body exposure`。命令虽经 `check_invocation_source`（仅受信 main 可调用）且不持久化，但 secret 在 API 响应里明文往返 → 前端日志/剪贴/缓存即捕获；违背"no prompt-secret logging or persistence"精神（即便非磁盘持久化，也应脱敏）。
- **影响命令**：仅 `agent_validate` + `skill_validate`（两条跑 `validate`）。`agent/skill_permission_preview` 与 `agent/skill_parse` 不回显 secret（`parse` 仅回显用户自身输入，非校验拒后泄露）。
- **修复建议（产品代码，A4 不编辑，仅建议）**：在 `PolicyError::CredentialLeak` 的 Display 脱敏，例如 `write!(f, "凭据/密钥泄露（禁止进入 Agent/Skill 定义）：<redacted>")`；或 `agent.rs`/`skills.rs` 不再 `clone` secret 进错误（只传标记）。A5 既有测试 `validate_inner_rejects_credential_leak` 仅断言 keyword（"凭据"/"泄露"/"credential"/"leak"），**不依赖 secret 值 → 脱敏后测试仍 PASS**。
- **具体坏样本（建议夹具 `AGSK_CREDENTIAL_NOT_ECHOED`，可进 `check-agent-skill-policy.py --self-test` 或 Rust `#[cfg(test)]`）**：
  ```rust
  // 输入含 secret 的 AgentDef，调 agent_validate_inner，断言错误不回显 secret
  let mut a: AgentDef = serde_json::from_str(&good_agent_json()).unwrap();
  a.system_prompt = "use sk-ABC123XYZsecret".into();
  let r = agent_validate_inner(&serde_json::to_string(&a).unwrap());
  assert!(!r.valid);
  let joined = r.errors.join(" ");
  assert!(!joined.contains("sk-ABC123XYZsecret"), "secret must NOT echo in errors: {joined}");
  ```
  ```python
  # check-agent-skill-policy.py 建议新增 self-test 坏样本
  add("AGSK_CREDENTIAL_NOT_ECHOED",
      "agent_validate 对含 sk- 的 system_prompt 不应在 errors 回显密文",
      mutate_validate_with_secret("sk-ABC123XYZsecret"),
      "src-tauri/src/bridge.rs")
  ```

### 1.2 bounded preview payloads — **F-W8-2（承 W7 R5-2/F2，仍 OPEN）**
- 6 命令均 `text: String` 无上限，直接 `serde_json::from_str` 于任意长输入 → OOM/DoS 面。
- W8 镜头 `bounded preview payloads`：preview/validate 返回本身有界（`permission_preview` 仅 gate+capabilities id；`validate` 仅 `{valid,errors}`），但**输入无界**是容量漏洞；且 `agent_parse`/`skill_parse` 回显完整 `AgentDef`/`SkillDef`（含 `metadata: serde_json::Value` 无界）→ 输出也随输入无界。
- **修复建议**：命令入口加 `if text.len() > MAX_AGENT_DEF_BYTES { return Err("AGENT_DEF_TOO_LARGE".into()) }`（建议对齐 `MAX_TEXT_FIELD_BYTES=64KiB` 或更小定值）；或在 `domain::parse` 层加。修复后 `parse` 响应也随之有界。

### 1.3 audit contents — **PASS（F-W8-4）**：6 命令体内无 `log_audit` 调用（已实扫 `bridge.rs` 6 命令体）。✓
### 1.4 no prompt/body persistence — **PASS（F-W8-3）**：6 命令体内无 `agent_kv`/file/DB 写（已实扫），仅 source check + 纯解析/校验/预览，不触碰 `agent_memory.rs`。✓（与 W7 R5-1/R5-5 一致）
### 1.5 no local secret capture — **PASS**：无密钥落盘/入 KV；`CredentialLeak` 是"拒绝"而非"存储"，唯一暴露点是 F-W8-1 的回显（已单列）。✓
### 1.6 permission_preview 有界且无密 — **PASS（F-W8-5）**：`agent/skill_permission_preview` 仅返 `PermissionPreview{gate, capabilities:[id]}`，不含 system_prompt/metadata/secret。✓
### 1.7 source check / ACL — **PASS（F-W8-7 / F-W8-6）**：每条命令首行 `check_invocation_source`；ACL 6 命令注册于 `default-commands.toml:118-123`，**K1 末条 `list_artifact_images` 保真（127）**。✓（注：A5 补丁同时捆绑的 `mcp_*` 命令落位于 124-126，仍在 K1 之前，不破坏不变量。）

## 2. W8 镜头复核 A8 graph UI + A9 plugin（已落地，跟踪确认）

- **A8 图谱 UI**：无 secret 字段；`summarizeNode`（graphUi.ts:74）显式剥离 props（K7），`NodeDetail.vue` 不渲染；`boundedInsert` 守 5000/20000；`guard()` 后端未就绪时零 invoke；store 内存态不持久化；无审计。→ **全 PASS**（bounded + 脱敏 + 无持久化）。
- **A9 插件**：`PLUGIN_NO_SECRETS` 通过；`metadata` 限 64KiB；`plugin.rs` 无 `agent_memory` 引用；`PLUGIN_CAPABILITY_V1` 空 fail-closed；纯状态机无 install/exec；审计脱敏。→ **全 PASS**。
- 二者在 W8 镜头下未引入新的凭据/持久化/审计风险（与 W7 跟踪复核一致）。

## 3. 跨 lane 边注（A3 在 W8 被 HOLD，仅标注不深审）
- A5 补丁同时捆绑 A3 的 `mcp_capability_preview`/`mcp_registry_list`/`mcp_policy_get`（已落地 `6c1f30e`）。A3 在 W8 被排除，A4-W8 不深审 MCP 表面；但就其返回 `McpRegistryEntryView{core_api,returns_url}` 与 `McpDecisionView`，关注点（URL 脱敏 `redact_sensitive_url`、core_api 是否泄露内部路径、ACL 末条保真）归 **A2/A10** 在 W8 评审。
- 上述 3 条 MCP 命令同样首行 `check_invocation_source`，符合只读红线；仅就其 payload 脱敏深度由 A2 核对。

## 4. 系统性发现 / 遗留
- **F-W8-1**（核心，建议阻断级修复）：`CredentialLeak` 明文回显 secret，mainline 现行生效。建议 A0 集成时一并修（`security_policy.rs` Display 脱敏 + `agent.rs`/`skills.rs` 不 clone secret）。非阻塞但应修，且与 W8 镜头直接冲突。
- **F-W8-2**（承 W7 R5-2/F2）：parse 输入无字节上限，仍 OPEN，建议 W8/W9 落地（修复后顺带让 parse 响应有界）。
- **F1（DRY，承 W6/W7）**：`SENSITIVE_*` 双份（`agent_memory.rs:134`/`graph.rs:18`）+ 图谱容量常量三处拷贝（`domain.rs`/`useGraphStore.ts:24-25`/`graphUi.ts:26-29`）。仍 OPEN，建议前端常量统一从 `src/types.ts`（5f92ece 已加）引入。

## 5. 解锁/二次复核清单（A5 命令已落地，本次即二次复核）
- [x] 命令只 parse/validate/permission_preview，无 agent_memory 调用（F-W8-3）
- [x] ACL 末条 K1 保真（F-W8-6）
- [x] source check 已就位（F-W8-7）
- [x] 无审计 / 无持久化（F-W8-4 / F-W8-3）
- [x] permission_preview 有界且无密（F-W8-5）
- [ ] **F-W8-1**：修 `CredentialLeak` 明文回显（脱敏）
- [ ] **F-W8-2**：parse 输入加字节上限

## 6. 与 W8 Hard Stops 对齐
- "A3 is excluded from W8" → 本评审不依赖 A3 产品代码改动；MCP 命令仅边注（§3）。
- 本 Lane **零产品代码**（仅文档 + 建议 fixture 文本），符合 REVIEW ONLY。
- "May propose policy fixtures but do not edit product code" → 已以坏样本形式提议 `AGSK_CREDENTIAL_NOT_ECHOED`（§1.1），未改 `check-agent-skill-policy.py`。
- 未 push。

## 7. LANE 输出模板

```text
LANE=A4
STATUS=PASS（REVIEW ONLY，无产品代码）
BASE=6c1f30e
HEAD=logs/assist/A4-M5-W8-privacy-memory-review-20260907-0756.md
FILES=logs/assist/A4-M5-W8-privacy-memory-review-20260907-0756.md
VERIFY=git pull --ff-only @6c1f30e；check-agent-memory-policy PASS(ACTIVE=5)/check-graph-policy PASS(ACTIVE=7)/check-plugin-policy PASS/check-agent-skill-policy PASS(ACTIVE=3 PENDING=4)/check-graph-ui-logic.mjs 34/34；bridge.rs:6155 6 命令落地、default-commands.toml:118-123 注册且 K1@127 保真；6 命令体无 agent_kv/审计/执行引用
CHECKPOINT=logs/assist/A4-M5-W8-privacy-memory-review-20260907-0756.md
MERGE_NOTES=W8 A4 对已落地 A5 Agent/Skill 只读桥做隐私/memory 具体复核：发现 F-W8-1（CredentialLeak 在 validate 错误里明文回显 secret，security_policy.rs:118-120 + agent.rs:31/33 + skills.rs:32/38，mainline 现行）+ F-W8-2（parse 输入无字节上限，承 W7 R5-2）；其余 F-W8-3~7 全 PASS（无 agent_kv/审计/持久化、permission_preview 有界无密、source check 就位、K1 末条保真）。A8 graph UI/A9 plugin 在 W8 镜头下全 PASS。A3 MCP 命令仅边注（W8 排除 A3）。边界真相源=A4 W4 5 码 + A7 W5 7 码 + A5 W4/W7 契约。建议 A0 集成时修 F-W8-1（脱敏 Display）与 F-W8-2（输入上限）
NEXT=A0 修 F-W8-1/F-W8-2（或交 A5 在 W8/W9 落补丁）；A2/A10 深审 A3 MCP 桥 payload 脱敏；F1 DRY 常量化由 A0 套用
```
