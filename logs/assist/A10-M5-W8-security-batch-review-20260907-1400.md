# A10 · M5-W8 安全批量复审（Excluding-A3）— 一份裁决，非逐文件滴灌

> Lane: A10（M4/M5 独立安全复审） · Wave: **M5-W8 Full-Lane Follow-up Dispatch**（board §M5-W8，A0 2026-09-07 09:45；"Excluding-A3"：A3 HOLD/NO ASSIGNMENT）
> A10 W8 任务（board L199）：**Batch security review of all non-A3 W8 outputs after at least A5/A6/A8/A9 report**. 焦点：source check / ACL drift / read-only guarantees / redaction / command payload bounds / no hidden execution·install·network. 交付：`logs/assist/A10-M5-W8-*.md`；concrete failures 可加 policy 夹具.
> 复审时间：2026-09-07 ~14:00 CST · BASE=`6c1f30e`
> 交付物：本批量裁决 + 策略夹具 `AGSK_CREDENTIAL_NOT_ECHOED`（已落 `scripts/check-agent-skill-policy.py`，自测 PASS、默认扫描锁住 S-W8-1）

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=PASS_WITH_DEBT
BASE=6c1f30e
HEAD=logs/assist/A10-M5-W8-security-batch-review-20260907-1400.md
FILES=logs/assist/A10-M5-W8-security-batch-review-20260907-1400.md, scripts/check-agent-skill-policy.py
VERIFY=实证（见 §4）：A5 只读桥 6 命令首行 check_invocation_source + ACL@118-123 居 list_artifact_images 前 + cargo agent_skill_bridge_tests 5 PASS + check-agent-skill-policy PASS(ACTIVE=3,PENDING=5)；新增 AGSK_CREDENTIAL_NOT_ECHOED 自测 PASS、默认扫描 FAIL（锁 S-W8-1）；A3 排除但已核对 mcp_* 3 命令干净；pre-merge FAIL（patch 尾随空白 + AGSK/PENDING + 未提交 plugin.rs 导入修复）
CHECKPOINT=logs/assist/A10-M5-W8-security-batch-review-20260907-1400.md
MERGE_NOTES=A10 W8 对已落地 non-A3 W8 产出做批量安全复审：① A5 Agent/Skill 只读桥（bridge.rs:6155-6224，6 命令）source-check/ACL/只读/无执行·安装·网络·持久化 全 PASS，cargo 5 tests PASS（含 credential_leak 拒绝）；② A6 UI 逻辑（79 断言，零后端命令、零实时执行）PASS；③ A8 图谱 UI 打磨（41 断言 + npm build，零后端命令，K7 无 props）PASS；④ A9 插件（零命令、纯策略、裁决有界脱敏）PASS；⑤ A4 隐私/A2 边界/A7 图卡 交叉确认。发现：S-W8-1（CRITICAL 具体失败，A4 F-W8-1）：agent_validate/skill_validate 在 CredentialLeak 错误明文回显 secret（security_policy.rs:118-119 Display "{s}"；agent.rs:31/34 + skills.rs:32/38 clone secret）→ A10 加静态夹具 AGSK_CREDENTIAL_NOT_ECHOED 锁死；S-W8-2（MEDIUM）：parse 输入无字节上限（F-W8-2）；S-W8-3（LOW，完整性）：skill_list/agent_list 前端包装无后端 handler（A2 F2）；S-W8-4（硬停/集成卫生，阻塞 A0 push）：pre-merge FAIL（patch 尾随空白 + AGSK PENDING + 未提交 plugin.rs 导入修复 + A3 --expect-pending）。A3 按 dispatch 排除，但其 mcp_* 3 命令已核对干净（source-check + 只读 + ACL@124-126），仅 --expect-pending 债归 A3 自有 W8 policy fix
NEXT=A0/A5 修 S-W8-1（security_policy.rs Display 改 <redacted> 或不插值；agent.rs/skills.rs 不 clone secret；现有 validate_inner_rejects_credential_leak 测试仍 PASS）；A5 修 S-W8-2（parse 入口加 ≤64KiB 上限）；A0 闭环 S-W8-3（落 skill_list/agent_list 只读后端 或 移除 bridge.ts 包装）；A0 在 push 前清 S-W8-4（套用 W7 plugin.rs 导入修复 + 去 patch 尾随空白 + A3 合 --expect-pending 债）
```

## 1. 复审范围与方法

W8 = Full-Lane Follow-up（Excluding-A3）。A10 批量复审 **non-A3 W8 产出**（dispatch 明示排除 A3）：

| 对象 | 性质 | A10 处理 |
|---|---|---|
| **A5 Agent/Skill 只读桥**（bridge.rs:6155-6224 + ACL:118-123 + main.rs + bridge.ts + types.ts + check-agent-skill-policy.py） | 产品代码（W7 已落，W8 加固） | **实证主审**（命令级 grep + cargo + 策略门） |
| A6 Agent/Skill 面板消费计划 + UI 逻辑测试 | 前端逻辑（79 断言） | 确认零后端命令、零实时执行 |
| A8 图谱 UI 纯逻辑打磨 | 前端（41 断言 + npm build） | 确认零后端 graph 命令、K7 无 props、无无界数组 |
| A9 插件命令面策略审查 | 纯策略审查 | 确认零命令、裁决有界脱敏 |
| A4 隐私/memory 评审 | 评审（发现 F-W8-1/F-W8-2） | **采纳并升级为 S-W8-1/S-W8-2**，加夹具 |
| A2 边界审查 | 评审（F1-F9） | 采纳 PASS 结论 + 引用 F2/F5/F6 |
| A7 图谱桥卡片 | docs-only | 确认无后端命令 |
| **A3 MCP 只读桥**（mcp_policy_get/mcp_registry_list/mcp_capability_preview） | 产品代码（W8 HOLD） | **按 dispatch 排除**；仅卫生核对（不计入裁决） |

方法：实证跑门 + `git`/`grep` + `cargo test`，非仅凭读码。

## 2. 综合安全裁决：**PASS_WITH_DEBT**

- 所有 non-A3 W8 产出的**安全姿态强**：source check 全就位、ACL 末条不变量保真、只读保证成立、无重复执行路径、无 core 泄漏、无审计/持久化 secret、策略门 PASS。
- **1 项具体安全缺陷 S-W8-1（secret 在 validate 错误明文回显）应 push 前修**（已加持久夹具锁死）。
- **1 项容量加固 S-W8-2**、**1 项完整性 parity S-W8-3**、**集成卫生门 S-W8-4（阻塞 A0 push）**。

## 3. 安全发现（合并，非逐文件）

### S-W8-1（HIGH · concrete failure · 来自 A4 F-W8-1，A10 代码级确认并加夹具）
- **现象**：`agent_validate`/`skill_validate` 经 `CredentialLeak` 错误**明文回显 secret**。
  - `security_policy.rs:118-119`：`PolicyError::CredentialLeak(s) => write!(f, "凭据/密钥泄露（禁止进入 Agent/Skill 定义）：{s}")` → 回显 `s`。
  - `agent.rs:31/34`、`skills.rs:32/38`：`Err(PolicyError::CredentialLeak(self.system_prompt.clone()))` / `(self.description.clone())` / `(input.name.clone())` → 把 secret clone 进错误。
  - `bridge.rs` `agent_validate_inner` → `ValidationReport::with_errors(vec![format!("{e}")])` → 错误含 secret 明文 → 前端拿到。
- **例**：`system_prompt="use sk-abc123XYZ"` → `agent_validate` 返回 `errors=["…：use sk-abc123XYZ"]`。
- **违规**：击穿 W8 镜头 `credential redaction` + 「no prompt-secret logging or persistence」精神（即便非磁盘持久化，API 响应明文往返 → 前端日志/剪贴/缓存即捕获）。
- **A10 动作（dispatch 授权 concrete failures 加 policy 夹具）**：`scripts/check-agent-skill-policy.py` 新增 PENDING 码位 `AGSK_CREDENTIAL_NOT_ECHOED`，静态锁死 `CredentialLeak` Display 不得回显 `{s}`。自测 PASS（ACTIVE=3,PENDING=5）；默认扫描已 FAIL 报该码（持久锁住真实缺陷）。
- **修复建议（产品代码，归 A0/A5）**：Display 改 `write!(f, "…：<redacted>")` 或不插值捕获变量；`agent.rs`/`skills.rs` 不 clone secret（只传标记）。现有 `validate_inner_rejects_credential_leak` 仅断言 keyword（"凭据"/"泄露"/"credential"/"leak"）→ 修复后测试仍 PASS。
- **影响命令**：仅 `agent_validate` + `skill_validate`（跑 `validate`）。`*_permission_preview`/`*_parse` 不回显 secret（F-W8-5/A4 PASS）。

### S-W8-2（MEDIUM · 来自 A4 F-W8-2）
- `agent_parse`/`skill_parse`/`*_validate` 入参 `text: String` 无字节上限，直接 `serde_json::from_str` 于任意长输入 → OOM/DoS；且 `parse` 回显完整 `AgentDef`/`SkillDef`（含 `metadata: serde_json::Value` 无界）→ 输出随输入无界。
- **建议**：命令入口（或 `domain::parse`）加 `if text.len() > MAX_TEXT_FIELD_BYTES { return Err("AGENT_DEF_TOO_LARGE".into()) }`（对齐 64KiB 定值）。修复后 parse 响应亦随之有界。

### S-W8-3（LOW · 完整性，来自 A2 §F2）
- `src/bridge.ts:361/363` 包装 `skillList`/`agentList`（`invoke("skill_list")`/`agent_list`），但 `bridge.rs` 无对应 handler（A5 仅交付 parse/validate/preview）。运行时悬空调用。
- **建议**：二选一 —— ① 落 `skill_list`/`agent_list` 只读后端（复用 `allowed_roots` + 单一加载器 + 原子 ACL/bridge.ts/types.ts/测试，守 `AGSK_RO_COMMAND_PARITY`）；② 或 W8 不做 list 则移除 `bridge.ts` 包装。`AGSK_RO_COMMAND_PARITY` 已守该 parity。非安全洞，属桥完整性。

### S-W8-4（HARD STOP / 集成卫生 · 阻塞 A0 push，独立于产品安全复审）
`bash scripts/pre-merge.sh` → **FAIL**：
1. `git diff --check`：staged `*.patch`/checkpoint 文档尾随空白（A3/A5 等 lane 补丁）。→ 去尾随空白。
2. `check-agent-skill-policy.py` 默认扫描 FAIL（`AGSK_CREDENTIAL_NOT_ECHOED`，即 S-W8-1 未修）。
3. A9 W7 报告指：`plugin.rs` 导入回归修复（`PluginCapability` 回测试模块）**仍在工作树未提交** → `cargo test plugin` 模块失败（违反 W6「tests PASS」）；A0 集成前应套用 `logs/checkpoints/Lane-A9-M5-W7-plugin-surface-review-20260907-0900.patch`。
4. A3 `--expect-pending` 相位债：`check-mcp-policy.py --expect-pending` FAIL（M5-2 产物已存在，应翻 ACTIVE 由 W2 接管）——A3 自有 W8 policy fix（board L192，仅 policy/checkpoint 文件）。
→ 上述 4 项不修，A0 不得 push。

## 4. 实证验证日志（可复跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# A5 只读桥 source check（6 命令首行）
grep -nE 'check_invocation_source' src-tauri/src/bridge.rs | grep -E 'agent_parse|agent_validate|agent_permission_preview|skill_parse|skill_validate|skill_permission_preview'
#   → 6155/6165/6175/6196/6206/6216 各一行 = PASS
# A5 只读性（6115-6345 区无执行/安装/网络/持久化）
sed -n '6115,6345p' src-tauri/src/bridge.rs | grep -nE 'std::process|Command::new|\.spawn\(|execute\(|skill_install|agent_install|reqwest|ureq|rmcp|TcpListener|fs::write|sqlx' || echo "NO_FORBIDDEN"
#   → 仅注释「无 rmcp/无 server/无监听/无网络」= PASS
# ACL 末条不变量（K1）
grep -nE 'list_artifact_images|agent_parse|agent_validate|agent_permission_preview|skill_parse|skill_validate|skill_permission_preview|mcp_policy_get|mcp_registry_list|mcp_capability_preview' src-tauri/permissions/default-commands.toml
#   → 118-123(A5) + 124-126(A3) 均居 list_artifact_images(127) 前 = PASS
# cargo 测试
cargo test --manifest-path src-tauri/Cargo.toml agent_skill_bridge_tests   # 5 passed（含 rejects_credential_leak）
# 策略门
python3 scripts/check-agent-skill-policy.py --self-test   # PASS(ACTIVE=3,PENDING=5)
python3 scripts/check-agent-skill-policy.py               # FAIL: AGSK_CREDENTIAL_NOT_ECHOED（锁 S-W8-1）
python3 scripts/check-mcp-policy.py --self-test           # PASS(ACTIVE=6,PENDING=9)
python3 scripts/check-mcp-policy.py --expect-pending      # FAIL（A3 W8 债）
python3 scripts/check-plugin-policy.py --self-test        # ALL_PASS
python3 scripts/check-graph-policy.py --self-test         # PASS(ACTIVE=7)
python3 scripts/check-agent-memory-policy.py --self-test  # PASS(ACTIVE=5)
python3 scripts/check-core-boundary.py --self-test        # PASS(ACTIVE=7)
# 脱敏（A5 命令体无 token/secret/prompt/body）
grep -rnE 'token|secret|password|Authorization|cookie|prompt|body' src-tauri/src/agent.rs src-tauri/src/skills.rs src-tauri/src/bridge.rs \
  | grep -iE 'agent_parse|agent_validate|agent_permission|skill_parse|skill_validate|skill_permission|mcp_policy|mcp_registry|mcp_capability' || echo "NO_ECHO_IN_CMDS"
#   → 空（注：secret 回显发生在 security_policy.rs Display，已由 AGSK_CREDENTIAL_NOT_ECHOED 锁）
# 集成卫生门
bash scripts/pre-merge.sh   # FAIL: git diff --check + AGSK PENDING
```

