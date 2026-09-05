import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { bridge } from "../bridge";
import type { Bookmark } from "../types";
import { useLayoutStore } from "./useLayoutStore";

// M1-3：收藏夹前端状态。只消费 M1-2 已交付的 add/list/remove_bookmark 契约，
// 不新增 IPC、不改后端。与 M1-0 的 useHomeStore（主页快捷方式）完全独立：
// 主页快捷方式存 localStorage，收藏夹由后端持久化到 data_dir/bookmarks.json。

// 后端 category 是自由字符串，前端首版只用一档默认值
export const DEFAULT_CATEGORY = "default";

// 本地路径（地址栏 📁 模式）不是可收藏网页
function looksLikeDir(s: string): boolean {
  const t = s.trim();
  return t.startsWith("/") || t.startsWith("~") || /^[A-Za-z]:[\\/]/.test(t);
}

// URL 归一化：忽略协议/主机名大小写与路径末尾斜杠差异，
// 避免"同一页面因末尾多一个 / 被判成两条收藏"导致 ⭐ 高亮与实际收藏对不上。
export function normalizeUrl(u: string): string {
  const t = (u ?? "").trim();
  if (!t) return "";
  try {
    const p = new URL(t);
    return (
      p.protocol.toLowerCase() +
      "//" +
      p.host.toLowerCase() +
      p.pathname.replace(/\/+$/, "") +
      p.search
    );
  } catch {
    // 非绝对 URL（如用户还没回车搜索时输入的裸域名）：只去掉末尾斜杠
    return t.replace(/\/+$/, "");
  }
}

// 是否可作为网页收藏：非空、非 about:blank、非本地目录路径
export function canBookmark(u: string): boolean {
  const t = (u ?? "").trim();
  return !!t && t !== "about:blank" && !looksLikeDir(t);
}

export const useBookmarkStore = defineStore("bookmark", () => {
  const layout = useLayoutStore();

  const items = ref<Bookmark[]>([]);
  const loaded = ref(false);
  const busy = ref(false);
  const error = ref("");
  // 收藏夹侧栏开关（状态放在本 store，避免改动 useLayoutStore）
  const panelOpen = ref(false);

  // 侧栏展示顺序：created_at 倒序（最新收藏在前）；同时间戳按 url 稳定排序
  const sorted = computed(() =>
    items.value.slice().sort((a, b) => {
      if (a.created_at === b.created_at) {
        return a.url < b.url ? -1 : a.url > b.url ? 1 : 0;
      }
      return a.created_at < b.created_at ? 1 : -1;
    })
  );

  function fail(e: unknown, what: string): string {
    const msg = String((e as { message?: unknown })?.message ?? e ?? "");
    error.value = msg;
    // IPC 失败必须可见：否则用户点了 ⭐ 却什么都没发生，会误判为功能未实现
    layout.showToast(`${what}失败: ${msg || "未知错误"}`);
    return msg;
  }

  async function load(): Promise<void> {
    try {
      error.value = "";
      items.value = (await bridge.bookmarkList()) ?? [];
    } catch (e) {
      items.value = [];
      fail(e, "读取收藏夹");
    } finally {
      loaded.value = true;
    }
  }

  function findByUrl(u: string): Bookmark | undefined {
    const key = normalizeUrl(u);
    if (!key) return undefined;
    return items.value.find((b) => normalizeUrl(b.url) === key);
  }

  function isBookmarked(u: string): boolean {
    return !!findByUrl(u);
  }

  async function add(
    url: string,
    title: string,
    category: string = DEFAULT_CATEGORY
  ): Promise<boolean> {
    const u = (url ?? "").trim();
    if (!canBookmark(u)) {
      layout.showToast("当前地址不可收藏");
      return false;
    }
    busy.value = true;
    try {
      // 后端语义：同 URL 视为更新（保留 id / created_at），返回该条最终态。
      // 因此这里不能用 push，必须按 id 或归一化 url 替换，否则侧栏会出现重复项。
      const saved = await bridge.bookmarkAdd({
        url: u,
        title: (title ?? "").trim() || u,
        category,
      });
      const i = items.value.findIndex(
        (b) => b.id === saved.id || normalizeUrl(b.url) === normalizeUrl(saved.url)
      );
      if (i >= 0) items.value.splice(i, 1, saved);
      else items.value.push(saved);
      error.value = "";
      layout.showToast("⭐ 已收藏: " + (saved.title || saved.url));
      return true;
    } catch (e) {
      fail(e, "收藏");
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function remove(id: string): Promise<boolean> {
    busy.value = true;
    try {
      await bridge.bookmarkRemove(id);
      const i = items.value.findIndex((b) => b.id === id);
      if (i >= 0) items.value.splice(i, 1);
      error.value = "";
      layout.showToast("已取消收藏");
      return true;
    } catch (e) {
      fail(e, "取消收藏");
      return false;
    } finally {
      busy.value = false;
    }
  }

  // 地址栏 ⭐ 的唯一入口：已收藏 → 按 id 移除；未收藏 → 写入
  async function toggle(url: string, title: string): Promise<boolean> {
    const existing = findByUrl(url);
    if (existing) return remove(existing.id);
    return add(url, title);
  }

  function togglePanel(): void {
    panelOpen.value = !panelOpen.value;
  }

  return {
    items,
    sorted,
    loaded,
    busy,
    error,
    panelOpen,
    load,
    findByUrl,
    isBookmarked,
    add,
    remove,
    toggle,
    togglePanel,
  };
});
