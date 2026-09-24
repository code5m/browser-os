# apps 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/apps/manifest.ts`。
> 诚实声明：从 `useSystemStore` 拆出（解除 Debt-8E-1）；`dependsOn:["bridge"]` 指向非 capability 适配层（命名债）；启动 detached 外部进程，本能力不管理其生命周期。

---

## 1. Purpose
系统应用域。负责列出已安装系统应用与启动外部应用（detached 进程）。

## 2. Domain Classification
- 领域：`apps`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 列出系统应用（`loadApps` → `bridge.listApps`）。
- 启动外部应用（`launchApp` → `bridge.launchApp`，经 toast 反馈）。
- 应用图标加载失败兜底（`onAppImgError`）。

## 4. Non-Responsibilities
- 不管理外部进程的生命周期（detached，启动后由 OS 接管）。
- 不创建 WebView/PTY/数据库连接。
- 不负责应用内部行为。

## 5. Ubiquitous Language
- `AppEntry`：一个系统应用条目（来自 `src/types.ts`）。
- `appFilter` / `filteredApps`：应用列表过滤。
- `brokenIcons`：加载失败的图标集合。

## 6. Domain Model
- 聚合根：应用列表（`useAppsStore` 管理）。
- 关键 state：`apps` / `appFilter` / `brokenIcons`（`src/capabilities/apps/state/useAppsStore.ts`）。

## 7. Invariants
- `launchApp` 必须经 `security_policy::check_launch_target` 白名单解析（禁 `sh -c`），不得启动未授权目标。
- 启动的进程 detached，本能力不追踪其退出/资源。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/apps/state/useAppsStore.ts`（`defineStore("apps")`，从 `useSystemStore` 拆出，解除 Debt-8E-1）。
- **semanticOwner**：`useAppsStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：RV1 弱 —— `useAppsStore` 未入语义 registry `02-STATE-SOURCES.md`（语义 registry 仅登记 browser）。

## 9. Commands / Intents
- 业务 action：`loadApps` / `launchApp` / `onAppImgError`；computed `filteredApps`。
- 原生命令（见 §17）：`list_apps` / `launch_app`。

## 10. Queries
- `list_apps`（原生）。
- 前端内存：`filteredApps` 派生。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/apps/public.ts`。
- 暴露：`useAppsStore`（再导出）、`appsManifest`、`type AppEntry`。
- 注释：仅再导出语义 owner，不创建镜像状态（SECOND_TRUTHS=0）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useAppsStore.ts` / `ui/AppPanel.vue`。

## 14. Dependencies
- `dependsOn: ["bridge"]`。⚠️ `bridge` **不是 capability**，此处指 `src/bridge.ts` 适配层 —— 命名语义债（同 home）。
- `optionalDependencies: []`。

## 15. Dependents
- `src/components/layout/UnifiedTabBar.vue`、`src/components/layout/ActivityBar.vue`（均经 public 取 `useAppsStore`）。

## 16. Frontend Boundary
- 贡献组件：`AppPanel.vue`（WORKBENCH_MAIN，view=`apps`）。
- 注册：`registerAppsContributions()`，懒加载（带 loading/error 占位）。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/capabilities/apps/ui/`（已隔离）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`list_apps` / `launch_app`）。
- 已注册（main.rs）：`bridge::list_apps` / `launch_app`（2 个就绪）。
- 前端封装：`src/bridge.ts`（`listApps→list_apps` / `launchApp→launch_app`）。
- 安全：`launch_app` 经 `security_policy::check_launch_target` 白名单（禁 `sh -c`）+ 审计。
- **诚实声明**：Native 仍集中于 `bridge.rs`，未物理模块化。

## 18. Resources
- `resources.class: ["LIGHT","PROCESS"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: [{kind:"CHILD_PROCESS", owned, evidence:"src-tauri/src/bridge.rs:launch_app (Command::spawn, detached)"}]`。
- `persistence.scope: "runtime_only"`；`sensitive: false`。

## 19. Side Effects
- 启动外部进程（`launch_app`，detached）。
- 触发 toast（经 `useLayoutStore`）。

## 20. Security
- `check_launch_target` 白名单解析，禁止 `sh -c` 等解释器注入。
- 启动审计日志。

## 21. Persistence
- 声明 `runtime_only`；无磁盘持久化（应用列表来自系统枚举，非用户数据）。

## 22. Failure Model
- 列表失败：`error` 态 + 空列表。
- 启动失败：toast 提示（经 `useLayoutStore`）。

## 23. Capability Absence
- Absent 时：`registerAppsContributions` 未执行 → WORKBENCH_MAIN 无 view=`apps` → MainArea 不渲染，且 `useAppsStore` 不实例化（注释：absent 不渲染则不实例化）。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: graceful`。

## 25. UI Contributions
- `apps.main.panel`（WORKBENCH_MAIN / surface / view=`apps` / AppPanel）。

## 26. Testing
- `src/capabilities/apps/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 原生侧：无专属单测（依赖脚本门禁）。

## 27. Gates
- `scripts/check-home-client-policy.py`（主要覆盖 home，apps 仅顺带）。
- `scripts/check-ui-boundaries.mjs`（通用 UI-08 门禁，查 `@tauri-apps`/invoke，不直接引用 `capabilities/apps` → RV2 弱）。
- **无专属 `check-apps-*` 脚本**。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useAppsStore.ts` → `ui/AppPanel.vue`。
- 关注点：启动白名单、依赖命名债、absence 门禁缺失。

## 29. AI Modification Guide
- 改启动逻辑：必须保持 `check_launch_target` 白名单，并补 `check-ui-boundaries.mjs` 或新增 `check-apps-*` 断言。
- 禁止：把 `bridge` 当 capability、用裸 `invoke`、扩大 public 再导出。
- 新增 UI 必须注册 Contribution Registry。

## 30. Known Debt
- `dependsOn:["bridge"]` 指向非 capability 适配层（命名债，同 home）。
- C2 未达 C3：`manifest` 注释自陈「无 absence 门禁 + `mainView='apps'` nav 硬编码」。
- `launch_app` 启动 detached 外部进程，本能力不管理其生命周期（HP0 限制理由）。
- RV1 弱：`useAppsStore` 未入语义 registry。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，自述非 C3（无 absence 门禁 + nav 硬编码）。
- **HP = HP0**：启动外部进程 detached、无独立可装卸生命周期（`manifest.v1.hotPlug.level="HP0"`，全 false）。
- **M = M1（完全）**：`src/capabilities/apps/` 目录隔离（state/ui 均在包内）。无独立 npm 包（非 M2）。
- **RV = RV1（弱） + RV2（弱） + RV3（否）**：owner 已登记但语义 registry 未入（RV1 弱）；无专属 checker，仅通用 `check-ui-boundaries`（RV2 弱）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、无专属 checker、语义 registry 未登记、依赖命名债。
- 物理隔离完备，但需补 RV2/RV3 后方可达 M2 候选。

## 33. Source of Truth
- manifest：`src/capabilities/apps/manifest.ts`
- public：`src/capabilities/apps/public.ts`
- state：`src/capabilities/apps/state/useAppsStore.ts`
- UI：`src/capabilities/apps/ui/`
- native：`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useAppsStore"`
- gates：`scripts/check-ui-boundaries.mjs`、`scripts/check-home-client-policy.py`（见 §27）
