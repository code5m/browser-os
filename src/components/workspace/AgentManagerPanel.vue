<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useAgentStore } from "../../stores/useAgentStore";
import { a2aSummary, dialectLabel, renderCapabilityList, panelState } from "../../utils/agentSkillUi";
import type { AgentDef } from "../../types";
import PermissionPreviewModal from "./PermissionPreviewModal.vue";
import AgentChatPanel from "./AgentChatPanel.vue";

const whitelist = computed(() => [] as string[]);
const store = useAgentStore();
const previewTarget = computed(() => store.selectedAgent);

const state = computed(() =>
  panelState("agents", {
    loading: store.loading,
    count: store.agents.length,
    error: store.error,
    backendReady: store.backendReady,
  }),
);

onMounted(() => {
  void store.loadAgents();
});

function caps(def: AgentDef) {
  return renderCapabilityList(def.defaultCapabilities, whitelist.value);
}
</script>

<template>
  <div class="panel agents-panel">
    <header class="panel-header">
      <h2>智能体</h2>
      <div class="actions">
        <button :disabled="!store.backendReady" @click="store.loadAgents()">刷新</button>
      </div>
    </header>

    <div v-if="!store.backendReady" class="banner banner-warn">
      后端 agent 命令尚未就绪，当前为只读壳。
    </div>
    <div v-if="store.error" class="banner banner-error">{{ store.error }}</div>

    <div v-if="state.state === 'loading'" class="loading-state">{{ state.message }}</div>
    <div v-else-if="state.state === 'empty'" class="empty-state">{{ state.message }}</div>
    <div v-else class="list">
      <div
        v-for="a in store.agents"
        :key="a.id"
        class="card"
        :class="{ active: store.selectedAgentId === a.id }"
        @click="store.selectAgent(a.id)"
      >
        <div class="card-title"><span>{{ a.displayName }}</span></div>
        <div class="card-sub">{{ a.id }} · {{ dialectLabel(a.dialect) }} · {{ a2aSummary(a.a2a) }}</div>
        <div class="chips">
          <span v-for="c in caps(a)" :key="c.id" class="chip" :class="{ granted: c.granted }">{{ c.label }}</span>
        </div>
      </div>
    </div>

    <AgentChatPanel v-if="store.selectedAgent" />

    <PermissionPreviewModal
      v-if="previewTarget"
      :def="previewTarget"
      :whitelist="whitelist"
      @close="store.selectAgent(previewTarget.id)"
    />
  </div>
</template>

<style scoped>
.banner { padding: 6px 10px; border-radius: 6px; margin: 8px; font-size: 12px; }
.banner-warn { background: #fff7e6; color: #ad6800; }
.banner-error { background: #fff1f0; color: #cf1322; }
.panel-header { display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; }
.list { padding: 0 12px 12px; display: flex; flex-direction: column; gap: 8px; }
.card { border: 1px solid #eee; border-radius: 8px; padding: 10px; cursor: pointer; }
.card.active { border-color: #4096ff; }
.card-title { font-weight: 600; }
.card-sub { color: #888; font-size: 12px; margin: 4px 0; }
.chips { display: flex; flex-wrap: wrap; gap: 4px; }
.chip { font-size: 11px; padding: 1px 6px; border-radius: 4px; background: #f0f0f0; color: #999; }
.chip.granted { background: #e6f4ff; color: #1677ff; }
.empty-state { padding: 24px; color: #999; text-align: center; }
.loading-state { padding: 24px; color: #888; text-align: center; }
.install-btn { margin-top: 6px; }
</style>
