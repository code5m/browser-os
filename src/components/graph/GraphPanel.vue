<script setup lang="ts">
import { onMounted } from "vue";
import { useGraphStore } from "../../stores/useGraphStore";
import { GRAPH_MAX_EDGES, GRAPH_MAX_NODES } from "../../utils/graphUi";
import GraphFilter from "./GraphFilter.vue";
import GraphViewer from "./GraphViewer.vue";
import NodeDetail from "./NodeDetail.vue";
import EdgeDetail from "./EdgeDetail.vue";

const store = useGraphStore();
onMounted(() => {
  void store.loadGraph();
});
</script>

<template>
  <div class="panel graph-panel" role="region" aria-label="知识图谱">
    <header class="panel-header">
      <h2>知识图谱</h2>
      <div class="actions">
        <span class="cap" role="status" aria-live="polite">节点 {{ store.capacity.nodeCount }}/{{ GRAPH_MAX_NODES }} · 边 {{ store.capacity.edgeCount }}/{{ GRAPH_MAX_EDGES }}</span>
        <button :disabled="!store.backendReady" aria-label="刷新图谱数据" @click="store.loadGraph()">刷新</button>
      </div>
    </header>

    <div v-if="!store.backendReady" class="banner banner-warn" role="status" aria-live="polite">
      后端 graph 命令尚未就绪，当前为只读壳（不发起任何后端调用）。
    </div>
    <div v-if="store.error" class="banner banner-error" role="alert" aria-live="assertive">{{ store.error }}</div>
    <div v-if="store.capState.level !== 'ok'" class="banner banner-cap" role="status" aria-live="polite">{{ store.capState.message }}</div>

    <GraphFilter />

    <div v-if="store.state.state === 'empty'" class="state-block empty-state" role="status">{{ store.state.message }}</div>
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
.empty-state { padding: 24px; color: #999; text-align: center; }
.error-state { padding: 24px; color: #cf1322; text-align: center; }
.state-block { margin: 8px; }
.body { flex: 1; display: flex; min-height: 0; }
.viewer-col { flex: 1; min-width: 0; }
.side { width: 280px; border-left: 1px solid #eee; overflow: auto; }
</style>
