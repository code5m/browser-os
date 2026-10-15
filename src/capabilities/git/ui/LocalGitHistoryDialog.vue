<script setup lang="ts">
import { useGitStore } from "../public";
import GitLocalInspector from "./GitLocalInspector.vue";

const git = useGitStore();
</script>

<template>
  <div v-if="git.localHistoryPath" class="modal-mask" @click.self="git.closeLocalHistory()" @keydown.esc.stop="git.closeLocalHistory()">
    <section class="local-git-history" role="dialog" aria-modal="true" aria-label="本地 Git 文件和目录历史">
      <GitLocalInspector :path="git.localHistoryPath" mode="history" close-label="关闭"
        :initial-action="git.localActionIntent"
        search-id="local-git-dialog-search" @close="git.closeLocalHistory()" />
    </section>
  </div>
</template>

<style scoped>
.local-git-history{width:min(1060px,92vw);height:min(750px,85vh);min-height:300px;overflow:hidden;border-radius:12px;box-shadow:0 16px 60px #0004;background:#fff}
</style>
