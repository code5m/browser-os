# Database Capability — Full-Stack Boundary（Capability Library Expansion v1）

> STAGE C 产物。物理：DatabasePanel 迁入 `src/capabilities/database/ui/`，经通用 Contribution Registry
> 的 `WORKBENCH_MAIN` 槽（view='db'）贡献给 MainArea；MainArea 只按槽渲染、不 import 能力内部。
> 最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`）。

评级依据：
- 实现经明确边界隔离：`manifest.ts` / `public.ts` / `index.ts` / `ui/` 四段边界；
  语义 owner `useDatabaseStore` 唯一（无第二真源）；
  MainArea 不再静态 import DatabasePanel（贡献驱动，C3 关键）。
- **非 C3**：① `useDatabaseStore` 物理仍在 `src/stores/`（未迁入 `capabilities/database/state`，同 git 债务）；
  ② 无 database 专属 absence 运行时门禁（`minimal` profile 不含 database，贡献自然缺席，但无自动化断言）；
  ③ `mainView='db'` 导航项仍硬编码于 `useLayoutStore`/`homeUi`/`HomeLaunchers`（未贡献驱动，故 database absent 时
     点该导航会落到空视图——不崩溃，但存在「dead nav」，属已知债务）。

## Full-Stack Trace

```text
Database UI (capabilities/database/ui/DatabasePanel.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts 消费语义 owner）
Database State Owner: useDatabaseStore (id="database", src/stores/useDatabaseStore.ts)
  ↓ 意图（intents）
  connect(password) / disconnect / requestRun / confirmRun / cancelQuery / execute
  ↓ PUBLIC_DEPENDENCY（bridge）
Database Adapter: src/bridge.ts → dbConnect / dbQuery / dbDisconnect / dbListConnections / dbCancel
  ↓ NATIVE_ADAPTER（Rust tauri::command）
src-tauri/src/bridge.rs:
  db_connect   (bridge.rs:6308)  → 探针连接 + 写 keyring
  db_query     (bridge.rs:6354)  → 重连 + 执行 + 脱敏
  db_disconnect(bridge.rs:6435)
  db_cancel    (bridge.rs:6134)
  db_list_connections (bridge.rs:6116)
  ↓ 后端资源（Rust）
DbPool::connect(&cfg, password, &roots)  — 瞬态建连即弃（不驻留全局，非 Send）
Credential: KeyringStore::save_token("db:<conn_id>", p)  (keyring_store.rs)
Repository / Driver (rusqlite / mysql / postgres)
```

## 资源法（§18）— 五态区分（绝不可合并成单一 bool）

| 概念 | 真身 | 位置 | 是否资源 |
|---|---|---|---|
| 保存的连接配置 | `connections` / `configs` | disk（store 字段） | 否（配置） |
| 凭据引用 | keyring key `db:<conn_id>` | OS keyring | 否（引用） |
| 连接可用性 / 活跃态 | `connected` 标志 | store 字段 | 否（标志） |
| 实际连接资源 | `DbPool` | **Rust 瞬态** | **是** |

关键事实（来自 `bridge.rs` 注释与实现）：`db_connect` 仅作可达性/凭据探针，**连接即弃**；
`db_query` 每次重新 `DbPool::connect`。故前端不持有任何长驻连接对象，`connected` 仅是标志。

## Credential Boundary（§24 安全优先）

- 密码只作组件本地瞬时态（`DatabasePanel.vue` `password` ref），不进 store、不持久化（`F2`）。
- `db_connect` 调 `KeyringStore::save_token("db:<conn_id>", p)` 写 OS keyring；`db_query` 重读。
- 前端**零明文凭据**；`DEV-03` 机器固化（check-developer-owners.mjs）。
- Rust `KeyringStore`（`src-tauri/src/core/keyring_store.rs`，SERVICE=`com.jizhijiandan.mvp`）。

## Absence Behavior（§18/§29）

- Database absent（profile 未列，如 `minimal`）→ `registerDatabaseContributions` 不运行
  → `WORKBENCH_MAIN` 槽中无 `view='db'` 贡献 → MainArea `viewOf('db')` 返回 `undefined`
  → 不渲染 DatabasePanel。
- 因 `connect()` 仅由用户点击触发（非 `onMounted`），且 `onMounted` 仅 `db.refreshConnections()`
  （列配置，不建连接），故 absent → **无 connect 调用 → 无 keyring 读、无 DbPool**。`§18` 满足。
- Shell 不崩溃：MainArea 通用 `viewOf(layout.mainView)` 分支对未知 view 返回 undefined 自然跳过。
- 已知缺口：硬编码 `mainView='db'` 导航项未贡献驱动 → absent 时点该导航落空视图（不崩溃，UX 债务）。

## Lifecycle / Dependencies

- dependsOn: `credential`, `bridge`（manifest 已声明）。
- lifecycle: supported `[ACTIVE, SUSPENDED]`，`activatable: true`，`resident: false`。
- 停用（deactivationPolicy=graceful）：须确保进行中查询已 `cancelQuery`、瞬态连接已释放
  （Rust 每查询重连无池，天然释放）。
- Hot-plug: **HP0 STATIC**（installPolicy=static, hotPlug.enable=false）。未验证 absent 无残留前不宣称 HP1。

## Legacy Debt（诚实，不静默消失）

1. `useDatabaseStore` 物理仍在 `src/stores/`（未迁入 `capabilities/database/state`）。
2. 无 database 专属 absence 运行时门禁（依赖 profile 预设 + 贡献缺席，缺自动化断言）。
3. `mainView='db'` 导航项硬编码于 `useLayoutStore.ts:62` / `homeUi.ts:65` / `HomeLaunchers.vue:32`（未贡献驱动）。
4. 物理目录仅含 `ui/` + `manifest/index/public`；`state/` `logic/` `adapter/` `tests/` 尚未补齐（state 在 src/stores）。

## SECOND_TRUTHS = 0

`public.ts` 仅再导出 `useDatabaseStore`（语义 owner），未创建 `runtime.databaseOpen` / `databaseVisible` 之类镜像状态。
DOMAIN STATE（useDatabaseStore）≠ CAPABILITY COMPOSITION STATE ≠ UI LOCAL STATE（`password` ref 瞬态）≠ RESOURCE RESULT（result）。