## 5. 安全保证矩阵（PASS 项，实证）

| 焦点 | 结论 | 证据 |
|---|---|---|
| source check | **PASS** | 9 新命令（A5×6 + A3×3）首行 `check_invocation_source`；`check_invocation_source`(bridge.rs:1183) 委托 `sp::check_remote_invocation`（远端 webview 无令牌拒） |
| ACL drift | **PASS** | 9 命令注册于 `default-commands.toml`，A5@118-123 + A3@124-126 均在末条 `list_artifact_images`@127 前（K1 保真）；`AGSK_ACL_TAIL`/`AGSK_RO_COMMAND_PARITY` PASS |
| 只读保证 | **PASS** | A5 6 命令纯 `parse`/`validate`/`permission_preview`（0 调 `script_runner`/`skill_run`/`agent_chat`/exec/install/网络/持久化，grep 6115-6345 空）；A3 3 命令纯（mcp.rs 无 rmcp/tokio/TcpListener/std::process/网络） |
| 无重复执行路径 | **PASS** | A2 F5/F6：只读命令 0 调 `script_runner`；capability 按域单源（MCP↔mcp.rs / Skill·Agent↔permission_preview / Plugin↔PermissionManifestRule），无跨域 verdict |
| 无 core 泄漏 | **PASS** | `core/` 无反向依赖 bin 模块（A2 F1）；A5 命令仅用 `crate::domain`（共享类型，合法） |
| 无 graph/plugin/MCP runtime 泄漏 | **PASS** | `bridge.rs` 0 引用 `crate::graph`/`crate::plugin`（A2 F9）；A3 MCP 命令隔离 |
| 脱敏（permission_preview） | **PASS** | `permission_preview` 仅返 `gate`+capability `id`（skills.rs:45，无 secret）；A8 图谱 UI 不渲染 props（K7）；A9 插件裁决有界+脱敏 |
| 无审计/无 secret 持久化 | **PASS** | A5 6 命令体无 `log_audit`（A4 F-W8-4）；无 `agent_kv`/file/DB 写（F-W8-3） |
| 命令 payload 边界 | **PARTIAL** | preview/validate 返回有界；**parse 输入无上限（S-W8-2）** |
| 前端面 | **PASS** | A6 79 断言零 bridge 调用、零实时执行、secret 仅键名脱敏不落前端态；A8 41 断言 + npm build，无后端命令、无无界数组 |
| 插件命令面 | **PASS** | A9：零插件命令、纯状态机、ACL 0 命中 `plugin`、裁决有界脱敏 |

