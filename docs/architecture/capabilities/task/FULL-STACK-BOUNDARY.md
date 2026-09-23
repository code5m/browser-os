# Task Capability — Full-Stack Boundary（Capability Library Expansion v1, STAGE G）

> STAGE G 产物。物理：TaskPanel（+ TaskEditDialog）迁入 `src/capabilities/task/ui/`，语义 owner
> `useTaskStore` 迁入 `src/capabilities/task/state/`；经通用 Contribution Registry 的 `WORKBENCH_MAIN`
> 槽（view='tasks'）贡献给 MainArea。面板懒加载（defineAsyncComponent）保留在能力适配器内（IF-2 体积闸门）。
> 最后更新：2026-09-23。

## 成熟度（诚实，不谎报）

**C2 ISOLATED**（status=`COMPATIBILITY_WRAPPED`，manifest.v1.maturity=`C2`；governanceStatus=`GOVERNED`）。

评级依据（按 `C0..C5` 标度）：
- **C2 ISOLATED 达成**：实现经 `manifest.ts` / `public.ts` / `index.ts` / `state/` / `ui/` 五段边界隔离；
  语义 owner `useTaskStore` 唯一（无第二真源）；MainArea 不再静态 import TaskPanel（贡献驱动）。
- **非 C3**：① 无 task 专属 absence 运行时门禁；② `mainView='tasks'` 导航项仍硬编码未贡献驱动。
- **非 C4/C5**：调度线程随 app 启动且为进程内单例（OnceLock），无独立生命周期可装卸。

## 十七段 Full-Stack 契约

```text
Task UI (capabilities/task/ui/TaskPanel.vue + TaskEditDialog.vue)
  ↓ OWNED_BY_CAPABILITY（经 public.ts 消费语义 owner）
Task State Owner: useTaskStore (id="tasks", src/capabilities/task/state/useTaskStore.ts)
  ↓ 意图（intents）
  loadAll / openCreate / openEdit / saveDraft / setEnabled / removeTask / runNow
  / selectTask / loadTargetRuns / closeEditor
  ↓ PUBLIC_DEPENDENCY（bridge）+ CROSS_CAPABILITY_PUBLIC（workspace/public：script & snippet 候选）
Task Adapter: src/bridge.ts → taskList / taskAdd / taskUpdate / taskRemove / taskRunNow（TASK_COMMANDS_AVAILABLE=true）
  ↓ NATIVE_ADAPTER（Rust tauri::command）
src-tauri/src/bridge.rs: task_list(1004) task_add(1017) task_update(1080) task_remove(1123) task_run_now(1148)
  ↓ 后端资源（Rust）
src-tauri/src/scheduler.rs: 进程内**唯一**调度线程（std::thread + tick=1s），OnceLock 保证单例；
  fire() 复用 script_runner::start_run/start_command（唯一子进程入口，非第二路径）。
  持久化：tasks.json / task-runs.json（session::atomic_write）。
```

## 十七段逐项事实

| 段 | 事实（证据） |
|---|---|
| Identity / Manifest | `src/capabilities/task/manifest.ts`（id=task，C2，HP0） |
| Public Contract | `src/capabilities/task/public.ts`（仅再导出 `useTaskStore`，SECOND_TRUTHS=0） |
| Dependencies | required `workspace` + `bridge`（后者为外部基础设施豁免）；optional `script` |
| State Owner | `useTaskStore`（id="tasks"）唯一 |
| Canonical Writers | 仅 `useTaskStore` 的 action 写 `tasks/runs/targets/draft/issues`；组件只读经 public |
| Intents | loadAll / openCreate / openEdit / saveDraft / setEnabled / removeTask / runNow / selectTask |
| Application Logic | `src/utils/taskUi.ts`（headless：cron 校验、retry clamp、序列化、相对时间、状态标签） |
| UI | `capabilities/task/ui/{TaskPanel,TaskEditDialog}.vue`（懒加载，经贡献注册） |
| Contributions | `task.main.panel`（slot=workbench-main, view='tasks'） |
| Side Effects | 全部经 `bridge.task*`；UI 零裸 invoke；跨能力只走 workspace **public** |
| Adapter / Native Boundary | `bridge.ts` → Rust `task_*`（5 命令，ACL 放行，来源校验） |
| Permissions | `process.spawn`（执行复用 script_runner） |
| Persistence | 后端 `data_dir/tasks.json` + `task-runs.json`（atomic_write）；前端**零浏览器存储**（`SCHEDUI_CRED_PERSISTED`） |
| Resource Ownership | `BACKGROUND_TASK` owned（`src-tauri/src/scheduler.rs`，进程内唯一调度线程） |
| Lifecycle | `ACTIVE` / `BACKGROUND`；suspendable/destroyable 声明，但调度线程无独立生命周期（HP0） |
| Absence Behavior | 见下（实测证据） |
| Tests / Gates | `check-scheduler-policy.py`（Rust，23 码）+ `check-scheduler-ui-policy.py`（17 码）+ `check-scheduler-ui-logic.mjs`（105 断言）+ `npm run check` |

