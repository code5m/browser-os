# database 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/database/manifest.ts`。
> 诚实声明：凭据不在前端落盘/明文，密码仅作瞬时参数传入后端；连接资源由 Rust 瞬态创建即弃。

---

## 1. Purpose
数据库领域。负责数据库连接、查询执行、查询取消、schema 浏览、风险预估与凭据引用（凭据落 OS keyring，前端零明文）。

## 2. Domain Classification
- 领域：`database`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 数据库连接管理（`connect`/`disconnect`）、查询执行（`execute`/`requestRun`/`confirmRun`）、查询取消（`cancelQuery`）。
- schema 浏览（`refreshSchema`/`previewTable`）、SQL 风险预估（`risk`/`verdict`）。
- 凭据引用经 OS keyring（键 `db:<conn_id>`），前端不持有明文密码。

## 4. Non-Responsibilities
- 不持有/不缓存数据库长连接（Rust 瞬态创建即弃）。
- 不持久化密码到 localStorage / 前端存储（F2 红线）。
- 不负责 Git / 终端 / 浏览器等其它域。

## 5. Ubiquitous Language
- `DbKind`：数据库类型（来自 `src/utils/dbUi.ts`）。
- `connection`：一个已登记连接配置（凭据在 keyring）。
- `verdict` / `risk`：SQL 风险判定结果。
- `pendingSql` / `runGate` / `requiresConfirm`：查询门禁态。

## 6. Domain Model
- 聚合根：连接集合 + 活动文档（`useDatabaseStore` 管理）。
- 关键 state：`form` / `connections` / `documents` / `activeDocument` / `configs` / `activeId` / `sql` / `result` / `busy` / `error` / `schema` / `risk` / `verdict` / `pendingSql` / `backendReady` / `connected` / `runGate` / `requiresConfirm` / `confirmOpen`（`src/stores/useDatabaseStore.ts`）。

## 7. Invariants
- 密码（`connect(password)`）仅作瞬时参数传入后端，不进 `form` / `store` / `localStorage`。
- 查询执行必须经 `runGate` / `requiresConfirm` 门禁（高危 SQL 需确认）。
- `backendReady` 未就绪时禁止 invoke（`guard` 等效）。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION**：`src/stores/useDatabaseStore.ts`（`defineStore("database")`）。
- **semanticOwner**：`useDatabaseStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：store 应迁入 `src/capabilities/database/state/`（与 bookmark 对齐），当前仍在 `src/stores/`，属物理债务。UI 直连 `src/stores/useDatabaseStore`（非经 public.ts）。

## 9. Commands / Intents
- 业务 action：`resetForm` / `setKind` / `selectConnection` / `refreshConnections` / `newDocument` / `closeDocument` / `cancelQuery` / `refreshSchema` / `previewTable` / `connect` / `disconnect` / `requestRun` / `cancelConfirm` / `confirmRun` / `execute` / `clearResult`。
- 原生命令（见 §17）：`db_connect` / `db_query` / `db_disconnect` / `db_list_connections` / `db_cancel`。

## 10. Queries
- `db_list_connections` / `db_query`（经门禁）/ schema 浏览。
- 前端内存：`connections` / `documents` 派生。

## 11. Events
- NOT_APPLICABLE（无独立领域事件总线）。

## 12. Public Contract
- 入口：`src/capabilities/database/public.ts`。
- 暴露：`useDatabaseStore`（再导出）、`databaseManifest`、`type DbKind`。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `ui/DatabasePanel.vue`。
- UI 经 `../../../stores/useDatabaseStore` 直连（**绕过 public 边界**，物理债务）。

## 14. Dependencies
- `dependsOn: ["credential","bridge"]`（硬）。
- `optionalDependencies: []`。

## 15. Dependents
- `src/capability/index.ts`、`src/capability/profiles.ts`。
- `src/capabilities/database/ui/DatabasePanel.vue`（直连 store）。
- `src/utils/dbUi.ts`（DB 类型/纯函数）。

## 16. Frontend Boundary
- 贡献组件：`DatabasePanel.vue`（WORKBENCH_MAIN，view=`db`，icon 🗄️）。
- 注册：`src/capabilities/database/index.ts` `registerDatabaseContributions()`，懒加载。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/capabilities/database/ui/`（已隔离）；UI 内部直连 `src/stores/useDatabaseStore`。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`db_connect` / `db_query` / `db_disconnect` / `db_list_connections` / `db_cancel`，均 `#[tauri::command]`）。
- 已注册（main.rs）：全部 5 个命令就绪。
- 前端封装：`src/bridge.ts`（`dbConnect→db_connect` / `dbQuery→db_query` / `dbDisconnect→db_disconnect` / `dbListConnections→db_list_connections` / `dbCancel→db_cancel`）。
- 凭据：后端落 OS keyring，键 `db:<conn_id>`。
- **诚实声明**：Native 仍集中于 `bridge.rs` / `main.rs`，未物理模块化。

