<script setup lang="ts">
import { computed } from "vue";
import { useGraphStore } from "../state/useGraphStore";
import { summarizeEdge } from "../../../utils/graphUi";
import type { GraphEdge } from "../../../types";

const props = defineProps<{ edge: GraphEdge | null }>();
const summary = computed(() => (props.edge ? summarizeEdge(props.edge) : null));
</script>

<template>
  <div class="detail" v-if="summary" role="region" aria-label="边详情" aria-live="polite">
    <h3>边详情</h3>
    <dl>
      <dt>从</dt><dd class="mono">{{ summary.from }}</dd>
      <dt>到</dt><dd class="mono">{{ summary.to }}</dd>
      <dt>类型</dt><dd>{{ summary.kindLabel }}</dd>
      <dt>权重</dt><dd>{{ summary.weight }}</dd>
      <dt>消费边</dt><dd>{{ summary.agentConsumption ? "是（Agent 消费关系）" : "否" }}</dd>
    </dl>
    <p class="note">属性(props)已脱敏，界面不展示。</p>
  </div>
  <div class="detail empty" v-else role="status">选择一条边查看详情</div>
</template>

<style scoped>
.detail { padding: 12px; }
.detail.empty { color: #999; }
.mono { font-family: monospace; word-break: break-all; }
.note { font-size: 11px; color: #999; margin-top: 8px; }
dl { display: grid; grid-template-columns: 64px 1fr; gap: 4px 8px; margin: 0; }
dt { color: #888; }
</style>
