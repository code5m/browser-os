# A5 · M5-W7 整包交付：Agent/Skill 只读命令桥（parse / validate / permission_preview）

> LANE=**A5**（M5-W7 Integration Dispatch，指挥板 §M5-W7 行 137-171）：**START PRODUCT CODE** — *Implement Agent/Skill read-only command bridge: parse/validate AgentDef/SkillDef and permission preview. No skill execution, no install, no network, no persistence writes. Must include source check, ACL, frontend bridge/types, policy coverage, and tests.*
> WAVE=M5-W7（A0 于 `4b438ef` 派发；`git fetch origin && git pull --ff-only` 已最新；W7 仅 A3/A5 可写产品代码）
> 交付物：6 个只读 Tauri 命令 + `ValidationReport` DTO + source check + ACL + `main.rs` 注册 + 前端 `bridge.ts`/`types.ts` 镜像 + `check-agent-skill-policy.py` 奇偶门禁 + 单测。零执行路径、零持久化、零联网、零密钥日志。
> 本文件外另产出补丁：`logs/assist/A5-M5-W7-agent-skill-readonly-bridge-20260907-0930.patch`（仅我方 6 文件，349 行插入；不含他 lane 并发改动）。**未 push**。

---

## 1. 任务边界（W7 Hard Stops 守门）

- 仅 A3/A5 在 W7 写产品代码；本波 A5 范围 = Agent/Skill **只读**桥（parse/validate/preview）。
- 命令**只读**：不执行 skill、不 install/uninstall/enable/disable、不起 MCP server/listener、不重建 graph worker。
- 每条新命令必须原子具备：source check + ACL + 前端 bridge/types + 策略覆盖 + 测试。
- 禁止 token/cookie/Authorization/body/prompt-secret 日志或持久化。

## 2. 交付内容（6 命令 + 支撑件）

| 命令 | 行为 | 返回 | 复用契约 |
|---|---|---|---|
| `agent_parse(text)` | `AgentDef::parse` | `AgentDef` | W4 `domain.rs::AgentDef` |
| `agent_validate(text)` | parse + `validate`（凭据/字段/能力校验） | `ValidationReport{valid,errors}` | W4 `AgentDef::validate()` |
| `agent_permission_preview(text)` | parse + `permission_preview()` | `PermissionPreview` | W4 `PermissionPreview`（K3 闸门档+能力） |
| `skill_parse(text)` | `SkillDef::parse` | `SkillDef` | W4 `domain.rs::SkillDef` |
| `skill_validate(text)` | parse + `validate` | `ValidationReport` | W4 `SkillDef::validate()` |
| `skill_permission_preview(text)` | parse + `permission_preview()` | `PermissionPreview` | W4 `SkillDef::permission_preview()` |

支撑件：
- **`ValidationReport`**（新增可序列化 DTO，`bridge.rs`，`serde::Serialize`）：`{ valid: bool, errors: Vec<String> }`，供前端消费校验结果。
- **source check**：每条命令首行 `check_invocation_source(&webview, "<cmd>", None, &app)?;`（复用 A2 的 `sp::check_remote_invocation` 红线），防前端裸 invoke；与 `db_disconnect` 同范式。
- **ACL**：`src-tauri/permissions/default-commands.toml` 在 `db_disconnect` 与末条 `list_artifact_images`（K1）之间注册 6 命令。
- **注册**：`src-tauri/src/main.rs` `generate_handler!` 增 6 条 `bridge::*`。
- **前端镜像**：`src/types.ts` 增 `ValidationReport` 接口；`src/bridge.ts` 增 6 个 `invoke` 封装（`agentParse`/`agentValidate`/`agentPermissionPreview`/`skillParse`/`skillValidate`/`skillPermissionPreview`），并置 `AGENT_SKILL_READONLY_COMMANDS_AVAILABLE = true`（与运行时命令 `AGENT_SKILL_COMMANDS_AVAILABLE=false` 区分，避免误判 M5-5 运行时命令已落地）。
- **策略覆盖**：`scripts/check-agent-skill-policy.py` 增 `AGSK_RO_COMMAND_PARITY`（ACTIVE，presence-gated）：W7 只读命令须 `bridge.rs handler + main.rs 注册 + ACL + bridge.ts` 四件套齐全；`--self-test` 增对应坏样本。

## 3. 关键设计决策

