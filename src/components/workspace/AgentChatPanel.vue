<script setup lang="ts">
import { computed } from "vue";
import { useAgentStore } from "../../stores/useAgentStore";
import { chunksToText } from "../../utils/agentSkillUi";

// 对话壳：仅渲染既有会话（只读视图）。执行通道（agent_chat / runAgent）在 W10 尚未开放，
// 此处不提供发送/输入控件（W10 A6：no execution buttons）。
const store = useAgentStore();

const agentId = computed(() => store.selectedAgentId ?? store.agents[0]?.id ?? null);
const current = computed(() => {
  const ids = Object.keys(store.sessions);
  if (!ids.length) return null;
  return store.sessions[ids[ids.length - 1]];
});
const text = computed(() => (current.value ? chunksToText(current.value.chunks) : ""));
</script>

<template>
  <section class="chat-shell">
    <header class="chat-head">
      <span>对话</span>
      <span v-if="agentId" class="muted">{{ agentId }}</span>
    </header>
    <div class="chat-body">
      <div v-if="!agentId" class="empty-state">选择或安装一个 Agent 后开始对话。</div>
      <div v-else-if="!current" class="empty-state">输入消息开始。</div>
      <pre v-else class="stream">{{ text || "（等待响应…）" }}</pre>
    </div>
    <div v-if="!store.backendReady" class="muted small">后端 agent_chat 命令未就绪，对话为只读视图。</div>
  </section>
</template>

<style scoped>
.chat-shell { border-top: 1px solid #eee; margin-top: 12px; padding-top: 8px; }
.chat-head { display: flex; justify-content: space-between; font-weight: 600; }
.chat-body { min-height: 80px; border: 1px solid #f0f0f0; border-radius: 8px; padding: 8px; margin: 6px 0; background: #fafafa; }
.stream { white-space: pre-wrap; word-break: break-word; margin: 0; font-family: inherit; font-size: 13px; }
.chat-foot { display: flex; gap: 6px; }
.chat-foot input { flex: 1; padding: 6px 8px; border: 1px solid #ddd; border-radius: 6px; }
.empty-state { color: #999; font-size: 13px; }
.muted { color: #999; font-size: 12px; }
.small { font-size: 11px; margin-top: 4px; }
</style>
