<script setup lang="ts">
import { ref } from "vue";
import { useRepoStore } from "../../stores/useRepoStore";
import { useModalFocus } from "../../composables/useModalFocus";
const rp = useRepoStore();
const root = ref<HTMLElement | null>(null);
// 确认类对话框：打开时焦点移入、焦点陷阱、关闭时归还焦点、ESC 关闭（M5-W15 A6）
useModalFocus(root, { variant: "confirm", onEscape: () => (rp.clearPreview()) });
</script>

<template>
  <div v-if="rp.preview" ref="root" class="modal-mask" @click.self="rp.clearPreview()">
    <div class="modal">
      <h3>确认推送到「{{ rp.preview.repo_name }}」？</h3>
      <p class="mono">{{ rp.preview.remote_url }}</p>
      <p>将同步 {{ rp.preview.artifact_count }} 个成果：</p>
      <ul>
        <li v-for="t in rp.preview.artifact_titles" :key="t">· {{ t }}</li>
      </ul>
      <div v-if="rp.busy" class="progress" role="status" aria-live="polite">推送中…</div>
      <div class="actions">
        <button :disabled="rp.busy" @click="rp.clearPreview()">取消</button>
        <button class="primary" :disabled="rp.busy" @click="rp.confirmSync">确认推送</button>
      </div>
    </div>
  </div>
</template>
