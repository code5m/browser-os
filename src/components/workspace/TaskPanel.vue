<script setup lang="ts">
import { EmptyState } from "../../shared/ui";
// src/components/workspace/TaskPanel.vue
// M4-8 定时任务面板：列表 + 启用开关 + 下次/上次执行 + 立即执行 + 编辑 + 删除确认 + 历史。
//
// 边界：本组件**不做任何调度与计时决策**——所有时间由后端 `next_run_at` / `last_fired_at`
// 提供，前端只做相对时间展示；「立即执行」只是下发 `task_run_now` 命令，
// 真正跑起来的是后端既有的 script_runner（A6 F6：不另起执行路径）。
// 下文的 30s 定时器仅用于刷新相对时间文案，**从不触发任何执行**。
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useTaskStore } from "../../stores/useTaskStore";
import type { TaskDef } from "../../types";
import {
  formatRelative,
  formatTrigger,
  kindLabel,
  lastRunLabel,
  nextRunState,
  skipReasonLabel,
  triggerLabelOf,
  statusLabel,
} from "../../utils/taskUi";
import { runStatusClass } from "../../utils/scriptUi";
import TaskEditDialog from "./TaskEditDialog.vue";

const tasks = useTaskStore();

const now = ref(Date.now());
let timer: number | null = null;
onMounted(() => {
  timer = window.setInterval(() => {
    now.value = Date.now();
  }, 30_000);
});
onUnmounted(() => {
  if (timer !== null) window.clearInterval(timer);
});

// 行内二次确认：删除与「立即执行高风险任务」都必须显式确认，不用原生 confirm，
// 避免不可测的浏览器对话框（M2-6 观察项同口径）。
const pendingRemove = ref<string | null>(null);
const pendingRun = ref<string | null>(null);

function targetName(def: TaskDef): string {
  return tasks.targets.find((t) => t.id === def.target_id && t.kind === def.kind)?.name ?? def.target_id;
}

function nextLabel(def: TaskDef): string {
  const s = nextRunState(def, now.value);
  if (s.kind === "disabled") return "未启用";
  if (s.kind === "stale") return "待重算（已过期）";
  if (s.kind === "none") return "—";
  return s.label;
}

function lastLabel(def: TaskDef): string {
  return lastRunLabel(def, now.value);
}

async function onToggle(def: TaskDef, checked: boolean) {
  pendingRemove.value = null;
  await tasks.setEnabled(def, checked);
}

async function onRemove(def: TaskDef) {
  await tasks.removeTask(def.id);
  pendingRemove.value = null;
}

async function onRunNow(def: TaskDef) {
  await tasks.runNow(def.id);
  pendingRun.value = null;
  // 若正查看该目标的执行历史，执行后顺带刷新（读 script_runs_list）。
  if (tasks.selectedTaskId === def.id) await tasks.loadTargetRuns(def.id);
}

async function onToggleHistory(def: TaskDef) {
  await tasks.selectTask(def.id);
}

const recentRuns = computed(() => tasks.sortedRuns.slice(0, 50));
const dangerousHint = computed(() => (def: TaskDef) => tasks.isDangerous(def));

// 选中任务的执行历史（读 script-runs.json，按目标过滤）；倒序展示。
const selectedTaskName = computed(
  () => tasks.tasks.find((t) => t.id === tasks.selectedTaskId)?.name ?? "",
);
const selectedTargetRuns = computed(() =>
  [...tasks.targetRuns].sort((a, b) => Date.parse(b.started_at) - Date.parse(a.started_at)),
);
</script>

