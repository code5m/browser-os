# Skill（技能）

> Pilot README（Phase 1 文档样板 · 简单域）。
> 证据来源：`src/capabilities/skill/{index,public,manifest}.ts`、`capability-registry/capabilities.yaml`、
> `semantic-registry/owners.yaml`（Skill 尚未单独进 owners.yaml，owner 仅登记于 capabilities.yaml）。

## 1. Purpose
提供 Skill 定义/校验/预览与安装壳，是平台「能力库」的一部分。当前执行后端未实现，仅为**只读壳**。

## 2. Domain Classification
SUPPORTING_DOMAIN（AI 能力库）。

## 3. Responsibilities
- 经通用 Contribution Registry 向 MainArea 贡献主视图 UI（`WORKBENCH_MAIN` 槽，`view='skills'`，`SkillManagerPanel`）。
- 暴露 Skill 状态与意图的公共边界（`public.ts` 再导出 `useSkillStore` / `skillManifest`）。
- 只读命令：`skill_parse` / `skill_validate` / `skill_permission_preview`。

## 4. Non-Responsibilities
- **不持有/不读写业务状态**（`index.ts` 仅为适配器）。
- 不实现 `skill_run` / `skill_install`（Rust 未实现，走 guard 拦截）。
- 不再经 Agent store 暴露（STAGE E 已与 `agent` 解除纠缠，互不依赖）。

## 5. Ubiquitous Language
- SkillManagerPanel：技能管理面板（主视图）。
- useSkillStore：Skill 语义 owner（STAGE E 从 useAgentStore 迁出）。
- WORKBENCH_MAIN 槽：主区贡献槽，`view='skills'` 为本能力标识。

## 6. Domain Model
NOT_APPLICABLE（无 Aggregate / Entity；当前为只读壳，无领域一致性边界）。

## 7. Invariants
- INV-SKILL-1：Skill absent → `WORKBENCH_MAIN` 槽中无 `view='skills'` 贡献 → `MainArea.viewOf('skills')` 返回 undefined → 不渲染面板（能力缺席即无 UI）。
- INV-SKILL-2：禁止第二状态真源（`public.ts` 仅 re-export，不创建 `skillOpen` 之类镜像）。
- ENFORCED_BY：`contributionRegistry` 槽渲染机制 + `check-developer-owners.mjs`。

## 8. State Ownership
- STATE：`useSkillStore`（技能列表/校验结果/预览）。
- OWNER：`useSkillStore`（语义 owner，仅登记于 capabilities.yaml，未进 Semantic Registry）。
- WRITER：Skill 能力包内（ui/ 与 store）。
- PERSISTENCE：disk，非敏感。

## 9. Commands / Intents
- `skill.list` / `skill.run` / `skill.install`（provides 声明）；`run`/`install` 当前被 guard 拦截（后端未就绪）。

## 10. Queries
- 经 `useSkillStore` 只读消费列表与预览。

## 11. Events
NONE（当前无跨边界事件）。

## 12. Public Contract
- 入口：`src/capabilities/skill/public.ts`（re-export `useSkillStore` / `skillManifest` / `SkillDef`）。
- 调用方经此消费，不得 import `state/*` 或 `ui/*` 内部。

## 13. Internal Boundary
- 禁止外部 import：`src/capabilities/skill/state/*`、`src/capabilities/skill/ui/*`。
- 允许：`public.ts`。

## 14. Dependencies
- Required：`bridge`（唯一 Native 通道）。
- Optional：无。

## 15. Dependents
- Shell / MainArea（经 `WORKBENCH_MAIN` 槽 `view='skills'`）。

## 16. Frontend Boundary
- UI 表达 Skill 状态、发送意图；`index.ts` 仅注册贡献，不持状态。
- 异步组件 `SkillManagerPanel`（loading/error 态已内置）。

## 17. Native / Backend Boundary
- 只读命令：`skill_parse` / `skill_validate` / `skill_permission_preview`（Rust `src-tauri/src/skills.rs`，readonly）。
- 执行命令：`skill_run` / `skill_install` —— **未实现**（`AGENT_SKILL_COMMANDS_AVAILABLE=false`）。

