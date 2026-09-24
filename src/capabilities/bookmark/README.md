# bookmark 模块 README（Phase D3 · 高质量文档化）

> 文档性质：本 README 是 **machine truth 的引用者**，不是第二真源。所有数量/命令以 `manifest` / `semantic registry` / `capability registry` / `native registry` / `resource registry` / `tests` 为权威。禁止复制会漂移的数量。
> 评级真源：`src/capabilities/bookmark/manifest.ts`（C/HP 机器字段）；M/RV/D 按官方定义结合代码证据推导。
> 本模块为 **Phase D3 参考范本**：物理迁移已完成（state 在 `state/`、UI 在 `ui/`），几乎无已知物理债务。

---

## 1. Purpose
收藏夹领域。负责书签的持久化存储、侧边栏面板、地址栏星标、活动栏入口，是 Pilot 试点能力（首个按 Capability Contract 收口的模块）。

## 2. Domain Classification
- 领域：`bookmark`
- `manifest.category` = `UI_COMPONENT`（提供 storage/panel/navigation 三类能力点）
- 角色：Shell 的导航/书签域，不持有后台任务、不创建重资源。

## 3. Responsibilities
- 书签条目的增删查改与导入（`add`/`remove`/`toggle`/`importFile`/`findByUrl`）。
- 书签持久化（后端 `data_dir/bookmarks.json`，前端 store 仅缓存）。
- 三个 UI 贡献的渲染与挂载：侧边栏面板、地址栏星标、活动栏入口。
- 面板开合态 `panelOpen` 的本地 UI 态管理。

## 4. Non-Responsibilities
- 不负责导航执行（由 `browser` 消费 `useBrowserStore` 承担）。
- 不负责凭据/数据库/终端等任何其他域。
- 不创建 WebView/PTY/进程等重资源。
- 不读写除 `bookmarks.json` 之外的磁盘文件。

## 5. Ubiquitous Language
- `BookmarkItem`：一条书签（url/category/title）。
- `category`：书签分类（默认 `DEFAULT_CATEGORY`）。
- `panelOpen`：侧边栏面板开合态。
- `canBookmark`：判定某 url 是否可收藏的纯函数。
- `normalizeUrl`：url 归一化纯函数。

## 6. Domain Model
- 聚合根：`BookmarkItem`（由 `useBookmarkStore` 管理 `items: BookmarkItem[]`）。
- 值对象：`category`（字符串枚举）。
- 纯函数：`normalizeUrl` / `canBookmark`（位于 `state/useBookmarkStore.ts`）。
- 关键 state：`items` / `loaded` / `busy` / `error` / `panelOpen` / computed `sorted`（`src/capabilities/bookmark/state/useBookmarkStore.ts`）。

