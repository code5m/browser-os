import { defineStore } from "pinia";
import { ref, reactive, computed, nextTick } from "vue";
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
    layout.showToast("已关闭宫格");
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

  return {
    url,
    tabs,
    activeTabId,
    gridOpen,
    gridCount,
    gridUrl,
    gridUrls,
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
    setResources,
    clearResources,
    gotoAI,
    openBrowser,
    bindPositionScheduler,
    bindGridScheduler,
    relocate,
  };
});
