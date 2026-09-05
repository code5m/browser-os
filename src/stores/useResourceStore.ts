import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import { bridge } from "../bridge";
import type {
  ResourceCaptureSettings,
  ResourceKind,
  ResourceReceived,
} from "../types";

// M1-8 资源瀑布状态层。
// 数据全部来自后端脱敏 DTO（不含 headers/body，敏感查询参数值为 ***）；
// 前端不再解析/展示任何凭据，本地仅做容量镜像与筛选。

export type ResourceFilter = "all" | "doc" | "js" | "css" | "image" | "xhr" | "other";

export const RESOURCE_FILTERS: { key: ResourceFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "doc", label: "Doc" },
  { key: "js", label: "JS" },
  { key: "css", label: "CSS" },
  { key: "image", label: "Image" },
  { key: "xhr", label: "XHR/Fetch" },
  { key: "other", label: "Other" },
];

// 筛选分类：与后端 ResourceKind 一一对应（font/media/other 归 Other）
export function kindMatchesFilter(kind: ResourceKind, f: ResourceFilter): boolean {
  switch (f) {
    case "all":
      return true;
    case "doc":
      return kind === "document";
    case "js":
      return kind === "script";
    case "css":
      return kind === "stylesheet";
    case "image":
      return kind === "image";
    case "xhr":
      return kind === "xhr_fetch";
    case "other":
      return kind === "font" || kind === "media" || kind === "other";
  }
}

// 展示用「域名 + 路径 + 已脱敏 query」（URL 已由后端脱敏，这里只做解析截断，
// 解析失败原样返回——原样串同样已是脱敏形态）
export function displayUrl(url: string): string {
  try {
    const u = new URL(url);
    return u.host + u.pathname + u.search;
  } catch {
    return url;
  }
}

// 大小展示：null = 平台未知（降级，不伪造），显示 "-"
export function formatSize(n: number | null): string {
  if (n == null) return "-";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(2)} MB`;
}

// 耗时展示：null = 未知（时钟回拨等），显示 "-"
export function formatDuration(n: number | null): string {
  if (n == null) return "-";
  if (n < 1000) return `${n} ms`;
  return `${(n / 1000).toFixed(2)} s`;
}

export const useResourceStore = defineStore("resource", () => {
  // 每 tab 的记录（按入库顺序）；本地镜像后端容量上限
  const byTab = reactive<Record<string, ResourceReceived[]>>({});
  // 每 tab 因容量上限被丢弃的累计条数（后端 evicted + 本地镜像增量）
  const evictedByTab = reactive<Record<string, number>>({});
  const settings = ref<ResourceCaptureSettings | null>(null);
  const filter = ref<ResourceFilter>("all");

  const enabled = computed(() => settings.value?.enabled ?? true);
  const maxPerTab = computed(() => settings.value?.max_per_tab ?? 200);

  // 实时事件入口（App.vue 全局订阅调用）
  function applyReceived(rec: ResourceReceived) {
    const arr = (byTab[rec.tab_id] ??= []);
    arr.push(rec);
    // 本地容量镜像：与后端 per-tab FIFO 对齐，超出丢最旧并计数（前端超限提示）
    while (arr.length > maxPerTab.value) {
      arr.shift();
      evictedByTab[rec.tab_id] = (evictedByTab[rec.tab_id] ?? 0) + 1;
    }
  }

  async function loadSettings() {
    settings.value = await bridge.getResourceCaptureSettings();
  }

  async function setEnabled(on: boolean) {
    settings.value = await bridge.setResourceCaptureSettings(on);
  }

  // 打开/切换 tab 时全量拉取（后端为权威数据源，本地镜像随之复位）
  async function loadTab(tabId: string) {
    const r = await bridge.listTabResources(tabId);
    byTab[tabId] = r.records;
    evictedByTab[tabId] = r.evicted;
    if (settings.value) settings.value.enabled = r.enabled;
  }

  async function clearTab(tabId: string) {
    await bridge.clearTabResources(tabId);
    byTab[tabId] = [];
    evictedByTab[tabId] = 0;
  }

  function recordsOf(tabId: string): ResourceReceived[] {
    return byTab[tabId] ?? [];
  }

  function evictedOf(tabId: string): number {
    return evictedByTab[tabId] ?? 0;
  }

  function filteredOf(tabId: string): ResourceReceived[] {
    return recordsOf(tabId).filter((r) => kindMatchesFilter(r.resource_type, filter.value));
  }

  return {
    byTab,
    evictedByTab,
    settings,
    filter,
    enabled,
    maxPerTab,
    applyReceived,
    loadSettings,
    setEnabled,
    loadTab,
    clearTab,
    recordsOf,
    evictedOf,
    filteredOf,
  };
});
