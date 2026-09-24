# task 模块 README（Phase D3 · 高质量文档化）

> 文档性质：machine truth 的引用者，非第二真源。评级真源：`src/capabilities/task/manifest.ts`。
> 诚实边界：调度契约已冻结（A1/A6）；调度线程随 app 启动（进程内 OnceLock 单例，tick=1s），前端激活只注册 UI 贡献，不创建调度线程。

---

## 1. Purpose
定时任务域。负责定时任务的登记、启用/禁用、立即运行、运行记录查看，以及调度生命周期管理。

## 2. Domain Classification
- 领域：`task`
- `manifest.category = "CAPABILITY"`

## 3. Responsibilities
- 任务 CRUD 意图（`saveDraft`/`setEnabled`/`removeTask`/`runNow`）。
- 任务列表/运行记录加载（`loadAll`/`loadTargetRuns`）。
- 调度契约消费（5 命令冻结、tick=1s、secret 不落盘、审计红线）。

## 4. Non-Responsibilities
- 不直接驱动调度线程（调度线程在 Rust 侧随 app 启动，前端仅 UI）。
- 不创建 WebView/PTY/数据库连接（除经由 script 运行目标）。
- 不负责其它域。

## 5. Ubiquitous Language
- `TaskDef`：任务定义（来自 `src/types.ts`）。
- `TaskRunRecord`：运行记录。
- `backendReady`：后端是否就绪（`=TASK_COMMANDS_AVAILABLE`）。
- `draft` / `issues` / `editorOpen`：编辑态。

## 6. Domain Model
- 聚合根：任务集合 + 运行记录（`useTaskStore` 管理，pinia id `tasks`）。
- 关键 state：`tasks` / `runs` / `targets` / `selectedTaskId` / `targetRuns` / `loading` / `error` / `backendReady` / `draft` / `issues` / `editorOpen`（`src/capabilities/task/state/useTaskStore.ts`）。
- 调度后端：`src-tauri/src/scheduler.rs`（进程内 OnceLock 单例，tick=1s）+ `src-tauri/src/tasks*.rs`。

## 7. Invariants
- 后端未就绪（`backendReady=false`）：`guard()` 拦截，零 invoke。
- 调度契约冻结（A1）：5 命令冻结、cron 仅 5 段、Interval 下界 60s、判重真源=`tasks.json` 的 `last_fired_at`、`cancel` 复用 `ScriptProcessTable::cancel`、新建任务默认 `enabled=false`。
- secret 参数不落盘、审计红线（`TASK_SECRET_PARAM_FORBIDDEN`）。

## 8. State Ownership
- **CURRENT PHYSICAL LOCATION（前端）**：`src/capabilities/task/state/useTaskStore.ts`（`defineStore("tasks")`）。
- **CURRENT PHYSICAL LOCATION（后端）**：`src-tauri/src/scheduler.rs` + `src-tauri/src/tasks*.rs`。
- **semanticOwner**：`useTaskStore`（manifest + public 登记）。
- **TARGET / KNOWN DEBT**：无物理债务（state/ui 均在包内）；无 MULTIPLE_WRITERS。

## 9. Commands / Intents
- 业务 action：`loadAll` / `saveDraft` / `setEnabled` / `removeTask` / `runNow` / `selectTask` / `loadTargetRuns`；`guard`（拦截）。
- 原生命令（见 §17，5 命令冻结）：`task_list` / `task_add` / `task_update` / `task_remove` / `task_run_now`。

## 10. Queries
- 前端内存：`tasks`/`runs`/`targetRuns` 派生。
- 原生：`task_list`（运行记录以 `script-runs.json` 为准，见债务）。

## 11. Events
- NOT_APPLICABLE。

## 12. Public Contract
- 入口：`src/capabilities/task/public.ts`。
- 暴露：`useTaskStore`（再导出）、`taskManifest`、`type TaskDef/TaskRunRecord`。

## 13. Internal Boundary
- `manifest.ts` / `public.ts` / `index.ts` / `state/useTaskStore.ts` / `ui/TaskPanel.vue` / `ui/TaskEditDialog.vue`。

## 14. Dependencies
- `dependsOn: ["workspace","bridge"]`（硬）。
- `optionalDependencies: ["script"]`（任务目标候选来自 `useScriptStore`/`useSnippetStore`，经 `../../workspace/public`）。

## 15. Dependents
- **无**（仅自消费；UI 经 Contribution Registry 到达 Shell，无模块 import `capabilities/task/public`）。

## 16. Frontend Boundary
- 贡献组件：`TaskPanel.vue` + `TaskEditDialog.vue`（WORKBENCH_MAIN，view=`tasks`）。
- 注册：`registerTaskContributions()`，懒加载 TaskPanel。
- **CURRENT PHYSICAL LOCATION**：UI 在 `src/capabilities/task/ui/`（已隔离）。

## 17. Native Boundary
- 原生命令真源：`src-tauri/src/bridge.rs`（`task_list`/`task_add`/`task_update`/`task_remove`/`task_run_now`）+ `src-tauri/src/scheduler.rs`（调度线程）。
- 已注册（main.rs）：`bridge::task_list`/`task_add`/`task_update`/`task_remove`/`task_run_now`。
- 前端封装：`src/bridge.ts`（`taskList`/`taskAdd`/`taskUpdate`/`taskRemove`/`taskRunNow`）；`TASK_COMMANDS_AVAILABLE=true`。
- **诚实声明**：Native 仍集中于 `bridge.rs`/`scheduler.rs`，未物理模块化。

