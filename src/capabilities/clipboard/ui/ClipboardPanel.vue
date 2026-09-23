<script setup lang="ts">
import { EmptyState } from "../../../shared/ui";
import { useClipboardStore } from "../state/useClipboardStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { redactSecrets } from "../../../utils/redact";
const clipboard = useClipboardStore();
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
        <button @click="clipboard.clipPaste">📥 刷新/粘贴</button>
        <button @click="clipboard.clipCopy">📤 复制当前</button>
        <button class="ghost" @click="clipboard.clearClipHistory">🗑 清空历史</button>
      </div>
      <textarea
        v-model="clipboard.clipText"
        class="clip-area"
        placeholder="在此编辑文本后点「复制当前」写入系统剪贴板；切回本应用或打开面板时自动同步"
      ></textarea>
      <div class="clip-tip">已保存 {{ clipboard.clipHistory.length }} 条历史（仅本次会话保留，关闭应用后清空，不写入磁盘）</div>
      <div class="clip-history">
        <div
          v-for="(item, idx) in clipboard.clipHistory.slice(0, 30)"
          :key="item.at + '-' + idx"
          class="clip-item"
          :title="redactSecrets(item.text)"
          @click="clipboard.useClipItem(item)"
        >
          <span class="clip-text">{{ redactSecrets(item.text) }}</span>
          <button class="clip-copy" @click.stop="clipboard.copyClipItem(item)">📋</button>
        </div>
        <EmptyState v-if="!clipboard.clipHistory.length" text="暂无历史记录，复制内容后会自动收集" />
      </div>
    </div>
  </div>
</template>
