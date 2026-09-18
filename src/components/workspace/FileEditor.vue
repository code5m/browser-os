<script setup lang="ts">
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../stores/useBrowserStore";

const ws = useWorkspaceStore();
const layout = useLayoutStore();
const browser = useBrowserStore();

function closeEditor() {
  layout.setView("browser");
  browser.relocate();
}
</script>

<template>
  <div class="editor-pane">
    <div class="preview-header">
      <h4>📝 {{ ws.filePath?.split("/").pop() }}</h4>
      <div class="preview-actions">
        <template v-if="ws.mdPreview">
          <button :class="{ active: true }" @click="ws.mdPreview = true">👁 预览</button>
          <button @click="ws.mdPreview = false; ws.editingFile = true">✎ 源码</button>
        </template>
        <button class="primary" @click="ws.saveFile">💾 保存</button>
        <button @click="closeEditor">✕ 关闭</button>
      </div>
    </div>
    <div v-if="ws.mdPreview" class="md-preview" v-html="ws.mdHtml"></div>
    <textarea
      v-else
      v-model="ws.fileContent"
      class="file-editor"
      spellcheck="false"
      placeholder="文件内容..."
    ></textarea>
  </div>
</template>