<template>
  <div class="task-panel">
    <div class="head">
      <div>
        <strong>定时任务</strong>
        <span class="muted">（{{ tasks.tasks.length }} 个，启用 {{ tasks.enabledCount }} 个）</span>
      </div>
      <div class="ops">
        <button :disabled="tasks.loading" @click="tasks.loadAll()">刷新</button>
        <button class="primary" :disabled="tasks.loading || !tasks.backendReady" @click="tasks.openCreate()">
          新建任务
        </button>
      </div>
    </div>

    <div v-if="!tasks.backendReady" class="banner">
      后端定时任务命令（`task_list` / `task_add` / `task_update` / `task_remove` / `task_run_now`）尚未落地，
      当前为只读壳：不发送任何命令。解锁条件见检查点 `M4-8`（待 A7 / M4-6.b 交付）。
    </div>
    <div v-if="tasks.error" class="banner err-banner">{{ tasks.error }}</div>

    <EmptyState v-if="tasks.tasks.length === 0">
      暂无定时任务。{{ tasks.backendReady ? "点击「新建任务」创建。" : "" }}
    </EmptyState>

    <ul v-else class="task-list">
      <li v-for="def in tasks.tasks" :key="def.id" class="task-row">
        <div class="main">
          <div class="title">
            <span class="name">{{ def.name }}</span>
            <span class="tag">{{ kindLabel(def.kind) }}</span>
            <span v-if="dangerousHint(def)" class="tag danger">高风险</span>
          </div>
          <div class="muted line">
            目标：{{ targetName(def) }} ｜ 触发：{{ formatTrigger(def.trigger) }}
          </div>
          <div class="muted line">
            下次：{{ nextLabel(def) }} ｜ 上次：{{ lastLabel(def) }} ｜
            重试：{{ def.retry?.max_attempts ?? 1 }} 次 ｜ 超时：{{ def.timeout_secs || 60 }}s
          </div>
        </div>

        <div class="side">
          <label class="switch" :title="def.enabled ? '点击停用' : '点击启用（启用后才会自动执行）'">
            <input
              type="checkbox"
              :checked="def.enabled"
              :disabled="tasks.loading || !tasks.backendReady"
              @change="onToggle(def, ($event.target as HTMLInputElement).checked)"
            />
            <span>{{ def.enabled ? "已启用" : "未启用" }}</span>
          </label>

          <div class="ops">
            <template v-if="pendingRun === def.id && dangerousHint(def)">
              <button class="danger" @click="onRunNow(def)">确认立即执行</button>
              <button @click="pendingRun = null">取消</button>
            </template>
            <button v-else :disabled="!tasks.backendReady" @click="pendingRun = def.id; if (!dangerousHint(def)) onRunNow(def)">
              立即执行
            </button>

            <button :disabled="!tasks.backendReady" @click="tasks.openEdit(def)">编辑</button>

            <button
              :disabled="!tasks.backendReady"
              :class="{ active: tasks.selectedTaskId === def.id }"
              @click="onToggleHistory(def)"
            >
              历史
            </button>

            <template v-if="pendingRemove === def.id">
              <button class="danger" @click="onRemove(def)">确认删除</button>
              <button @click="pendingRemove = null">取消</button>
            </template>
            <button v-else :disabled="!tasks.backendReady" @click="pendingRemove = def.id">删除</button>
          </div>
        </div>
      </li>
    </ul>

    <div v-if="tasks.selectedTaskId" class="history target-history">
      <div class="hist-head">
        <strong>执行历史 · {{ selectedTaskName }}</strong>
        <span class="muted">（读 script-runs.json，按目标过滤；含该目标的手工触发。后端暂未提供 task-runs 读取命令，故不保证 task_id 精确匹配）</span>
      </div>
      <div v-if="selectedTargetRuns.length === 0" class="muted">暂无该目标的执行记录。</div>
      <ul v-else class="run-list">
        <li v-for="r in selectedTargetRuns" :key="r.run_id" class="run-row">
          <span class="tag" :class="runStatusClass(r.status)">{{ statusLabel(r.status) }}</span>
          <span class="muted">退出码 {{ r.exit_code ?? '—' }} ｜ {{ formatRelative(r.started_at, now) }}</span>
        </li>
      </ul>
    </div>

    <div class="history">
      <div class="hist-head">
        <strong>运行历史</strong>
        <span class="muted">（最近 {{ recentRuns.length }} 条）</span>
      </div>
      <div v-if="recentRuns.length === 0" class="muted">暂无运行记录。</div>
      <ul v-else class="run-list">
        <li v-for="r in recentRuns" :key="`${r.task_id}:${r.run_id}:${r.attempt}`" class="run-row">
          <span class="tag" :class="runStatusClass(r.status)">{{ statusLabel(r.status) }}</span>
          <span class="tag trigger">{{ triggerLabelOf(r.trigger) }}</span>
          <span class="muted">第 {{ r.attempt }} 次 ｜ {{ formatRelative(r.started_at, now) }}</span>
          <span v-if="r.error_code" class="muted">｜ {{ skipReasonLabel(r.error_code) }}</span>
        </li>
      </ul>
    </div>

    <TaskEditDialog />
  </div>
</template>

<style scoped>
.task-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  height: 100%;
  overflow: auto;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.ops {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-wrap: wrap;
}
.banner {
  border: 1px solid #ffb454;
  color: #ffb454;
  border-radius: 6px;
  padding: 8px 10px;
  font-size: 12px;
}
.err-banner {
  border-color: #ff6b6b;
  color: #ff6b6b;
}
.empty {
  opacity: 0.7;
  padding: 16px 0;
}
.task-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.task-row {
  border: 1px solid var(--line, #2a2f3a);
  border-radius: 8px;
  padding: 10px 12px;
  display: flex;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.title {
  display: flex;
  align-items: center;
  gap: 8px;
}
.name {
  font-weight: 600;
}
.tag {
  font-size: 12px;
  padding: 1px 6px;
  border-radius: 4px;
  border: 1px solid var(--line, #2a2f3a);
}
.tag.danger {
  border-color: #ff6b6b;
  color: #ff6b6b;
}
.trigger {
  opacity: 0.85;
}
.muted {
  opacity: 0.7;
  font-size: 12px;
}
.line {
  margin-top: 2px;
}
.side {
  display: flex;
  flex-direction: column;
  gap: 6px;
  align-items: flex-end;
}
.switch {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
}
.history {
  margin-top: 8px;
}
.hist-head {
  display: flex;
  align-items: baseline;
  gap: 6px;
  margin-bottom: 6px;
}
.run-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.run-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.danger {
  border-color: #ff6b6b;
  color: #ff6b6b;
}
</style>