## 安全 / 边界

- **默认不启用**：新建任务 `enabled=false`（`DEFAULT_TASK_ENABLED=false`，门禁 `SCHEDUI_ENABLED_DEFAULT_TRUE` / `SCHEDUI_ENABLED_MUTATED`）。
- **secret 不入库/不可编辑**：参数值 secret 不保存（`SCHEDUI_SECRET_EDITABLE`）；前端零浏览器存储（`SCHEDUI_CRED_PERSISTED`）。
- **执行单一入口**：任务触发复用 `script_runner`（非第二执行路径，门禁 `SCHED_SINGLE_EXEC_PATH`）。
- **凭据不回显**：错误信息仅稳定本地文案；`SCHEDUI_STORE_UNGUARDED` 保证 backendReady 未就绪时零 invoke。
- **跨能力边界**：仅经 `capabilities/workspace/public`（已登记 `ui04b_cross_capability_public_baseline`）。

## Absence Behavior（§18/§29，实测证据）

STAGE G 用探针实测（esbuild 真实 bundle + `bootstrapCapabilityRuntime(profile)`）：

```text
framework: activated=true graph=false tasks=false plugin=false
minimal:   activated=true graph=false tasks=false plugin=false
developer: activated=true graph=false tasks=false plugin=false
full:      activated=true graph=true  tasks=true  plugin=true
```

- Task absent（framework/minimal/developer）→ `registerTaskContributions` 不运行 → `WORKBENCH_MAIN` 槽无 `view='tasks'`
  → MainArea `viewOf('tasks')` 返回 `undefined` → 不渲染 TaskPanel → `useTaskStore` 不被实例化 → **零 bridge.task\* invoke**。
- Shell 不崩溃：通用 `viewOf` 分支对未知 view 返回 undefined 自然跳过。
- **注意（诚实）**：Rust 侧调度线程随 **app 启动**（`main.rs`）创建，**不**随前端能力激活而创建；
  前端能力 absent 只保证「UI 与前端 invoke 不出现」，不改变后端调度线程存在性（前端不得谎称能控制后端线程生命周期）。

## Legacy Debt（诚实，不静默消失）

1. 无 task 专属 absence 运行时门禁（调度线程为 app 级单例，前端无法证明其 absent）。
2. `mainView='tasks'` 导航项硬编码（`useLayoutStore`/`homeUi`/`HomeLaunchers`，未贡献驱动）。
3. `check-scheduler-ui-logic.mjs` / `check-scheduler-ui-policy.py` 已接线（pre-merge），但 `check-scheduler-ui-policy` 的迁移后路径为 STAGE G 新值。

## SECOND_TRUTHS = 0 / RESOURCE_LEAKS = 0

`public.ts` 仅再导出 `useTaskStore`（语义 owner），未创建镜像状态。DOMAIN STATE（useTaskStore）
≠ CAPABILITY COMPOSITION STATE ≠ UI LOCAL STATE（draft/editorOpen）≠ RESOURCE RESULT。
新增前端资源泄漏 = 0（调度线程为既有后端资源，非本 stage 新增）。
