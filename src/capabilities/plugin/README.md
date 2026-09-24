# plugin 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/plugin/manifest.ts`。
> 诚实边界：后端 `plugin_*` 命令已实现（登记簿 + 生命周期状态机 + 受信任公钥指纹，原子写盘），但**无运行时**：不解包、不真验签、不动态加载、不执行、不下载、不联网。`AVAILABLE/ACTIVE/RESOURCE_EXISTS` 恒 false，`ENABLED≠ACTIVE`。本能力当前**运行时 LOCKED**。

---

## 1. Purpose
插件域。负责插件登记簿的查看、安装/启用/禁用意图的 UI 壳，以及受信任公钥管理。当前仅为**只读壳 + 生命周期状态机展示**，无执行面。

## 2. Domain Classification
- 领域：`plugin`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 插件列表查看（`refreshList`）、详情查看（`getDetail`）。
- 安装/启用/禁用/公钥管理意图（UI 壳 + bridge 调用），但后端**不执行**插件。
- 五态投影（`pluginFacets()` 派生，来自 `utils/pluginUi.ts`）：AVAILABLE/ENABLED/ACTIVE/DISABLED/RESOURCE_EXISTS（后三者/前两者有恒 false 约束）。

## 4. Non-Responsibilities
- **不执行插件**（无 loader/执行面，`plugin_invoke` 全 src-tauri 0 命中）。
- 不动态加载/验签/解包/下载/联网。
- 不创建 WebView/PTY/子进程（尽管 manifest 声明 `HEAVY,NATIVE`，仅为潜在声明）。
- 不负责其它域。

## 5. Ubiquitous Language
- `PluginState` / `PluginSummary` / `PluginDetail`：插件状态/摘要/详情（来自 `src/types.ts`）。
- `TrustedKeyRecord`：受信任公钥指纹记录。
- `backendReady`：8 个 bridge 方法均为 function 的 computed。
- `actionsFor`：按 `detail.state` 门控 enable/disable/uninstall 的投影。

## 6. Domain Model
- 聚合根：插件登记簿（`usePluginStore` 管理）。
- 关键 state：`list` / `detail` / `keys` / `filterState` / `busy` / `error` / `manifestText` / `resourcePath`（`src/capabilities/plugin/state/usePluginStore.ts`）。
- 后端真源：`src-tauri/src/plugin.rs`（仅只读加载器 + 状态机 + 指纹；无执行面）。

## 7. Invariants
- 五态由 `pluginFacets()` 派生投影，**禁止合并为单一 boolean**（§18）。
- 只调 bridge，**绝不用裸 invoke**（红线）。
- `describeError` 稳定文案，零 secret echo。
- `ENABLED≠ACTIVE`：启用不等于已加载执行。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION（前端）**：`src/capabilities/plugin/state/usePluginStore.ts`（`defineStore("plugin")`）。
- **CURRENT PHYSICAL LOCATION（后端）**：`src-tauri/src/plugin.rs`（只读加载器 + 状态机）。
- **semanticOwner**：`usePluginStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：无物理债务（state/ui 均在包内）；LOCK 事实由 C2+HP0+`check-plugin-policy.py` 的 `PLUGIN_NO_EXEC_SURFACE` 表达，不由 `governanceStatus` 表达。

## 9. Commands / Intents
- 业务 action：`refreshList` / `getDetail` / `install` / `enable` / `disable` / `refreshKeys` / `addKey` / `removeKey` / `selectFromList` / `clearError`。
- 原生命令（见 §17，后端已实现但无执行）：`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_keys_add` / `plugin_keys_list` / `plugin_keys_remove`。

## 10. Queries
- 前端内存：`list`/`detail`/`keys`/`actionsFor` 派生。
- 原生：`plugin_list` / `plugin_get` / `plugin_keys_list`。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/plugin/public.ts`。
- 暴露：`usePluginStore`（再导出）、`pluginManifest`、`type PluginState/PluginSummary/PluginDetail/TrustedKeyRecord/PluginManifest`。
- 五态由 `utils/pluginUi.ts` 的 `pluginFacets()` 派生投影，非镜像状态。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/usePluginStore.ts` / `ui/PluginManager.vue`。

## 14. Dependencies
- `dependsOn: ["bridge"]`（硬）。
- `optionalDependencies: []`。

## 15. Dependents
- `src/capability/index.ts`、`profiles.ts`、`catalog.ts`。
- `MainArea.vue` 仅注释引用。

## 16. Frontend Boundary
- 贡献组件：`PluginManager.vue`（WORKBENCH_MAIN，view=`plugin`）。
- 注册：`registerPluginContributions()`，懒加载。
- **CURRENT PHYSICAL LOCATION**：UI 已在 `src/capabilities/plugin/ui/`（已确认 `src/components` 下 0 残留 PluginManager）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`plugin_install`/`plugin_enable`/`plugin_disable`/`plugin_list`/`plugin_get`/`plugin_keys_add`/`plugin_keys_list`/`plugin_keys_remove`，均经 `check_invocation_source`）+ 后端 `src-tauri/src/plugin.rs`。
- 已注册（main.rs）：上述 8 个命令。
- **`plugin_invoke` 全 src-tauri 0 命中**（运行时 LOCKED 实证）。
- 前端封装：`src/bridge.ts`（`pluginInstall`/`pluginEnable`/`pluginDisable`/`pluginList`/`pluginGet`/`pluginKeysAdd`/`pluginKeysList`/`pluginKeysRemove`）；**无 `plugin_invoke` 封装**。
- **诚实声明**：Native 仍集中于 `bridge.rs`/`plugin.rs`，未物理模块化；本能力无独立原生模块；后端仅有登记簿/状态机/指纹，**无执行面**。

## 18. Resources
- `resources.class: ["HEAVY","NATIVE"]`（**声明潜在**，非实际 —— 当前 Stage-I 从不创建 load 实例）。
- `v1.resources: []`（诚实区分：manifest 声明潜在资源，v1 声明无 owned 资源）。
- `persistence.scope: "disk"`（plugins.json/trusted-pubkeys.json 元数据，原子写）；`sensitive: false`。

## 19. Side Effects
- 写 plugins.json / trusted-pubkeys.json（元数据，原子写盘）。
- **无**插件执行副作用（LOCKED）。

## 20. Security
- `PLUGIN_NO_EXEC_SURFACE`：无执行面（由 `check-plugin-policy.py` 强制）。
- 公钥指纹受信任机制；`check_invocation_source` 校验来源。
- `describeError` 零 secret echo。

## 21. Persistence
- 声明 `disk`；实际仅插件元数据（plugins.json/trusted-pubkeys.json），非插件代码/数据。

## 22. Failure Model
- 后端未执行：UI 仅展示状态机，`ENABLED≠ACTIVE` 明确区分。
- 命令失败：`error` 态 + `describeError` 稳定文案。

## 23. Capability Absence
- Absent 时：`registerPluginContributions` 未执行 → WORKBENCH_MAIN 无 view=`plugin` → MainArea 不渲染。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`installPolicy: static`。

## 25. UI Contributions
- `plugin.main.panel`（WORKBENCH_MAIN / surface / view=`plugin` / PluginManager）。

## 26. Testing
- `src/capabilities/plugin/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 门禁：4 个脚本（见 §27）。

