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
  <!-- 全局醒目 toast：固定在工具栏下方右上（此区域永不被宫格/页签原生窗口覆盖），
       之前只在状态栏角落显示 11px 小字 2 秒即消失，用户完全看不到 -->
  <div v-if="layout.msg" class="toast-pop">{{ layout.msg }}</div>
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
.toast-pop {
  position: fixed;
  top: 46px;
  right: 14px;
  z-index: 99999;
  background: rgba(15, 23, 42, 0.92);
  color: #fff;
  font-size: 14px;
  padding: 10px 18px;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  max-width: 60vw;
  pointer-events: none;
}
</style>
