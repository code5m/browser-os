# Agent Capability — Full-Stack Boundary（Capability Library Expansion v1）

> STAGE D 产物。物理：AgentManagerPanel(+AgentChatPanel 子视图) 迁入 `src/capabilities/agent/ui/`，
> 经通用 Contribution Registry 的 `WORKBENCH_MAIN` 槽（view='agents'）贡献给 MainArea；
> MainArea 只按槽渲染、不 import 能力内部。最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C1 WRAPPED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C1`）。

评级依据：
- 边界隔离已完成：manifest/public/index/ui 四段边界；语义 owner `useAgentStore` 唯一（无第二真源）；
  MainArea 不再静态 import AgentManagerPanel（贡献驱动，C3 关键）。
- **功能缺口（非边界缺失）**：执行后端 `agent_chat` / `agent_run` / `agent_install` 在 Rust 侧**未实现**
  （`bridge.ts: AGENT_SKILL_COMMANDS_AVAILABLE=false` → `guard()` 返回 false，前端绝不 invoke）。
  仅只读命令就绪：`agent_parse` / `agent_validate` / `agent_permission_preview`（bridge.rs:6529/6539/6549）。
  故当前 Agent 为「只读壳 + 解析/校验/权限预览」，chat/run 不可执行。
- 非 C3：同 database 债务（无 agent 专属 absence 门禁；mainView='agents' 导航硬编码）。

## Full-Stack Trace

```text
Agent UI (capabilities/agent/ui/AgentManagerPanel.vue + AgentChatPanel.vue 子视图)
  ↓ OWNED_BY_CAPABILITY（经 public.ts 消费语义 owner）
Agent State Owner: useAgentStore (id="agent", src/capabilities/agent/state/useAgentStore.ts)
  ↓ 意图（intents）
  chat() / run() / install() / validate() / parse() / permissionPreview()
  ↓ PUBLIC_DEPENDENCY（bridge，AGENT_SKILL_COMMANDS_AVAILABLE=false）
Agent Adapter: src/bridge.ts → agentChat?/agentRun?/...（未注册）
  ↓ NATIVE_ADAPTER（Rust，仅只读）
src-tauri/src/bridge.rs:
  agent_parse          (bridge.rs:6529)  ← 已实现
  agent_validate       (bridge.rs:6539)  ← 已实现
  agent_permission_preview (bridge.rs:6549) ← 已实现
  agent_chat/agent_run/agent_install/list  ← 未注册（前端 guard 拦截，不 invoke）
```

## 依赖关系（§20）

- dependsOn: `bridge`（manifest 已声明）。
- optionalDependencies: `knowledge_graph`（图谱节点反查，未强制）。
- **Agent ↔ Skill 纠缠（已知）**：`useAgentStore.skills` 持有 skill 状态，skill 无专属 store
  （registry `semanticOwner: null` / OWNER_PENDING_SCR）。本 stage 不拆 skill，仅记录；STAGE E 处理。
- **Agent ↔ Plugin**：无运行时耦合（plugin LOCKED，无 plugin_invoke）。
- **Agent ↔ Task / Session**：task/session 为独立能力，Agent 不拥有其 truth。

## Credential / Resource

- Credential：useAgentStore 内 SECRET 已脱敏（DEVA 固化，check-developer-owners.mjs）。前端不持有执行凭据。
- Resource：当前无执行资源（后端未实现）。若未来 agent_chat 流式执行，须明确 process/network 资源 owner + 释放（C5 证据待补）。

## Absence Behavior（§18/§29）

- Agent absent（profile 未列，如 `minimal`）→ `registerAgentContributions` 不运行
  → `WORKBENCH_MAIN` 槽中无 `view='agents'` 贡献 → MainArea `viewOf('agents')` 返回 `undefined` → 不渲染。
- Shell 不崩溃：通用 `viewOf` 分支对未知 view 返回 undefined 自然跳过。
- 已知缺口：mainView='agents' 导航项未贡献驱动 → absent 点该导航落空视图（不崩溃，UX 债务）。

## Lifecycle / Hot-plug

- lifecycle: supported `[ACTIVE, SUSPENDED]`, `activatable: true`, `resident: false`.
- Hot-plug: **HP0 STATIC**（执行后端未实现，未验证 absent 无残留前不宣称 HP1）。

## Legacy Debt（诚实，不静默消失）

1. 执行后端（agent_chat/run/install）Rust 未实现 —— 真实功能缺口，留作后续 stage。
2. `useAgentStore` 物理仍在 `src/stores/`（未迁入 `capabilities/agent/state`）；同时持有 skill 状态（STAGE E 拆分）。
3. 无 agent 专属 absence 运行时门禁。
4. `mainView='agents'` 导航项硬编码于 `useLayoutStore`/`homeUi`/`HomeLaunchers`（未贡献驱动）。

## SECOND_TRUTHS = 0

`public.ts` 仅再导出 `useAgentStore`（语义 owner），未创建 `runtime.agentOpen` / `agentVisible` 镜像状态。
DOMAIN STATE（useAgentStore）≠ CAPABILITY COMPOSITION STATE ≠ UI LOCAL STATE ≠ RESOURCE RESULT。