## 27. Gates
- `scripts/check-plugin-policy.py`（`PLUGIN_NO_EXEC_SURFACE` 强制无执行面，强）。
- `scripts/check-plugin-privacy.py`。
- `scripts/check-plugin-ui-logic.mjs`。
- `scripts/check-plugin-ui-privacy.py`。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/usePluginStore.ts` → `ui/PluginManager.vue` → 后端 `plugin.rs`。
- 关注点：LOCKED 边界、五态不可合并、absence 门禁缺失。

## 29. AI Modification Guide
- 实现插件执行前：先落地 loader/执行面后端 + 安全沙箱，再放开 `plugin_invoke` + 更新 `check-plugin-policy.py` 放行；严禁只放开 UI 而不实现后端。
- 禁止：用裸 `invoke`、把五态合并为单一 boolean、移除 `PLUGIN_NO_EXEC_SURFACE` 门禁。
- 改状态机：同步 `utils/pluginUi.ts` 的 `pluginFacets()` 与门禁断言。

## 30. Known Debt
- 运行时 LOCKED：无 loader/执行面（manifest 注释 / index.ts 注释）；五态分别判定禁止合并。
- C2→C3 缺口：设计 LOCKED + 无插件专属 absence 门禁 + `mainView='plugin'` 硬编码。
- `resources.class=HEAVY/NATIVE` 为潜在声明，`v1.resources=[]` 已诚实区分（无实际资源）。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，诚实自述非 C3（设计 LOCKED + 无 absence 门禁 + nav 硬编码）；有 4 个强 checker。
- **HP = HP0**：`manifest.v1.hotPlug.level="HP0"`（全 false）；LOCKED 设计，无运行时装卸需求声明。
- **M = M1（完全）**：`src/capabilities/plugin/` 目录隔离（state/ui 均在包内，旧 `src/components` 0 残留）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（强） + RV3（否）**：owner 已登记（RV1）；4 个定向 checker（RV2 强）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、absence 门禁缺失、LOCKED 设计（解锁后方有真实执行边界）。
- 物理隔离完备，可作 Package Extraction 范本候选（解锁执行面前需重写边界）。

## 33. Source of Truth
- manifest：`src/capabilities/plugin/manifest.ts`
- public：`src/capabilities/plugin/public.ts`
- state：`src/capabilities/plugin/state/usePluginStore.ts`
- UI：`src/capabilities/plugin/ui/`
- native：`src-tauri/src/plugin.rs`、`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "usePluginStore"`
- gates：`scripts/check-plugin-policy.py`、`scripts/check-plugin-privacy.py`、`scripts/check-plugin-ui-logic.mjs`、`scripts/check-plugin-ui-privacy.py`（见 §27）