## 7. Invariants
- `items` 是唯一前端真源；持久化由后端落盘，前端不直写文件。
- `canBookmark` 必须为纯函数（无副作用），供地址栏星标判定。
- `panelOpen` 仅本地 UI 态，不持久化、不影响数据。
- 排序 `sorted` 由 computed 派生，不得被 action 直接改写。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/capabilities/bookmark/state/useBookmarkStore.ts`（`defineStore("bookmark")`）。
- **semanticOwner**：`useBookmarkStore`（manifest + public 双重登记）。
- **TARGET / KNOWN DEBT**：无。已完全迁入 capability 内，无 `src/stores` 残留。
- 依赖：`useLayoutStore`（toast）、`bridge.bookmarkList/Add/Remove`。

## 9. Commands / Intents
- 业务命令（action）：`add` / `remove` / `toggle` / `importFile` / `togglePanel` / `load`。
- 原生命令（经 bridge，见 §17）：`add_bookmark` / `list_bookmarks` / `remove_bookmark`。

## 10. Queries
- `findByUrl(url)` / `isBookmarked(url)`（同步内存查询）。
- `list_bookmarks`（原生，后端读 `bookmarks.json`）。

## 11. Events
- NOT_APPLICABLE（无独立领域事件总线；面板开合经 Pinia 响应式传播）。

## 12. Public Contract
- 入口：`src/capabilities/bookmark/public.ts`。
- 暴露：`useBookmarkStore` / `canBookmark`（纯再导出，无镜像状态，SECOND_TRUTHS=0）。
- manifest 中 `v1.publicContract`：`publicApi → src/capabilities/bookmark/public.ts`。

## 13. Internal Boundary
- `state/`（owner）、`ui/`（3 个 .vue）、`lifecycle/bookmark.ts`（onActivate 注册、onSuspend 不释放数据）、`manifest.ts`、`public.ts`、`index.ts`。
- 内部 UI 经 `../state/useBookmarkStore` 直接引用（位于包内，不违反 CB-02）。

## 14. Dependencies
- `dependsOn: []`（硬依赖无）。
- `optionalDependencies: ["browser"]`：Browser 缺失时降级而非拒绝（`BookmarkStar` 经 `../../browser/public` 取 `useBrowserStore`）。

## 15. Dependents
- `src/capability/index.ts`（装配 `bookmarkCapability`）。
- `src/capability/profiles.ts`（minimal/developer/full 均含）。
- `src/capabilities/home/public.ts`（引用 bookmark）。
- 本包内 UI 组件（均 import 包内 `state/useBookmarkStore`）。

## 16. Frontend Boundary
- 三个贡献组件：`BookmarkPanel.vue`（BROWSER_SIDEBAR）、`BookmarkStar.vue`（ADDRESS_BAR_ACTIONS）、`BookmarkEntryButton.vue`（ACTIVITY_BAR_TRAILING）。
- 注册：`src/capabilities/bookmark/index.ts` `registerBookmarkContributions()`，经通用 Contribution Registry。
- **CURRENT PHYSICAL LOCATION**：全部 UI 在 `src/capabilities/bookmark/ui/`。
- 渲染隔离：MainArea 只按 slot 渲染，不 import 本能力内部。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`add_bookmark` / `list_bookmarks` / `remove_bookmark`，均 `#[tauri::command]` 且写审计日志）。
- 注册：`src-tauri/src/main.rs` `generate_handler!`（`bridge::add_bookmark / list_bookmarks / remove_bookmark`）。
- 前端封装：`src/bridge.ts`（`bookmarkAdd→add_bookmark` / `bookmarkList→list_bookmarks` / `bookmarkRemove→remove_bookmark`）。
- **诚实声明**：原生实现仍集中于 `bridge.rs` 单一文件，本能力未单独抽出原生模块；不得在文档中假装 Native 已物理模块化。Native ownership source = `src-tauri/src/main.rs` + `src-tauri/src/bridge.rs` + `src/bridge.ts`。

## 18. Resources
- `resources.class: ["LIGHT"]`；`permissions: []`。
- `v1.resources: [{kind:"CACHE", ownership:"owned", evidence:"src/capabilities/bookmark/state/useBookmarkStore.ts"}]`。
- 无 network / secret / keyring / db 连接 / PTY / 进程。

## 19. Side Effects
- 写 `bookmarks.json`（经原生 `add_bookmark`/`remove_bookmark`，非前端直写）。
- 触发 toast（经 `useLayoutStore`）。

## 20. Security
- 无敏感数据（书签 url 非机密）。
- 原生命令写审计日志（`log_audit`）。

## 21. Persistence
- `persistence.scope: "disk"`；`sensitive: false`。
- 后端落 `data_dir/bookmarks.json`；前端 store 仅内存缓存（`loaded` 后不重复拉取）。

## 22. Failure Model
- 加载失败：`error` 态 + toast，面板仍可打开（空列表）。
- 增删失败：原生命令返回错误 → store 设 `error`，不破坏既有 `items`。

