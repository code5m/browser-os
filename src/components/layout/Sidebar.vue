<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import FilePanel from "../workspace/FilePanel.vue";
import ArtifactPanel from "../workspace/ArtifactPanel.vue";
import ClipboardPanel from "../system/ClipboardPanel.vue";
import AuditPanel from "../workspace/AuditPanel.vue";
import RepoPanel from "../workspace/RepoPanel.vue";
import AppPanel from "../system/AppPanel.vue";

const layout = useLayoutStore();
</script>

<template>
  <aside v-if="layout.sidebarOpen && (layout.mainView === 'files' || layout.mainView === 'clip' || layout.mainView === 'arts' || layout.mainView === 'repo' || layout.mainView === 'apps' || layout.mainView === 'audit')" class="sidebar" :style="{ width: layout.sidebarWidth + 'px' }">
    <!-- 文件 / 成果（共用一个带 Tab 的面板） -->
    <div v-if="layout.mainView === 'files' || layout.mainView === 'arts'" class="side-inner">
      <div class="tabs">
        <button :class="{ active: layout.leftTab === 'files' }" @click="layout.leftTab = 'files'">📂 文件</button>
        <button :class="{ active: layout.leftTab === 'artifacts' }" @click="layout.leftTab = 'artifacts'">📦 成果</button>
        <button class="close" @click="layout.sidebarOpen = false" title="收起">✕</button>
      </div>
      <FilePanel v-if="layout.leftTab === 'files'" />
      <ArtifactPanel v-else />
    </div>

    <!-- 剪贴板 -->
    <ClipboardPanel v-else-if="layout.mainView === 'clip'" />

    <!-- 自有仓库 -->
    <RepoPanel v-else-if="layout.mainView === 'repo'" />

    <!-- 系统应用 -->
    <AppPanel v-else-if="layout.mainView === 'apps'" />

    <!-- 审计日志 -->
    <AuditPanel v-else-if="layout.mainView === 'audit'" />
  </aside>
</template>
