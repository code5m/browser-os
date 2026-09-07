<script setup lang="ts">
import { computed } from "vue";
import { useGraphStore } from "../../stores/useGraphStore";
import {
  clampRender,
  edgeKey,
  edgeKindLabel,
  layoutPositions,
  nodeColor,
  nodeKindLabel,
  RENDER_EDGE_CAP,
  RENDER_NODE_CAP,
} from "../../utils/graphUi";

const store = useGraphStore();

// 渲染有界：正常数据不会截断（cap=数据上限），仅作 UI 安全网（W10 有界渲染）。
const renderNodes = computed(() => clampRender(store.visibleNodes, RENDER_NODE_CAP));
const renderEdges = computed(() => clampRender(store.visibleEdges, RENDER_EDGE_CAP));
const renderTruncated = computed(
  () => renderNodes.value.truncated || renderEdges.value.truncated,
);

const points = computed(() => {
  const map = new Map<string, { x: number; y: number }>();
  for (const p of layoutPositions(renderNodes.value.items, renderEdges.value.items)) {
    map.set(p.id, { x: p.x, y: p.y });
  }
  return map;
});

function onNode(id: string): void {
  store.selectNode(id);
}
function onEdge(key: string): void {
  store.selectEdge(key);
}
</script>

<template>
  <div class="viewer">
    <p class="sr-only" aria-live="polite">{{ store.selectionText }}</p>
    <p v-if="renderTruncated" class="render-note" role="status">
      视图仅渲染前 {{ RENDER_NODE_CAP }} 个节点 / {{ RENDER_EDGE_CAP }} 条边，完整数据可通过筛选查看。
    </p>
    <svg
      v-if="renderNodes.total"
      viewBox="0 0 800 600"
      class="canvas"
      role="img"
      :aria-label="`图谱视图，共 ${renderNodes.total} 个节点`"
    >
      <line
        v-for="e in renderEdges.items"
        :key="'e:' + edgeKey(e)"
        :x1="points.get(e.from)?.x ?? 0"
        :y1="points.get(e.from)?.y ?? 0"
        :x2="points.get(e.to)?.x ?? 0"
        :y2="points.get(e.to)?.y ?? 0"
        class="edge"
        :class="{ active: store.selectedEdgeKey === edgeKey(e) }"
        tabindex="0"
        role="button"
        :aria-label="`${edgeKindLabel(e.kind)}边，从 ${e.from} 到 ${e.to}`"
        @click="onEdge(edgeKey(e))"
        @keydown.enter="onEdge(edgeKey(e))"
      />
      <g
        v-for="n in renderNodes.items"
        :key="n.id"
        :transform="`translate(${points.get(n.id)?.x ?? 0}, ${points.get(n.id)?.y ?? 0})`"
        class="node"
        :class="{ active: store.selectedNodeId === n.id }"
        tabindex="0"
        role="button"
        aria-roledescription="图谱节点"
        :aria-pressed="store.selectedNodeId === n.id"
        :aria-label="`${nodeKindLabel(n.kind)} 节点 ${n.label}`"
        @click="onNode(n.id)"
        @keydown.enter="onNode(n.id)"
      >
        <circle r="14" :fill="nodeColor(n.kind)" />
        <text x="0" y="28" text-anchor="middle" class="nlabel">{{ n.label }}</text>
      </g>
    </svg>
    <div v-else class="viewer-empty" role="status">暂无节点可绘制（后端数据未载入或已被过滤）。</div>
  </div>
</template>

<style scoped>
.viewer { width: 100%; height: 100%; min-height: 320px; }
.sr-only {
  position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px;
  overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;
}
.canvas { width: 100%; height: 100%; background: #fafafa; border-radius: 8px; }
.edge { stroke: #bbb; stroke-width: 1.5; cursor: pointer; }
.edge.active { stroke: #4096ff; stroke-width: 3; }
.node { cursor: pointer; }
.node.active circle { stroke: #4096ff; stroke-width: 3; }
.nlabel { font-size: 11px; fill: #444; }
.viewer-empty { padding: 24px; color: #999; text-align: center; }
.render-note { padding: 6px 10px; border-radius: 6px; margin: 0 8px 8px; font-size: 12px; background: #f9f0ff; color: #722ed1; text-align: center; }
</style>
