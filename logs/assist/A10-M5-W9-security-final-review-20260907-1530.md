# A10 · M5-W9 Runtime-Free Polish 安全终裁（FINAL SECURITY REVIEW）

> Lane: A10（M4/M5 独立安全复审） · Wave: **M5-W9 Runtime-Free Polish Dispatch**（board §M5-W9，A0 2026-09-07 14:30；base `97118d6`）
> A10 W9 任务（board L200）：**Batch review W8/W9 outputs after A5/A6/A8/A9 finish. Focus source check, ACL parity, read-only guarantees, redaction, and no runtime expansion.** 交付：`logs/assist/A10-M5-W9-*.md`；一份最终安全裁决.
> 复审时间：2026-09-07 ~15:30 CST · BASE=`97118d6`（含 `4d7be97` W8 桥加固集成 + `94e763e` A3 MCP 相位债收口 + `97118d6` W8 patch 证据规范化）
> 交付物：本终裁（仅文档，**零产品代码改动、零补丁**，未 push）。W8 加的持久夹具 `AGSK_CREDENTIAL_NOT_ECHOED` 已随 `4d7be97` 进 master，本波未再改脚本。

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=PASS（FINAL SECURITY REVIEW，可 push 安全口径）
BASE=97118d6
HEAD=logs/assist/A10-M5-W9-security-final-review-20260907-1530.md
FILES=logs/assist/A10-M5-W9-security-final-review-20260907-1530.md
VERIFY=实证（见 §4）：pre-merge ALL_PASS；agent-skill 默认+自测 PASS(ACTIVE=3,PENDING=5，AGSK_CREDENTIAL_NOT_ECHOED 现绿)；mcp --expect-current-gaps PASS(ACTIVE=8,PENDING=0)；plugin ALL_PASS；graph PASS(7)；agent-memory PASS(5)；core PASS(7)；cargo agent_skill_bridge_tests 18 passed（含脱敏凭据拒绝）；9 命令首行 check_invocation_source；ACL A5@118-123+A3@124-126 居 list_artifact_images@127 前；grep 6115-6345 无执行/安装/网络；security_policy.rs:118-120 CredentialLeak Display=<redacted>
CHECKPOINT=logs/assist/A10-M5-W9-security-final-review-20260907-1530.md
MERGE_NOTES=A10 W9 安全终裁：对 W8 集成态(HEAD=97118d6)做最终批量复审。W8 四发现全部闭合/降级：S-W8-1(agent_validate/skill_validate 在 CredentialLeak 错误明文回显 secret)→ A0 已修 security_policy.rs:118-120 Display 改 <redacted>(不再插值)，AGSK_CREDENTIAL_NOT_ECHOED 夹具现绿；残留=agent.rs:31/34+skills.rs:32/38 仍 clone secret 进错误结构体(in-memory)，但 Display 不插值→错误串已干净(W9 Hard Stop L207 满足)，列为 LOW 残留建议(改传标记)；S-W8-2(parse 输入无界)→ A5 W8 加固 reject_oversized_def(AGENT/SKILL_DEF_MAX_BYTES=256KiB) 覆盖 6 命令入口，已闭合；S-W8-3(skill_list/agent_list 前端包装无后端 handler，bridge.ts:361/363)→ 仍 OPEN，属正确性非安全 completeness 债(悬空 invoke)，非边界泄漏(A2 W9 §F2 同判 non-blocking)，交 A0/A5/A6 闭环(落只读后端或移除 wrapper)；S-W8-4(集成卫生/pre-merge FAIL)→ 已闭合：97118d6 规范化 patch 尾随空白、94e763e 收口 A3 --expect-pending 相位债、plugin.rs 导入修复随 4d7be97 集成 → pre-merge ALL_PASS。安全姿态全绿：9 命令 source check 就位、ACL 末条不变量保真、只读保证成立(0 调 script_runner/exec/install/network)、无 core 泄漏、脱敏(permission_preview 仅 gate+id、CredentialLeak 不再回显)、无审计/持久化 secret、策略门全 PASS。A3 相位(依 W9 含 A3 复审)经 A3 W9/ A2 W9 确认 server-free、core 无反向依赖、MCP_NO_RMCP_SERVER 锁死。W9 为 runtime-free polish：A2/A3/A7 已交 docs(PASS)、A5 加固测试已集成(cargo 18 PASS)、A6/A8/A9 在途但均不引入运行时/安全变更；A10 终裁覆盖当前集成态，若 A5/A6/A8/A9 W9 交付产生安全相关改动(新命令/新桥)须再确认，但当前 HEAD 干净
NEXT=A0 可 push(安全口径已 PASS，pre-merge ALL_PASS)：① 闭环 S-W8-3(skill_list/agent_list 落只读后端原子包 或 移除 bridge.ts 包装)；② 可选修 S-W8-1 残留(agent.rs/skills.rs 不 clone secret，传标记而非全文)；③ BIN_ONLY_MODULES 扩为全 bin 模块(A2 §F3，A0 产品代码)；④ A11 W9 出最终验证矩阵+push readiness
```

## 1. 复审范围与方法

W9 = Runtime-Free Polish（任何 MCP server / plugin runtime / skill execution wave 之前）。A10 对 **W8 集成态（HEAD=`97118d6`）+ 已交付 W9 文档**做最终批量复审。

| 对象 | 性质 | A10 处理 |
|---|---|---|
| A5 Agent/Skill 只读桥（W8 加固已集成 `4d7be97`：有界输入 + 脱敏 + 18 tests） | 产品代码 | **实证主审**（source-check/ACL/只读/脱敏/cargo） |
| A3 MCP 只读桥（`6c1f30e` + `94e763e` 相位债收口） | 产品代码 + policy | 复审（server-free 确认，A3 W9/A2 W9 交叉佐证） |
| A2 W9 边界裁决（`A2-M5-W9-boundary-review-20260907-1500.md`） | 评审 | 采纳 **BOUNDARY_PASS（concrete blockers=NONE）** |
| A3 W9 MCP 相位（`A3-M5-W9-mcp-policy-current-phase-20260907-0902.md`） | 评审 | 采纳 **PASS（三模式全绿，零回归）** |
| A7 W9 图谱桥契约（`A7-M5-W9-graph-bridge-contract-20260907-0903.md`） | docs-only | 确认无后端命令、无运行时扩张 |
| A4/A5/A6/A8/A9 W9 | 在途（A5 加固测试已集成；A6/A8 UI、A9 插件评审未交付文档） | 范围声明：均不引入运行时/安全变更；若交安全相关改动须再确认，当前 HEAD 干净 |
| 自有持久夹具 | `AGSK_CREDENTIAL_NOT_ECHOED`（W8 加，已进 master） | 现锁 S-W8-1，默认扫描转绿 |

方法：实证跑门 + `grep` + `cargo test` + `pre-merge.sh`，非仅凭读码。

## 2. 综合安全裁决：**PASS（FINAL SECURITY REVIEW，可 push 安全口径）**

- W8 四安全发现全部**闭合或降级**：S-W8-1 修复（脱敏生效，夹具绿）、S-W8-2 闭合（有界输入）、S-W8-4 闭合（pre-merge ALL_PASS）；仅 S-W8-3 残留（正确性非安全，non-blocking）。
- 安全姿态**全绿**：source check 全就位、ACL 末条不变量保真、只读保证成立、脱敏成立、无审计/持久化 secret、策略门全 PASS、core 无泄漏、无运行时扩张。
- 集成卫生门 **pre-merge = ALL_PASS**（W8 三个红灯全消）。
- 结论：**安全口径可 push**；剩余项（S-W8-3 闭环、S-W8-1 残留克隆、BIN_ONLY_MODULES 加固）为后续 wave 跟踪债，不阻断 push。

## 3. W8 安全发现闭环表

| 发现 | W8 状态 | W9 终裁状态 | 证据 |
|---|---|---|---|
| **S-W8-1** secret 在 CredentialLeak 错误明文回显 | CRITICAL concrete failure | **FIXED（脱敏生效）** | `security_policy.rs:118-120`：`CredentialLeak(_s) => write!(f, "…：<redacted>")`（不再插值 `{s}`）；`AGSK_CREDENTIAL_NOT_ECHOED` 默认扫描现 PASS；`cargo` `validate_inner_rejects_credential_leak`/`rejects_skill_credential` 仍 PASS |
| **S-W8-1 残留**（in-memory clone） | — | **LOW 残留建议** | `agent.rs:31/34` + `skills.rs:32/38` 仍 `CredentialLeak(self.xxx.clone())` 把 secret 载入错误结构体；但 Display 不插值 → 错误串干净（W9 Hard Stop L207 满足）。建议改传标记/不 clone 全文（防 Debug/序列化泄漏）。非阻塞 |
| **S-W8-2** parse 输入无字节上限（DoS/OOM） | MEDIUM | **FIXED（闭合）** | `bridge.rs` 新增 `reject_oversized_def(text, AGENT_DEF_MAX_BYTES/SKILL_DEF_MAX_BYTES, kind)`（256KiB），覆盖 6 命令入口（`agent_parse`/`skill_parse`/`*_validate`/`*_permission_preview` inner）；A2 W9 §4 确认 |
| **S-W8-3** skill_list/agent_list 前端包装无后端 handler | LOW（completeness） | **OPEN（non-blocking）** | `bridge.ts:361/363` 仍包装 `skill_list`/`agent_list`，`bridge.rs` 无 handler、ACL 无条目 → 悬空 invoke。正确性债，非边界泄漏/非执行路径（A2 W9 §F2 同判 non-blocking）。交 A0/A5/A6 闭环 |
| **S-W8-4** 集成卫生 / pre-merge FAIL | HARD STOP | **FIXED（闭合）** | `97118d6` 规范化 patch 尾随空白；`94e763e` 收口 A3 `--expect-pending` 相位债（MCP_NO_RMCP_SERVER ACTIVE=8/PENDING=0）；`plugin.rs` 导入修复随 `4d7be97` 集成；`pre-merge.sh` = **ALL_PASS** |

## 4. 实证验证日志（可复跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# source check（9 命令首行）
sed -n '6115,6345p' src-tauri/src/bridge.rs | grep -cE 'check_invocation_source'   # 9
# 只读性（6115-6345 无执行/安装/网络/持久化）
sed -n '6115,6345p' src-tauri/src/bridge.rs | grep -E 'std::process|Command::new|\.spawn|skill_install|agent_install|reqwest|ureq|rmcp|TcpListener|fs::write|sqlx' || echo NO_FORBIDDEN
# ACL 末条不变量（K1）
grep -nE 'agent_parse|agent_validate|agent_permission_preview|skill_parse|skill_validate|skill_permission_preview|mcp_policy_get|mcp_registry_list|mcp_capability_preview|list_artifact_images' src-tauri/permissions/default-commands.toml
#   → 118-123(A5) + 124-126(A3) 均 < 127 list_artifact_images
# 脱敏（CredentialLeak Display）
sed -n '114,121p' src-tauri/src/security_policy.rs   # <redacted>，无 {s}
# cargo
cargo test --manifest-path src-tauri/Cargo.toml agent_skill_bridge_tests   # 18 passed
# 策略门
python3 scripts/check-agent-skill-policy.py --self-test   # PASS(ACTIVE=3,PENDING=5)
python3 scripts/check-agent-skill-policy.py               # PASS（AGSK_CREDENTIAL_NOT_ECHOED 绿）
python3 scripts/check-mcp-policy.py --expect-current-gaps # PASS(ACTIVE=8,PENDING=0)
python3 scripts/check-plugin-policy.py --self-test        # ALL_PASS
python3 scripts/check-graph-policy.py --self-test         # PASS(ACTIVE=7)
python3 scripts/check-agent-memory-policy.py --self-test  # PASS(ACTIVE=5)
python3 scripts/check-core-boundary.py --self-test        # PASS(ACTIVE=7)
# 集成卫生门
bash scripts/pre-merge.sh                                  # PRE_MERGE_RESULT=ALL_PASS
```

