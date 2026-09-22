<script setup lang="ts">
import { EmptyState } from "../../shared/ui";
import { useSystemStore } from "../../stores/useSystemStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { redactSecrets } from "../../utils/redact";
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
      <div class="clip-tip">已保存 {{ system.clipHistory.length }} 条历史（仅本次会话保留，关闭应用后清空，不写入磁盘）</div>
      <div class="clip-history">
        <div
          v-for="(item, idx) in system.clipHistory.slice(0, 30)"
          :key="item.at + '-' + idx"
          class="clip-item"
          :title="redactSecrets(item.text)"
          @click="system.useClipItem(item)"
        >
          <span class="clip-text">{{ redactSecrets(item.text) }}</span>
          <button class="clip-copy" @click.stop="system.copyClipItem(item)">📋</button>
        </div>
        <EmptyState v-if="!system.clipHistory.length" text="暂无历史记录，复制内容后会自动收集" />
      </div>
    </div>
  </div>
</template>
