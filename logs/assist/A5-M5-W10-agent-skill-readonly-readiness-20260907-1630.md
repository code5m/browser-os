# A5 · M5-W10 Controlled Runtime Prep — Agent/Skill Read-Only Bridge Readiness

> LANE=A5（M5-W10 Controlled Runtime Prep Dispatch，板 §M5-W10 行 176-227，A5 行 196）：**START TEST/POLICY ONLY** — *Agent/Skill read-only bridge remains locked from execution. Add missing policy/tests only if A10/A4 identify a concrete leak; otherwise produce no product-code patch and record readiness for future execution design.*
> BASE=3792115（W9 已集成：`3792115 feat(M5): integrate W9 runtime-free polish`）
> 处置：本波**零产品代码改动**——A4 W10 隐私评审笔记已生成（`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`，REVIEW ONLY），但其对 Agent/Skill 面**未判定任何具体泄露**：W9 残项 R-W9-1/R-W9-2 被 A4 明确定性为「建议」而非「具体漏洞」，且 A4 不强制 A5 处理（行 179-185）；R-W9-2 所建议的「errors.join 不含 secret 断言」已在 W9 由 A5 增补断言锁定（bridge.rs `agent_validate_redacts_credential_leak`/`skill_validate_redacts_credential_leak`）。A10 W10 终审笔记本波未见，A4 为 Agent/Skill 隐私主审且未报泄露。故仍判定**无具体泄露点背书**，仅出「就绪/PASS」记录，**无补丁**。
> 交付物：`logs/assist/A5-M5-W10-agent-skill-readonly-readiness-20260907-1630.md`（本文件）。**未 push**。

---

## 1. W10 Hard Stops 守门（板 §M5-W10 行 204-211）

- 仅 A3 可触 MCP 运行时预备代码；其余运行时面保持锁定 → **满足**（A5 仅评审 Agent/Skill 只读桥，未触 MCP）。
- 无 TCP 监听/HTTP server/网络绑定/后台 daemon/插件 install/enable/delete/download/skill·agent 执行/模型调用/隐藏 script·db 执行 → **满足**（只读桥零执行）。
- 任何 rmcp/tokio 新增须可选 + feature-gated + 默认构建不变 → **满足**（本波未新增任何依赖/代码）。
- 所有命令面保持来源校验 + ACL 同步 → **满足**（6 只读命令均有 `check_invocation_source` 且 ACL 注册）。
- 构建指标阈值 22%；cargo 警告不得增加 → **满足**（零产品代码改动，指标/警告不受影响）。
- 仅 A0 push → **满足**（未 push）。

## 2. Agent/Skill 只读桥执行锁定证据（实查）

6 个只读命令（`src-tauri/src/bridge.rs` 6144-6271）逐个核对：

| 命令 | 来源校验 | 内部路径 | 执行调用 |
|---|---|---|---|
| `agent_parse` (6176) | `check_invocation_source` (6181) | `agent_parse_inner` → `reject_oversized_def` + `AgentDef::parse` | 无 |
| `agent_validate` (6186) | `check_invocation_source` (6191) | `agent_validate_inner` → parse + `validate()` | 无 |
| `agent_permission_preview` (6196) | `check_invocation_source` (6201) | `agent_permission_preview_inner` → parse + `permission_preview()` | 无 |
| `skill_parse` (6244) | `check_invocation_source` (6249) | `skill_parse_inner` → `reject_oversized_def` + `SkillDef::parse` | 无 |
| `skill_validate` (6254) | `check_invocation_source` (6259) | `skill_validate_inner` → parse + `validate()` | 无 |
| `skill_permission_preview` (6264) | `check_invocation_source` (6269) | `skill_permission_preview_inner` → parse + `permission_preview()` | 无 |

- 全 6 命令在 `src-tauri/src/main.rs` `invoke_handler` 注册（行 1434-1439）；ACL `src-tauri/permissions/default-commands.toml` 行 118-123 同步（置于 `read_artifact_defs` 能力下）。
- 全 6 命令体仅 parse/validate/preview，**无任何** `script_runner`/`Command::new`/`.spawn()`/`tokio::process`/网络调用。grep `skill_exec|agent_exec|execute_skill|run_skill|script_runner|spawn|Command::new|tokio::process` 在 `bridge.rs` 的命中全部位于 script 运行时（893-3391）与 OS 集成（4149/4825/4835/4854）段，无一处落在 6146-6271 只读桥区。
- 输入上限：`reject_oversized_def` 以 `AGENT_DEF_MAX_BYTES=SKILL_DEF_MAX_BYTES=256*1024` 在来源校验之后拒绝超大 payload（A4 W7 R5-2/F2）。
- 错误脱敏：`CredentialLeak` Display 经 A0 改为 `<redacted>`（W8/W9 已闭环 F-W8-1），校验错误不回显密文（W9 已在 `bridge.rs` 增补 3 例断言锁定）。