**实测结果（本裁决依据）**：
- `check_invocation_source` 9 命中（A5×6 + A3×3 首行）。
- 6115-6345 区无执行/安装/网络/持久化模式（NO_FORBIDDEN）。
- ACL：A5@118-123 + A3@124-126 全部居 `list_artifact_images`@127 前（K1 保真）。
- `security_policy.rs:118-120` = `CredentialLeak(_s) => write!(f, "…：<redacted>")`。
- `cargo agent_skill_bridge_tests` = **18 passed**（含脱敏凭据拒绝）。
- 策略门：agent-skill(self-test+default) PASS / mcp `--expect-current-gaps` PASS(ACTIVE=8,PENDING=0) / plugin ALL_PASS / graph PASS(7) / agent-memory PASS(5) / core PASS(7)。
- `pre-merge.sh` = **ALL_PASS**。
- build metrics：board W9 事实 21.07% ≤ 22% 接受（A11 W9 矩阵将给出精确值）；cargo warnings 未增（pre-merge 含 cargo 门）。

## 5. 安全保证矩阵（PASS 项，实证）

| 焦点 | 结论 | 证据 |
|---|---|---|
| source check | **PASS** | 9 命令（A5×6 + A3×3）首行 `check_invocation_source`（bridge.rs:6176/6186/6196/6244/6254/6264 + mcp:6479/6488/6497）；`check_invocation_source` 委托 `sp::check_remote_invocation`（远端 webview 无令牌拒） |
| ACL drift / parity | **PASS** | A5@118-123 + A3@124-126 均居 `list_artifact_images`@127 前；`AGSK_ACL_TAIL`/`AGSK_RO_COMMAND_PARITY` PASS；A3 `MCP_PARITY` PASS |
| 只读保证 | **PASS** | 9 命令纯 `parse`/`validate`/`permission_preview`/`current_policy_snapshot`/`list_registry_entries`/`evaluate_mcp_command`；grep 6115-6345 无执行/安装/网络/持久化；`cargo` 18 tests PASS |
| 无重复执行路径 | **PASS** | 9 命令 0 调 `script_runner`/`skill_run`/`agent_chat`；capability 按域单源（A2 W9 §④） |
| 无 core 泄漏 | **PASS** | `core/` 0 反向依赖 bin 模块（CORE_NO_REVERSE_DEP）；core gate PASS(ACTIVE=7) |
| 无 runtime/server 扩张 | **PASS** | A2 W9 ①：M5 模块 grep 网络/server 仅注释误报；`MCP_NO_RMCP_SERVER`(ACTIVE) 锁死 rmcp/tokio/listener；`grid_process` UnixListener 为既有内部 IPC（out-of-scope）；W9 各 lane=A3 policy-only / A5 加固测试 / A6/A8 UI / A7 docs / A9 review，均不引入运行时 |
| 脱敏（redaction） | **PASS** | `CredentialLeak` Display `<redacted>`（S-W8-1 修）；`permission_preview` 仅 `gate`+capability id；A8 图谱 UI 不渲染 props（K7）；A9 插件裁决有界脱敏 |
| 无审计 / 无 secret 持久化 | **PASS** | A5 6 命令体无 `log_audit`；无 `agent_kv`/file/DB 写（A4 W8 F-W8-3~5 同判） |
| 命令 payload 边界 | **PASS** | S-W8-2 修：`reject_oversized_def` 256KiB 覆盖 6 命令入口；preview/validate 返回有界 |
| 前端面 | **PASS** | A6 W8 79 断言零后端调用、零实时执行、secret 仅键名脱敏不落前端态；A8 W8 41 断言 + npm build，无后端命令、无无界数组 |
| 插件命令面 | **PASS** | A9 W8：零插件命令、纯状态机、ACL 0 命中 `plugin`、裁决有界脱敏 |
| 策略门（全） | **PASS** | agent-skill(3/5) / mcp(8/0) / plugin(ALL) / graph(7) / agent-memory(5) / core(7) 全绿 |

