// src/stores/useTaskStore.ts
// M4-8 定时任务面板状态机。
//
// 约束（board + A6 契约）：
//   - 组件不得直接 invoke，一切经 `bridge.ts`；
//   - 后端 5 条命令（task_list/add/update/remove/run_now）由 A7（M4-6.b）落地；
//     未落地前 `backendReady=false`，**所有动作 no-op 且零 invoke**（LIMITED START 口径）；
//   - 新建任务默认 `enabled=false`（A6 裁定 R-A6-1），store 不得偷偷置 true；
//   - 参数值里的 secret 一律不保存（A6 §3.3 R-3），凭据不进前端持久化。

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { TASK_COMMANDS_AVAILABLE, bridge } from "../bridge";
import type {
  RunSnapshot,
  ScriptRunRecord,
  TaskDef,
  TaskRunRecord,
} from "../types";
import {
  emptyTaskDraft,
  loadTaskDraft,
  runRecordFromSnapshot,
  runsByTask,
  serializeTaskAddArgs,
  serializeTaskDraft,
  sortRuns,
  validateTaskDraft,
  type TaskDraft,
  type TaskIssue,
  type TaskTargetOption,
} from "../utils/taskUi";
import { useScriptStore } from "./useScriptStore";
import { useSnippetStore } from "./useSnippetStore";

