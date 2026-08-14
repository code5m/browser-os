<script setup lang="ts">
import { useSystemStore } from "../../stores/useSystemStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { convertFileSrc } from "@tauri-apps/api/core";

const system = useSystemStore();
const layout = useLayoutStore();
</script>

<template>
  <div class="side-inner">
    <div class="tabs">
      <span>🚀 系统应用</span>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>
    <div class="apps-panel">
      <input v-model="system.appFilter" class="app-filter" placeholder="搜索应用" />
      <div class="app-grid">
        <div
          v-for="a in system.filteredApps"
          :key="a.exec"
          class="app-cell"
          :title="a.name"
          @click="system.launchApp(a)"
        >
          <img
            v-if="a.icon_path && !system.brokenIcons.has(a.exec)"
            class="app-img"
            :src="convertFileSrc(a.icon_path)"
            @error="() => system.onAppImgError(a.exec)"
          />
          <span v-else class="app-emoji">{{ a.icon ? a.icon.slice(0, 2) : "▣" }}</span>
          <span class="app-name">{{ a.name }}</span>
        </div>
        <div v-if="!system.filteredApps.length" class="empty full">未找到应用</div>
      </div>
      <button class="block" @click="system.loadApps">↻ 刷新应用列表</button>
    </div>
  </div>
</template>
