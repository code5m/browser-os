<script setup lang="ts">
import { computed, ref } from "vue";
import { useHomeStore } from "../state/useHomeStore";
import type { HomeShortcut } from "../state/useHomeStore";

const home = useHomeStore();

// 有界渲染：默认只渲染前 N 个。本地存储来自用户可编辑的 localStorage，
// 可能被手改或写坏，这里做渲染上限，避免损坏数据把主页撑成一屏垃圾。
const VISIBLE_LIMIT = 12;
const showAll = ref(false);

const TYPE_LABELS: Record<HomeShortcut["type"], string> = {
  url: "网页",
  app: "应用",
  dir: "目录",
};

// 防御性归一化：A3 会在 store 层做校验/限长，这里在渲染层再兜一层，
// 保证 malformed 数据不会把主页渲染成空白或抛错（字段名/类型缺失都能撑住）。
function safeType(t: unknown): HomeShortcut["type"] {
  return t === "app" || t === "dir" ? t : "url";
}
function norm(s: Partial<HomeShortcut> | null | undefined) {
  return {
    id: typeof s?.id === "string" && s.id ? s.id : "",
    type: safeType(s?.type),
    name: (typeof s?.name === "string" && s.name.trim()) || "未命名",
    target: typeof s?.target === "string" ? s.target : "",
    icon: (typeof s?.icon === "string" && s.icon.trim()) || "🔗",
  };
}

const items = computed(() => (home.shortcuts ?? []).map(norm));
const visible = computed(() =>
  showAll.value ? items.value : items.value.slice(0, VISIBLE_LIMIT)
);
const hiddenCount = computed(() => Math.max(0, items.value.length - visible.value.length));

// 打开：url → 内嵌浏览器；dir → 文件视图；app → 启动系统应用（均由 store 负责）
function onOpen(s: HomeShortcut) {
  home.open(s);
}
</script>

<template>
  <section class="home-section" aria-labelledby="home-shortcuts-title">
    <div class="hs-head">
      <h2 id="home-shortcuts-title" class="home-section-title">快捷方式</h2>
      <span v-if="items.length" class="hs-count">{{ items.length }} 个</span>
    </div>

    <!-- 空态：给出明确的下一步动作，而不是一句灰色提示 -->
    <div v-if="!items.length" class="home-empty">
      <div class="he-icon" aria-hidden="true">🧭</div>
      <p class="he-title">还没有快捷方式</p>
      <p class="he-desc">
        把常用网页、系统应用或本地目录收藏到主页，下次一键打开。
      </p>
      <div class="he-actions">
        <button type="button" class="he-primary" @click="home.startAdd">
          ＋ 新增快捷方式
        </button>
        <button type="button" @click="home.resetDefault">↺ 恢复默认</button>
      </div>
    </div>

    <template v-else>
      <div class="shortcut-grid">
        <div v-for="s in visible" :key="s.id || s.name" class="sc-card">
          <!-- 打开动作是真正的 button（可 Tab 聚焦 + 回车触发），
               编辑/删除是同级按钮，不再嵌套在可点元素里。 -->
          <button
            type="button"
            class="sc-open"
            :aria-label="`打开 ${s.name}（${TYPE_LABELS[s.type]}）`"
            @click="onOpen(s)"
          >
            <span class="sc-icon" aria-hidden="true">{{ s.icon }}</span>
            <span class="sc-name">{{ s.name }}</span>
            <span class="sc-type">{{ TYPE_LABELS[s.type] }}</span>
          </button>
          <div class="sc-ops">
            <button
              type="button"
              class="op"
              :aria-label="`编辑 ${s.name}`"
              title="编辑"
              @click="home.startEdit(s)"
            >
              ✏️
            </button>
            <button
              type="button"
              class="op"
              :aria-label="`删除 ${s.name}`"
              title="删除"
              @click="home.remove(s.id)"
            >
              🗑
            </button>
          </div>
        </div>
      </div>

      <!-- 有界列表的显式出口：超过上限时可展开/收起 -->
      <div v-if="items.length > VISIBLE_LIMIT" class="sc-more">
        <button type="button" @click="showAll = !showAll" :aria-expanded="showAll ? 'true' : 'false'">
          {{ showAll ? "收起" : `显示全部（还有 ${hiddenCount} 个）` }}
        </button>
      </div>
    </template>
  </section>
</template>

<style scoped>
.home-section {
  margin-bottom: 22px;
}
.hs-head {
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
.hs-count {
  font-size: 11px;
  color: #86909c;
}

.shortcut-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 14px;
}
.sc-card {
  position: relative;
  background: #fff;
  border: 1px solid #e8ebf0;
  border-radius: 12px;
  transition: transform 0.15s, box-shadow 0.15s, border-color 0.15s;
}
.sc-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 14px rgba(43, 108, 176, 0.12);
  border-color: #c6d8ef;
}
.sc-open {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  width: 100%;
  background: transparent;
  border: none;
  border-radius: 12px;
  padding: 18px 10px 12px;
  cursor: pointer;
  font: inherit;
  color: inherit;
  text-align: center;
}
.sc-open:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: -2px;
}
.sc-icon {
  font-size: 26px;
  line-height: 1;
}
.sc-name {
  font-size: 12px;
  color: #1d2129;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sc-type {
  font-size: 10px;
  color: #86909c;
}
/* 操作按钮常驻可见（原来 hover 才出现，触屏/键盘用户根本摸不到） */
.sc-ops {
  position: absolute;
  top: 6px;
  right: 6px;
  display: flex;
  gap: 2px;
}
.sc-ops .op {
  border: 1px solid transparent;
  background: #f4f6fb;
  border-radius: 6px;
  padding: 2px 4px;
  font-size: 11px;
  cursor: pointer;
  line-height: 1;
}
.sc-ops .op:hover {
  border-color: #c6d8ef;
  background: #fff;
}
.sc-ops .op:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 1px;
}

.sc-more {
  margin-top: 12px;
  display: flex;
  justify-content: center;
}
.sc-more button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 6px;
  padding: 5px 14px;
  font-size: 12px;
  cursor: pointer;
}
.sc-more button:hover {
  border-color: #2b6cb0;
  color: #2b6cb0;
}
.sc-more button:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 2px;
}

/* 空态 */
.home-empty {
  background: #fff;
  border: 1px dashed #d5dbe7;
  border-radius: 12px;
  padding: 28px 18px;
  text-align: center;
}
.he-icon {
  font-size: 30px;
  line-height: 1;
  margin-bottom: 8px;
}
.he-title {
  margin: 0 0 4px;
  font-size: 14px;
  font-weight: 600;
  color: #1d2129;
}
.he-desc {
  margin: 0 0 14px;
  font-size: 12px;
  color: #86909c;
}
.he-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
  flex-wrap: wrap;
}
.he-actions button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 12px;
  cursor: pointer;
}
.he-actions button:hover {
  border-color: #2b6cb0;
  color: #2b6cb0;
}
.he-actions button:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 2px;
}
.he-primary {
  background: #2b6cb0 !important;
  border-color: #2b6cb0 !important;
  color: #fff !important;
}

@media (max-width: 720px) {
  .shortcut-grid {
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 10px;
  }
  .sc-open {
    padding: 14px 6px 10px;
  }
  .sc-icon {
    font-size: 22px;
  }
}
</style>