**A4 W10 隐私评审交叉确认（行 127-129、179-185）**：A4 明确 `Agent/Skill validate errors 走 agent_validate_inner / skill_validate_inner → format!("{e}") → <redacted>`，并将 W9 残项 R-W9-1（升 `AGSK_CREDENTIAL_NOT_ECHOED` PENDING→ACTIVE）/R-W9-2（补 errors 不含 secret 断言）定性为「建议」而非「具体漏洞」，声明「不属具体漏洞，A4 不强制 A5 升/补」。A4 未对 Agent/Skill 只读桥提出任何需 A5 落代码/测试的泄露项 → 与 A5 本波「无具体泄露点」判定一致。

## 3. 校验（实跑，零改动下复测）

```text
cargo test --manifest-path src-tauri/Cargo.toml agent_skill_bridge_tests → 26 passed / 0 failed
python3 scripts/check-agent-skill-policy.py --self-test                → PASS（ACTIVE=3，PENDING=5）
python3 scripts/check-agent-skill-policy.py                           → PASS（无违规）
git status --short                                                   → A5 产品代码范围零改动（git diff --stat bridge.rs/agent.rs/skills.rs/domain.rs/check-agent-skill-policy.py 为空）；工作树共存他 lane W10 产物（A8 Graph UI: src/stores/useGraphStore.ts、src/utils/graphUi.ts；A4/A9 评审笔记；M5-0-overview.md）——均不在 A5 Agent/Skill 只读桥范围，未触碰。
```

对照板 §M5-W10 行 179 事实「Agent/Skill bridge tests 26 passed」一致，无回归。

## 4. 未来执行面解锁前的就绪记录（READINESS）

本桥当前为**只读/无执行**设计；未来若需开放 skill/agent 执行（下一 wave），须满足以下前置且保持红线：

- **来源校验不降级**：`check_invocation_source` 三态（main 免令牌；tab-*/grid-* 无令牌拒绝；其余拒绝）须对新增执行命令同样强制。
- **能力白名单**：`validate()` 已按 `SKILL_CAPABILITY_V1`（W4 §4 单一真源，当前空）校验；执行命令须复用同一白名单，禁止新增未声明 capability。
- **错误/审计脱敏**：执行结果、tool URL、capability reason、序列化错误不得含 token/secret/body/prompt（沿用 A0 `<redacted>` Display + A4 隐私双扫 `SENSITIVE_*`）。
- **输入有界 + 响应有界**：保留 256KiB 输入上限；执行面须新增明确超时/并发/取消（复用 `script_runner` 的 `ScriptProcessTable::cancel` 语义，不另造）。
- **命令同包原子性**：任何新执行命令必须 bridge/types/policy/tests 同包交付，且 ACL 插在末条 `list_artifact_images` 之前。
- **不触 MCP 运行时预备（A3 专属）**：执行面不得引入 rmcp/tokio 监听/网络；若需经 MCP 暴露，待 A3 恢复并 feature-gated。

## 5. 结论

- **PASS / 无具体泄露点**。Agent/Skill 只读桥执行锁定完整、来源校验齐备、错误脱敏、输入有界、测试 + 策略全绿。
- **无产品代码补丁**（W10 A5 = TEST/POLICY ONLY；无 A10/A4 背书的具体泄露，故不补代码/测试）。
- 工作树干净，未 push，待 A0 集成 W10 各 lane 后统一推送。

## 6. Lane 输出模板（A5 · M5-W10）

```text
LANE=A5
STATUS=DONE
BASE=3792115
HEAD=logs/assist/A5-M5-W10-agent-skill-readonly-readiness-20260907-1630.md
FILES=logs/assist/A5-M5-W10-agent-skill-readonly-readiness-20260907-1630.md
VERIFY=cargo test agent_skill_bridge_tests 26/0；check-agent-skill-policy.py --self-test PASS(ACTIVE=3/PENDING=5)；默认扫描 PASS（无违规）；git status 干净
CHECKPOINT=logs/assist/A5-M5-W10-agent-skill-readonly-readiness-20260907-1630.md
MERGE_NOTES=W10 A5=TEST/POLICY ONLY：零产品代码改动。6 只读命令逐个确认执行锁定（来源校验齐备、零执行调用、256KiB 输入上限、错误脱敏）；测试 26/0 全绿、策略 PASS。无 A10/A4 W10 泄露背书，故不出补丁，仅记录就绪（未来执行面解锁前置红线）。未 push。
NEXT=A0 集成 W10（A3 MCP stdio-prep 专属 + 其余 lane 文档/评审）；A5 后续 wave 接 skill/agent 执行设计须满足 §4 红线。
```
