# A5 · M5-W8 整包交付：Agent/Skill 只读桥硬化（after A0 validation）

> LANE=A5（**M5-W8 Excluding-A3 Dispatch**，指挥板 §M5-W8 行 137-203）：**START PRODUCT CODE** — *Harden Agent/Skill read-only bridge after A0 validation: add missing edge-case tests, improve validation error shape if needed, ensure ACL/source check/policy script and frontend bridge/types stay atomic.*
> BASE=6c1f30e（W7 A3 已集成 master；W7 A5 源码已落 master，本波在其上硬化）
> 交付物：输入大小上限（关 A4 W7 评审 R5-2/F2）+ 4 个 `*_inner` 复用辅助（使大小上限/解析错误可被单测覆盖）+ 13 例边界/形状单测 + 策略脚本 `AGSK_RO_COMMAND_PARITY` 增 source-check 守门（原子性）。零执行路径/持久化/联网/插件启用/密钥日志新增。
> 补丁：`logs/assist/A5-M5-W8-agent-skill-readonly-bridge-20260907-1100.patch`（bridge.rs=`git diff` vs HEAD；policy.py=W8-only diff，基线=已含 W7+A10 的 worktree）。**未 push**。

---

## 1. W8 Hard Stops 守门（板 §M5-W8 行 202-209）

- 每条 A5 触碰命令保持只读 + 含 source check + ACL + 前端 bridge/types + 策略覆盖 + 同包测试 → **满足**（命令在 `bridge.rs` 含 `check_invocation_source`；ACL/main.rs/bridge.ts/types.ts 沿用 W7 已落地四件套；策略 `AGSK_RO_COMMAND_PARITY` 覆盖；新增单测同包）。
- 无 token/cookie/Authorization/body/prompt-secret 日志、审计、持久化、checkpoint、前端状态 → **满足**（本波未新增任何日志/持久化；validate 错误沿用 W7 行为）。
- 仅 A0 push → **满足**（本波未提交未 push）。

## 2. 交付内容

| 改动 | 文件 | 说明 |
|---|---|---|
| 输入大小上限 | `src-tauri/src/bridge.rs` | `AGENT_DEF_MAX_BYTES`/`SKILL_DEF_MAX_BYTES`=256KiB；`reject_oversized_def`；`agent_validate_inner`/`skill_validate_inner` 超限返回 `ValidationReport{valid:false}` |
| `*_inner` 复用辅助 | `src-tauri/src/bridge.rs` | `agent_parse_inner`/`skill_parse_inner`/`agent_permission_preview_inner`/`skill_permission_preview_inner`：复用大小上限拒超 + 解析；命令委托之，便于单测 |
| 边界/形状单测 +13 | `src-tauri/src/bridge.rs` `#[cfg(test)] mod agent_skill_bridge_tests` | oversized(malformed/parse/preview×Agent/Skill)、invalid dialect/acl 类型、report 形状、permission_preview 形状(gate/空 capabilities)、reject_oversized 辅助 |
| 策略 source-check 守门 | `scripts/check-agent-skill-policy.py` `c_readonly_command_parity` | 除四件套奇偶外，要求各只读 handler 含 `check_invocation_source`；缺则报错 |
| 策略坏样本 | `scripts/check-agent-skill-policy.py` `--self-test` | 新增「四件套齐全但 handler 缺 check_invocation_source」坏样本 |

注：`main.rs` / ACL / `bridge.ts` / `types.ts` 沿用 W7 已落地，本波无需改动（原子性已满足）。

## 3. 设计决策 / 边界

- **R5-2 闭环**：A4 W7 评审指出 parse 输入无界（DoS 面）。本波在 6 命令路径统一加 256KiB 上限（Agent/Skill 定义为小结构，256KiB 留足余量）；超限在 source check 之后直接拒。
- **可测性**：解析/预览原内联于命令（需 `AppHandle` 不可单测），抽 `*_inner` 后可在 `#[cfg(test)]` 直接覆盖大小上限与解析错误路径。
- **能力白名单零新增**：沿用 W7（validate 走 W4 既有 `contains_credential_leak`；permission_preview 走 `PermissionPreview`，capabilities 来自空集合 fail-closed）。本波未引入新 capability 字面量/注册表。
- **未触碰**：`security_policy.rs`/`agent.rs`/`skills.rs` 的凭据泄漏错误 `Display` 仍回显 `{s}`（A10 的 `AGSK_CREDENTIAL_NOT_ECHOED` 已识别，属 A10/S-A10 W8 修复范围，不在 A5 文件清单内）。

