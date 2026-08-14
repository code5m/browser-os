<script setup lang="ts">
import { useSystemStore } from "../../stores/useSystemStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
const system = useSystemStore();
const layout = useLayoutStore();
</script>

<template>
  <div class="side-inner">
    <div class="tabs">
      <span>📋 剪贴板</span>
      <button class="close" @click="layout.clipOpen = false" title="收起">✕</button>
    </div>
    <div class="clip-panel">
      <div class="clip-actions">
        <button @click="system.clipPaste">📥 刷新/粘贴</button>
        <button @click="system.clipCopy">📤 复制当前</button>
        <button class="ghost" @click="system.clearClipHistory">🗑 清空历史</button>
      </div>
      <textarea
        v-model="system.clipText"
        class="clip-area"
        placeholder="在此编辑文本后点「复制当前」写入系统剪贴板；切回本应用或打开面板时自动同步"
      ></textarea>
      <div class="clip-tip">切回本应用或打开面板时自动读取系统剪贴板，已保存 {{ system.clipHistory.length }} 条历史</div>
      <div class="clip-history">
        <div
          v-for="(item, idx) in system.clipHistory.slice(0, 30)"
          :key="item.at + '-' + idx"
          class="clip-item"
          :title="item.text"
          @click="system.useClipItem(item)"
        >
          <span class="clip-text">{{ item.text }}</span>
          <button class="clip-copy" @click.stop="system.copyClipItem(item)">📋</button>
        </div>
        <div v-if="!system.clipHistory.length" class="empty">暂无历史记录，复制内容后会自动收集</div>
      </div>
    </div>
  </div>
</template>
