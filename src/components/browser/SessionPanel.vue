<script setup lang="ts">
// M1-9 历史会话面板（挂在浏览器视图右侧 Dock 第四 Tab）。
// 列表 + 详情（含已脱敏资源瀑布）+ 恢复/删除/导出 + 会话策略开关。
// 全部数据来自后端脱敏存档；详情中的资源记录与 M1-8 瀑布同构。
import { computed, onMounted } from "vue";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useSessionStore } from "../../stores/useSessionStore";
import { displayUrl, formatDuration, formatSize } from "../../stores/useResourceStore";

const browser = useBrowserStore();
const layout = useLayoutStore();
const session = useSessionStore();

const detailResources = computed(() => session.detail?.resources ?? []);

const CLOSE_REASON_LABEL: Record<string, string> = {
  user_saved: "手动保存",
  app_shutdown_flush: "退出自动保存",
};

function reasonLabel(reason: string): string {
  return CLOSE_REASON_LABEL[reason] ?? reason;
}

async function saveActive() {
  const id = browser.activeTabId;
  if (!id) {
    layout.showToast("没有打开的页签");
    return;
  }
  const preview = await session.capturePreview(id);
  await session.saveTab(id, preview);
}

async function toggleClosePrompt() {
  await session.setPolicy(!session.policy.close_prompt, undefined);
}

async function toggleAutoSave() {
  await session.setPolicy(undefined, !session.policy.auto_save_on_exit);
}

async function toggleAutoSaveOnClose() {
  session.setAutoSaveOnClose(!session.autoSaveOnClose);
}

onMounted(async () => {
  await session.loadPolicy();
  await session.loadSessions();
});
</script>

