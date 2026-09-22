<script setup lang="ts">
import { EmptyState } from "../../../shared/ui";
import { useWorkspaceStore } from "../state/useWorkspaceStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
const ws = useWorkspaceStore();
const layout = useLayoutStore();
</script>

<template>
  <div class="side-inner">
    <div class="tabs">
      <span>📜 审计日志</span>
      <button class="close" @click="layout.sidebarOpen = false" title="收起">✕</button>
    </div>
    <ul class="audit-list">
      <li v-for="(e, i) in ws.audit.slice().reverse()" :key="i">
        <div class="audit-row">
          <span class="at">{{ e.at.slice(0, 19) }}</span>
          <span class="act">{{ e.action }}</span>
        </div>
        <div class="det">{{ e.detail }}</div>
      </li>
      <EmptyState v-if="!ws.audit.length" as="li" live text="暂无记录" />
    </ul>
  </div>
</template>
