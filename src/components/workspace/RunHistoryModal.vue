<script setup lang="ts">
import { EmptyState } from "../../shared/ui";
import { computed } from "vue";
import type { AgentRunRecord, SkillRunRecord } from "../../types";
import { runStatusUi } from "../../utils/agentSkillUi";

const props = defineProps<{
  records: (SkillRunRecord | AgentRunRecord)[] | null;
  title?: string;
}>();
const emit = defineEmits<{ (e: "close"): void }>();

const items = computed(() => props.records ?? []);
</script>

<template>
  <div class="modal-mask" @click.self="emit('close')">
    <div class="modal">
      <header>
        <h3>{{ title ?? "运行历史" }}</h3>
        <button class="close" @click="emit('close')">×</button>
      </header>
      <EmptyState v-if="items.length === 0" live text="暂无运行记录。" />
      <ul v-else class="records">
        <li v-for="r in items" :key="r.runId" class="rec">
          <span class="tag" :class="runStatusUi(r.status).cls">{{ runStatusUi(r.status).label }}</span>
          <span class="id">{{ r.runId }}</span>
          <span v-if="'skillId' in r" class="meta">skill {{ r.skillId }}</span>
          <span v-else class="meta">agent {{ r.agentId }}</span>
          <span v-if="r.error" class="err">{{ r.error.message }}</span>
        </li>
      </ul>
      <footer><button class="primary" @click="emit('close')">关闭</button></footer>
    </div>
  </div>
</template>

<style scoped>
.modal-mask { position: fixed; inset: 0; background: rgba(0,0,0,.35); display: flex; align-items: center; justify-content: center; z-index: 50; }
.modal { background: #fff; border-radius: 10px; width: 460px; max-width: 92vw; padding: 16px; box-shadow: 0 8px 30px rgba(0,0,0,.2); }
header { display: flex; justify-content: space-between; align-items: center; }
.close { border: none; background: transparent; font-size: 18px; cursor: pointer; }
.empty { color: #999; padding: 16px; text-align: center; }
.records { list-style: none; margin: 8px 0; padding: 0; max-height: 320px; overflow: auto; }
.rec { display: flex; gap: 8px; align-items: center; padding: 6px 0; border-bottom: 1px solid #f5f5f5; font-size: 13px; }
.id { font-family: monospace; color: #555; }
.meta { color: #888; }
.err { color: #cf1322; }
.tag { font-size: 11px; padding: 1px 6px; border-radius: 4px; }
.tag.succeeded, .tag.running, .tag.failed, .tag.cancelled, .tag.timeout { background: #f0f0f0; color: #666; }
footer { text-align: right; margin-top: 12px; }
.primary { background: #1677ff; color: #fff; border: none; padding: 6px 14px; border-radius: 6px; cursor: pointer; }
</style>
