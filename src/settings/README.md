# settings（框架/服务候选）文档（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/settings/manifest.ts`。
> **重要**：settings 当前是 **framework/service candidate**（`manifest.category = "SERVICE"`），**不是产品 CAPABILITY**，**不得擅自升格为 Capability**。本文件是候选文档，供未来决定是否升格时参考。
> 诚实声明：store 已迁入 `src/settings/state/`（store 物理债务解除）；UI 仍在 `src/components/system/SettingsPanel.vue`（UI 物理债务未解除）。settings 仍为 SERVICE 候选，未升格 Capability。

---

## 1. Purpose
框架偏好域。负责主题、键位方案、页签休眠等**全局框架偏好**的持久化与提供。这些是框架级偏好，不归属任何单一业务能力。

## 2. Domain Classification
- 领域：`settings`（候选，`manifest.category = "SERVICE"`）
- **非 CAPABILITY**：仅登记为常驻框架服务，不作为产品能力暴露。
- 当前状态：candidate（framework/service），未升格。

## 3. Responsibilities
- 主题（theme）、键位方案（keymapScheme）、页签休眠（tabHibernation）偏好的读写。
- `SettingsPanel` 经通用 WORKBENCH_MAIN 贡献（view=`settings`）解耦，Shell 经 `src/settings/public.ts` 显式契约访问。

## 4. Non-Responsibilities
- 不负责任何单一业务能力（如数据库/终端/浏览器）。
- 不创建重资源 / 后台任务。
- 不被升格为产品 CAPABILITY（当前禁止）。

## 5. Ubiquitous Language
- `theme`：视觉主题偏好。
- `keymapScheme`：键位方案偏好。
- `tabHibernation`：页签休眠策略偏好。

## 6. Domain Model
- 聚合根：框架偏好集合（由 `useSettingsStore` 管理）。
- 物理位置：**`src/settings/state/useSettingsStore.ts`**（不在 `src/settings/` 内 —— 物理债务）。

## 7. Invariants
- 偏好为框架级，不随任何业务能力的 absent 而改变（常驻）。
- `useSettingsStore` 是唯一前端真源。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/settings/state/useSettingsStore.ts`（`defineStore("settings")`）。
- **semanticOwner**：`useSettingsStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：store 已迁入 `src/settings/state/`（物理位置债务解除）；settings 仍为 SERVICE 候选，未升格 Capability；UI 仍在 `src/components/system/SettingsPanel.vue`（UI 物理债务未解除）。若未来升格为 Capability，UI 应迁入 `src/settings/ui/`。

## 9. Commands / Intents
- 业务 action：偏好读写（具体见 `useSettingsStore`）。
- 原生命令：NOT_APPLICABLE（纯前端偏好，无专属原生命令）。

## 10. Queries
- 前端内存：偏好派生查询。
- 无原生查询命令。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/settings/public.ts`。
- 暴露：框架偏好窄契约（经此访问，禁止 Shell 直连 store 内部）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts`（位于 `src/settings/`）。
- UI：**`src/components/system/SettingsPanel.vue`**（不在 `src/settings/ui/`，物理债务）。

## 14. Dependencies
- `dependsOn: []`（硬）。
- `optionalDependencies: []`。

## 15. Dependents
- Shell 经 `src/settings/public.ts` 与 WORKBENCH_MAIN 贡献（view=`settings`）消费。

## 16. Frontend Boundary
- 贡献组件：`SettingsPanel.vue`（WORKBENCH_MAIN，view=`settings`）。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/components/system/SettingsPanel.vue`（物理债务，不在 `src/settings/ui/`）。

## 17. Native Boundary
- **无原生 tauri command**（纯前端偏好持久化）。
- **诚实声明**：Native 集中化与本模块无关；本模块不拥有任何原生命令。

## 18. Resources
- `resources.class: ["LIGHT"]`；`suspendable: false`；`destroyable: false`（常驻）。
- `v1.resources: [{kind:"CACHE", owned, evidence:"src/settings/state/useSettingsStore.ts"}]`（**证据路径已指向 src/settings/state/useSettingsStore.ts，store 物理债务已解除**）。

## 19. Side Effects
- 写框架偏好（disk 持久化，具体机制见 `useSettingsStore`）。

## 20. Security
- 无敏感数据（框架偏好非机密）。

## 21. Persistence
- 声明 `disk` / `sensitive: false`；框架偏好落盘（具体路径见 `useSettingsStore`）。

## 22. Failure Model
- 偏好加载失败：回退默认主题/键位。

## 23. Capability Absence
- 作为常驻框架服务，**不适用** absence 语义（始终 ACTIVE）。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE"]`；`default: "ACTIVE"`；`activatable: false`；`resident: true`。

## 25. UI Contributions
- `settings.main`（WORKBENCH_MAIN / surface / view=`settings` / SettingsPanel）。

## 26. Testing
- `src/settings/` 下 `*.spec.ts`：**0**（RV3 不满足，全仓前端无 vitest）。

## 27. Gates
- `scripts/check-capability-registry.mjs`、`scripts/check-capability-composition.mjs`（manifest `maturityEvidence` 引用，为通用 registry/组合门禁，未特指 settings）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `src/settings/state/useSettingsStore.ts` → `src/components/system/SettingsPanel.vue`。
- 关注点：候选身份（禁止升格）、物理债务、框架偏好与业务偏好的边界。

## 29. AI Modification Guide
- **禁止**把 settings 升格为产品 CAPABILITY（当前为 candidate）。
- 若未来升格：必须将 `useSettingsStore` 迁入 `src/settings/state/`、UI 迁入 `src/settings/ui/`，并同步 manifest `category` 与所有 import 点 + 补专属 checker。
- 禁止：用裸 `invoke`（本模块无原生命令）。

## 30. Known Debt
- **候选身份**：`SERVICE` 分类，非 CAPABILITY，不得擅自升格。
- **物理债务（仅 UI）**：store 已迁入 `src/settings/state/useSettingsStore.ts`（store 物理债务解除）；UI 仍在 `src/components/system/SettingsPanel.vue`（非 `src/settings/ui/`，UI 物理债务未解除）。
- `v1.resources` 的 evidence 路径 `src/settings/state/useSettingsStore.ts` 反映此债务。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，有 registry/组合门禁；作为 SERVICE 候选，成熟度含义以框架服务计。
- **HP = HP2**：`manifest.v1.hotPlug.level="HP2"`（enable/disable/register/unregister=true；install/uninstall=false），由 `check-capability-platform.mjs` 验证；HP3 本夜不做。
- **M = M1（部分物理隔离）**：store 已迁入 `src/settings/state/`（store 物理隔离达成）；UI 仍在 `src/components/system/SettingsPanel.vue`（UI 未迁入 `src/settings/ui/`，目录隔离未完全达成）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（间接） + RV3（否）**：owner 已登记（RV1）；maturityEvidence 为通用门禁（RV2 间接）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3（含候选身份与物理债务诚实记录）。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：候选身份（先决策是否升格）、物理债务（store/UI 未迁入 `src/settings/`）、无 vitest。
- 升格前不评估 Package Extraction。

## 33. Source of Truth
- manifest：`src/settings/manifest.ts`
- public：`src/settings/public.ts`
- state（**已内迁 src/settings/state**）：`src/settings/state/useSettingsStore.ts`
- UI（**物理债务·在 components/system**）：`src/components/system/SettingsPanel.vue`
- semantic owner：manifest `semanticOwner: "useSettingsStore"`
- gates：`scripts/check-capability-registry.mjs`、`scripts/check-capability-composition.mjs`（见 §27）
