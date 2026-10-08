<script setup lang="ts">
import { defineAsyncComponent, onMounted, ref } from "vue";
const EngineeringHealthPanel = defineAsyncComponent(() => import("./EngineeringHealthPanel.vue"));
import { bridge } from "../../bridge";
import { useSettingsStore } from "../../settings/public";
import { useLayoutStore } from "../../stores/useLayoutStore";

const settings = useSettingsStore();
const healthOpen = ref(false);
const layout = useLayoutStore();

// M1-4：默认浏览器设置。硬约束：只有在用户点击按钮并二次确认后才调用
// set_default_browser；应用安装/启动路径绝不触碰系统默认浏览器。
const defaultBrowser = ref<string>("");
const confirming = ref(false);
const setting = ref(false);

async function refreshDefaultBrowser() {
  try {
    defaultBrowser.value = await bridge.getDefaultBrowser();
  } catch (e) {
    defaultBrowser.value = "（查询失败）";
    // eslint-disable-next-line no-console
    console.warn("[settings] get_default_browser failed", e);
  }
}

async function onSetDefault() {
  if (!confirming.value) {
    confirming.value = true;
    return;
  }
  confirming.value = false;
  setting.value = true;
  try {
    const desktop = await bridge.setDefaultBrowser();
    layout.showToast(`✅ 已设为默认浏览器（${desktop}）`);
    await refreshDefaultBrowser();
  } catch (e) {
    layout.showToast("❌ 设为默认浏览器失败: " + String(e));
  } finally {
    setting.value = false;
  }
}

onMounted(refreshDefaultBrowser);
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
      <div class="section-title">默认浏览器</div>
      <div class="setting-item">
        <label>当前默认</label>
        <span class="mono-text">{{ defaultBrowser || "查询中…" }}</span>
      </div>
      <div class="setting-item">
        <label>设为默认</label>
        <button class="default-btn" :disabled="setting" @click="onSetDefault">
          {{
            setting
              ? "设置中…"
              : confirming
                ? "确认修改系统默认浏览器？再次点击确认"
                : "将本应用设为系统默认浏览器"
          }}
        </button>
        <button v-if="confirming" class="cancel-btn" @click="confirming = false">
          取消
        </button>
      </div>
      <div class="setting-desc">
        仅点击上方按钮并二次确认后，才会通过 xdg-settings 修改系统默认浏览器
        （仅接管 http/https 链接）；应用安装与启动过程不会自动更改。
      </div>
    </div>

    <div class="settings-section">
      <div class="section-title">模块管理</div>
      <div class="setting-item">
        <span class="setting-desc">查看能力依赖、运行状态并停用可选模块。</span>
        <button class="default-btn" @click="layout.setView('capability-manager')">打开 Capability Manager</button>
      </div>
    </div>


    <div class="settings-section">
      <div class="section-title">工程健康 · 验证与证据</div>
      <div class="setting-item">
        <span class="setting-desc">查看当前 GitHub master 门禁、Rust 语义、供应链安全和证据资产。网络不可用时显示“未知”，不会假报通过。</span>
        <button type="button" class="default-btn" aria-controls="engineering-health-region" :aria-expanded="healthOpen" @click="healthOpen = !healthOpen">
          {{ healthOpen ? "收起工程健康" : "打开工程健康中心" }}
        </button>
      </div>
      <div id="engineering-health-region" v-if="healthOpen"><EngineeringHealthPanel /></div>
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
.mono-text {
  font-family: monospace;
  font-size: 12px;
  color: #2b3a55;
}
.default-btn {
  height: 30px;
  padding: 0 14px;
  border: 1px solid #2b6cb0;
  border-radius: 6px;
  background: #fff;
  color: #2b6cb0;
  font-size: 13px;
  cursor: pointer;
}
.default-btn:hover:not(:disabled) {
  background: #2b6cb0;
  color: #fff;
}
.default-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
.cancel-btn {
  height: 30px;
  padding: 0 12px;
  border: 1px solid #e5e6eb;
  border-radius: 6px;
  background: #fff;
  color: #4e5969;
  font-size: 13px;
  cursor: pointer;
}
</style>