<template>
  <div class="sp">
    <div class="sp-head">
      <span class="sp-title">💾 历史会话</span>
      <button class="sp-btn" :disabled="session.loading" @click="session.loadSessions()">刷新</button>
    </div>

    <div class="sp-policy">
      <label class="sp-toggle" title="关闭页签时弹「保存/删除」选择">
        <input type="checkbox" :checked="session.policy.close_prompt" @change="toggleClosePrompt" />
        关闭时询问
      </label>
      <label class="sp-toggle" title="退出应用前自动保存仍打开的页签（默认关）">
        <input type="checkbox" :checked="session.policy.auto_save_on_exit" @change="toggleAutoSave" />
        退出自动保存
      </label>
      <label class="sp-toggle" title="关闭页签时自动保存并直接关闭，不弹确认框（默认关）">
        <input type="checkbox" :checked="session.autoSaveOnClose" @change="toggleAutoSaveOnClose" />
        关闭时自动保存
      </label>
    </div>

    <div class="sp-actions">
      <button class="sp-btn primary" :disabled="!browser.activeTabId" @click="saveActive">
        保存当前页签
      </button>
    </div>

    <div v-if="browser.recentlyClosed.length" class="sp-recent">
      <div class="sp-recent-head">最近关闭（Ctrl+Shift+T 恢复）</div>
      <ul class="sp-recent-list">
        <li
          v-for="(item, i) in browser.recentlyClosed"
          :key="i"
          class="sp-recent-item"
          :title="item.title || item.url"
          @click="browser.tabNew(item.url)"
        >
          <span class="sp-recent-title">{{ item.title || item.url }}</span>
          <span class="sp-recent-url">{{ displayUrl(item.url) }}</span>
        </li>
      </ul>
    </div>

    <div v-if="session.error" class="sp-banner">{{ session.error }}</div>
    <div v-else-if="!session.sessions.length" class="sp-empty">
      暂无历史会话。关闭页签时选择「保存」，或点上方「保存当前页签」。
    </div>

    <!-- 详情视图 -->
    <div v-else-if="session.detail" class="sp-detail">
      <div class="sp-detail-head">
        <button class="sp-btn" @click="session.closeDetail()">← 返回列表</button>
        <span class="sp-detail-title" :title="session.detail.title">{{ session.detail.title }}</span>
      </div>
      <div class="sp-detail-meta" :title="session.detail.url">{{ displayUrl(session.detail.url) }}</div>
      <div class="sp-detail-meta dim">
        {{ reasonLabel(session.detail.close_reason) }} · 资源 {{ session.detail.resource_count }} 条 ·
        {{ new Date(session.detail.updated_at).toLocaleString() }}
      </div>
      <div v-if="session.detail.preview" class="sp-preview">
        {{ session.detail.preview }}{{ session.detail.preview_truncated ? "…" : "" }}
      </div>
      <div class="sp-detail-actions">
        <button class="sp-btn primary" @click="session.restoreSession(session.detail.id)">恢复</button>
        <button class="sp-btn" @click="session.exportSession(session.detail.id)">导出</button>
        <button class="sp-btn danger" @click="session.deleteSession(session.detail.id)">删除</button>
      </div>
      <div v-if="detailResources.length" class="sp-res-title">资源瀑布（已脱敏）</div>
      <ul v-if="detailResources.length" class="sp-res">
        <li v-for="r in detailResources" :key="r.id" class="sp-res-row">
          <span class="m">{{ r.method }}</span>
          <span class="s" :class="{ bad: r.status != null && r.status >= 400 }">{{ r.status ?? "-" }}</span>
          <span class="k">{{ r.resource_type }}</span>
          <span class="z">{{ formatSize(r.size_bytes) }}</span>
          <span class="d">{{ formatDuration(r.duration_ms) }}</span>
          <span class="u" :title="r.url">{{ displayUrl(r.url) }}</span>
        </li>
      </ul>
    </div>

    <!-- 列表视图 -->
    <ul v-else class="sp-list">
      <li v-for="s in session.sessions" :key="s.id" class="sp-item" @click="session.openDetail(s.id)">
        <div class="sp-item-title" :title="s.title">{{ s.title || "(无标题)" }}</div>
        <div class="sp-item-url" :title="s.url">{{ displayUrl(s.url) }}</div>
        <div class="sp-item-meta">
          <span class="sp-badge">{{ reasonLabel(s.close_reason) }}</span>
          <span>资源 {{ s.resource_count }}</span>
          <span>{{ new Date(s.updated_at).toLocaleString() }}</span>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.sp {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  font-size: 12px;
  color: var(--fg, #d4d4d4);
}
.sp-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border, #333);
}
.sp-title {
  font-weight: 600;
  flex: 1;
}
.sp-policy {
  display: flex;
  gap: 12px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border, #333);
}
.sp-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  user-select: none;
  font-size: 11px;
}
.sp-actions {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border, #333);
}
.sp-recent {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border, #333);
}
.sp-recent-head {
  font-weight: 600;
  margin-bottom: 4px;
}
.sp-recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 160px;
  overflow-y: auto;
}
.sp-recent-item {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 4px 6px;
  border-radius: 4px;
  cursor: pointer;
}
.sp-recent-item:hover {
  background: rgba(128, 128, 128, 0.1);
}
.sp-recent-title {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-recent-url {
  color: #7ec8ff;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-btn {
  background: none;
  border: 1px solid var(--border, #333);
  color: inherit;
  border-radius: 4px;
  padding: 2px 8px;
  cursor: pointer;
  font-size: 12px;
}
.sp-btn:disabled {
  opacity: 0.4;
  cursor: default;
}
.sp-btn.primary {
  background: var(--accent, #2b6cb0);
  border-color: var(--accent, #2b6cb0);
  color: #fff;
}
.sp-btn.danger {
  border-color: #a03d3d;
  color: #e08a8a;
}
.sp-banner {
  padding: 6px 8px;
  background: rgba(200, 60, 60, 0.15);
  color: #e08a8a;
}
.sp-empty {
  padding: 16px 10px;
  color: var(--fg-dim, #888);
  text-align: center;
  line-height: 1.6;
}
.sp-list {
  flex: 1;
  overflow-y: auto;
  margin: 0;
  padding: 0;
  list-style: none;
}
.sp-item {
  padding: 6px 8px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.12);
  cursor: pointer;
}
.sp-item:hover {
  background: rgba(128, 128, 128, 0.08);
}
.sp-item-title {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-item-url {
  color: #7ec8ff;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-item-meta {
  display: flex;
  gap: 8px;
  color: var(--fg-dim, #999);
  margin-top: 2px;
  font-size: 11px;
}
.sp-badge {
  border: 1px solid var(--border, #333);
  border-radius: 8px;
  padding: 0 6px;
}
.sp-detail {
  flex: 1;
  overflow-y: auto;
  padding: 6px 8px;
}
.sp-detail-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.sp-detail-title {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sp-detail-meta {
  color: #7ec8ff;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 6px;
}
.sp-detail-meta.dim {
  color: var(--fg-dim, #999);
}
.sp-preview {
  margin-top: 6px;
  padding: 6px;
  border: 1px solid var(--border, #333);
  border-radius: 4px;
  color: var(--fg-dim, #aaa);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 120px;
  overflow-y: auto;
}
.sp-detail-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
.sp-res-title {
  margin-top: 10px;
  font-weight: 600;
}
.sp-res {
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}
.sp-res-row {
  display: grid;
  grid-template-columns: 40px 32px 60px 60px 52px 1fr;
  gap: 5px;
  padding: 2px 0;
  border-bottom: 1px solid rgba(128, 128, 128, 0.1);
  white-space: nowrap;
}
.sp-res-row .m {
  color: #7ec8ff;
}
.sp-res-row .s {
  color: #8fd18f;
  text-align: right;
}
.sp-res-row .s.bad {
  color: #e06c60;
}
.sp-res-row .k {
  color: #c5a5e8;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sp-res-row .z,
.sp-res-row .d {
  text-align: right;
  color: var(--fg-dim, #999);
}
.sp-res-row .u {
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
