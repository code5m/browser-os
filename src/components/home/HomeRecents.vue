<script setup lang="ts">
import { computed } from "vue";
import { useHomeStore } from "../../stores/useHomeStore";
import type { HomeRecent } from "../../stores/useHomeStore";

const home = useHomeStore();

// 有界渲染：最近访问在 store 层已封顶 HOME_MAX_RECENTS；这里再渲前 N 个做视觉封顶，
// 本地数据来自用户可编辑的 localStorage，损坏数据绝不进 UI（store 层已 normalizeRecents）。
const VISIBLE_LIMIT = 8;
const showAll = computed(() => home.recents.length <= VISIBLE_LIMIT);
const visible = computed(() => home.recents.slice(0, VISIBLE_LIMIT));

const TYPE_LABELS: Record<HomeRecent["type"], string> = {
  url: "网页",
  app: "应用",
  dir: "目录",
};

// 打开：复用 store.open（url→浏览器 / dir→文件视图 / app→启动应用），不新增运行时路径。
function onOpen(s: HomeRecent) {
  home.open(s);
}
</script>

<template>
  <section class="home-section" aria-labelledby="home-recents-title">
    <div class="hr-head">
      <h2 id="home-recents-title" class="home-section-title">最近访问</h2>
      <span v-if="home.hasRecents" class="hr-count">{{ home.recents.length }} 条</span>
    </div>

    <!-- 空态：给出明确说明，而不是一句灰色提示（共享验收 #2 连贯空态）。 -->
    <div v-if="!home.hasRecents" class="home-empty hr-empty">
      <div class="he-icon" aria-hidden="true">🕘</div>
      <p class="he-title">还没有最近访问记录</p>
      <p class="he-desc">打开任一快捷方式后，会在这里留下最近记录，方便快速回到。</p>
    </div>

    <template v-else>
      <ul class="recent-list">
        <li v-for="s in visible" :key="s.id || s.name" class="rc-item">
          <!-- 打开动作是真正的 button（可 Tab 聚焦 + 回车触发）。
               展示/无障碍文案只用 store 的安全摘要（displayTarget / accessibleLabel），
               绝不回显完整路径 / URL query / 凭据（共享验收 #4）。 -->
          <button
            type="button"
            class="rc-open"
            :aria-label="home.accessibleLabel(s) || `打开 ${s.name}`"
            @click="onOpen(s)"
          >
            <span class="rc-icon" aria-hidden="true">{{ s.icon }}</span>
            <span class="rc-text">
              <span class="rc-name">{{ s.name }}</span>
              <span class="rc-target">{{ home.displayTarget(s) }}</span>
            </span>
            <span class="rc-type">{{ TYPE_LABELS[s.type] }}</span>
          </button>
          <button
            type="button"
            class="rc-rm"
            :aria-label="`移除「${s.name}」的最近记录`"
            title="移除"
            @click="home.removeRecent(s.id)"
          >
            ✕
          </button>
        </li>
      </ul>

      <div v-if="home.recents.length > VISIBLE_LIMIT" class="rc-more">
        <button type="button" @click="home.clearRecents">清空最近访问</button>
      </div>
    </template>
  </section>
</template>

<style scoped>
.home-section {
  margin-bottom: 22px;
}
.hr-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}
.home-section-title {
  margin: 0 0 10px;
  font-size: 13px;
  font-weight: 600;
  color: #4e5969;
  letter-spacing: 0.3px;
}
.hr-count {
  font-size: 11px;
  color: #86909c;
}

.recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.rc-item {
  display: flex;
  align-items: center;
  gap: 6px;
  background: #fff;
  border: 1px solid #e8ebf0;
  border-radius: 10px;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.rc-item:hover {
  border-color: #c6d8ef;
  box-shadow: 0 2px 10px rgba(43, 108, 176, 0.1);
}
.rc-open {
  flex: 1 1 auto;
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  background: transparent;
  border: none;
  border-radius: 10px;
  padding: 10px 12px;
  cursor: pointer;
  font: inherit;
  color: inherit;
  text-align: left;
}
.rc-open:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: -2px;
}
.rc-icon {
  font-size: 18px;
  line-height: 1;
  flex-shrink: 0;
}
.rc-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 2px;
}
.rc-name {
  font-size: 12px;
  color: #1d2129;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.rc-target {
  font-size: 11px;
  color: #86909c;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.rc-type {
  font-size: 10px;
  color: #86909c;
  flex-shrink: 0;
}
.rc-rm {
  border: 1px solid transparent;
  background: #f4f6fb;
  border-radius: 6px;
  margin-right: 8px;
  padding: 3px 7px;
  font-size: 11px;
  cursor: pointer;
  line-height: 1;
  flex-shrink: 0;
}
.rc-rm:hover {
  border-color: #c6d8ef;
  background: #fff;
}
.rc-rm:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 1px;
}

.rc-more {
  margin-top: 10px;
  display: flex;
  justify-content: flex-end;
}
.rc-more button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 6px;
  padding: 5px 14px;
  font-size: 12px;
  cursor: pointer;
}
.rc-more button:hover {
  border-color: #2b6cb0;
  color: #2b6cb0;
}
.rc-more button:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 2px;
}

/* 空态（与 shortcut 空态同语言） */
.home-empty {
  background: #fff;
  border: 1px dashed #d5dbe7;
  border-radius: 12px;
  padding: 22px 18px;
  text-align: center;
}
.he-icon {
  font-size: 26px;
  line-height: 1;
  margin-bottom: 6px;
}
.he-title {
  margin: 0 0 4px;
  font-size: 13px;
  font-weight: 600;
  color: #1d2129;
}
.he-desc {
  margin: 0;
  font-size: 12px;
  color: #86909c;
}

@media (max-width: 720px) {
  .rc-open {
    padding: 9px 10px;
    gap: 8px;
  }
  .rc-icon {
    font-size: 16px;
  }
}
</style>
