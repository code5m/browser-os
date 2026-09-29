# agent 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/agent/manifest.ts`。
> 重要诚实声明：Agent 执行后端（agent_chat / agent_run / agent_install 等）在 Rust 侧**尚未实现**（`bridge.ts: AGENT_SKILL_COMMANDS_AVAILABLE=false`），当前为「只读壳 + 解析/校验/权限预览」。这是真实功能缺口，不是边界缺失。

---

## 1. Purpose
智能体领域。负责 Agent 定义/安装/校验与对话壳。当前为只读视图 + 解析/校验/权限预览；chat/run 通道后端未实现，走 `guard()` 拦截、不 invoke。

## 2. Domain Classification
- 领域：`agent`
- `manifest.category = "CAPABILITY"`
- 角色：Agent 定义与对话壳；同时持有 `skill` 状态（`STAGE E` 处理 skill 拆分，semanticOwner 冻结为 `useAgentStore`）。

## 3. Responsibilities
- Agent 定义解析（`agent_parse`）、校验（`agent_validate`）、权限预览（`agent_permission_preview`）。
- Agent 面板与对话子视图的 UI 壳渲染。
- 安装/对话意图的 `guard()` 拦截（后端未就绪时零 invoke）。

## 4. Non-Responsibilities
- 不执行 agent_chat / agent_run / agent_install（后端未实现）。
- 不持有 Agent 运行时/执行引擎。
- 不创建 WebView/PTY/进程/数据库。
- 当前不真正拥有 `skill` 的执行（仅状态共存，待拆分）。

## 5. Ubiquitous Language
- `AgentDef`：Agent 定义结构（来自 `src/types.ts`）。
- `session`：一次对话会话（仅壳，无后端流）。
- `backendReady`：后端是否就绪（`=AGENT_SKILL_COMMANDS_AVAILABLE`）。
- `guard()` / `fail()`：执行拦截器（后端未就绪时拦截）。

## 6. Domain Model
- 聚合根：Agent 定义集合（由 `useAgentStore` 管理 `agents` / `agentInstalls` / `sessions`）。
- 关键 state：`agents` / `agentInstalls` / `sessions` / `selectedAgentId` / `loading` / `error` / `backendReady` / `agentValidation` / `agentParseDef` / `agentPreview`（`src/capabilities/agent/state/useAgentStore.ts`）。
- 校验/预览结果：`agentValidation` / `agentParseDef` / `agentPreview`。

## 7. Invariants
- 后端未就绪（`backendReady=false`）时，`runAgent` / `agentChat` 必须被 `guard()` 拦截，不得 invoke 任何未注册命令。
- `useAgentStore` 是 Agent + Skill 状态的唯一前端真源（直至 skill 拆分）。
- 只读命令（`parse/validate/permission_preview`）可安全调用；执行命令一律禁止。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/agent/state/useAgentStore.ts`（`defineStore("agent")`）。
- **semanticOwner**：`useAgentStore`（manifest + public 登记）。
- **PUBLIC ACCESS**：Agent store 已迁入 `src/capabilities/agent/state/`；UI 组件在能力内部读取 state，跨能力消费经 `public.ts`。

## 9. Commands / Intents
- 业务 action：`loadAgents` / `selectAgent` / `installAgent` / `ackConfirm` / `runAgent`（拦截）/ `cancelRun` / `pushChunk` / `endSession` / `validateAgent` / `clearValidation`。
- 原生命令（仅 3 个只读就绪）：`agent_parse` / `agent_validate` / `agent_permission_preview`（见 §17）。

## 10. Queries
- `agent_parse` / `agent_validate` / `agent_permission_preview`（只读，后端就绪）。
- 前端内存查询：`agents` / `sessions` 派生。

## 11. Events
- NOT_APPLICABLE（无独立领域事件总线）。

## 12. Public Contract
- 入口：`src/capabilities/agent/public.ts`。
- 暴露：`useAgentStore`（再导出）、`agentManifest`、`type AgentDef`。
- 注意：public.ts 仅再导出，**未封装新 API**（SECOND_TRUTHS=1 仅再导出，无镜像状态）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `ui/AgentManagerPanel.vue` / `ui/AgentChatPanel.vue`（子视图）。
- UI 经 `../../../stores/useAgentStore` 直连（**绕过 public 边界**，物理债务）。

## 14. Dependencies
- `dependsOn: ["bridge"]`（硬）。
- `optionalDependencies: ["graph"]`（Agent 节点与 graph 关联）。

## 15. Dependents
- `src/capability/index.ts`（装配 `agentCapability`）。
- `src/capability/profiles.ts`（full profile 含）。
- `src/stores/useInstallConfirmStore.ts`（被 store 引用）。

## 16. Frontend Boundary
- 贡献组件：`AgentManagerPanel.vue`（WORKBENCH_MAIN，view=`agents`），含子视图 `AgentChatPanel.vue`。
- 注册：`src/capabilities/agent/index.ts` `registerAgentContributions()`，懒加载 `defineAsyncComponent`。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/capabilities/agent/ui/`（已隔离），state owner 位于 `src/capabilities/agent/state/`。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/agent.rs` 仅含纯逻辑（`AgentDef::parse/validate/permission_preview`，非 `#[tauri::command]`）。
- 已注册（main.rs）：`bridge::agent_parse` / `agent_validate` / `agent_permission_preview`（**仅 3 个只读**）。
- 未注册（后端未实现）：`agent_list` / `agent_install` / `agent_chat` / `agent_chat_cancel` / `agent_runs_list`。
- 前端封装：`src/bridge.ts`（`agentParse→agent_parse` / `agentValidate→agent_validate` / `agentPermissionPreview→agent_permission_preview` 等，但执行命令未注册）。
- 开关：`bridge.ts: AGENT_SKILL_COMMANDS_AVAILABLE=false`、`AGENT_SKILL_READONLY_COMMANDS_AVAILABLE=true`。
- **诚实声明**：Native 仍集中于 `bridge.rs` / `main.rs`，未物理模块化；执行后端命令缺失是真实缺口。

