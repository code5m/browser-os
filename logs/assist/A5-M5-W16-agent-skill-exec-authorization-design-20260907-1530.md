# M5-W16 Lane A5：Agent/Skill 执行「授权」设计（Contract Note）

> LANE=**A5**｜Dispatch：**M5-W16 M5 Closeout and M6 Charter**｜Scope：**Agent/Skill execution authorization design only; no execution code**｜Deliverable：**Contract note**
> 性质：**DOCS ONLY**。零产品代码、不新增/不注册命令、不改 ACL/DTO/bridge、不实现执行、未提交、未 push。
> W16 硬停：no product code, no new command/ACL/bridge/DTO, no plugin invoke/execution, 无动态加载/网络/daemon/model call/Agent-Skill 执行/MCP 扩张/graph 写导出/后台 worker/裸 invoke/敏感渲染。

---

## 1. 目的与边界

本文只设计**「执行授权」（authorization）**，即：当未来某个 wave 被批准开放 Agent/Skill 执行时，**凭什么、经由哪条既有管道、满足哪些闸门才算被授权**。

- ✅ 本文做：授权模型、既有可复用骨架、放开前置清单、闸门映射。
- ❌ 本文不做：任何执行实现、命令注册、ACL/DTO/bridge 改动、UI 新增执行按钮。

授权 ≠ 执行。授权设计成立**不代表**执行被开放；执行仍由 `AGSK_EXEC_LOCKED` 总闸 FAIL-CLOSED 阻断（见 §4）。

---

## 2. 现状基线（授权设计的起点，全部经实读/实跑核对）

| 层 | 事实 | 依据 |
|---|---|---|
| 后端命令 | `agent_chat` / `skill_run` / `agent_chat_cancel` / `skill_runs_list` / `agent_runs_list` **均未注册**（未进 `generate_handler!`、未进 ACL） | grep `src-tauri` 全树，唯一命中是 `domain.rs:1924` 守门注释 |
| 只读桥 | `agent_parse` / `agent_validate` / `agent_permission_preview` 及 `skill_*` 同款**已落地可用**（M5-W7 A5） | `bridge.ts:102,396-405` |
| 前端执行封装 | `bridge.skillRun`(`→skill_run`, `:422`)、`bridge.agentRunCancel`(`:429`) **0 调用方**；`bridge.agentChat`(`:426`) 唯一调用方为 store 内 `runAgent` | grep 全树 |
| store | `useAgentStore.runAgent`(`:239`，导出 `:370`) **全树 0 调用方** → 预留执行链，无 UI 入口 | grep 全树 |
| UI | `AgentChatPanel.vue` 只读（无输入/发送控件）；`AgentManagerPanel`/`SkillManagerPanel` grep `run/execute/invoke/执行/运行` = **0 匹配** | 实读 + grep |
| 域探针 | `SkillDef`(`domain.rs:1968`) / `AgentDef`(`:2006`) 已存在 → `_agent_skill_present` = **真** | `check-agent-skill-policy.py:66-80` |
| 门禁 | `check-agent-skill-policy.py` = **ACTIVE=3 / PENDING=10，self-test PASS**；已接入 `pre-merge.sh:457-460` + `:609-611` | 实跑 + grep |

**关键推论**：因域探针为真，10 个 PENDING 码位**已在守门**（非"未生效"）。即执行锁当前由策略脚本主动强制，不只是"没人调用而已"。

---

## 3. 授权模型（三要素 + 既有骨架）

### 3.1 三要素

- **主体**：用户显式确认（经 `PermissionPreviewModal` + 二段式 `request_id` 确认）。
- **客体**：具体目标 —— skill（`id` + `inputs`）或 agent（`agentId` + `prompt` + `sessionId`）。
- **许可档位**：来自后端 **`PermissionPreview.gate`**，**不是** `AgentDef.acl`。
  > `types.ts:697` 明载：`AgentDef` 自身**无** acl 字段；其闸门来自后端 `PermissionPreview.gate`。

### 3.2 既有可复用骨架（未来执行 wave **接线即可，无需新发明授权机制**）

1. **许可预览（只读，已可用）**
   `bridge.agentPermissionPreview` / `skillPermissionPreview` → `agent_permission_preview` / `skill_permission_preview`；展示态 `buildPermissionPreview`（`agentSkillUi.ts:161`），其语义明载为「**安装/运行前**用户可见闸门档与所需能力」。
2. **闸门弹窗（已接线）**
   `PermissionPreviewModal.vue` 已被 `AgentManagerPanel.vue:6,65` 与 `SkillManagerPanel.vue:6,68` 同时引入 —— 运行授权应复用同一弹窗，不另造确认 UI。
