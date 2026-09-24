# home 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/home/manifest.ts`。
> 诚实声明：纯 JS + localStorage 持久化，无原生命令；`dependsOn` 含非 capability 的 `bridge`（命名债）。

---

## 1. Purpose
主页/启动域。负责快捷方式（目录/网页）、最近文件、收藏入口、应用启动入口，以及「收藏当前页/目录」窄契约。

## 2. Domain Classification
- 领域：`home`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 快捷方式管理（`addShortcut`/`remove`/`resetDefault`/`seedDirShortcuts`）。
- 最近文件（`recents`）加载/保存。
- 收藏当前页/目录/应用的窄契约（`favoriteDirectory`/`favoriteCurrentPage`/`favoriteCurrentDir`）。

## 4. Non-Responsibilities
- 不负责实际导航执行（由 `browser` 承担）。
- 不创建 WebView/PTY/进程/数据库连接。
- 不负责终端/数据库/插件。

## 5. Ubiquitous Language
- `shortcut`：主页快捷方式（dir/web/favorite）。
- `recents`：最近访问文件列表。
- `STORAGE_KEY` / `RECENTS_KEY`：localStorage 键（`browser-os-home-shortcuts` / `browser-os-home-recents-v1`）。
- `isStorageSafe` / `HOME_NO_SECRET_PERSIST`：收藏持久化安全门禁（禁落浏览器存储中的 app 命令体）。

## 6. Domain Model
- 聚合根：快捷方式 + 最近文件（`useHomeStore` 管理）。
- 关键 state：`loading` / `error` / `shortcuts` / `recents`（`src/capabilities/home/state/useHomeStore.ts`）。

## 7. Invariants
- `isStorageSafe` 必须过滤 app 命令体，禁止把敏感内容落浏览器存储（`HOME_NO_SECRET_PERSIST`）。
- `shortcuts`/`recents` 落 localStorage 直写，但不得含 secret。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/home/state/useHomeStore.ts`（`defineStore("home")`，STORAGE_KEY/RECENTS_KEY 见该文件）。
- **semanticOwner**：`useHomeStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：物理迁移已完成（store→state/、UI→ui/，原 `src/components/home/` 已删除）。RV1 弱：`useHomeStore` 未入语义 registry `02-STATE-SOURCES.md`（仅 `module-identity.yaml` 登记了 UI 模块）。

## 9. Commands / Intents
- 业务 action：`seedDirShortcuts` / `load` / `loadRecents` / `save` / `saveRecents` / `addShortcut` / `favoriteDirectory` / `favoriteCurrentPage` / `favoriteCurrentDir` / `open` / `remove` / `resetDefault`。
- 无原生命令（纯 JS + localStorage）。

## 10. Queries
- 前端内存：`shortcuts` / `recents` 派生查询。
- 无原生查询命令。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/home/public.ts`（**窄契约**，禁止 re-export 整个 store）。
- 暴露：`favoriteDirectory(dir)` / `favoriteCurrentPage()` / `favoriteCurrentDir()`。
- 不 re-export `HomePanel.vue`（经 index.ts 懒加载，避免 .vue 注入 runtime 打包图）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useHomeStore.ts` / `ui/`（HomePanel/HomeLaunchers/HomeRecents/HomeShortcuts/HomeShortcutEditor）。

## 14. Dependencies
- `dependsOn: ["browser","workspace","apps","bridge"]`。⚠️ `bridge` **不是 capability**（全仓无 `capabilities/bridge/manifest.ts`），此处指 `src/bridge.ts` 适配层 —— 命名语义债。
- `optionalDependencies: []`。

## 15. Dependents
- 唯一跨域消费者：`composables/homeNav.ts`（经 public 窄契约 `favoriteDirectory`）。
- `MainArea.vue` 经 WORKBENCH_MAIN 贡献渲染 home view（不直接 import store）。

## 16. Frontend Boundary
- 贡献组件：`HomePanel.vue`（WORKBENCH_MAIN，view=`home`）+ 包内 4 个子组件。
- 注册：`registerHomeContributions()`，懒加载（带 loading/error 占位）。
- **CURRENT PHYSICAL LOCATION**：全部 UI 在 `src/capabilities/home/ui/`（已隔离，原 `src/components/home/` 已删除）。

