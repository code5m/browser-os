<script setup lang="ts">
import { computed, onMounted, ref, type ComponentPublicInstance } from "vue";
import type { Bookmark } from "../../../types";
import { useBrowserStore } from "../../browser/public";
import { useBookmarkStore } from "../state/useBookmarkStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { bridge } from "../../../bridge";
import { contributionRegistry } from "../../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../../capability/contribution/types";

// 收藏夹侧栏（M1-3）：按 created_at 倒序展示 bridge.bookmarkList()，
// 点项在内嵌浏览器打开（不走系统默认浏览器，那是 M1-4），按 id 删除。
const browser = useBrowserStore();
const bookmarks = useBookmarkStore();
const layout = useLayoutStore();
const fileInput = ref<HTMLInputElement>();
const passwordInput = ref<HTMLInputElement>();
const expanded = ref(new Set<string>());
// 「账号」视图：查看已导入的浏览器账号（只读，绝不显示密码）
const showCredentials = ref(false);
// 凭证列表：经通用 Contribution Registry 由 browser 贡献，BookmarkPanel 不 import browser 内部 UI（C3 关键）。
const credentialListComp = contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.BOOKMARK_CREDENTIALS,
)[0]?.component
const credList = ref<ComponentPublicInstance | null>(null);

// 侧栏可能先于 ⭐ 按钮挂载（如刷新后直接展开），这里兜底加载一次
onMounted(() => {
  if (!bookmarks.loaded) bookmarks.load();
});

const groups = computed(() => {
  const map = new Map<string, Bookmark[]>();
  for (const item of bookmarks.sorted) {
    const category = item.category && item.category !== "default" ? item.category : "未分类";
    map.set(category, [...(map.get(category) || []), item]);
  }
  return [...map.entries()].sort(([a], [b]) => a.localeCompare(b, "zh-CN"));
});
const total = computed(() => bookmarks.items.length);

function toggleGroup(name: string) {
  const next = new Set(expanded.value);
  next.has(name) ? next.delete(name) : next.add(name);
  expanded.value = next;
}

function hostOf(u: string): string {
  try {
    return new URL(u).hostname;
  } catch {
    return u;
  }
}

// 侧栏点击一律开内嵌页签：openBrowser 走 bridge.tabNew，不会触发系统默认浏览器
async function openItem(b: Bookmark) {
  browser.url = b.url;
  layout.activateBrowser();
  await browser.openBrowser();
}

async function removeItem(b: Bookmark) {
  if (bookmarks.busy) return;
  await bookmarks.remove(b.id);
}

async function importFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    await bookmarks.importFile(file);
  } catch {
    bookmarks.error = "收藏文件格式无法解析";
  } finally {
    input.value = "";
  }
}

function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [], value = "", quoted = false;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quoted && ch === '"' && text[i + 1] === '"') { value += '"'; i += 1; }
    else if (ch === '"') quoted = !quoted;
    else if (!quoted && ch === ',') { row.push(value); value = ""; }
    else if (!quoted && (ch === '\n' || ch === '\r')) {
      if (ch === '\r' && text[i + 1] === '\n') i += 1;
      row.push(value); value = "";
      if (row.some(Boolean)) rows.push(row);
      row = [];
    } else value += ch;
  }
  row.push(value);
  if (row.some(Boolean)) rows.push(row);
  return rows;
}

async function importPasswords(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  try {
    const rows = parseCsv(await file.text());
    const headers = (rows.shift() || []).map((item) => item.trim().toLowerCase().replace(/^\ufeff/, ""));
    const find = (...names: string[]) => headers.findIndex((header) => names.includes(header));
    const urlIndex = find("url", "origin", "website");
    const usernameIndex = find("username", "user", "login_username");
    const passwordIndex = find("password", "login_password");
    if (urlIndex < 0 || usernameIndex < 0 || passwordIndex < 0) throw new Error("CSV 必须包含 url、username、password 列");
    const credentials = rows.map((row) => ({ url: row[urlIndex]?.trim() || "", username: row[usernameIndex]?.trim() || "", password: row[passwordIndex] || "" })).filter((row) => row.url && row.username && row.password);
    const count = await bridge.importBrowserCredentials(credentials);
    credentials.forEach((row) => { row.password = ""; });
    layout.showToast(`已安全导入 ${count} 条账号密码`);
    // 导入后切到「账号」视图，让结果可见；已挂载则显式刷新，未挂载时组件挂载会自行加载
    showCredentials.value = true;
    if (credList.value) (credList.value as unknown as { load: () => void }).load();
  } catch (error) {
    layout.showToast("账号密码导入失败: " + String((error as Error)?.message || error));
  } finally {
    input.value = "";
  }
}
</script>

