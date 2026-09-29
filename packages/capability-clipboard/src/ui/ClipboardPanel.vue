<template>
  <div class="clipboard-panel">
    <div class="clip-head">
      <span class="clip-title">剪贴板</span>
      <button class="close" @click="ui.requestClose()" title="关闭">×</button>
    </div>

    <div class="clip-actions">
      <button @click="clipboard.clipPaste()">粘贴</button>
      <button @click="clipboard.clipCopy()">复制</button>
      <button @click="clipboard.clearClipHistory()">清空历史</button>
    </div>

    <textarea
      class="clip-text"
      v-model="clipboard.clipText"
      placeholder="当前剪贴板内容（仅本次会话保留，关闭应用后清空，不写入磁盘）"
      rows="4"
    ></textarea>

    <div class="clip-history" v-if="clipboard.clipHistory.length">
      <div class="clip-history-head">
        <span>历史记录（仅本次会话，不落盘）</span>
      </div>
      <ul>
        <li v-for="item in clipboard.clipHistory" :key="item.id">
          <span class="clip-item-text" :title="redactSecrets(item.text)">{{ redactSecrets(item.text) }}</span>
          <button @click="clipboard.useClipItem(item)">使用</button>
          <button @click="clipboard.copyClipItem(item)">复制</button>
        </li>
      </ul>
    </div>
    <div v-else class="clip-empty">暂无历史记录，复制内容后会自动收集</div>
  </div>
</template>

<script setup lang="ts">
import { inject } from "vue";
import { useClipboardStore } from "../state/useClipboardStore";
import { CLIPBOARD_PORTS_KEY, assertClipboardPorts } from "../ports";

// 包不直连 Host 的 useLayoutStore / shared/ui / utils/redact：端口经 Host 注入。
const clipboard = useClipboardStore();
const ports = inject(CLIPBOARD_PORTS_KEY);
assertClipboardPorts(ports);
const ui = ports.ui;
const redactSecrets = ui.redactSecrets;
</script>

<style scoped>
.clipboard-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 4px 12px 12px;
  height: 100%;
  overflow: auto;
  background: var(--panel-bg, #fff);
  color: var(--panel-fg, #222);
}
.clip-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.clip-title {
  font-weight: 600;
}
.close {
  border: none;
  background: transparent;
  font-size: 18px;
  cursor: pointer;
  line-height: 1;
}
.clip-actions {
  display: flex;
  gap: 8px;
}
.clip-actions button {
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border, #ccc);
  background: var(--btn-bg, #f3f3f3);
  cursor: pointer;
}
.clip-text {
  width: 100%;
  resize: vertical;
}
.clip-history-head {
  font-size: 12px;
  opacity: 0.7;
  margin: 4px 0;
}
.clip-history ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.clip-history li {
  display: flex;
  align-items: center;
  gap: 8px;
}
.clip-item-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