## 17. Native Boundary
- **无原生 tauri command**。Home 为纯 JS + localStorage 持久化（`manifest.resources.class=["LIGHT"]`，`permissions=["fs.read"]` 仅声明，无对应 command 实现）。
- `src-tauri/src/bridge.rs` / `main.rs` `generate_handler!` 中**无 `home_*`** 命令（已确认）。
- **诚实声明**：Native 集中化与本模块无关；本模块不拥有任何原生命令。

## 18. Resources
- `resources.class: ["LIGHT"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: [{kind:"CACHE", owned, evidence:"src/capabilities/home/state/useHomeStore.ts"}]`。
- `persistence.scope: "disk"`；`sensitive: false`。

## 19. Side Effects
- 写 localStorage（`shortcuts`/`recents`），受 `isStorageSafe` 约束。

## 20. Security
- `HOME_NO_SECRET_PERSIST`：禁止把 app 命令体等敏感内容落浏览器存储。

## 21. Persistence
- 声明 `disk`；实际落 `localStorage`（STORAGE_KEY/RECENTS_KEY），非磁盘文件；受安全门禁约束。

## 22. Failure Model
- 加载失败：`error` 态 + UI 提示；使用默认快捷方式种子（`seedDirShortcuts`）。

## 23. Capability Absence
- Absent 时：`registerHomeContributions` 未执行 → WORKBENCH_MAIN 无 view=`home` → MainArea 不渲染。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: manual`。

## 25. UI Contributions
- `home.main`（WORKBENCH_MAIN / surface / view=`home` / HomePanel）。

## 26. Testing
- `src/capabilities/home/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 原生侧：无。

## 27. Gates
- `scripts/check-home-ui-logic.mjs`（**定向覆盖 home**，✅）
- `scripts/check-home-store-logic.mjs`（**定向覆盖 useHomeStore**，✅）
- `scripts/check-home-client-policy.py`（`HOME_STORE`/`HOME_DIR` 指向 `capabilities/home`，✅ 强 RV2）
- `scripts/check-capability-platform.mjs`（组合）

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts`（窄契约）→ `state/useHomeStore.ts` → `ui/HomePanel.vue`。
- 关注点：窄契约是否被扩大、`HOME_NO_SECRET_PERSIST` 是否覆盖新持久化、依赖命名债。

## 29. AI Modification Guide
- 新增收藏类型：必须过 `isStorageSafe`，并补 `check-home-client-policy.py` 断言。
- 禁止：把整个 store re-export 进 public、把 `bridge` 当 capability 处理、用裸 `invoke`。
- 新增 UI 必须注册 Contribution Registry。

## 30. Known Debt
- `dependsOn` 含非 capability 的 `bridge`（命名语义债，同 apps）。
- RV1 弱：`useHomeStore` 未入语义 registry `02-STATE-SOURCES.md`（仅 module-identity.yaml 登记 UI 模块）。
- 物理迁移已完成，无 state/UI 物理债务。

## 31. C / HP / M / RV / D
- **C = C2**：有 3 个定向 checker + 窄契约；`manifest.v1.maturity="C2"`，自述非 C3（无 absence 门禁 + `mainView='home'` 导航硬编码）。
- **HP = HP2**：`hotPlug.level="HP2"`（enable/disable/register/unregister=true；install/uninstall=false），由 `check-capability-platform.mjs` 验证。
- **M = M1（完全）**：`src/capabilities/home/` 目录隔离，原 `src/components/home/` 已删除，state/ui 均在包内。无独立 npm 包（非 M2）。
- **RV = RV1（弱） + RV2（强） + RV3（否）**：owner 已登记但语义 registry 路径未更新（RV1 弱）；3 个定向 checker（RV2 强）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、语义 registry 路径更新、依赖命名债清理。
- 物理隔离完备，可作 Package Extraction 范本候选。

## 33. Source of Truth
- manifest：`src/capabilities/home/manifest.ts`
- public：`src/capabilities/home/public.ts`
- state：`src/capabilities/home/state/useHomeStore.ts`
- UI：`src/capabilities/home/ui/`
- native：无（无 `home_*` 命令）
- semantic owner：manifest `semanticOwner: "useHomeStore"`
- gates：`scripts/check-home-ui-logic.mjs`、`scripts/check-home-store-logic.mjs`、`scripts/check-home-client-policy.py`（见 §27）
