<script setup lang="ts">
import { computed } from "vue";
import { useGraphStore } from "../../stores/useGraphStore";
import {
  edgeKindLabel,
  layoutPositions,
  nodeColor,
  nodeKindLabel,
} from "../../utils/graphUi";

const store = useGraphStore();

const points = computed(() => {
  const map = new Map<string, { x: number; y: number }>();
  for (const p of layoutPositions(store.visibleNodes, store.visibleEdges)) {
    map.set(p.id, { x: p.x, y: p.y });
  }
  return map;
});

function onNode(id: string): void {
  store.selectNode(id);
}
function onEdge(idx: number): void {
  store.selectEdge(idx);
}
</script>

<template>
  <div class="viewer">
    <p class="sr-only" aria-live="polite">{{ store.selectionText }}</p>
    <svg
      v-if="store.visibleNodes.length"
      viewBox="0 0 800 600"
      class="canvas"
      role="img"
      :aria-label="`图谱视图，共 ${store.visibleNodes.length} 个节点`"
    >
      <line
        v-for="(e, i) in store.visibleEdges"
        :key="'e' + i"
        :x1="points.get(e.from)?.x ?? 0"
        :y1="points.get(e.from)?.y ?? 0"
        :x2="points.get(e.to)?.x ?? 0"
        :y2="points.get(e.to)?.y ?? 0"
        class="edge"
        :class="{ active: store.selectedEdgeIdx === i }"
        tabindex="0"
        role="button"
        :aria-label="`${edgeKindLabel(e.kind)}边，从 ${e.from} 到 ${e.to}`"
        @click="onEdge(i)"
        @keydown.enter="onEdge(i)"
      />
      <g
        v-for="n in store.visibleNodes"
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
</style>
