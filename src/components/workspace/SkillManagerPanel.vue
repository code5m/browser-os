<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useAgentStore } from "../../stores/useAgentStore";
import { aclLabel, aclTone, execSummary, renderCapabilityList, panelState } from "../../utils/agentSkillUi";
import type { SkillDef } from "../../types";
import PermissionPreviewModal from "./PermissionPreviewModal.vue";

// 后端能力单一真源（SKILL_CAPABILITY_V1）W4 尚未落齐 → 传空白名单，
// 列表统一标 unknown（不谎报已授权）。
const whitelist = computed(() => [] as string[]);

const store = useAgentStore();
const previewTarget = computed(() => store.selectedSkill);

const state = computed(() =>
  panelState("skills", {
    loading: store.loading,
    count: store.skills.length,
    error: store.error,
    backendReady: store.backendReady,
  }),
);

onMounted(() => {
  void store.loadSkills();
});

function caps(def: SkillDef) {
  return renderCapabilityList(def.capabilities, whitelist.value);
}
</script>

<template>
  <div class="panel skills-panel">
    <header class="panel-header">
      <h2>技能</h2>
      <div class="actions">
        <button :disabled="!store.backendReady" @click="store.loadSkills()">刷新</button>
      </div>
    </header>

    <div v-if="!store.backendReady" class="banner banner-warn">
      后端 skill 命令尚未就绪，当前为只读壳。
    </div>
    <div v-if="store.error" class="banner banner-error">{{ store.error }}</div>

    <div v-if="state.state === 'loading'" class="loading-state">{{ state.message }}</div>
    <div v-else-if="state.state === 'empty'" class="empty-state">{{ state.message }}</div>
    <div v-else class="list">
      <div
        v-for="s in store.skills"
        :key="s.id"
        class="card"
        :class="{ active: store.selectedSkillId === s.id }"
        @click="store.selectSkill(s.id)"
      >
        <div class="card-title">
          <span>{{ s.displayName }}</span>
          <span class="tag" :class="aclTone(s.acl)">{{ aclLabel(s.acl) }}</span>
        </div>
        <div class="card-sub">{{ s.id }} · {{ execSummary(s.exec) }}</div>
        <div class="chips">
          <span v-for="c in caps(s)" :key="c.id" class="chip" :class="{ granted: c.granted }">{{ c.label }}</span>
        </div>
      </div>
    </div>

    <PermissionPreviewModal
      v-if="previewTarget"
      :def="previewTarget"
      :whitelist="whitelist"
      @close="store.selectSkill(previewTarget.id)"
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
.card-title { display: flex; justify-content: space-between; align-items: center; font-weight: 600; }
.card-sub { color: #888; font-size: 12px; margin: 4px 0; }
.chips { display: flex; flex-wrap: wrap; gap: 4px; }
.chip { font-size: 11px; padding: 1px 6px; border-radius: 4px; background: #f0f0f0; color: #999; }
.chip.granted { background: #e6f4ff; color: #1677ff; }
.tag { font-size: 11px; padding: 1px 6px; border-radius: 4px; }
.tag.safe { background: #e6ffed; color: #389e0d; }
.tag.warn { background: #fff7e6; color: #ad6800; }
.tag.danger { background: #fff1f0; color: #cf1322; }
.empty-state { padding: 24px; color: #999; text-align: center; }
.loading-state { padding: 24px; color: #888; text-align: center; }
.install-btn { margin-top: 6px; }
</style>