export const useTaskStore = defineStore("tasks", () => {
  const tasks = ref<TaskDef[]>([]);
  const runs = ref<TaskRunRecord[]>([]);
  const targets = ref<TaskTargetOption[]>([]);
  /// 选中任务（用于「执行历史」抽屉）；为空表示未选中。
  const selectedTaskId = ref<string | null>(null);
  /// 选中目标的执行历史：读 `script_runs_list`（按 target_id 过滤），含手工触发。
  /// 后端未提供读取 task-runs 的命令（A6 §6），故历史以 script-runs 为准、并明确标注。
  const targetRuns = ref<ScriptRunRecord[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  /// 后端命令可用性。A7 落地后置 true（见 `bridge.TASK_COMMANDS_AVAILABLE`）。
  const backendReady = ref<boolean>(TASK_COMMANDS_AVAILABLE);

  const draft = ref<TaskDraft>(emptyTaskDraft());
  const issues = ref<TaskIssue[]>([]);
  const editorOpen = ref(false);

  const sortedRuns = computed(() => sortRuns(runs.value));
  const enabledCount = computed(() => tasks.value.filter((t) => t.enabled).length);
  const runsFor = (taskId: string) => runsByTask(runs.value)[taskId] ?? [];
  const issueFor = (field: string) => issues.value.find((i) => i.field === field)?.message ?? null;

  /// 后端未就绪时统一拦截：**一条 invoke 都不发**，避免对不存在的命令反复报错。
  function guard(): boolean {
    if (backendReady.value) return true;
    error.value = "后端定时任务命令尚未就绪（A7 / M4-6.b 未交付），面板为只读壳";
    return false;
  }

  function fail(e: unknown): void {
    error.value = e instanceof Error ? e.message : String(e ?? "未知错误");
  }

  /// 目标候选：脚本库（script）+ 命令片段库（command）。
  /// 二者都来自既有 store，不新增读取通道。
  async function loadTargets(): Promise<void> {
    const ss = useScriptStore();
    const sn = useSnippetStore();
    if (ss.scripts.length === 0) await ss.loadScripts();
    if (sn.snippets.length === 0) await sn.loadSnippets();
    targets.value = [
      ...ss.scripts.map((s) => ({
        id: s.id,
        name: s.name,
        kind: "script" as const,
        params: s.params ?? [],
        dangerous: false,
      })),
      ...sn.snippets.map((s) => ({
        id: s.id,
        name: s.name,
        kind: "command" as const,
        params: s.params ?? [],
        dangerous: Boolean(s.dangerous),
      })),
    ];
  }

  /// A6 §6：`task_list` 返回任务列表，历史可附带返回。
  /// 两种形态都接受（纯数组 / { tasks, runs }），A7 定型后只需改本函数一处。
  function applyTaskList(payload: unknown): void {
    if (Array.isArray(payload)) {
      tasks.value = payload as TaskDef[];
      return;
    }
    const composite = payload as { tasks?: TaskDef[]; runs?: TaskRunRecord[] } | null;
    tasks.value = composite?.tasks ?? [];
    if (Array.isArray(composite?.runs)) runs.value = composite?.runs ?? [];
  }

  async function loadAll(): Promise<void> {
    if (!guard()) return;
    loading.value = true;
    error.value = null;
    try {
      applyTaskList(await bridge.taskList());
      await loadTargets();
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  function openCreate(): void {
    draft.value = emptyTaskDraft();
    issues.value = [];
    editorOpen.value = true;
  }

  function openEdit(def: TaskDef): void {
    draft.value = loadTaskDraft(def);
    issues.value = [];
    editorOpen.value = true;
  }

  function closeEditor(): void {
    editorOpen.value = false;
    issues.value = [];
  }

  /// 前端前置校验（后端仍会再判一次，此处只为即时反馈与减少无效往返）。
  function validate(): boolean {
    issues.value = validateTaskDraft(draft.value, {
      targets: targets.value,
      taskCount: tasks.value.length,
    });
    return issues.value.length === 0;
  }

  async function saveDraft(): Promise<boolean> {
    if (!guard()) return false;
    if (!validate()) return false;
    loading.value = true;
    error.value = null;
    try {
      if (draft.value.id) {
        // 编辑：保留服务端侧的 created_at / last_fired_at / next_run_at（序列化载荷不含），
        // 仅在既有 TaskDef 上覆盖本次变更字段，避免回写时把它们清零。
        const existing = tasks.value.find((t) => t.id === draft.value.id) ?? ({} as TaskDef);
        await bridge.taskUpdate({
          ...existing,
          ...serializeTaskDraft(draft.value),
          id: draft.value.id,
        } as TaskDef);
      } else {
        // 新建：task_add 是平铺 camelCase 入参（Tauri 默认转换），整包传出。
        await bridge.taskAdd(serializeTaskAddArgs(draft.value));
      }
      editorOpen.value = false;
      await loadAll();
      return true;
    } catch (e) {
      fail(e);
      return false;
    } finally {
      loading.value = false;
    }
  }

  async function setEnabled(def: TaskDef, enabled: boolean): Promise<void> {
    if (!guard()) return;
    loading.value = true;
    error.value = null;
    try {
      await bridge.taskUpdate({ ...def, enabled });
      await loadAll();
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  async function removeTask(id: string): Promise<void> {
    if (!guard()) return;
    loading.value = true;
    error.value = null;
    try {
      await bridge.taskRemove(id);
      tasks.value = tasks.value.filter((t) => t.id !== id);
      if (selectedTaskId.value === id) {
        selectedTaskId.value = null;
        targetRuns.value = [];
      }
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  async function runNow(id: string): Promise<RunSnapshot | null> {
    if (!guard()) return null;
    error.value = null;
    try {
      const snap = await bridge.taskRunNow(id);
      // 把本次会话的手工触发快照并入 runs，供面板即时展示（持久历史仍由后端落盘）。
      if (snap) {
        runs.value = [runRecordFromSnapshot(id, snap), ...runs.value].slice(0, 50);
      }
      return snap;
    } catch (e) {
      fail(e);
      return null;
    }
  }

  // 选中任务并加载其目标执行历史（读既有 script_runs_list，按 target_id 过滤）。
  // 后端未提供 task-runs 读取命令（A6 §6），历史以 script-runs.json 为准、面板明确标注。
  async function loadTargetRuns(id: string): Promise<void> {
    const def = tasks.value.find((t) => t.id === id);
    // 未就绪时不发 IPC（历史读取依赖后端命令）；不污染主错误条。
    if (!def || !backendReady.value) {
      targetRuns.value = [];
      return;
    }
    try {
      targetRuns.value = await bridge.scriptRunsList(def.target_id);
    } catch {
      targetRuns.value = [];
    }
  }

  async function selectTask(id: string): Promise<void> {
    if (selectedTaskId.value === id) {
      selectedTaskId.value = null;
      targetRuns.value = [];
      return;
    }
    selectedTaskId.value = id;
    await loadTargetRuns(id);
  }

  function isDangerous(def: TaskDef): boolean {
    return targets.value.some((t) => t.id === def.target_id && t.kind === def.kind && t.dangerous);
  }

  return {
    tasks,
    runs,
    targets,
    selectedTaskId,
    targetRuns,
    loading,
    error,
    backendReady,
    draft,
    issues,
    editorOpen,
    sortedRuns,
    enabledCount,
    runsFor,
    issueFor,
    loadAll,
    loadTargets,
    openCreate,
    openEdit,
    closeEditor,
    validate,
    saveDraft,
    setEnabled,
    removeTask,
    runNow,
    selectTask,
    loadTargetRuns,
    isDangerous,
  };
});
