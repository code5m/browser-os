# M5-W15 Lane A5：Agent/Skill 执行锁回归评审（Verdict）

> LANE=**A5**｜Dispatch：**M5-W15 Release Readiness**｜Scope：**Agent/Skill execution-lock regression review**｜Deliverable：**Verdict**
> 评审性质：**REVIEW ONLY，零产品代码**；未提交、未 push。
> 硬停依据：`PARALLEL_COMMAND_BOARD.md` M5-W15 → "no Agent/Skill execution" + "UI must call only `src/bridge.ts`; no raw Tauri `invoke`"。

---

## 1. 评审方法

四层穿透核对（后端命令注册 → bridge 封装 → store 动作 → UI 组件），覆盖 W15 硬停中的 Agent/Skill 执行锁与「无裸 invoke」两条：

- `grep -rn` 后端 `src-tauri` 全树（`default-commands.toml` / `main.rs` / `bridge.rs` / `domain.rs`）确认 `agent_chat` / `skill_run` 等命令是否被注册；
- `grep` `src/bridge.ts` 中 `skillRun` / `agentChat` / `agentRunCancel` 的调用方；
- `grep` `src/stores/useAgentStore.ts` 中 `runAgent` 的调用方；
- `grep` 全部 `*.vue` 的裸 `invoke(` 与执行可享项（`run`/`execute`/`执行`/`运行` 按钮）。

---

## 2. 证据

### 2.1 后端层：命令未注册 → 后端不暴露执行能力（fail-closed 第一层）

`grep` `src-tauri` 全树（含 `permissions/default-commands.toml`、`main.rs`、`bridge.rs`）：

- `agent_chat` / `skill_run` / `agent_chat_cancel` / `skill_runs_list` / `agent_runs_list` 的**唯一命中**是 `src-tauri/src/domain.rs:1924` 的注释：
  > `skill_runtime` 执行期也不得引入（由 `check-agent-skill-policy.py` 守门）。
- 上述命令**均未进入 `generate_handler!`，也未进入 ACL**。

→ 即便前端误调用 `bridge.agentChat` / `bridge.skillRun`，后端也会以「命令不存在」失败（fail-closed）。

### 2.2 bridge 封装层：执行封装存在但无调用方

- `src/bridge.ts:422` `skillRun → invoke("skill_run")`：**全树 0 调用方**（仅定义 + `types.ts` 的 `SkillRunRecord` 类型）。
- `src/bridge.ts:426` `agentChat → invoke("agent_chat")`：**唯一调用方** = `useAgentStore.ts:248` 的 `runAgent`（见 §2.3）。
- `src/bridge.ts:429` `agentRunCancel → invoke("agent_chat_cancel")`：**0 调用方**。
- `src/bridge.ts:453/454` `skillRunsList` / `agentRunsList` → `skill_runs_list` / `agent_runs_list`：只读列表命令（安全；后端亦未注册）。

### 2.3 store 层：`runAgent` 保留但未接线

- `src/stores/useAgentStore.ts:239` `async function runAgent(...)` 定义，`:370` 导出；**全树 0 个调用方**（无 `.vue`、无其它 store 触发它）。
- 即 `runAgent → bridge.agentChat → agent_chat` 是**预留执行链**（W10「执行通道尚未开放」），当前无任何 UI 入口可达。

### 2.4 UI 层：锁的真正表面 — 无执行可享项

- `src/components/workspace/AgentChatPanel.vue`：仅渲染既有会话的**只读**对话壳；**无输入框/发送按钮**（`.chat-foot` 样式存在但模板无对应元素；注释明示 "no execution buttons"）。
- `AgentManagerPanel.vue` / `SkillManagerPanel.vue`：grep `run|execute|invoke|执行|运行` → **0 匹配**，无 run/execute 按钮。
- **全树 `.vue` 文件 0 处裸 `invoke(`**；所有 UI 调用均经 `src/bridge.ts`。
- `src/bridge.ts` 内 46 处裸 `invoke(` 是唯一的授权封装点；stores / composables / components 均不直接 invoke（满足 W14/W15「no raw Tauri invoke」硬停）。

### 2.5 既有主动门禁

- `scripts/check-agent-skill-policy.py`（35 KB，9-07 14:38 更新）已接入 `scripts/pre-merge.sh:457-460`（运行段）+ `:609-611`（self-test 段），守「第二执行路径 / Inline / ACL 末条 / 能力漂移」。

---

## 3. 结论（VERDICT）

**Agent/Skill 执行锁在 W15 无回归，保持 INTACT（双层 fail-closed）：**

1. **后端层**：不注册 `agent_chat` / `skill_run` 等命令 → 即便前端误调也失败；
2. **前端层**：无任何 UI 可享项触发执行；`runAgent` / `bridge.skillRun` / `bridge.agentChat` 均为**无调用方的预留代码**（W10 起「执行通道尚未开放」）。

**评审结论 = PASS（执行锁 intact，无回归）。**

---

## 4. 备注 / 交 A0

- `runAgent` + `bridge.agentChat` / `bridge.skillRun` 是 W10 起保留的**预留执行入口，非回归**；若未来 wave 要开放 Agent/Skill 执行，必须：(a) 后端注册命令并满足 `check-agent-skill-policy.py`；(b) UI 增加受控可享项。当前零风险。
- 本评审**仅覆盖 Agent/Skill 执行锁回归**。其余 W15 硬停（plugin invoke / graph write-export / 后台 worker / 敏感渲染 / 原始 invoke）分别由 A3 / A4 / A7 / A10 评审；A5 顺带确认「UI 无裸 invoke」成立（支持性观察，非本 Lane 主责）。
- A5 自有 **M4-4 数据库 UI 交付包**（`logs/checkpoints/A5-M4-4-database-ui-20260905-2320.md` + `logs/checkpoints/Lane-A5-M4-4-database-ui-20260905-2320.patch`，live wiring + pre-merge 接线已完成）本次重跑 `node scripts/check-database-ui-logic.mjs` = **119 断言全绿**，整包对发布就绪态仍有效，未受 W15 影响。
- 未提交、未 push、未 reset、未 force-push。

## 5. 交付物清单（本 W15 包）

| 文件 | 类型 | 说明 |
|---|---|---|
| `logs/assist/A5-M5-W15-agent-skill-lock-verdict-20260907-1505.md` | 新增（评审） | 本 verdict，零产品代码 |
| `logs/checkpoints/A5-M4-4-database-ui-20260905-2320.md` | 既有（M4-4） | 数据库 UI 整包 checkpoint，发布就绪仍有效 |
| `logs/checkpoints/Lane-A5-M4-4-database-ui-20260905-2320.patch` | 既有（M4-4） | 数据库 UI 整包补丁（A5 独占文件） |

**STATUS = PASS_WITH_CONTEXT**（W15 执行锁评审 PASS；M4-4 数据库 UI 整包已完成且发布就绪有效）。