<template>
  <!-- 收藏夹侧栏可见性：panelOpen（owner=useBookmarkStore）+ 浏览器视图。
       原由 Shell(MainArea) 派生 bmPanelOpen 控制，现下沉到能力自身 UI：
       Shell 经通用 Contribution Registry 按 slot 渲染，零 Bookmark 专属知识（8B.1 / C3）。 -->
  <template v-if="bookmarks.panelOpen && layout.mainView === 'browser'">
  <aside class="bookmark-side">
    <div class="tabs">
      <span class="bm-title">📑 收藏夹</span>
      <span class="bm-count">{{ total }}</span>
      <button title="刷新" @click="bookmarks.load()">↻</button>
      <button title="导入 Chrome/Firefox/HTML 收藏夹" @click="fileInput?.click()">导入</button>
      <button title="导入 Chrome/Edge 导出的账号密码 CSV，密码保存到系统密钥库" @click="passwordInput?.click()">密码</button>
      <button :class="{ active: showCredentials }" title="查看已导入的浏览器账号（只显示站点/用户名/是否保存，绝不显示密码）" @click="showCredentials = !showCredentials">账号</button>
      <button class="close" title="收起" @click="bookmarks.togglePanel">✕</button>
      <input ref="fileInput" class="hidden-file" type="file" accept=".json,.html,.htm" @change="importFile" />
      <input ref="passwordInput" class="hidden-file" type="file" accept=".csv,text/csv" @change="importPasswords" />
    </div>
    <div v-if="bookmarks.error" class="bm-error">{{ bookmarks.error }}</div>
    <component :is="credentialListComp" v-if="showCredentials" ref="credList" />
    <div v-else class="bm-list">
      <template v-if="total">
        <section v-for="[category, entries] in groups" :key="category" class="bm-group">
          <button class="bm-folder" @click="toggleGroup(category)">
            <span>{{ expanded.has(category) ? '▾' : '▸' }}</span>
            <span>📁 {{ category }}</span>
            <small>{{ entries.length }}</small>
          </button>
        <div v-show="expanded.has(category)" v-for="b in entries" :key="b.id" class="bm-item">
          <button class="bm-open" :title="b.url" @click="openItem(b)">
            <span class="bm-name">{{ b.title || b.url }}</span>
            <span class="bm-host">{{ hostOf(b.url) }}</span>
          </button>
          <button class="bm-del" title="删除这条收藏" @click="removeItem(b)">✕</button>
        </div>
        </section>
      </template>
      <div v-else class="bm-empty">
        {{ bookmarks.loaded ? "还没有收藏：点地址栏 ☆ 收藏当前网页" : "正在读取收藏夹…" }}
      </div>
    </div>
  </aside>
  </template>
</template>

<style scoped>
.bookmark-side {
  width: 260px;
  flex-shrink: 0;
  border-right: 1px solid #e5e6eb;
  display: flex;
  flex-direction: column;
  background: #f7f8fa;
  min-height: 0;
}
.hidden-file { display: none; }
.tabs {
  flex-shrink: 0;
}
.bm-title {
  font-size: 13px;
  font-weight: 600;
  color: #4e5969;
  padding: 0 4px 0 2px;
}
.bm-count {
  font-size: 11px;
  color: #86909c;
  margin-right: auto;
}
.tabs button {
  background: none;
  border: none;
  border-bottom: none;
  padding: 6px 8px;
  cursor: pointer;
  font-size: 13px;
  color: #86909c;
}
.tabs button:hover {
  color: #2b6cb0;
}
.tabs .close {
  color: #bbb;
}
.tabs button.active {
  color: #2b6cb0;
  font-weight: 600;
}
.bm-error {
  flex-shrink: 0;
  font-size: 11px;
  color: #c0392b;
  padding: 4px 8px;
  background: #fdecea;
  border-bottom: 1px solid #f5c6c2;
  word-break: break-all;
}
.bm-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 4px;
}
.bm-item {
  display: flex;
  align-items: center;
  gap: 2px;
  border-radius: 5px;
}
.bm-group { display: block; }
.bm-folder { width: 100%; display: flex; align-items: center; gap: 5px; border: 0; background: transparent; padding: 6px; text-align: left; cursor: pointer; color: #4e5969; }
.bm-folder:hover { background: #eef2f7; }
.bm-folder small { margin-left: auto; color: #86909c; }
.bm-item:hover {
  background: #eef2f7;
}
.bm-open {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  background: none;
  border: none;
  padding: 5px 6px;
  cursor: pointer;
  text-align: left;
}
.bm-name {
  font-size: 12px;
  color: #1d2129;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bm-host {
  font-size: 10px;
  color: #86909c;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bm-del {
  flex-shrink: 0;
  background: none;
  border: none;
  color: #bbb;
  cursor: pointer;
  font-size: 12px;
  padding: 4px 6px;
  border-radius: 4px;
}
.bm-del:hover {
  color: #c0392b;
  background: #fdecea;
}
.bm-empty {
  font-size: 12px;
  color: #86909c;
  padding: 12px 8px;
  line-height: 1.6;
}
</style>
