<script setup lang="ts">
// M1-8 资源瀑布面板（挂在浏览器视图右侧 Dock 第三 Tab）。
// 展示字段全部来自后端脱敏 DTO：URL 敏感查询参数已替换为 ***；
// status/mime/size 为 null 时显示 "-"（平台未提供，降级不伪造）。
import { computed, onMounted, watch } from "vue";
import { useBrowserStore } from "../state/useBrowserStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import {
  RESOURCE_FILTERS,
  displayUrl,
  formatDuration,
  formatSize,
  useResourceStore,
} from "../state/useResourceStore";

const browser = useBrowserStore();
const layout = useLayoutStore();
const res = useResourceStore();

const tabId = computed(() => browser.activeTabId);
const records = computed(() => (tabId.value ? res.filteredOf(tabId.value) : []));
const totalCount = computed(() => (tabId.value ? res.recordsOf(tabId.value).length : 0));
const evicted = computed(() => (tabId.value ? res.evictedOf(tabId.value) : 0));
// 降级采集提示：存在平台未提供的字段（不伪造，仅如实标注）
const hasDegraded = computed(() =>
  records.value.some((r) => r.status == null || r.size_bytes == null)
);

async function toggleCapture() {
  try {
    await res.setEnabled(!res.enabled);
    layout.showToast(res.enabled ? "已开启资源采集" : "已关闭资源采集");
  } catch (e) {
    layout.showToast("⚠️ 设置资源采集失败");
  }
}

async function clearCurrent() {
  if (!tabId.value) return;
  try {
    await res.clearTab(tabId.value);
    layout.showToast("已清空当前页签的资源记录");
  } catch (e) {
    layout.showToast("⚠️ 清空资源记录失败");
  }
}

onMounted(async () => {
  try {
    await res.loadSettings();
    if (tabId.value) await res.loadTab(tabId.value);
  } catch (e) {
    // 后端不可达时保持空态，不伪造数据
  }
});

// 切换激活页签时拉取该页签的瀑布记录（后端为权威数据源）
watch(tabId, async (id, prev) => {
  if (id && id !== prev) {
    try {
      await res.loadTab(id);
    } catch (e) {
      // 忽略：保持本地镜像
    }
  }
});
</script>

<template>
  <div class="wf">
    <div class="wf-head">
      <span class="wf-title">🌊 资源瀑布</span>
      <label class="wf-toggle" :title="res.enabled ? '点击关闭采集' : '点击开启采集'">
        <input type="checkbox" :checked="res.enabled" @change="toggleCapture" />
        采集
      </label>
      <button class="wf-clear" :disabled="!totalCount" @click="clearCurrent" title="清空当前页签记录">
        清空
      </button>
    </div>

    <div v-if="!res.enabled" class="wf-banner">资源采集已关闭，新请求不会记录</div>
    <div v-else-if="evicted > 0" class="wf-banner warn">
      已达容量上限：最旧的 {{ evicted }} 条已被丢弃（每页签最多 {{ res.maxPerTab }} 条）
    </div>

    <div class="wf-filters">
      <button
        v-for="f in RESOURCE_FILTERS"
        :key="f.key"
        :class="{ active: res.filter === f.key }"
        @click="res.filter = f.key"
      >
        {{ f.label }}
      </button>
    </div>

    <div v-if="!tabId" class="wf-empty">打开网页页签后，这里展示它的请求瀑布</div>
    <div v-else-if="!records.length" class="wf-empty">
      {{ res.enabled ? "暂无匹配的资源记录（刷新页面后重试）" : "采集已关闭" }}
    </div>
    <template v-else>
      <div class="wf-count">{{ records.length }} / {{ totalCount }} 条</div>
      <ul class="wf-list">
        <li v-for="r in records" :key="r.id" class="wf-row">
          <span class="wf-method">{{ r.method }}</span>
          <span class="wf-status" :class="{ bad: r.status != null && r.status >= 400 }">{{
            r.status ?? "-"
          }}</span>
          <span class="wf-kind" :class="'k-' + r.resource_type">{{ r.resource_type }}</span>
          <span class="wf-size">{{ formatSize(r.size_bytes) }}</span>
          <span class="wf-dur">{{ formatDuration(r.duration_ms) }}</span>
          <span class="wf-url" :title="r.url">{{ displayUrl(r.url) }}</span>
        </li>
      </ul>
      <div v-if="hasDegraded" class="wf-note">
        “-” 表示平台未提供该字段（降级采集，不伪造）；URL 中敏感参数已脱敏为 ***
      </div>
    </template>
  </div>
</template>

<style scoped>
.wf {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  font-size: 12px;
  color: var(--fg, #d4d4d4);
}
.wf-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border, #333);
}
.wf-title {
  font-weight: 600;
  flex: 1;
}
.wf-toggle {
  display: flex;
  align-items: center;
  gap: 4px;
  cursor: pointer;
  user-select: none;
}
.wf-clear {
  background: none;
  border: 1px solid var(--border, #333);
  color: inherit;
  border-radius: 4px;
  padding: 2px 8px;
  cursor: pointer;
}
.wf-clear:disabled {
  opacity: 0.4;
  cursor: default;
}
.wf-banner {
  padding: 4px 8px;
  background: rgba(200, 160, 40, 0.15);
  color: #e0b34d;
}
.wf-banner.warn {
  color: #e08a4d;
}
.wf-filters {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border, #333);
}
.wf-filters button {
  background: none;
  border: 1px solid var(--border, #333);
  color: inherit;
  border-radius: 10px;
  padding: 1px 8px;
  cursor: pointer;
  font-size: 11px;
}
.wf-filters button.active {
  background: var(--accent, #2b6cb0);
  border-color: var(--accent, #2b6cb0);
  color: #fff;
}
.wf-empty {
  padding: 16px 10px;
  color: var(--fg-dim, #888);
  text-align: center;
}
.wf-count {
  padding: 4px 8px;
  color: var(--fg-dim, #888);
}
.wf-list {
  flex: 1;
  overflow-y: auto;
  margin: 0;
  padding: 0;
  list-style: none;
}
.wf-row {
  display: grid;
  grid-template-columns: 44px 36px 64px 64px 56px 1fr;
  gap: 6px;
  align-items: baseline;
  padding: 3px 8px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.12);
  white-space: nowrap;
}
.wf-method {
  color: #7ec8ff;
}
.wf-status {
  color: #8fd18f;
  text-align: right;
}
.wf-status.bad {
  color: #e06c60;
}
.wf-kind {
  overflow: hidden;
  text-overflow: ellipsis;
  color: #c5a5e8;
}
.wf-size,
.wf-dur {
  text-align: right;
  color: var(--fg-dim, #999);
}
.wf-url {
  overflow: hidden;
  text-overflow: ellipsis;
  direction: ltr;
}
.wf-note {
  padding: 4px 8px;
  color: var(--fg-dim, #888);
  border-top: 1px solid var(--border, #333);
}
</style>