## 6. A3 排除说明（dispatch L199/L204）

A3 在 W8 被 HOLD（board L177/L192/L204）。其 W7 已落 `mcp_*` 3 命令**不在 A10 W8 裁决范围**，仅作卫生核对：
- `mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview`（bridge.rs:6316-6345）首行 `check_invocation_source` + 纯（调 `current_policy_snapshot`/`list_registry_entries`/`evaluate_mcp_command`），无 rmcp/server/监听/网络 → **干净**。
- ACL@124-126 居 K1 前。
- 唯一债务：`check-mcp-policy.py --expect-pending` FAIL（W1 pending 语义待 W7/W8 翻 ACTIVE）——属 **A3 自有 W8 policy fix**（board L192，仅改 `check-mcp-policy.py`/`pre-merge.sh`），**不计入 A10 裁决**，亦不阻塞 A10 对 non-A3 的 PASS_WITH_DEBT 结论。

## 7. 与 W8 Hard Stops 对齐

- "A3 excluded" → 本裁决不覆盖 A3 产品代码改动；MCP 命令仅卫生核对（§6）。
- "no rmcp/server/plugin install/skill exec/network" → A5/A6/A8/A9 全 PASS；A3 核对无违规。
- "every A5 command atomic: source check + ACL + frontend bridge/types + policy + tests" → A5 6 命令四件套齐（bridge.rs/main.rs/ACL/bridge.ts）+ policy(`AGSK_RO_COMMAND_PARITY`)+cargo 测试 → **原子 PASS**（仅 S-W8-1 脱敏缺陷待修，不影响原子性）。
- "no token/cookie/Authorization/body/prompt-secret logging/audit/persistence/frontend state" → 除 S-W8-1（validate 错误回显 secret）外全 PASS；S-W8-1 已由夹具锁死待修。
- "lanes must not push" → 本交付未 push；仅 A0 可 push（须先清 S-W8-4）。

