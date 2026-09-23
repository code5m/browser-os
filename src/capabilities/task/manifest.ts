import type { CapabilityDefinition } from "../../capability/types"

// Task 能力 Manifest（Capability Library Expansion v1 — STAGE G）
// 语义 owner = useTaskStore（STAGE G 物理迁入 capabilities/task/state，单一真源）。
// 物理：TaskPanel（+ TaskEditDialog）经通用 Contribution Registry 的 WORKBENCH_MAIN 槽（view='tasks'）
// 贡献给 MainArea。
//
// 诚实边界：后端 `task_list/add/update/remove/run_now` **已实现**（`TASK_COMMANDS_AVAILABLE=true`），
// 并由 Rust 进程内**唯一调度线程**（`src-tauri/src/scheduler.rs`，tick=1s）驱动；执行复用 `script_runner`
// （唯一子进程入口，非第二路径）。权限 `process.spawn`；持久化 `data_dir/tasks.json` + `task-runs.json`（原子写）。
// 前端**零浏览器存储**（门禁 `SCHEDUI_CRED_PERSISTED`）。
//
// 成熟度：C2 ISOLATED（实现经 manifest/public/index/ui/state 边界隔离 + 贡献驱动）。
//   非 C3：① 无 task 专属 absence 运行时门禁；② mainView='tasks' 导航项仍硬编码未贡献驱动。
//   注意：后端调度线程随 app 启动（main.rs），不随前端能力激活而创建 —— 故前端激活只注册 UI 贡献。
export const taskManifest: CapabilityDefinition = {
  id: "task",
  name: "定时任务",
  category: "CAPABILITY",
  provides: [
    "task.schedule",
    "task.run",
    "task.cancel",
  ],
  dependsOn: ["workspace", "bridge"],
  optionalDependencies: ["script"],
  lifecycle: {
    supported: ["ACTIVE", "BACKGROUND"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["BACKGROUND"],
    suspendable: true,
    destroyable: true,
  },
  permissions: ["process.spawn"],
  persistence: {
    scope: "disk",
    sensitive: false,
  },
  entrypoint: "index.ts",
  semanticOwner: "useTaskStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）。
  v1: {
    id: "task",
    version: "1.0.0",
    displayName: "定时任务",
    description:
      "定时任务调度/运行/取消 + 运行历史。后端 5 命令已实现，进程内唯一调度线程驱动，执行复用 script_runner。",
    maturity: "C2",
    maturityEvidence: ["scripts/check-scheduler-policy.py", "scripts/check-scheduler-ui-logic.mjs"],
    dependencies: ["workspace", "bridge"],
    optionalDependencies: ["script"],
    conflicts: [],
    provides: ["task.schedule", "task.run", "task.cancel"],
    requires: [],
    contributions: [{ id: "task.main.panel", slot: "workbench-main", type: "surface", view: "tasks" }],
    permissions: ["process.spawn"],
    resources: [
      { kind: "BACKGROUND_TASK", ownership: "owned", evidence: "src-tauri/src/scheduler.rs (进程内唯一调度线程，tick=1s)" },
    ],
    persistenceScope: "disk",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP0",
      enable: false,
      disable: false,
      register: false,
      unregister: false,
      install: false,
      uninstall: false,
      limitationReason:
        "HP0(STATIC)：调度线程随 app 启动且为进程内单例（OnceLock），无独立生命周期可装卸；未验证 absent 无残留前不宣称 HP1。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/task/public.ts" }],
    entrypoint: "src/capabilities/task/index.ts",
    semanticOwner: "useTaskStore",
  },
}
