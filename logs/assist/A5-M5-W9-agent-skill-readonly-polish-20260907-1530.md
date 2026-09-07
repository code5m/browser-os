# A5 · M5-W9 Runtime-Free Polish 整包交付

> LANE=A5（M5-W9 Runtime-Free Polish Dispatch，板 §M5-W9 行 175-203）：**START PRODUCT CODE SMALL** — *Finish Agent/Skill read-only bridge hardening only: add focused tests for redacted validation errors and frontend parse/permission preview edge cases. No execution, install, persistence write, network, or model call.*
> BASE=ef87401（W8 已集成：`4d7be97 feat(M5): integrate W8 command bridge polish` + `97118d6 chore(M5): normalize W8 patch evidence whitespace`）
> 交付物：9 例聚焦测试（脱敏校验错误 + 解析/权限预览边界），闭环 A4 W8 评审 F-W8-1（凭据回显）。零执行/持久化/联网/插件启用/模型调用。
> 补丁：`logs/assist/A5-M5-W9-agent-skill-readonly-polish-20260907-1530.patch`（bridge.rs = `git diff` vs HEAD，仅 W9 增量）。**未 push**。

---

## 1. W9 Hard Stops 守门（板 §M5-W9 行 202-209 同类）

- 仅聚焦测试 + 只读桥硬化，无执行/安装/持久化写/联网/模型调用 → **满足**（仅 `bridge.rs` `#[cfg(test)]` 新增 9 测试）。
- 无 token/cookie/密钥日志新增 → **满足**。
- 仅 A0 push → **满足**（未提交未 push）。

## 2. 交付内容

| 改动 | 文件 | 说明 |
|---|---|---|
| 脱敏校验错误单测 | `src-tauri/src/bridge.rs` `#[cfg(test)] mod agent_skill_bridge_tests` | `credential_leak_display_is_redacted`：直接断言 `PolicyError::CredentialLeak` Display 不再回显密文（`<redacted>`） |
| 端到端脱敏校验 | `src-tauri/src/bridge.rs` | `agent_validate_redacts_credential_leak` / `skill_validate_redacts_credential_leak`：经 `agent_validate_inner`/`skill_validate_inner` 走真实 validate 路径，断言 `ValidationReport.errors` 不含 `sk-abc123XYZ` 且指示泄露（`<redacted>`/`凭据`/`密钥`） |
| 权限预览边界 | `src-tauri/src/bridge.rs` | `agent_permission_preview_gate_is_confirm`（Agent 恒 Confirm）、`skill_permission_preview_gate_maps_acl`（safe/confirm/dangerous 映射）、`skill_permission_preview_reflects_declared_capabilities`（预览原样透出声明能力，不臆造/不遗漏） |
| 解析边界 | `src-tauri/src/bridge.rs` | `agent_parse_inner_rejects_empty` / `skill_parse_inner_rejects_empty`（空串/纯空白拒） |

注：`agent.rs`/`skills.rs`/`bridge.ts`/`types.ts` 无需改动——W9 测试经 bridge 命令路径（`*_inner` 复用辅助，W8 已落）端到端覆盖校验/预览/解析；`CredentialLeak` 脱敏由 A0 在 `security_policy.rs` Display 修复（`4d7be97` 集成链），本波仅加测试锁定行为。

## 3. 设计决策 / 边界

- **F-W8-1 闭环（A4 W8）**：A0 已将 `security_policy.rs::PolicyError::CredentialLeak` Display 改为 `<redacted>`（不再 `{s}` 回显密文）。本波以 3 层测试锁定：① Display 单元不回显；② 经 `agent_validate_inner` 的 `ValidationReport.errors` 不回显；③ 经 `skill_validate_inner` 不回显。三层均断言密文缺失 + 泄露指示存在。
- **权限预览只读性**：`permission_preview` 仅解析 + 返回 `gate`/`capabilities`，无执行/网络；`capabilities` 透出定义声明值（上游 `validate` 已按 `SKILL_CAPABILITY_V1` 白名单校验），测试确认不臆造。
- **输入上限沿用 W8**：解析路径仍受 `AGENT_DEF_MAX_BYTES`/`SKILL_DEF_MAX_BYTES`(256KiB) 约束（W8 落地），本波空串/空白边界在此基础上补充。

## 4. 校验（实跑）

```text
cargo fmt --manifest-path src-tauri/Cargo.toml        → OK
cargo test --manifest-path src-tauri/Cargo.toml        → 408 passed / 0 failed
  （含新增 9 例：credential_leak_display_is_redacted / agent_validate_redacts_credential_leak /
   skill_validate_redacts_credential_leak / agent_permission_preview_gate_is_confirm /
   skill_permission_preview_gate_maps_acl / skill_permission_preview_reflects_declared_capabilities /
   agent_parse_inner_rejects_empty / skill_parse_inner_rejects_empty + 既有）
python3 scripts/check-agent-skill-policy.py --self-test → PASS（ACTIVE=3，PENDING=5）
python3 scripts/check-agent-skill-policy.py            → PASS（无违规）  ← A0 脱敏修复后默认扫描转绿，F-W8-1 彻底闭环
git diff --check -- src-tauri/src/bridge.rs            → 干净（仅 W9 增量）
```

并发工作树提示：当前 dirty 仅 `logs/checkpoints/M5-20260906/M5-0-overview.md`（他 lane 进行中文档，非本任务文件，未触碰）+ 他 lane W9 笔记（A2/A7，未跟踪）。本波补丁仅含 `bridge.rs` W9 增量。

## 5. 风险/挂账

- **无新增挂账**：W8 的凭据回显（A10 债）已由 A0 在 `security_policy.rs` Display 修复并经验证（默认扫描 PASS）；本波测试将其锁定为回归防护。
- **W4 §4 能力单一真源**：沿用 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`（空），未新增片段。
- **R5-6 响应有界**：校验错误沿用脱敏 Display + 256KiB 输入上限，无密文/无超长回显。

## 6. Lane 输出模板（A5 · M5-W9）

```text
LANE=A5
STATUS=DONE
BASE=ef87401
HEAD=logs/assist/A5-M5-W9-agent-skill-readonly-polish-20260907-1530.md
FILES=logs/assist/A5-M5-W9-agent-skill-readonly-polish-20260907-1530.md + .patch
VERIFY=cargo test 408 passed/0 failed；cargo fmt OK；check-agent-skill-policy.py --self-test PASS(ACTIVE=3/PENDING=5)；默认扫描 PASS（无违规）；bridge.rs git diff --check 干净
CHECKPOINT=logs/assist/A5-M5-W9-agent-skill-readonly-polish-20260907-1530.md
MERGE_NOTES=W9 聚焦测试：9 例（脱敏校验错误×3 闭环 F-W8-1 + 权限预览边界×3 + 解析空串边界×2）经 bridge 命令路径端到端覆盖；零执行/持久化/联网/插件启用/模型调用；agent.rs/skills.rs/bridge.ts/types.ts 无需改动。policy 默认扫描转绿。未 push。
NEXT=A0 集成补丁（git diff vs HEAD，直接可应用）；A6 接 Agent/Skill 面板消费；后续 MCP server/plugin 运行时 wave 开启。
```