## 23. Capability Absence
- Absent 时：`registerBookmarkContributions` 未执行 → 三个 slot 无对应贡献 → Shell 渲染空集。
- `check-capability-pilot.mjs` 断言 absent 无残留（含禁止 `useBookmarkStore` 反向依赖）。
- 无死页签、无悬挂引用。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["REGISTERED","ACTIVE","SUSPENDED"]`；`default: "REGISTERED"`。
- `activatable: true`；`resident: false`。
- `onActivate`：调用 `registerBookmarkContributions`；`onSuspend`：不释放数据（destroyable=false 一致）。

## 25. UI Contributions
- `bookmark.sidebar`（BROWSER_SIDEBAR / surface / BookmarkPanel）
- `bookmark.address-star`（ADDRESS_BAR_ACTIONS / navigation / BookmarkStar）
- `bookmark.entry-button`（ACTIVITY_BAR_TRAILING / navigation / BookmarkEntryButton）
- 全部 `defineAsyncComponent` 懒加载。

## 26. Testing
- `src/capabilities/bookmark/` 下 `*.spec.ts`：**0**（全仓前端无 vitest，RV3 不满足）。
- 原生侧：无专属 Rust 单测（依赖脚本门禁）。

## 27. Gates
- `scripts/check-capability-pilot.mjs`（**bookmark 专属 bounded 集成门禁**：注册/贡献/suspend/owner 解析/反向依赖断言）。
- `scripts/check-composition-profiles.mjs`、`scripts/check-capability-platform.mjs`（组合/平台门禁，命中 bookmark）。
- `scripts/check-semantic-registry.mjs`、`scripts/check-capability-runtime.mjs`、`scripts/check-capability-boundaries.mjs`、`scripts/check-capability-registry.mjs`、`scripts/check-capability-composition.mjs`、`scripts/check-capability-resource-boundary.mjs`（registry 一致性门禁）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useBookmarkStore.ts` → `ui/*.vue`。
- 关注点：state 是否在包内、是否经 Contribution Registry 解耦、是否有第二真源。
- 修改 owner 必须先改 manifest `semanticOwner` + `public.ts` 再导出。

## 29. AI Modification Guide
- 新增书签字段：改 `BookmarkItem` 类型 + `state` + 原生 `bookmarks.json` schema + `check-capability-pilot.mjs` 如有断言。
- 禁止：把 store 移回 `src/stores/`、把 UI 移回 `src/components/`、新增裸 `invoke`（必须走 `bridge`）。
- 新增 UI 必须注册到 Contribution Registry，不得让 MainArea 直 import。

## 30. Known Debt
- 轻微：`default:"REGISTERED"` 与 `deactivationPolicy:"manual"` 语义一致（disable 不销毁用户数据）。
- HP2 限制理由：HP3 需外部能力包动态加载/沙箱，本夜不做（`manifest.ts` 注释）。

## 31. C / HP / M / RV / D
- **C = C3**：目录隔离 + state 已内迁 + 三贡献经 Contribution/Slot 解耦，声明与实现一致（`manifest.v1.maturity="C3"`）。
- **HP = HP2**：`hotPlug.level="HP2"`（enable/disable/register/unregister=true；install/uninstall=false），由 `check-capability-platform.mjs` 验证。
- **M = M1**：`src/capabilities/bookmark/` 目录隔离达成；无独立 npm 包（非 M2）。
- **RV = RV1 + RV2**：`semanticOwner` 已登记（RV1）；`check-capability-pilot.mjs` 为 bookmark 专属 bounded 门禁（RV2）；RV3=不成立（无 vitest）。
- **D = D3**：本 README 满足 D3（README + ResponsibilityBoundary + DomainContractResourceTest）。文档化前为 D0。

## 32. Extraction Readiness
- 物理隔离完备（state/ui/lifecycle/manifest/public/index 齐全），可作为未来 Package Extraction（M2+）的范本。
- 阻塞项：无独立 vitest（RV3）；无独立 npm 包。

## 33. Source of Truth
- manifest：`src/capabilities/bookmark/manifest.ts`
- public：`src/capabilities/bookmark/public.ts`
- state：`src/capabilities/bookmark/state/useBookmarkStore.ts`
- UI：`src/capabilities/bookmark/ui/`
- native：`src-tauri/src/bridge.rs`（命令）、`src-tauri/src/main.rs`（注册）、`src/bridge.ts`（封装）
- semantic owner：manifest `semanticOwner: "useBookmarkStore"` + public 再导出
- gates：`scripts/check-capability-pilot.mjs` 等（见 §27）
