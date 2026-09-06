<script setup lang="ts">
import { computed } from "vue";
import type { AgentDef, SkillDef } from "../../types";
import { buildPermissionPreview } from "../../utils/agentSkillUi";

const props = defineProps<{
  def: SkillDef | AgentDef | null;
  whitelist?: string[];
}>();
const emit = defineEmits<{ (e: "close"): void }>();

const view = computed(() =>
  props.def ? buildPermissionPreview(props.def, props.whitelist ?? []) : null,
);
</script>

<template>
  <div v-if="view" class="modal-mask" @click.self="emit('close')">
    <div class="modal">
      <header>
        <h3>权限预览</h3>
        <button class="close" @click="emit('close')">×</button>
      </header>
      <div class="row"><span class="label">名称</span><span>{{ def!.displayName }} <small>({{ def!.id }})</small></span></div>
      <div class="row">
        <span class="label">闸门</span>
        <span class="tag" :class="view.gateTone">{{ view.gateLabel }}</span>
      </div>
      <div class="row">
        <span class="label">目标</span><span>{{ view.targetSummary }}</span>
      </div>
      <div class="row">
        <span class="label">能力</span>
        <span class="caps">
          <span v-for="c in view.capabilities" :key="c.id" class="chip" :class="{ granted: c.granted, unknown: c.unknown }">{{ c.label }}</span>
        </span>
      </div>
      <p v-if="view.dangerHint" class="danger-hint">{{ view.dangerHint }}</p>
      <footer><button class="primary" @click="emit('close')">我已知晓</button></footer>
    </div>
  </div>
</template>

<style scoped>
.modal-mask { position: fixed; inset: 0; background: rgba(0,0,0,.35); display: flex; align-items: center; justify-content: center; z-index: 50; }
.modal { background: #fff; border-radius: 10px; width: 420px; max-width: 92vw; padding: 16px; box-shadow: 0 8px 30px rgba(0,0,0,.2); }
header { display: flex; justify-content: space-between; align-items: center; }
.close { border: none; background: transparent; font-size: 18px; cursor: pointer; }
.row { display: flex; gap: 12px; margin: 8px 0; align-items: flex-start; }
.label { width: 48px; color: #888; font-size: 13px; }
.caps { display: flex; flex-wrap: wrap; gap: 4px; }
.chip { font-size: 11px; padding: 1px 6px; border-radius: 4px; background: #f0f0f0; color: #999; }
.chip.granted { background: #e6f4ff; color: #1677ff; }
.chip.unknown { background: #fff7e6; color: #ad6800; }
.tag { font-size: 11px; padding: 1px 6px; border-radius: 4px; }
.tag.safe { background: #e6ffed; color: #389e0d; }
.tag.warn { background: #fff7e6; color: #ad6800; }
.tag.danger { background: #fff1f0; color: #cf1322; }
.danger-hint { background: #fff7e6; color: #ad6800; padding: 8px; border-radius: 6px; font-size: 12px; }
footer { text-align: right; margin-top: 12px; }
.primary { background: #1677ff; color: #fff; border: none; padding: 6px 14px; border-radius: 6px; cursor: pointer; }
</style>
