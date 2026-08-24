<script setup lang="ts">
import { useSettingsStore } from "../../stores/useSettingsStore";

const settings = useSettingsStore();
</script>

<template>
  <div class="settings-panel">
    <div class="settings-section">
      <div class="section-title">外观</div>
      <div class="setting-item">
        <label>主题</label>
        <select v-model="settings.theme">
          <option value="light">浅色</option>
          <option value="dark">深色</option>
        </select>
      </div>
    </div>

    <div class="settings-section">
      <div class="section-title">快捷键方案</div>
      <div class="setting-item">
        <label>键盘映射</label>
        <select v-model="settings.keymapScheme">
          <option value="vscode">VSCode</option>
          <option value="idea">IntelliJ IDEA</option>
          <option value="eclipse">Eclipse</option>
        </select>
      </div>
      <div class="keymap-preview">
        <div v-for="(keys, action) in settings.currentKeymap" :key="action" class="keymap-row">
          <span class="keymap-action">{{ settings.actionLabels[action] || action }}</span>
          <span class="keymap-keys">{{ keys }}</span>
        </div>
      </div>
    </div>

    <div class="settings-section">
      <div class="section-title">性能</div>
      <div class="setting-item">
        <label>页签休眠</label>
        <input
          type="checkbox"
          :checked="settings.tabHibernation"
          @change="settings.setTabHibernation(($event.target as HTMLInputElement).checked)"
        />
        <span class="setting-desc">
          默认关闭。开启后，非激活超过 10 分钟的页签会销毁 webview 仅留网址（每个约省 300MB 内存），
          重新点击该页签时按网址重建（滚动位置/表单不保留，网站登录态保留）
        </span>
      </div>
    </div>

    <div class="settings-section">
      <div class="section-title">关于</div>
      <div class="setting-item">
        <label>版本</label>
        <span>v0.9.10</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-panel {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
  background: #fff;
}
.settings-section {
  margin-bottom: 24px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #2b3a55;
  margin-bottom: 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid #e5e6eb;
}
.setting-item {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
  font-size: 13px;
}
.setting-item label {
  width: 80px;
  color: #4e5969;
  flex-shrink: 0;
}
.setting-desc {
  font-size: 12px;
  color: #86909c;
  line-height: 1.5;
}
.setting-item select {
  height: 28px;
  border: 1px solid #e5e6eb;
  border-radius: 5px;
  padding: 0 8px;
  font-size: 13px;
  outline: none;
}
.keymap-preview {
  margin-top: 12px;
  border: 1px solid #e5e6eb;
  border-radius: 6px;
  overflow: hidden;
}
.keymap-row {
  display: flex;
  justify-content: space-between;
  padding: 8px 12px;
  font-size: 12px;
  border-bottom: 1px solid #f0f0f0;
}
.keymap-row:last-child {
  border-bottom: none;
}
.keymap-row:nth-child(even) {
  background: #fafbfc;
}
.keymap-action {
  color: #4e5969;
}
.keymap-keys {
  color: #2b6cb0;
  font-family: monospace;
  font-weight: 600;
}
</style>