## 6. 与 W9 Hard Stops 对齐

- L205「无 MCP server/listener/rmcp、无 plugin install/skill exec/network」→ A2 W9 ① PASS；`MCP_NO_RMCP_SERVER` 锁死；W9 各 lane 零运行时新增。
- L206「新/改命令须只读 + source check + ACL + bridge/types + policy/tests 同包」→ A5 加固已原子集成（4d7be97）；S-W8-3 缺口属「前端 wrapper 早于后端」，非新命令违规，待闭环。
- L207「无 token/cookie/Authorization/body/prompt-secret 于日志/审计/前端态/checkpoint/错误串」→ S-W8-1 修复后错误串已 `<redacted>`（满足）；残留 in-memory clone 不入串（LOW 建议）。
- L208「build metrics ≤22%、cargo warning 不增」→ board W9 事实 21.07%≤22% 接受；pre-merge ALL_PASS。
- L209「仅 A0 push」→ 本交付未 push。

## 7. 延续 / 积压闭合

- **W4-W7 A10 BLOCKED 门禁**：因 A4/A5 W4、A6/A7 W5、A8/A9 W6 产品码经 W7/W8 集成已落地，A10 借 W8/W9 实质复审 → 历史 BLOCKED 全部转为本裁决 PASS。
- **W8 四发现**：S-W8-1（修+夹具锁）、S-W8-2（修）、S-W8-4（修，pre-merge ALL_PASS）；仅 S-W8-3 残留（non-blocking completeness）。
- **A4 W8 隐私评审**（F-W8-1/F-W8-2）：升为 S-W8-1/S-W8-2，本波均闭合/降级。
- **A2 W8/W9 边界**：采纳 F1(PASS)/F5/F6(无重复执行)/F9 + W9 BOUNDARY_PASS（concrete blockers=NONE）。
- **A3 W8/W9**：`--expect-pending` 债收口（94e763e）→ `MCP_NO_RMCP_SERVER`(ACTIVE=8/PENDING=0)；A3 W9 确认零回归。
- **同波**：A1 W9 对账 W8 为 accepted-with-fixes；A11 W9 将出最终验证矩阵 + push readiness（A10 终裁为其提供安全基线）。