## 18. Resources
- `resources.class: ["NETWORK","SECRET"]`；`permissions: ["network.connect","keyring.access"]`。
- `v1.resources: [{kind:"NETWORK", ownership:"owned", evidence:"src-tauri/src/database.rs db_connect/db_query"}]`。
- `persistence.scope: "disk"`；`sensitive: true`（凭据引用经 keyring，前端零明文）。

## 19. Side Effects
- 写入数据库连接配置（凭据在 keyring，非前端文件）。
- 执行 SQL（用户发起，经门禁）。

## 20. Security
- 密码不落前端存储（F2）。
- SQL 风险预估 `verdict` 防止破坏性语句。
- 凭据经 keyring，前端 `sanitize_message` 脱敏错误回显。

## 21. Persistence
- 声明 `disk` / `sensitive=true`；凭据引用经 OS keyring（`db:<conn_id>`），前端不持久化密码。

## 22. Failure Model
- 连接失败：`error` 态 + UI 提示；连接资源瞬态回收。
- 查询失败：结果区显示脱敏错误（`sanitize_message`），不回显 secret。

## 23. Capability Absence
- Absent 时：`registerDatabaseContributions` 未执行 → WORKBENCH_MAIN 无 view=`db` → MainArea 不渲染。
- 但 `src/stores/useDatabaseStore` 仍被 import（物理债务），absent 不保证该 store 完全不实例化。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","SUSPENDED"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`deactivationPolicy: graceful`。

## 25. UI Contributions
- `database.main.panel`（WORKBENCH_MAIN / surface / view=`db` / icon 🗄️ / DatabasePanel）。

## 26. Testing
- `src/capabilities/database/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 原生侧：依赖脚本门禁。

## 27. Gates
- `scripts/check-database-ui-logic.mjs`（headless 测 `src/utils/dbUi.ts`，**存在**）。
- `scripts/check-developer-owners.mjs`（**真实覆盖 database**：含 DB-01..06 有界断言）。
- `scripts/measure-resources.mjs`（被 maturityEvidence 引用，但属通用 harness，非 database 专属 → 引用名实不符）。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `src/stores/useDatabaseStore.ts` → `ui/DatabasePanel.vue`。
- 关注点：凭据处理红线、查询门禁、store 物理位置债务。

## 29. AI Modification Guide
- 改查询门禁/风险预估：同步 `check-database-ui-logic.mjs` 断言。
- 禁止：把密码写入前端存储、把 store 移出时不同步 manifest、用裸 `invoke`。
- store 迁入 `state/` 时需同步所有 import 点与 public.ts。

## 30. Known Debt
- **物理债务**：`useDatabaseStore` 仍在 `src/stores/`，未迁入 `src/capabilities/database/state/`；UI 直连 `src/stores/useDatabaseStore`。
- **maturityEvidence 引用名实不符**：引 `measure-resources.mjs`（通用 harness）而非专属 `check-database-ui-logic.mjs`。
- HP0 限制理由：未验证 absent 态无残留连接前不宣称 HP1。

## 31. C / HP / M / RV / D
- **C = C2**：已有 developer-owners 边界门禁 + 资源测量思路；store 未内迁故非 C3。
- **HP = HP0**：命令已实现但声明 HP0 并留验证缺口（`manifest.v1.hotPlug.level="HP0"`）。
- **M = M1**：`src/capabilities/database/` 目录隔离；无独立 npm 包（非 M2）。
- **RV = RV1 + RV2**：`semanticOwner` 已登记（RV1）；`check-developer-owners.mjs` 含 DB 有界断言（RV2）；RV3=不成立（无 vitest）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：store 物理债务、无 vitest、maturityEvidence 失准。
- 完成后可达 C3/M2 候选。

## 33. Source of Truth
- manifest：`src/capabilities/database/manifest.ts`
- public：`src/capabilities/database/public.ts`
- state（**物理债务位置**）：`src/stores/useDatabaseStore.ts`
- UI：`src/capabilities/database/ui/`
- native：`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useDatabaseStore"`
- gates：`scripts/check-developer-owners.mjs`、`scripts/check-database-ui-logic.mjs`（见 §27）
