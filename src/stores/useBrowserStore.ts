import { defineStore } from "pinia";
import { ref, reactive, computed, nextTick, watch } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { useWorkspaceStore } from "./useWorkspaceStore";

export interface AISite {
  name: string;
  url: string;
  region: "国内" | "海外";
}

export const useBrowserStore = defineStore("browser", () => {
  const layout = useLayoutStore();

  const url = ref("");
  const tabs = reactive<TabInfo[]>([]);
  const activeTabId = ref("");
  const gridOpen = ref(false);
  const gridCount = ref(2);
  const gridUrl = ref("");
  const gridUrls = reactive<string[]>(Array(12).fill(""));
  // 宫格布局模式：horizontal 横向 / vertical 纵向 / quad 四分 / grid 宫格 / free 自由堆叠
  const gridLayout = ref<"horizontal" | "vertical" | "quad" | "grid" | "free">("grid");
  // 每格相对 host 的 rect（供关闭按钮覆盖层定位），scheduleGrid 时填充
  const gridRects = reactive<{ x: number; y: number; w: number; h: number }[]>([]);
  const resources = ref<BrowserResources | null>(null);
  const aiNavOpen = ref(false);
  const aiFilter = ref<"全部" | "国内" | "海外">("全部");

  const aiSites: AISite[] = [
    { name: "豆包", url: "https://www.doubao.com", region: "国内" },
    { name: "Kimi", url: "https://kimi.moonshot.cn", region: "国内" },
    { name: "DeepSeek", url: "https://chat.deepseek.com", region: "国内" },
    { name: "通义千问", url: "https://tongyi.aliyun.com", region: "国内" },
    { name: "文心一言", url: "https://yiyan.baidu.com", region: "国内" },
    { name: "智谱清言", url: "https://chatglm.cn", region: "国内" },
    { name: "讯飞星火", url: "https://xinghuo.xfyun.cn", region: "国内" },
    { name: "GPT (ChatGPT)", url: "https://chatgpt.com", region: "海外" },
    { name: "Claude", url: "https://claude.ai", region: "海外" },
    { name: "Gemini", url: "https://gemini.google.com", region: "海外" },
    { name: "Copilot", url: "https://copilot.microsoft.com", region: "海外" },
    { name: "Perplexity", url: "https://www.perplexity.ai", region: "海外" },
    { name: "Grok", url: "https://grok.x.com", region: "海外" },
    { name: "Poe", url: "https://poe.com", region: "海外" },
  ];

  const activeTab = computed(() => tabs.find((t) => t.id === activeTabId.value));
  const isBrowserVisible = computed(
    () => !gridOpen.value && layout.mainView === "browser"
  );
  const aiFiltered = computed(() =>
    aiFilter.value === "全部"
      ? aiSites
      : aiSites.filter((s) => s.region === aiFilter.value)
  );

  async function tabNew(target?: string) {
    const u = target ?? url.value.trim() ?? "";
    // 网页内 target=_blank / window.open 触发的新页签：若当前不在浏览器视图则切过去
    if (!layout.isBrowserView()) layout.mainView = "browser";
    const t = await bridge.tabNew(u);
    tabs.push(t);
    activeTabId.value = t.id;
    url.value = t.url;
    layout.showToast("已新建页签: " + t.title);
    await nextTick();
    schedulePosition();
  }
  async function tabSwitch(id: string) {
    activeTabId.value = id;
    const t = tabs.find((x) => x.id === id);
    if (t) url.value = t.url;
    await bridge.tabActivate(id);
    await nextTick();
    schedulePosition();
  }
  async function tabClose(id: string) {
    const idx = tabs.findIndex((t) => t.id === id);
    if (idx < 0) return;
    await bridge.tabClose(id);
    tabs.splice(idx, 1);
    if (activeTabId.value === id) {
      const next = tabs[idx] || tabs[idx - 1];
      activeTabId.value = next ? next.id : "";
      if (next) {
        url.value = next.url;
        await bridge.tabActivate(next.id);
      }
    }
    if (!tabs.length) layout.mainView = "browser";
    await nextTick();
    schedulePosition();
  }
  async function tabReload(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    await bridge.tabOpen(id, t.url);
    if (activeTabId.value === id) url.value = t.url;
    await nextTick();
    schedulePosition();
  }
  async function tabNavigate(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    const u = t.url.trim() || "https://www.baidu.com";
    await bridge.tabOpen(id, u);
    if (activeTabId.value === id) url.value = u;
    layout.showToast("页签导航: " + u);
    await nextTick();
    schedulePosition();
  }
  async function goBack() {
    if (!activeTabId.value) return;
    await bridge.tabGoBack(activeTabId.value);
  }
  async function goForward() {
    if (!activeTabId.value) return;
    await bridge.tabGoForward(activeTabId.value);
  }
  async function reloadActive() {
    if (!activeTabId.value) return;
    await bridge.tabReload(activeTabId.value);
    layout.showToast("已刷新当前页签");
    await nextTick();
    schedulePosition();
  }
  function setTitle(t: TabInfo) {
    const existing = tabs.find((x) => x.id === t.id);
    if (existing && t.title) existing.title = t.title;
    if (t.id === activeTabId.value) nextTick(schedulePosition);
  }
  // 子 webview 内导航完成（点链接/前进/后退/刷新）：同步页签 URL 与地址栏
  function setNavigated(id: string, navUrl: string) {
    const t = tabs.find((x) => x.id === id);
    if (t) t.url = navUrl;
    if (id === activeTabId.value) url.value = navUrl;
  }

  // ===== 宫格 =====
  function gridCols(n: number): number {
    if (n <= 2) return n;
    if (n === 3) return 3;
    if (n === 4) return 2;
    if (n <= 6) return 3;
    if (n <= 8) return 4;
    if (n <= 10) return 5;
    return 6;
  }
  async function buildGrid() {
    const n = gridCount.value;
    await bridge.createGrid(n);
    gridOpen.value = true;
    layout.mainView = "browser";
    for (let i = 0; i < n; i++) {
      const u = gridUrls[i] || url.value || "https://www.baidu.com";
      gridUrls[i] = u;
      await bridge.gridOpen(i, u);
    }
    layoutGrid();
    layout.showToast(`已打开 ${n} 宫格对比`);
  }
  function layoutGrid() {
    scheduleGrid();
  }
  async function gridSetUrl(i: number) {
    const u = gridUrls[i] || url.value || "https://www.baidu.com";
    gridUrls[i] = u;
    await bridge.gridOpen(i, u);
    layout.showToast(`宫格 ${i + 1} → ${u}`);
  }
  async function closeGridAll() {
    await bridge.closeGrid().catch(() => {});
    gridOpen.value = false;
    layout.gridToolbarOpen = false;
    gridRects.splice(0, gridRects.length);
    layout.showToast("已关闭宫格");
  }
  // 关闭单个宫格：销毁对应子 webview，其余保留，并按剩余数量重排
  async function closeGridOne(i: number) {
    await bridge.gridCloseOne(i).catch(() => {});
    // 从 gridUrls/gridRects 移除该格，gridCount 减 1 后重排
    gridUrls.splice(i, 1);
    gridUrls.push(""); // 保持数组长度
    gridRects.splice(i, 1);
    if (gridCount.value > 2) gridCount.value -= 1;
    else if (gridCount.value === 2) {
      // 只剩 1 格意义不大，整体关闭
      await closeGridAll();
      return;
    }
    layoutGrid();
    layout.showToast(`已关闭宫格 ${i + 1}`);
  }

  // ===== 资源扫描（按 id 更新，不覆盖其他页签） =====
  function setResources(r: BrowserResources) {
    resources.value = r;
  }
  function clearResources() {
    resources.value = null;
  }

  async function gotoAI(site: AISite) {
    aiNavOpen.value = false;
    url.value = site.url;
    await openBrowser();
  }
  async function openBrowser() {
    if (gridOpen.value) await closeGridAll();
    const target = url.value.trim() || "https://www.baidu.com";
    useWorkspaceStore().addRecentUrl(target);
    layout.mainView = "browser";
    await tabNew(target);
    layout.showToast("已打开。在网页右键 → 保存选区/整页");
  }

  // 定位调度器从 composable 注入（避免 store 直接依赖 DOM）
  let schedulePosition: () => void = () => {};
  let scheduleGrid: () => void = () => {};
  function bindPositionScheduler(fn: () => void) {
    schedulePosition = fn;
  }
  function bindGridScheduler(fn: () => void) {
    scheduleGrid = fn;
  }
  // 供编辑器/侧栏切换后强制重新定位
  function relocate() {
    schedulePosition();
  }

  // ===== 子 webview 显隐同步 =====
  // 子 webview 是独立置顶 GTK 窗口，不受前端 v-if / visibility 控制。
  // 切到非浏览器视图（编辑器/文件/终端等）时，主 UI 的 addrbar/TabBar 会因
  // v-if 不渲染，但子 webview 仍盖在屏幕上（遮住主区、看似"没有地址栏"）。
  // 因此必须在视图切换时显式把子 webview 移出屏幕 / 移回。
  function hideAllWebviews() {
    // 隐藏页签 webview
    for (const t of tabs) {
      bridge
        .tabPosition(t.id, { x: -30000, y: -30000, width: 1, height: 1 })
        .catch(() => {});
    }
    // 隐藏宫格 webview（grid-0..11，宫格开着时切到其它视图也要移出屏幕）
    for (let i = 0; i < 12; i++) {
      bridge
        .gridPosition(i, { x: -30000, y: -30000, width: 1, height: 1 })
        .catch(() => {});
    }
  }

  // 按当前视图同步子 webview 显隐：browser/grid 视图重新定位显示，其它视图移出屏幕
  function syncViewVisibility() {
    if (layout.mainView === "browser" || layout.mainView === "grid") {
      relocate();
    } else {
      hideAllWebviews();
    }
  }

  // 监听视图切换，自动同步子 webview 显隐
  watch(
    () => layout.mainView,
    () => {
      // 等 Vue 完成 DOM 更新（v-if 切换）后再定位/隐藏
      nextTick(syncViewVisibility);
    }
  );

  return {
    url,
    tabs,
    activeTabId,
    gridOpen,
    gridCount,
    gridUrl,
    gridUrls,
    gridLayout,
    gridRects,
    resources,
    aiNavOpen,
    aiFilter,
    aiSites,
    activeTab,
    isBrowserVisible,
    aiFiltered,
    tabNew,
    tabSwitch,
    tabClose,
    tabReload,
    tabNavigate,
    goBack,
    goForward,
    reloadActive,
    setTitle,
    setNavigated,
    buildGrid,
    layoutGrid,
    gridSetUrl,
    closeGridAll,
    closeGridOne,
    setResources,
    clearResources,
    gotoAI,
    openBrowser,
    bindPositionScheduler,
    bindGridScheduler,
    relocate,
  };
});