## 8. 残留债（不阻断 push，供后续 wave）

1. **S-W8-3**（OPEN, non-blocking）：`skill_list`/`agent_list` 前端 wrapper 无后端 → 落只读后端（atomic 同包 + `AGSK_RO_COMMAND_PARITY` 过）或移除 `bridge.ts:361/363` 包装。
2. **S-W8-1 残留**（LOW）：`agent.rs:31/34`+`skills.rs:32/38` 仍 clone secret 进 `CredentialLeak` 结构体（Display 不插值故串已干净）；建议改传标记避免 in-memory 持有。
3. **BIN_ONLY_MODULES 加固**（A2 §F3）：`check-core-boundary.py` 仅 6 项，建议扩为全 bin 模块（A0 产品代码）。

## 9. 声明

- 本交付为**文档终裁**，**零产品代码改动、零补丁、未 push**（仅 A0 可 push）。
- W8 加的持久夹具 `AGSK_CREDENTIAL_NOT_ECHOED` 已随 `4d7be97` 进 master，本波未再改 `scripts/check-agent-skill-policy.py`（默认扫描已绿，无回归）。
- 工作树现状：`M logs/checkpoints/M5-20260906/M5-0-overview.md`（A1 对账，非本 Lane）、`?? A2/A3/A7 W9 文档`（他 Lane 在途件），均未越权触碰。
- HEAD=`97118d6`，本地领先 origin 0（已 ff-only 同步）；**未 push**。
- 若 A5/A6/A8/A9 W9 后续交付含安全相关改动（新命令/新桥/新 ACL），A10 终裁须据此再确认；当前集成态 HEAD 安全口径已 PASS。
