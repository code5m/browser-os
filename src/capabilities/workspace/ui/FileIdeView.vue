<script setup lang="ts">
// files 主视图（IDE 布局）：FilePanel 的 :ide=true 变体，作为能力贡献提供给 Shell。
// Shell 只按 view="files" 渲染本组件，不 import FilePanel 内部。
import FilePanel from "./FilePanel.vue";
</script>

<template>
  <FilePanel :ide="true" />
</template>
