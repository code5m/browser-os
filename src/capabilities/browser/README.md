# browser 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/browser/manifest.ts`。
> 重要：本模块存在**真实物理债务**（见 §30），须诚实区分 CURRENT PHYSICAL LOCATION 与 TARGET / KNOWN DEBT。

---

## 1. Purpose
浏览器领域。负责 WebView 宿主、标签页管理、宫格（grid）多视图、会话/资源瀑布、凭据填充、AI 导航面板。是 Shell 的核心宿主能力。

## 2. Domain Classification
- 领域：`browser`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- WebView 宿主（`BrowserHost`）与标签页生命周期（new/close/switch/reload/navigate/back/forward）。
- 宫格（`gridOpen`/`gridSession`/`gridCount`/`gridLayout`）多 WebView 编排。
- 会话面板、资源瀑布（ResourceWaterfall/SessionPanel）。
- 凭据导入/列表/填充、AI 导航面板。

## 4. Non-Responsibilities
- 不负责数据库/终端/插件等其它域。
- 不创建非 WebView 的子进程（工具子 webview 归 `tools`）。
- 不负责书签持久化（归 `bookmark`，browser 仅可选依赖）。

## 5. Ubiquitous Language
- `tab`：一个浏览器标签页（id/url/title）。
- `grid` / `gridSession`：宫格多视图会话。
- `gridArchive`：宫格归档回复（AI 对话存档）。
- `recentlyClosed`：最近关闭标签（可恢复）。
- `aiNavOpen`：AI 导航面板开合。

## 6. Domain Model
- 聚合根：标签页集合 + 宫格会话（`useBrowserStore` 管理）。
- 关键 state：`url` / `tabs` / `activeTabId` / `gridOpen` / `gridSession` / `gridCount` / `gridUrl` / `gridUrls` / `gridLayout` / `gridMode` / `gridRects` / `resources` / `aiNavOpen` / `recentlyClosed`（`src/capabilities/browser/state/useBrowserStore.ts`）。

## 7. Invariants
- `gridSession`/`gridOpen`/`gridCount` 的真源应为 `useBrowserStore`，但当前存在**第二写入者** `useGridArchiveStore`（见 §8 物理债务，MULTIPLE_WRITERS 风险）。
- WebView 生命周期在 suspend 时未验证 GRACEFUL 回收（HP0 限制理由）。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION（canonical）**：`src/capabilities/browser/state/useBrowserStore.ts`（`defineStore("browser")`）。
- **CURRENT PHYSICAL LOCATION（并行/第二真源·已内迁）**：`src/capabilities/browser/state/useGridArchiveStore.ts` —— 属 browser 域的 grid 归档状态，物理位置已迁入 capability 内；仍直接读写 `useBrowserStore` 的 `gridSession`/`gridOpen`/`gridCount`（MULTIPLE_WRITERS / S2 风险，待收敛）。语义 registry `02-STATE-SOURCES.md` 已将其列为 writer。
- **semanticOwner**：`useBrowserStore`（manifest 登记）。
- **TARGET / KNOWN DEBT**：位置已内迁至 `src/capabilities/browser/state/`；剩余债务为收敛 grid 状态写入者、消除 MULTIPLE_WRITERS（writer 收敛与 owner 待裁决）。

## 9. Commands / Intents
- 业务 action：`tabNew` / `tabSwitch` / `tabClose` / `restoreRecent` / `tabReload` / `tabNavigate` / `goBack` / `goForward` / `buildGrid` / `aiAdapterFor`。
- 原生命令（见 §17，browser 域大量命令）：grid 系列、tab 系列、resource 系列、credential 系列、AI 归档系列。

## 10. Queries
- 前端内存：`tabs` / `activeTabId` / `grid*` 派生。
- 原生：`tab_list` / `resource_stats` / `list_tab_resources` / `list_browser_credentials` 等。

## 11. Events
- NOT_APPLICABLE（无独立领域事件总线；经 Pinia 响应式 + Contribution Registry）。

