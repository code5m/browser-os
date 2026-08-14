<script setup lang="ts">
import { ref } from 'vue';
import { BrowserHost } from '@tauri-browser-tabs/vue';
import type { TabState } from '@tauri-browser-tabs/core';

const host = ref<InstanceType<typeof BrowserHost>>();
const urlInput = ref('https://example.com');

let counter = 0;

function createTab() {
  const id = `tab-${++counter}`;
  host.value?.createTab({
    id,
    url: urlInput.value,
  });
}

function onTabCreated(tab: TabState) {
  console.log('Tab created:', tab);
}

function onTabClosed(tab: TabState) {
  console.log('Tab closed:', tab);
}
</script>

<template>
  <div class="app">
    <header class="toolbar">
      <input
        v-model="urlInput"
        type="text"
        placeholder="Enter URL"
        class="url-input"
      />
      <button @click="createTab" class="btn">New Tab</button>
      <span class="status">{{ host?.tabs?.length ?? 0 }} tabs</span>
    </header>

    <main class="main">
      <BrowserHost
        ref="host"
        class="browser-host"
        @tab-created="onTabCreated"
        @tab-closed="onTabClosed"
      >
        <template #default="{ tabs, activeTabId, setActiveTab, closeTab }">
          <div class="tab-bar">
            <div
              v-for="tab in tabs"
              :key="tab.id"
              class="tab"
              :class="{ active: tab.id === activeTabId }"
              @click="setActiveTab(tab.id)"
            >
              <span class="tab-title">{{ tab.title || tab.url }}</span>
              <button class="tab-close" @click.stop="closeTab(tab.id)">
                ×
              </button>
            </div>
          </div>
        </template>
      </BrowserHost>
    </main>
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto,
    'Helvetica Neue', Arial, sans-serif;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1e1e1e;
  color: #fff;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #2d2d2d;
  border-bottom: 1px solid #404040;
  flex-shrink: 0;
}

.url-input {
  flex: 1;
  padding: 6px 12px;
  border: 1px solid #404040;
  border-radius: 4px;
  background: #1e1e1e;
  color: #fff;
  font-size: 14px;
}

.btn {
  padding: 6px 16px;
  border: none;
  border-radius: 4px;
  background: #0078d4;
  color: #fff;
  font-size: 14px;
  cursor: pointer;
}

.btn:hover {
  background: #106ebe;
}

.status {
  font-size: 12px;
  color: #888;
}

.main {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.browser-host {
  flex: 1;
  min-height: 0;
}

.tab-bar {
  display: flex;
  background: #2d2d2d;
  border-bottom: 1px solid #404040;
  overflow-x: auto;
  flex-shrink: 0;
}

.tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #2d2d2d;
  border-right: 1px solid #404040;
  cursor: pointer;
  font-size: 13px;
  max-width: 200px;
}

.tab:hover {
  background: #3d3d3d;
}

.tab.active {
  background: #1e1e1e;
  border-bottom: 2px solid #0078d4;
}

.tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: #888;
  font-size: 14px;
  cursor: pointer;
}

.tab-close:hover {
  background: #404040;
  color: #fff;
}
</style>
