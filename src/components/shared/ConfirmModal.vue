<script setup lang="ts">
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
const ws = useWorkspaceStore();
</script>

<template>
  <div v-if="ws.preview" class="modal-mask" @click.self="ws.preview = null">
    <div class="modal">
      <h3>确认推送到「{{ ws.preview.repo_name }}」？</h3>
      <p class="mono">{{ ws.preview.remote_url }}</p>
      <p>将同步 {{ ws.preview.artifact_count }} 个成果：</p>
      <ul>
        <li v-for="t in ws.preview.artifact_titles" :key="t">· {{ t }}</li>
      </ul>
      <div v-if="ws.busy" class="progress">推送中…</div>
      <div class="actions">
        <button :disabled="ws.busy" @click="ws.preview = null">取消</button>
        <button class="primary" :disabled="ws.busy" @click="ws.confirmSync">确认推送</button>
      </div>
    </div>
  </div>
</template>