## 18. Resources
- `resources.class: ["MEDIUM","NETWORK"]`；`permissions: ["network.connect"]`。
- `v1.resources: [{kind:"NETWORK", ownership:"owned", evidence:"src-tauri/src/agent*.rs (readonly)"}]`。
- `persistence.scope: "disk"`；`sensitive: false`。

## 19. Side Effects
- 当前仅只读命令，无写入副作用。
- 安装/对话（未来后端）将产生写入，当前被 `guard()` 拦截。

## 20. Security
- 执行命令被 `guard()` 拦截，避免对未实现后端的非法调用。
- 权限预览 `agent_permission_preview` 供 UI 展示所需权限。

## 21. Persistence
- 声明 `disk`；当前主要为前端内存态（agents/sessions），具体落盘策略由后端决定（后端未实现）。

## 22. Failure Model
- 后端未就绪：`runAgent`/`agentChat` 经 `guard()` 直接 fail，返回稳定错误，不 invoke。
- 只读命令失败：`error` 态 + UI 提示。

## 23. Capability Absence
- Absent 时：`registerAgentContributions` 未执行 → WORKBENCH_MAIN 无 view=`agents` → MainArea 不渲染。
- Agent store 已无旧路径残留；absent 结论仍受 Agent 专属 absence 门禁缺失约束。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: graceful`。

## 25. UI Contributions
- `agent.main.panel`（WORKBENCH_MAIN / surface / view=`agents` / icon 🤖 / AgentManagerPanel）。

## 26. Testing
- `src/capabilities/agent/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 原生侧：`src-tauri/src/agent.rs` 含 `#[cfg(test)]` 单测（parse/validate/permission_preview）。

## 27. Gates
- `scripts/check-agent-skill-ui-logic.mjs`（agent/skill UI 逻辑门禁，**存在但未在 maturityEvidence 引用**）。
- `scripts/check-developer-owners.mjs`（被 manifest `maturityEvidence` 引用，但 grep 实际**不覆盖 agent** → 引用名实不符）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `src/capabilities/agent/state/useAgentStore.ts` → `ui/AgentManagerPanel.vue`。
- 关注点：执行后端是否仍缺失、`guard()` 是否覆盖所有执行入口、UI 是否绕过 capability public contract。

## 29. AI Modification Guide
- 实现执行后端前：先实现 Rust 命令并注册 main.rs + 补 default ACL + 设 `AGENT_SKILL_COMMANDS_AVAILABLE=true`，再放开 `guard()`。
- 禁止：把 store 移回/移出时不同步 manifest `semanticOwner`；用裸 `invoke` 代替 `bridge`。
- skill 拆分（STAGE E）会改变 `semanticOwner`，需同步 Registry。

## 30. Known Debt
- **执行后端未实现**：`AGENT_SKILL_COMMANDS_AVAILABLE=false`，`agent_list/install/chat/cancel` 未注册 → 最大功能缺口（manifest 诚实声明 C1/HP0）。
- **物理债务**：Agent state owner 的物理迁移已完成；剩余债务是 absence 门禁与导航硬编码。
- **maturityEvidence 引用陈旧**：引 `check-developer-owners.mjs` 但实际不覆盖 agent（应改引 `check-agent-skill-ui-logic.mjs`）。

## 31. C / HP / M / RV / D
- **C = C1**：声明诚实（只读壳），物理未隔离 store，匹配债务；待 store 迁入后升 C3。
- **HP = HP0**：执行后端缺失，enable/disable 全 false（`manifest.v1.hotPlug.level="HP0"`）。
- **M = M1**：`src/capabilities/agent/` 目录隔离达成；无独立 npm 包（非 M2）。store 物理债务不影响 M1（目录边界已存在）。
- **RV = RV1（RV2 弱）**：`semanticOwner` 已登记（RV1）；有 `check-agent-skill-ui-logic.mjs` 但未被 maturityEvidence 引用、无 dedicated bounded-checker（RV2 弱/UNKNOWN）；RV3=不成立（无 vitest）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：store 物理债务（需迁入 state/）、执行后端未实现、无 vitest、maturityEvidence 失准。
- 完成后可达 C3/M2 候选。

## 33. Source of Truth
- manifest：`src/capabilities/agent/manifest.ts`
- public：`src/capabilities/agent/public.ts`
- state：`src/capabilities/agent/state/useAgentStore.ts`
- UI：`src/capabilities/agent/ui/`
- native：`src-tauri/src/agent.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useAgentStore"`
- gates：`scripts/check-agent-skill-ui-logic.mjs`（见 §27）
