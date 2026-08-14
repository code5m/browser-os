<script setup lang="ts">
import { computed } from "vue";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useSystemStore } from "../../stores/useSystemStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useLayoutStore } from "../../stores/useLayoutStore";

const browser = useBrowserStore();
const system = useSystemStore();
const ws = useWorkspaceStore();
const layout = useLayoutStore();

const tabCount = computed(() => browser.tabs.length);
const auditCount = computed(() => ws.audit.length);
const termReady = computed(() => system.terminalOpen && !!system.termId);
const repoReady = computed(() => ws.repos.length > 0);
</script>

<template>
  <footer class="status">
    <span class="ok">● 已连接</span>
    <span>页签 {{ tabCount }}</span>
    <span>· 终端{{ termReady ? "就绪" : "未启" }}</span>
    <span>· 仓库{{ repoReady ? "已配置" : "未配" }}</span>
    <span>· 审计 {{ auditCount }} 条</span>
    <span v-if="layout.msg" class="msg">{{ layout.msg }}</span>
  </footer>
</template>

<style scoped>
.status {
  height: 26px;
  background: #1f2733;
  color: #9aa4b2;
  font-size: 11px;
  display: flex;
  align-items: center;
  padding: 0 12px;
  gap: 16px;
  flex-shrink: 0;
  user-select: none;
}
.status .ok {
  color: #52c41a;
}
.status .msg {
  margin-left: auto;
  color: #cbd5e0;
}
</style>
