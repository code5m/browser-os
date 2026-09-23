<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from "vue";
import { useGraphStore } from "../state/useGraphStore";
import {
  GRAPH_DEBOUNCE_MS,
  GRAPH_MAX_EDGES,
  GRAPH_MAX_NODES,
  GRAPH_TRUNCATION_NOTICE,
} from "../../../utils/graphUi";
import GraphFilter from "./GraphFilter.vue";
import GraphViewer from "./GraphViewer.vue";
import NodeDetail from "./NodeDetail.vue";
import EdgeDetail from "./EdgeDetail.vue";

const store = useGraphStore();
// 起始节点输入（W12 增量；与 A7 §3.1 graph_query 的 start_id 字段对应）
// 旧 fixture 测试可能没设 start_id；后端落地后用户可在此输入 ID 触发实时 query。
const startIdInput = ref<string>(store.startId ?? "");

onMounted(() => {
  // W12 行为：组件挂载时不再盲目 loadGraph()，而是保留用户当前 startId。
  // 若 startId 已有值（fixture / 上次会话），按 refresh 路径；
  // 否则保持面板只读壳（不 invoke）——这是 W11 LIMITED START 口径。
  if (store.backendReady && store.startId) {
    void store.loadGraph({ start_id: store.startId });
  }
});

onBeforeUnmount(() => {
  // 切 Tab / 卸载面板时取消所有 in-flight 请求（A7 §5 取消语义）
  store.cancelInFlight();
});

function onSubmit(): void {
  const v = startIdInput.value.trim();
  if (!v) return;
  void store.loadGraph({ start_id: v });
}
function onRefresh(): void {
  if (store.startId) {
    void store.loadGraph({ start_id: store.startId });
  }
}
</script>

<template>
  <div class="panel graph-panel" role="region" aria-label="知识图谱">
    <header class="panel-header">
      <h2>知识图谱</h2>
      <div class="actions">
        <span class="cap" role="status" aria-live="polite">节点 {{ store.capacity.nodeCount }}/{{ GRAPH_MAX_NODES }} · 边 {{ store.capacity.edgeCount }}/{{ GRAPH_MAX_EDGES }}</span>
        <button :disabled="!store.backendReady || !startIdInput.trim()" aria-label="按起始节点查询" @click="onSubmit">查询</button>
        <button :disabled="!store.backendReady || !store.startId" aria-label="刷新图谱数据" @click="onRefresh">刷新</button>
      </div>
    </header>

    <!-- W12 增量：起始节点输入区（与 A7 §3.1 start_id 对齐） -->
    <div class="query-bar">
      <label class="query-label" for="graph-start-id">起始节点 ID</label>
      <input
        id="graph-start-id"
        v-model="startIdInput"
        class="query-input"
        :placeholder="store.backendReady ? '输入起始节点 ID 后回车查询' : '后端命令尚未就绪'"
        :disabled="!store.backendReady"
        :aria-invalid="!!store.error"
        @keydown.enter="onSubmit"
      />
      <span class="query-hint" aria-hidden="true">防抖 {{ GRAPH_DEBOUNCE_MS }} ms</span>
    </div>

    <div v-if="!store.backendReady" class="banner banner-warn" role="status" aria-live="polite">
      后端 graph 命令尚未就绪，当前为只读壳（不发起任何后端调用）。
    </div>
    <div v-if="store.error" class="banner banner-error" role="alert" aria-live="assertive">{{ store.error }}</div>
    <div v-if="store.capState.level !== 'ok'" class="banner banner-cap" role="status" aria-live="polite">{{ store.capState.message }}</div>
    <div v-if="store.truncated" class="banner banner-trunc" role="status" aria-live="polite">{{ GRAPH_TRUNCATION_NOTICE }}</div>

    <GraphFilter />

    <div v-if="store.state.state === 'empty'" class="state-block" :class="store.readOnly ? 'readonly-state' : 'empty-state'" role="status">
      <template v-if="store.readOnly">只读模式：后端 graph 命令尚未就绪，面板为只读壳，不发起任何后端调用。</template>
      <template v-else>{{ store.state.message }}</template>
    </div>
    <div v-else-if="store.state.state === 'error'" class="state-block error-state" role="alert">{{ store.state.message }}</div>
    <div v-else class="body">
      <GraphViewer class="viewer-col" />
      <aside class="side">
        <NodeDetail :node="store.selectedNode" />
        <EdgeDetail :edge="store.selectedEdge" />
      </aside>
    </div>
  </div>
</template>

<style scoped>
.panel { display: flex; flex-direction: column; height: 100%; }
.panel-header { display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; }
.actions { display: flex; align-items: center; gap: 8px; }
.cap { font-size: 12px; color: #888; }
.banner { padding: 6px 10px; border-radius: 6px; margin: 0 8px; font-size: 12px; }
.banner-warn { background: #fff7e6; color: #ad6800; }
.banner-error { background: #fff1f0; color: #cf1322; }
.banner-cap { background: #f9f0ff; color: #722ed1; }
.banner-trunc { background: #e6f4ff; color: #0958d9; }
.empty-state { padding: 24px; color: #999; text-align: center; }
.readonly-state { padding: 24px; color: #ad6800; text-align: center; background: #fff7e6; border-radius: 8px; }
.error-state { padding: 24px; color: #cf1322; text-align: center; }
.state-block { margin: 8px; }
.body { flex: 1; display: flex; min-height: 0; }
.viewer-col { flex: 1; min-width: 0; }
.side { width: 280px; border-left: 1px solid #eee; overflow: auto; }
.query-bar { display: flex; align-items: center; gap: 8px; padding: 4px 12px; border-bottom: 1px solid #eee; }
.query-label { font-size: 12px; color: #888; flex-shrink: 0; }
.query-input { flex: 1; padding: 4px 8px; border: 1px solid #d9d9d9; border-radius: 4px; font-size: 13px; font-family: monospace; }
.query-input:focus { outline: none; border-color: #4096ff; }
.query-input:disabled { background: #f5f5f5; color: #999; }
.query-hint { font-size: 11px; color: #bbb; flex-shrink: 0; }
</style>
