<script setup lang="ts">
// src/components/workspace/ScriptRunHistory.vue
// M2-5.c 运行历史视图：列表 + 状态筛选 + 行展开 output_tail（纯文本插值，禁 XSS）
// 数据源 = bridge.scriptRunsList()，仅打开时取一次；运行态由 RunPanel（b 卡）负责。
import { ref, computed, onMounted } from "vue";
import { bridge } from "../../../bridge";
import { useScriptStore } from "../state/useScriptStore";
import {
  filterRuns,
  groupRunsByDay,
  runStatusClass,
  runStatusLabel,
  formatRunDuration,
  summarizeRuns,
  tailPreview,
  type RunStatusFilter,
} from "../../../utils/scriptUi";
import type { ScriptRunRecord } from "../../../types";

const ws = useScriptStore();

const records = ref<ScriptRunRecord[]>([]);
const loading = ref(false);
const errorMsg = ref<string | null>(null);
const expanded = ref<Set<string>>(new Set());
const statusFilter = ref<RunStatusFilter>("all");

const filtered = computed(() => filterRuns(records.value, statusFilter.value));
const grouped = computed(() => groupRunsByDay(filtered.value));
const summary = computed(() => summarizeRuns(records.value));

const STATUS_OPTIONS: Array<{ value: RunStatusFilter; label: string }> = [
  { value: "all", label: "全部" },
  { value: "succeeded", label: "成功" },
  { value: "failed", label: "失败" },
  { value: "cancelled", label: "已取消" },
  { value: "timeout", label: "超时" },
  { value: "running", label: "运行中" },
];

function scriptName(scriptId: string): string {
  const m = ws.scripts.find((s) => s.id === scriptId);
  return m ? m.name : scriptId;
}

function toggle(runId: string) {
  if (expanded.value.has(runId)) expanded.value.delete(runId);
  else expanded.value.add(runId);
  // 触发响应式
  expanded.value = new Set(expanded.value);
}

function formatStart(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const hh = String(d.getHours()).padStart(2, "0");
  const mm = String(d.getMinutes()).padStart(2, "0");
  const ss = String(d.getSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

async function refresh() {
  loading.value = true;
  errorMsg.value = null;
  try {
    records.value = await bridge.scriptRunsList();
  } catch (e) {
    errorMsg.value = String(e);
    records.value = [];
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
</script>

<template>
  <div class="run-history">
    <div class="toolbar">
      <select v-model="statusFilter" class="filter">
        <option v-for="o in STATUS_OPTIONS" :key="o.value" :value="o.value">
          {{ o.label }}（{{ summary[o.value as keyof typeof summary] ?? (o.value === "all" ? records.length : 0) }}）
        </option>
      </select>
      <button class="refresh" :disabled="loading" @click="refresh">
        {{ loading ? "刷新中…" : "刷新" }}
      </button>
    </div>

    <div v-if="errorMsg" class="error">⚠ {{ errorMsg }}</div>

    <div v-else-if="!records.length" class="empty">
      暂无运行历史，去运行一个脚本试试
    </div>

    <div v-else-if="!grouped.length" class="empty">该筛选下没有记录</div>

    <div v-else class="groups">
      <div v-for="g in grouped" :key="g.day" class="day-group">
        <div class="day-title">{{ g.day }}（{{ g.items.length }}）</div>
        <ul>
          <li v-for="r in g.items" :key="r.run_id" class="run-item">
            <button class="row" @click="toggle(r.run_id)">
              <span class="badge" :class="runStatusClass(r.status)">
                {{ runStatusLabel(r.status) }}
              </span>
              <span class="script-name">{{ scriptName(r.script_id) }}</span>
              <span class="meta">
                {{ formatStart(r.started_at) }} · {{ formatRunDuration(r) }}
                <template v-if="r.exit_code !== null"> · exit {{ r.exit_code }}</template>
                <template v-if="r.truncated"> · <span class="trunc">尾部截断</span></template>
              </span>
            </button>
            <div v-if="expanded.has(r.run_id)" class="detail">
              <div v-if="r.error" class="err-line">⚠ {{ r.error }}</div>
              <pre class="output">{{ tailPreview(r, 4000) || "(无输出)" }}</pre>
              <div class="ids">run_id={{ r.run_id }} · script_id={{ r.script_id }}</div>
            </div>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>

<style scoped>
.run-history { display: flex; flex-direction: column; height: 100%; }
.toolbar { display: flex; gap: 6px; padding: 6px 8px; border-bottom: 1px solid #e5e6eb; }
.filter { flex: 1; padding: 4px 6px; border: 1px solid #d5dbe7; border-radius: 5px; font-size: 12px; background: #fff; }
.refresh { padding: 4px 10px; border: 1px solid #d5dbe7; background: #fff; border-radius: 5px; font-size: 12px; cursor: pointer; }
.refresh:disabled { opacity: 0.5; cursor: default; }
.empty { color: #86909c; font-size: 12px; padding: 24px; text-align: center; }
.error { color: #c0392b; font-size: 12px; padding: 12px; }
.groups { overflow: auto; flex: 1; padding: 4px 0; }
.day-group { margin: 4px 0 8px; }
.day-title { font-size: 11px; color: #86909c; padding: 4px 8px; }
.day-group ul { list-style: none; margin: 0; padding: 0; }
.run-item { border-bottom: 1px solid #f0f1f3; }
.row { display: flex; align-items: center; gap: 6px; width: 100%; padding: 6px 8px; border: none; background: transparent; cursor: pointer; text-align: left; font: inherit; }
.row:hover { background: #eef2fb; }
.badge { font-size: 10px; padding: 1px 6px; border-radius: 8px; flex: 0 0 auto; }
.badge.status-running { background: #e6f4ff; color: #1677ff; }
.badge.status-succeeded { background: #e8f5e9; color: #2e7d32; }
.badge.status-failed { background: #ffebee; color: #c0392b; }
.badge.status-cancelled { background: #f5f5f5; color: #616161; }
.badge.status-timeout { background: #fff3e0; color: #e65100; }
.script-name { font-size: 13px; color: #1f2329; flex: 0 0 auto; max-width: 40%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.meta { font-size: 11px; color: #86909c; margin-left: auto; }
.trunc { color: #e65100; }
.detail { padding: 6px 8px 10px 32px; background: #fafbfc; border-top: 1px dashed #e5e6eb; }
.err-line { color: #c0392b; font-size: 12px; margin-bottom: 4px; white-space: pre-wrap; word-break: break-word; }
.output { font-family: monospace; font-size: 11px; background: #1f2329; color: #e6e6e6; padding: 6px 8px; border-radius: 4px; max-height: 240px; overflow: auto; margin: 0; white-space: pre-wrap; word-break: break-word; }
.ids { font-size: 10px; color: #a0a4ad; margin-top: 4px; word-break: break-all; }
</style>
