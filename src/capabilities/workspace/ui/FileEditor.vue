<script setup lang="ts">
import { ref, watch } from "vue";
import { documentPathName } from "../../../application/documentPaths.mjs";
import { useFileStore } from "../state/useFileStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { relocateBrowser } from "../../../composables/browserNav";

const ws = useFileStore();
const layout = useLayoutStore();
const recoveryTarget = ref("");
watch(() => ws.editorRecoveryId, () => { recoveryTarget.value = ""; });

function closeEditor() {
  if (layout.setView("browser")) relocateBrowser();
}
</script>

<template>
  <div class="editor-pane">
    <div class="preview-header">
      <h4>📝 {{ ws.editorFilePath ? documentPathName(ws.editorFilePath) : "" }}</h4>
      <div class="preview-actions">
        <template v-if="ws.mdPreview">
          <button :class="{ active: true }" @click="ws.setEditorView(true)">👁 预览</button>
          <button @click="ws.setEditorView(false)">✎ 源码</button>
        </template>
        <button v-if="!ws.editorRecoveryId" class="primary" @click="ws.saveFile">💾 保存</button>
        <button @click="closeEditor">✕ 关闭</button>
      </div>
    </div>
    <section v-if="ws.editorRecoveryId" class="recovery-save-as" aria-label="恢复草稿另存">
      <p>正在编辑独立的加密恢复副本。原记录继续保留；另存不会覆盖已有文件。</p>
      <form @submit.prevent="ws.saveRecoveredDraftAs(recoveryTarget)">
        <label for="recovery-target">新文件绝对路径</label>
        <input id="recovery-target" v-model="recoveryTarget" autocomplete="off" spellcheck="false" placeholder="/项目/恢复文件.txt 或 C:\项目\恢复文件.txt" :disabled="ws.recoverySaving">
        <button class="primary" type="submit" :disabled="ws.recoverySaving || !recoveryTarget.trim()">{{ ws.recoverySaving ? "正在另存…" : "另存为新文件" }}</button>
      </form>
    </section>
    <div v-if="ws.mdPreview" class="md-preview" v-html="ws.mdHtml"></div>
    <textarea
      v-else
      :value="ws.fileContent" @input="ws.setFileContent(($event.target as HTMLTextAreaElement).value)"
      class="file-editor"
      spellcheck="false"
      placeholder="文件内容..."
    ></textarea>
  </div>
</template>

<style scoped>
.recovery-save-as { flex: 0 0 auto; padding: 10px 14px; border-bottom: 1px solid #e5e6eb; }
.recovery-save-as p { margin: 0 0 8px; font-size: 12px; }
.recovery-save-as form { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.recovery-save-as input { flex: 1; min-width: 180px; padding: 6px; }
</style>