- **无第二执行路径**：6 命令纯解析/校验/预览，不调 `script_runner`、不触发 `skill_run`、不写 `agent_kv`/文件。符合 AGSK_1（复用 M2-4 禁第二执行路径）。
- **能力白名单零新增**：parse/validate/preview 复用 W4 既有 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`（空名单 → fail-closed），**未新建 capability 注册表、未内嵌能力字面量**，守住 W4 §4 单一真源（白名单仍在 `security_policy.rs`；MCP 在 `domain.rs:1540`，此碎片化待 A0 收口，非本波引入）。
- **类型复用**：`PermissionPreview`/`AgentDef`/`SkillDef` 直接作为命令返回类型，前端无需新 DTO；`ValidationReport` 为唯一新增类型。
- **测试可达性**：命令逻辑下沉到 `agent_validate_inner`/`skill_validate_inner` 纯函数（无 `Webview` 依赖），`#[cfg(test)]` 直接覆盖良好/凭据泄漏/技能泄漏/报告结构，规避 Tauri 命令难单测问题。

## 4. 校验（实跑）

```text
cargo fmt --manifest-path src-tauri/Cargo.toml        → OK（无新增格式问题）
cargo test --manifest-path src-tauri/Cargo.toml        → 387 passed / 0 failed
  （含新增 5 例：accepts_good_agent / rejects_credential_leak / accepts_good_skill /
    rejects_skill_credential / report_ok_and_err）
python3 scripts/check-agent-skill-policy.py --self-test → PASS（ACTIVE=3，PENDING=4）
python3 scripts/check-agent-skill-policy.py            → PASS（无违规）
npm run build                                         → ✓ built（无 TS 错误；仅既有 useBrowserStore 动静混引告警，非本波）
git diff --check                                      → 干净（无尾随空白/冲突标记）
```

并发工作树提示：`git diff`（全量）含 A1/A2/A3/A9 等 lane 的未提交改动（reconciliation、mcp.rs、plugin.rs、check-mcp-policy.py、M5 卡）；本波补丁已用 `git diff -- <6 文件>` 收窄为仅我方交付。

## 5. 风险/挂账

- **W4 §4 能力单一真源仍 open**：MCP 在 `domain.rs:1540`、Skill/Agent 在 `security_policy.rs`，无统一 `capability.rs`；本波未引入新片段，但根治待 A0 裁决（见 W6 评审 `A5-M5-W6-plugin-manifest-review-20260906-2315.md` F1）。
- **UI 接线非本波**：`useAgentStore` 调用封装留待 A6（W7 为 SUPPORT DOCS ONLY）；`AGENT_SKILL_READONLY_COMMANDS_AVAILABLE=true` 已为后续接线放行信号。
- **运行时命令未落地**：`skill_install`/`agent_chat` 等 M5-5 命令仍仅前端占位（`AGENT_SKILL_COMMANDS_AVAILABLE=false`），本波只读桥不覆盖，符合 W7 "narrow read-only command bridge" 定位。

## 6. Lane 输出模板（A5 · M5-W7）

```text
LANE=A5
STATUS=DONE
BASE=4b438ef
HEAD=logs/assist/A5-M5-W7-agent-skill-readonly-bridge-20260907-0930.md
FILES=logs/assist/A5-M5-W7-agent-skill-readonly-bridge-20260907-0930.md + logs/assist/A5-M5-W7-agent-skill-readonly-bridge-20260907-0930.patch
VERIFY=cargo test 387 passed/0 failed；cargo fmt OK；check-agent-skill-policy.py --self-test PASS(ACTIVE=3/PENDING=4) 且默认扫描 PASS；npm run build 无 TS 错误；git diff --check 干净
CHECKPOINT=logs/assist/A5-M5-W7-agent-skill-readonly-bridge-20260907-0930.md
MERGE_NOTES=W7 产品代码落地：6 只读命令(agent/skill × parse/validate/permission_preview) + ValidationReport DTO + source check(check_invocation_source) + ACL(default-commands.toml, K1 末条 list_artifact_images 保真) + main.rs 注册 + 前端 bridge.ts/types.ts 镜像 + check-agent-skill-policy.py AGSK_RO_COMMAND_PARITY 四件套奇偶门禁 + 5 单测。零执行路径/零持久化/零联网/零密钥日志；能力白名单复用 W4 security_policy.rs（未新建注册表、未内嵌字面量）。并发工作树含他 lane 未提交改动，补丁已收窄至我方 6 文件。不 push。
NEXT=A0 集成（apply logs/assist/A5-M5-W7-*-20260907-0930.patch）；A6 后续接线 useAgentStore 调用只读桥；W4 §4 能力单一真源待 A0 收口。不移动主文档 NEXT、不 push。
```