3. **二段式确认管道（已有，含 `run_skill` 预留槽位）**
   `setPending(action, payload)` → 产出 `request_id` → `confirmSkill` / `confirmAgent(requestId, decision)`（`useAgentStore.ts:127,222-223`）。
   `PendingConfirmAction`（`types.ts:779`）= `"install_skill" | "run_skill" | "install_agent"` —— **`run_skill` 已声明但 store 未处理**（`:222-223` 只分支 `install_*`）。
   → **结论**：运行授权必须接入这个既有 `request_id → confirm` 槽位与同一条管道；**不得另起旁路**（旁路即触 `AGSK_SECOND_PATH`）。
4. **取消/中断（已预留）**
   `bridge.agentRunCancel`（`→ agent_chat_cancel`）已存在，供执行期取消/超时收敛。

---

## 4. 授权前必须满足的闸门（映射现存策略码位）

放开执行授权时，下列码位必须逐项 PASS；**`AGSK_EXEC_LOCKED` 只有 A0/A10 书面批准后才可定向放开**。

**PENDING 10（探针为真 → 已在守门）**

| 码位 | 在授权语境下的含义 |
|---|---|
| `AGSK_EXEC_LOCKED` | **执行锁总闸**。未批准前保持 FAIL-CLOSED；批准是"定向放开 + 转 ACTIVE"，不是删除检查 |
| `AGSK_SECOND_PATH` | 禁止第二执行路径：运行授权只走既有 `request_id → confirm` 管道 |
| `AGSK_INLINE_SHELL_ENUM` / `AGSK_INLINE_SHELL_REF` | 禁止内联 shell/执行变体：skill 执行不得退化为内联命令拼接 |
| `AGSK_COMMAND_PARITY` | 命令与 ACL 齐平：注册的每个命令必须同步进 ACL |
| `AGSK_CREDENTIAL_NOT_ECHOED` | 凭据不回显：授权/预览/错误渲染链路均不得回显 secret |
| `AGSK_PLUGIN_MANIFEST_AGENT_CAP` | 插件清单不得声明 agent 执行能力 |
| `AGSK_PLUGIN_CMD_NOT_EXEC` / `AGSK_PLUGIN_UI_EXEC_AFFORDANCE` / `AGSK_PLUGIN_BRIDGE_NO_EXEC` | 插件侧不得成为执行旁路（命令/UI/bridge 三层） |

**ACTIVE 3（始终守门）**：`AGSK_ACL_TAIL`（末条恒为 `list_artifact_images`，新命令插其之前）、`AGSK_CAPABILITY_DRIFT`（能力漂移）、`AGSK_RO_COMMAND_PARITY`（只读命令齐平）。

---

## 5. 执行授权放开的最小前置清单（未来 wave，非本 wave 实施）

1. 后端**注册** `skill_run` / `agent_chat`（当前缺失）并进 ACL —— 新命令插在末条 `list_artifact_images` **之前**。
2. `bridge.ts` 仍为**唯一** `invoke` 封装点（`skillRun`/`agentChat` 已存在，仅需接线；全树 `.vue` 保持 0 裸 invoke）。
3. UI 只走既有 `PermissionPreviewModal` + 二段式 `request_id` 确认（`run_skill` 槽位），**不新增旁路按钮**。
4. 按脚本注释（`check-agent-skill-policy.py:23`）语义处理码位：「一旦执行层/命令开始，把对应 PENDING 码位 gate 去掉、转 ACTIVE」。
5. 取消/超时/审计：复用 `agentRunCancel`；审计与错误渲染不得回显凭据（对齐 `AGSK_CREDENTIAL_NOT_ECHOED`）。

---

## 6. 明确不做（non-goals，对齐 W16 硬停）

- 不写执行代码、不注册命令、不改 ACL / DTO / bridge、不新增 UI 执行按钮。
- 不引入网络下载、内联 shell、动态加载、daemon、model call、后台 worker、第二执行路径。
- 不触碰插件执行、MCP、graph 写导出等他 Lane 表面。

## 7. 与 A5 W15 结论的衔接

W15 verdict（`logs/assist/A5-M5-W15-agent-skill-lock-verdict-20260907-1505.md`）判定「执行锁 intact、双层 fail-closed」。本文是其**正向延续**：锁继续保持，同时把"将来如何**被授权**地打开"固化为契约，避免未来 wave 各行其是地新造授权机制或开旁路。

## 8. 交付物与状态

| 文件 | 类型 | 说明 |
|---|---|---|
| `logs/assist/A5-M5-W16-agent-skill-exec-authorization-design-20260907-1530.md` | 新增（docs） | 本文，零产品代码 |
| `logs/assist/A5-M5-W15-agent-skill-lock-verdict-20260907-1505.md` | 既有（W15） | 执行锁回归 verdict = PASS |
| `logs/checkpoints/A5-M4-4-database-ui-20260905-2320.md` + `.patch` | 既有（M4-4） | 数据库 UI 整包，发布就绪有效（fixture 119/119、build PASS） |

**STATUS = DOCS_ONLY_COMPLETE**（W16 A5 授权设计契约已成型；执行仍锁，未提交、未 push）。