## 12. Public Contract
- 入口：`src/capabilities/browser/public.ts`。
- 暴露：`useBrowserStore`（再导出，无镜像状态）。
- 注意：仅再导出，**未封装新 API**（SECOND_TRUTHS=0 仅再导出）。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useBrowserStore.ts` / `ui/BrowserHost.vue` / `ui/ResourceWaterfall.vue` / `ui/SessionPanel.vue`。
- 注册：`registerBrowserContributions()`（WORKBENCH_MAIN 的 `browser.host` + BROWSER_DOCK 的 `browser.dock.net`/`browser.dock.session`）。

## 14. Dependencies
- `dependsOn: []`（无声明硬依赖）。
- 被大量 Shell 组件经 public 消费（App.vue、layout 系列、composables）。

## 15. Dependents
- 消费 `capabilities/browser/public` 的：`App.vue`、`components/browser/*`（直连 store）、`components/layout/*`、`composables/useBrowserHost.ts`/`browserNav.ts`、`src/capabilities/browser/state/useGridArchiveStore.ts`/`useSessionStore.ts`/`useWorkbenchStore.ts`/`useLayoutStore.ts`（动态 import；gridArchive/session 已内迁 browser/state，workbench/layout 仍在 `src/stores` 待裁决）。

## 16. Frontend Boundary
- 贡献组件：`BrowserHost.vue`（BROWSER_HOST）、`ResourceWaterfall.vue`（BROWSER_DOCK view=net）、`SessionPanel.vue`（BROWSER_DOCK view=session）。
- **CURRENT PHYSICAL LOCATION（已隔离）**：`src/capabilities/browser/ui/` 下 3 个组件。
- **CURRENT PHYSICAL LOCATION（物理债务）**：以下 4 个 .vue 仍在 `src/components/browser/`，未迁入 `ui/`：`AINavPanel.vue`、`CredentialList.vue`、`GridArchiveBar.vue`、`ResourcePanel.vue`。其中 `GridArchiveBar.vue` 直连 `src/capabilities/browser/state/useGridArchiveStore`（该 store 已内迁 browser/state，债务链收窄为"state 在 capability 内、UI 仍在 components/browser"两处，非跨非 capability 目录）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（grid/tab/resource/credential/AI 归档命令）+ `src-tauri/src/main.rs` `generate_handler!` 注册。
- 代表性命令：`create_grid`/`close_grid`/`grid_open`/`grid_position`/`grid_set_zoom`/`grid_close_one`/`hide_all_webviews`；`tab_new`/`tab_close`/`tab_open`/`tab_list`/`tab_set_title`/`tab_activate`/`eval_in_tab`/`set_tab_hibernation`；`resource_stats`/`list_tab_resources`/`clear_tab_resources`；`import_browser_credentials`/`list_browser_credentials`/`fill_browser_credential`；`grid_read_replies`/`archive_replies`；`get_default_browser`/`set_default_browser`。
- 前端封装：`src/bridge.ts`（`createGrid`/`gridOpen`/`gridPosition`/`gridSetZoom`/`gridCloseOne`/`listApps`/`importBrowserCredentials`/`listBrowserCredentials`/`fillBrowserCredential`/`gridReadReplies`/`archiveReplies`/`tabNew` 等）。
- **诚实声明**：Native 仍高度集中于 `bridge.rs` 单一文件，本能力未单独抽出原生模块；所有 browser 域命令与其它域命令混在 `bridge.rs`，不得在文档中假装 Native 已物理模块化。Native ownership source = `src-tauri/src/main.rs` + `src-tauri/src/bridge.rs` + `src/bridge.ts`。

## 18. Resources
- `resources.class: ["HEAVY","WEBVIEW","NATIVE"]`；`suspendable: false`；`destroyable: false`（`manifest.ts`）。
- `v1.resources: [{kind:"WEBVIEW",owned},{kind:"CHILD_PROCESS",owned}]`（evidence=measure-resources.mjs）。
- `persistence.scope: "session"`；`sensitive: false`。

## 19. Side Effects
- 创建/销毁 WebView（重资源）。
- 凭据导入/填充（写入凭据存储）。
- AI 归档回复读写（`grid_read_replies`/`archive_replies`）。

## 20. Security
- 凭据填充经 `fill_browser_credential` 受控。
- WebView 权限受 `webview.create` 权限约束。

## 21. Persistence
- 声明 `session`（WebViews/宫格会话不落盘）；凭据由原生侧凭据存储管理，非前端落盘。

## 22. Failure Model
- WebView 创建失败：`error` / 资源瀑布显示失败。
- 宫格命令失败：`gridOpen` 回退单视图。

## 23. Capability Absence
- Absent 时：`registerBrowserContributions` 未执行 → BROWSER_HOST / BROWSER_DOCK 无贡献 → Shell 不渲染宿主。
- 但 `useBrowserStore` / `useGridArchiveStore` 被多处 import（物理债务），absent 不保证完全不实例化。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: graceful`（必须先处理 WebView/会话）。

## 25. UI Contributions
- `browser.host`（BROWSER_HOST / surface / BrowserHost）
- `browser.dock.net`（BROWSER_DOCK / surface / view=net / ResourceWaterfall）
- `browser.dock.session`（BROWSER_DOCK / surface / view=session / SessionPanel）

## 26. Testing
- `src/capabilities/browser/` 下 `*.spec.ts`：**0**（RV3 不满足，全仓前端无 vitest）。
- 原生侧：依赖脚本门禁。

## 27. Gates
- `scripts/check-browser-runtime.mjs`（边界检查，实际只查 `BrowserRuntime/BrowserScene/syncScene` 符号，不直接引用 `capabilities/browser` → RV2 弱）。
- `scripts/check-composition-profiles.mjs`、`scripts/measure-resources.mjs`（存在）。
- `scripts/check-canonical-module-location.mjs`（R10 单实现检查）**未登记** `useGridArchiveStore`/`useBrowserStore` → browser 物理债务无 checker 兜底。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useBrowserStore.ts` → `ui/*` + （债务）`src/components/browser/*` + `src/capabilities/browser/state/useGridArchiveStore.ts`。
- 关注点：MULTIPLE_WRITERS 风险、WebView 资源回收、grid 状态真源收敛。

## 29. AI Modification Guide
- 改 grid 状态：必须收敛到 `useBrowserStore`，避免 `useGridArchiveStore` 双写；迁移后同步语义 registry `02-STATE-SOURCES.md`。
- 禁止：把 WebView 资源释放逻辑漏写、用裸 `invoke`、新增 UI 不进 Contribution Registry。
- 迁移 `src/components/browser/*` 到 `ui/` 时同步所有 import。

## 30. Known Debt
- **物理债务①（位置已解除，writer 未收敛）**：`useGridArchiveStore` 已迁入 `src/capabilities/browser/state/`（物理债务解除），但仍列为 `gridOpen`/`gridSession` 的 writer（MULTIPLE_WRITERS / S2 风险），writer 收敛与 owner 待裁决。
- **物理债务②**：4 个 .vue 在 `src/components/browser/`（非 `ui/`）：`AINavPanel`/`CredentialList`/`GridArchiveBar`/`ResourcePanel`。
- **物理债务③**：`GridArchiveBar.vue` ↔ `useGridArchiveStore` 跨两处非 capability 目录。
- **语义漂移**：`02-STATE-SOURCES.md` 仍引用旧路径 `src/stores/useBrowserStore.ts`（实已迁至 `capabilities/browser/state/`）。
- HP0 限制理由：WebView 生命周期未验证真正 GRACEFUL 停用。

## 31. C / HP / M / RV / D
- **C = C3**：目录隔离 + 贡献驱动 + 资源测量（`manifest.v1.maturity="C3"`）；但物理债务使 C3 的实现完整度打折（state 主体已内迁，grid 并行 store 未归一）。
- **HP = HP0**：WebView 生命周期未验证 GRACEFUL（`manifest.v1.hotPlug.level="HP0"`，全 false）。
- **M = M1（带物理债务）**：`src/capabilities/browser/` 目录隔离达成；`useGridArchiveStore` 与 4 个 .vue 在 capability 外（M1 不完整，待迁入）。无独立 npm 包（非 M2）。
- **RV = RV1（路径漂移） + RV2（弱）**：`semanticOwner` 已登记但 `02-STATE-SOURCES.md` 路径旧；`check-browser-runtime.mjs` 为通用边界检查非定向（RV2 弱）；RV3=不成立（无 vitest）。
- **D = D3**：本 README 满足 D3（含物理债务诚实记录）。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：grid 状态 MULTIPLE_WRITERS、UI 未迁入 `ui/`、语义 registry 路径漂移、无 vitest。
- 完成物理债务收敛后可达 M2 候选。

## 33. Source of Truth
- manifest：`src/capabilities/browser/manifest.ts`
- public：`src/capabilities/browser/public.ts`
- state（canonical）：`src/capabilities/browser/state/useBrowserStore.ts`
- state（**并行真源·位置已内迁**）：`src/capabilities/browser/state/useGridArchiveStore.ts`
- UI（已隔离）：`src/capabilities/browser/ui/`
- UI（**物理债务**）：`src/components/browser/`（AINavPanel/CredentialList/GridArchiveBar/ResourcePanel）
- native：`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useBrowserStore"`
- gates：`scripts/check-browser-runtime.mjs`、`scripts/check-canonical-module-location.mjs`（见 §27）
