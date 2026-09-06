<script setup lang="ts">
import { computed } from "vue";
import { useGraphStore } from "../../stores/useGraphStore";
import { GRAPH_NODE_KINDS, nodeKindLabel } from "../../utils/graphUi";

const store = useGraphStore();
const kindOptions = GRAPH_NODE_KINDS.map((k) => ({ value: k, label: nodeKindLabel(k) }));
const activeKinds = computed(() => store.filter.kinds);

function onSearch(e: Event): void {
  const v = (e.target as HTMLInputElement).value;
  store.setFilter({ query: v });
}
</script>

<template>
  <div class="filter">
    <input
      class="search"
      type="search"
      placeholder="搜索节点标签 / ID / 类型…"
      :value="store.filter.query"
      @input="onSearch"
      aria-label="搜索图谱节点"
    />
    <div class="chips">
      <button
        v-for="opt in kindOptions"
        :key="opt.value"
        class="chip"
        :class="{ on: activeKinds.includes(opt.value) }"
        :aria-pressed="activeKinds.includes(opt.value)"
        @click="store.toggleKind(opt.value)"
      >{{ opt.label }}</button>
    </div>
    <button class="clear" @click="store.clearFilter()">清除</button>
  </div>
</template>

<style scoped>
.filter { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; padding: 8px 12px; }
.search { flex: 1 1 200px; padding: 6px 8px; border: 1px solid #ddd; border-radius: 6px; }
.chips { display: flex; flex-wrap: wrap; gap: 4px; }
.chip { font-size: 12px; padding: 3px 8px; border: 1px solid #ddd; border-radius: 12px; background: #f5f5f5; cursor: pointer; }
.chip.on { background: #e6f4ff; border-color: #4096ff; color: #1677ff; }
.clear { font-size: 12px; padding: 3px 8px; border: none; background: transparent; color: #888; cursor: pointer; }
</style>