## 8. 延续 / 积压闭合

- **W4/W5/W6 A10 门禁（此前 BLOCKED）本波实质闭合**：A4/A5 W4、A6/A7 W5、A8/A9 W6 产品代码经 W7 集成后均已落地，A10 借 W8 对已落地 non-A3 面做批量复审 → 历史 BLOCKED 门禁的 A5/A6/A8/A9 部分**全部转换为本裁决的 PASS/PARTIAL**。A3 仍 HOLD（其 W4/W5/W7 产品码在树但 W8 不得扩展，A10 不深审）。
- **A4 W8 隐私评审**：采纳 F-W8-1→S-W8-1（升为 concrete failure + 夹具）、F-W8-2→S-W8-2。
- **A2 W8 边界评审**：采纳 F1(PASS)/F5/F6(无重复执行)/F9(graph/plugin 不泄漏)；F2→S-W8-3(完整性)。
- **A9 W8 插件评审**：采纳零命令/纯策略/裁决脱敏结论；其 W7 `plugin.rs` 导入修复未提交 → 列入 S-W8-4(3)。
- 同波参考：A11 W8 验证（待产出，将收口 pre-merge 全过 + A3 债闭合后给出「A0 可 push」结论）；A1 W8 对账（M5 readiness/debt）。

## 9. 声明

- 本交付含 1 份文档 + 1 个 policy 夹具（`check-agent-skill-policy.py` 新增 `AGSK_CREDENTIAL_NOT_ECHOED`，属 dispatch 授权「concrete failures 可加 policy 夹具」）。**未改任何产品运行时代码**（security_policy.rs/agent.rs/skills.rs 的 S-W8-1 修复归 A0/A5）。
- 工作树其他未提交/暂存物（他 lane 文档、plugin.rs MM、check-agent-skill-policy.py M 等）未越权触碰（本夹具编辑落在已 staged 的 policy 脚本内，属 A10 授权范围）。
- HEAD=`6c1f30e`，本地领先 origin 3（他 lane 在途件）；**未 push**。