## 18. Resources
- class：LIGHT（声明式，见 `resources.yaml`）。
- 实测：`measured_memory_per_capability: NOT_AVAILABLE`（Debt-7A-1）。
- 来源口径：`capability-registry/resources.yaml`。

## 19. Side Effects
- 无写盘副作用（当前只读）；安装/执行未就绪。

## 20. Permissions / Security
- 无敏感权限（`permissions: []`）。

## 21. Persistence
- scope：disk；sensitive：false。

## 22. Failure Model
- 后端未就绪：`run`/`install` 走 guard 拦截，避免静默失败。
- UI 失败：异步组件 `errorComponent` 显示「该面板暂时无法显示」，不白屏。

## 23. Capability Absence
- Skill 未注册/未 activate → 无 `view='skills'` 贡献 → MainArea 不渲染 SkillManagerPanel（无死视图）。

## 24. Runtime Lifecycle
- register：`onActivate` 调 `registerSkillContributions`。
- HP：**HP0（STATIC）**（manifest `v1.hotPlug.level`，理由：执行后端未实现，未验证 absent 无残留前不宣称 HP1）。
- activationPolicy：auto；deactivationPolicy：graceful。

## 25. UI Contribution
- `WORKBENCH_MAIN` 槽，`view='skills'`，label「技能」，icon 🛠️。

## 26. Testing
- contract：`scripts/check-developer-owners.mjs`（owner 固化）。
- 其余（unit/domain/capability/composition/runtime/human）：UNVERIFIED（README 全 D0 阶段，待补）。

## 27. Gates
- `scripts/check-developer-owners.mjs`、`scripts/check-capability-registry.mjs`。

## 28. Review Guide
- PRIMARY：`src/capabilities/skill/{index,public,manifest}.ts`、`state/useSkillStore.ts`、`ui/SkillManagerPanel.vue`。
- OUT_OF_SCOPE：agent 内部、bridge 实现。

## 29. AI Modification Guide
- 修改前必读：本 README + `manifest.ts` + `useSkillStore.ts`。
- 允许改：ui/ 与 store 内部。
- 禁止改：`public.ts` 再导出契约（改契约需 SCR）；禁止新增镜像状态。
- 改后必跑：`npm run check`（含 check-developer-owners.mjs）。

## 30. Known Debt
- BLOCKING：无。
- NON_BLOCKING：执行后端未实现（C1 只读壳）；`skill` owner 未进 Semantic Registry（`owners.yaml`）。
- FUTURE：HP1 需先验证 absent 无残留。

## 31. Physical Modularity
M1（DIRECTORY_ISOLATED：`src/capabilities/skill/{manifest,public,index}.ts` + `ui/` + `state/`）。

## 32. Reviewability
RV1（OWNER_NAVIGABLE；单一 owner + 明确 entrypoint）。无 README 历史 → 未达 RV2。

## 33. Extraction Readiness
DIRECTORY_READY（单一 owner + 窄契约 + LIGHT + 目录隔离）；未达 PACKAGE_READY（缺 README/测试面实证）。

## 34. Package Extraction Notes
待 D3 文档 + 独立测试面 + review surface 有限后评估。

## 35. Repository Extraction Notes
不适用（手册 §40 默认禁止新建仓库）。

## 36. Architecture Decisions
- STAGE E：Skill 从 useAgentStore 迁出为独立 owner（`useSkillStore`），解除与 Agent 纠缠。
- 参考：`capability-registry/capabilities.yaml` skill 条目注释。

## 37. Related Documentation
- `docs/architecture/domain/DOMAIN-INVENTORY.md`（skill 行）
- `docs/architecture/domain/CONTEXT-MAP.md`（CTX-SKILL）
- `docs/architecture/modularity/*-MATRIX.md`

## 38. Source of Truth
- 机器真源：`capability-registry/capabilities.yaml`、`capability-registry/resources.yaml`、`semantic-registry/owners.yaml`。
- 本 README **不是**第二真源；HP/C/资源以代码 `manifest.ts v1` 与 registry YAML 为准。
- **漂移警示**：`capabilities.yaml` 中 skill `lifecycle.activatable:false` / 无 maturity，与代码 `manifest.ts`（`activatable:true` / `maturity:C1` / `hotPlug:HP0`）**不一致**——以代码 manifest 为运行时权威，注册表待同步（登记为文档漂移，待修）。