## 4. 校验（实跑）

```text
cargo fmt --manifest-path src-tauri/Cargo.toml        → OK
cargo test --manifest-path src-tauri/Cargo.toml        → 400 passed / 0 failed
  （含新增 13 例：reject_oversized_def_guard / agent_validate_inner_rejects_oversized /
   skill_validate_inner_rejects_oversized / agent_parse_inner_rejects_oversized /
   agent_parse_inner_rejects_malformed / skill_parse_inner_rejects_oversized /
   agent_validate_rejects_invalid_dialect / skill_validate_rejects_invalid_acl /
   agent_permission_preview_inner_shape / agent_permission_preview_inner_rejects_malformed /
   skill_permission_preview_inner_shape / report_ok_has_empty_errors / report_with_errors_shape）
python3 scripts/check-agent-skill-policy.py --self-test → PASS（ACTIVE=3，PENDING=5）
python3 scripts/check-agent-skill-policy.py            → FAIL：仅 AGSK_CREDENTIAL_NOT_ECHOED（A10 W8 PENDING 码）命中真实 security_policy.rs（CredentialLeak Display 回显 {s}）；非本波引入，属 A10 待修范围（security_policy.rs 不在 A5 文件清单）
npm run build                                       → ✓ built（无 TS 错误）
git diff --check -- src-tauri/src/bridge.rs         → 干净（我的 W8 仅 bridge.rs 改动）
```

并发工作树提示：`policy.py` 当前 worktree = HEAD（W7 未提交）+ A10 W8（`c_credential_not_echoed`）。本波补丁对 `policy.py` 取 **W8-only diff**（基线=`/tmp/policy_w8_base.py`，已含 W7+A10），A0 在集成 W7+A10 后即可应用；`bridge.rs` 用 `git diff` vs HEAD（W7 已提交），直接可应用。

## 5. 风险/挂账

- **凭据回显（A10 债）**：`security_policy.rs::PolicyError::CredentialLeak` Display 仍回显 `{s}`，A10 的 `AGSK_CREDENTIAL_NOT_ECHOED`(PENDING) 在真实仓库默认扫描命中。A10/S-A10 W8 修复（Display 改 `<redacted>`）后默认扫描方可转绿。A5 不越界改 `security_policy.rs`。
- **W4 §4 能力单一真源**：同 W7，未新增片段；根治待 A0。
- **R5-6 响应有界**：validate 错误沿用 W7（命中行回显，已受 256KiB 输入上限约束）。彻底脱敏属 A10 债。

## 6. Lane 输出模板（A5 · M5-W8）

```text
LANE=A5
STATUS=DONE
BASE=6c1f30e
HEAD=logs/assist/A5-M5-W8-agent-skill-readonly-bridge-20260907-1100.md
FILES=logs/assist/A5-M5-W8-agent-skill-readonly-bridge-20260907-1100.md + .patch
VERIFY=cargo test 400 passed/0 failed；cargo fmt OK；check-agent-skill-policy.py --self-test PASS(ACTIVE=3/PENDING=5)；npm build 无 TS 错误；bridge.rs git diff --check 干净
CHECKPOINT=logs/assist/A5-M5-W8-agent-skill-readonly-bridge-20260907-1100.md
MERGE_NOTES=W8 硬化 Agent/Skill 只读桥：输入大小上限(256KiB, AGENT/SKILL_DEF_MAX_BYTES, R5-2/F2) + 4 个 *_inner 复用辅助(可单测) + 13 边界/形状单测 + 策略 AGSK_RO_COMMAND_PARITY 增 source-check 守门(原子性) + 坏样本。零执行/持久化/联网/插件启用/密钥日志新增。main.rs/ACL/bridge.ts/types.ts 沿用 W7。policy.py 取 W8-only diff(基线含 W7+A10)。未 push。
NEXT=A0 集成(apply 补丁；bridge.rs git diff 直接可应用，policy.py W8-only 应用于 W7+A10 基线)；A10 关闭 security_policy.rs 凭据回显(AGSK_CREDENTIAL_NOT_ECHOED)；A6 后续接线 useAgentStore。
```