## 18. Resources
- `resources.class: ["BACKGROUND"]`；`suspendable: true`；`destroyable: true`。
- `v1.resources: [{kind:"BACKGROUND_TASK", owned, evidence:"scheduler.rs tick=1s"}]`。
- `persistence.scope: "disk"`（data_dir/tasks.json + task-runs.json，原子写）；`sensitive: false`。

## 19. Side Effects
- 写 tasks.json / task-runs.json（原子写）。
- 触发 script/snippet 运行（经 workspace 目标）。

## 20. Security
- secret 参数不落盘（`TASK_SECRET_PARAM_FORBIDDEN`）。
- 审计红线（审计上限 1000 FIFO，tick 内禁写审计）。

## 21. Persistence
- 声明 `disk`；实际 tasks.json（任务定义）+ task-runs.json（历史，原子写）。

## 22. Failure Model
- 后端未就绪：`guard()` 拦截，UI 显示未就绪。
- 运行失败：运行记录标记失败（以 `script-runs.json` 为准，见债务）。

## 23. Capability Absence
- Absent 时：`registerTaskContributions` 未执行 → WORKBENCH_MAIN 无 view=`tasks` → MainArea 不渲染。
- 注意：调度线程在 Rust 侧随 app 启动，**不随前端能力 absent 而停止**（设计如此，调度是全局后台）。

## 24. Runtime Lifecycle
- `lifecycle.supported: ["ACTIVE","BACKGROUND"]`；`default: "ACTIVE"`；`activatable: true`；`resident: false`。
- `activationPolicy: auto`；`installPolicy: static`。

## 25. UI Contributions
- `task.main.panel`（WORKBENCH_MAIN / surface / view=`tasks` / TaskPanel）。

## 26. Testing
- `src/capabilities/task/` 下 `*.spec.ts`：**0**（RV3 不满足）。
- 门禁：`scripts/check-scheduler-policy.py`、`scripts/check-scheduler-ui-logic.mjs`、`scripts/check-scheduler-ui-policy.py`、`scripts/check-task-boundary.mjs`。

## 27. Gates
- `scripts/check-scheduler-policy.py`（**强**，M4-5 调度契约，重检 scheduler/tasks、5 命令冻结、secret 不落盘、审计红线）。
- `scripts/check-scheduler-ui-logic.mjs`、`scripts/check-scheduler-ui-policy.py`、`scripts/check-task-boundary.mjs`。

## 28. Review Guide
- 入口：`manifest.ts` → `public.ts` → `state/useTaskStore.ts` → `ui/TaskPanel.vue` → 后端 `scheduler.rs`/`tasks*.rs`。
- 关注点：调度契约冻结、secret 不落盘、absence 门禁缺失。

## 29. AI Modification Guide
- 改调度契约：必须先更新 `check-scheduler-policy.py` 断言并确认冻结文档（A1 M4-5），再改后端。
- 禁止：用裸 `invoke`、把 store 移出时不同步 manifest、破坏 5 命令冻结。
- 新增 UI 必须注册 Contribution Registry。

## 30. Known Debt
- A6 §6：后端未提供 task-runs 读取命令，执行历史以 `script-runs.json` 为准、不保证 `task_id` 精确匹配（`useTaskStore.ts`/`TaskPanel.vue`）。
- 调度契约冻结（A1）：调度线程随 app 启动、不随前端激活创建，前端激活只注册 UI。
- 非 C3 缺口：无 task 专属 absence 运行时门禁 + `mainView='tasks'` 导航硬编码。

## 31. C / HP / M / RV / D
- **C = C2**：`manifest.v1.maturity="C2"`，边界隔离 + 贡献驱动 + 强 scheduler checker；自述非 C3（缺 absence 门禁 + nav 硬编码）。
- **HP = HP0**：`manifest.v1.hotPlug.level="HP0"`（全 false）；调度线程全局后台，无前端独立装卸需求声明。
- **M = M1（完全）**：`src/capabilities/task/` 目录隔离（state/ui 均在包内）。无独立 npm 包（非 M2）。
- **RV = RV1 + RV2（强） + RV3（否）**：owner 已登记（RV1）；`check-scheduler-policy.py` + `check-scheduler-ui-logic.mjs` 定向覆盖（RV2 强）；无 vitest（RV3 否）。
- **D = D3**：本 README 满足 D3。文档化前为 D0。

## 32. Extraction Readiness
- 阻塞项：无 vitest、absence 门禁缺失、task-runs 读取缺口。
- 物理隔离完备，可作 Package Extraction 范本候选。

## 33. Source of Truth
- manifest：`src/capabilities/task/manifest.ts`
- public：`src/capabilities/task/public.ts`
- state：`src/capabilities/task/state/useTaskStore.ts`
- UI：`src/capabilities/task/ui/`
- native：`src-tauri/src/scheduler.rs`、`src-tauri/src/tasks.rs`、`src-tauri/src/bridge.rs`、`src-tauri/src/main.rs`、`src/bridge.ts`
- semantic owner：manifest `semanticOwner: "useTaskStore"`
- gates：`scripts/check-scheduler-policy.py`、`scripts/check-scheduler-ui-logic.mjs`、`scripts/check-task-boundary.mjs`（见 §27）
