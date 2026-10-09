import { defineStore } from "pinia";
import { ref, reactive, computed, nextTick, watch } from "vue";
import { bridge } from "../../../bridge";
import { WEBVIEW_FREEZE_JS, WEBVIEW_UNFREEZE_JS } from "../../../utils/webviewFreeze";
import { isBrowserResourceAllowed } from "../resource/guard";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { recordRecentUrl } from "../../../composables/recentsNav";
import { registerBrowserLifecycleBinding } from "../lifecycle";
import type { RecentlyClosedEntry, TabRecoveryEvent } from "../../../types";

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
  const resources = ref<BrowserResources | null>(null);
  const aiNavOpen = ref(false);
  const aiFilter = ref<"全部" | "国内" | "海外">("全部");
  // Phase 04：最近关闭页签内存栈（封顶 20，不落盘；恢复时按 URL 重开）
  const RECENTLY_CLOSED_CAP = 20;
  const recentlyClosed = ref<RecentlyClosedEntry[]>([]);

  const aiSites: AISite[] = [
    { name: "豆包", url: "https://www.doubao.com", region: "国内" },
    { name: "Kimi", url: "https://kimi.moonshot.cn", region: "国内" },
    { name: "DeepSeek", url: "https://chat.deepseek.com", region: "国内" },
    { name: "通义千问", url: "https://tongyi.aliyun.com/qianwen/", region: "国内" },
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
    () => layout.mainView === "browser"
  );
  const aiFiltered = computed(() =>
    aiFilter.value === "全部"
      ? aiSites
      : aiSites.filter((s) => s.region === aiFilter.value)
  );

  async function tabNew(target?: string) {
    if (!isBrowserResourceAllowed()) {
      bridge.debugLog("tabNew refused: browser capability not ACTIVE");
      return;
    }
    const u = target === undefined ? "about:blank" : target.trim() || "about:blank";
    // 网页内 target=_blank / window.open 触发的新页签：若当前不在浏览器视图则切过去
    if (!layout.isBrowserView()) layout.setView("browser");
    const t = await bridge.tabNew(u);
    tabs.push(t);
    activeTabId.value = t.id;
    url.value = t.url;
    // Native create_tab already starts hidden. Do not race an extra offscreen
    // tabPosition against the first real viewport position: when it arrives last,
    // the active tab stays offscreen (blank browser area).
    await nextTick();
    relocate();
    syncFreeze();
  }
  async function tabSwitch(id: string) {
    activeTabId.value = id;
    const t = tabs.find((x) => x.id === id);
    if (t) url.value = t.url;
    await bridge.tabActivate(id);
    await nextTick();
    relocate();
    syncFreeze();
  }
  // ===== 普通 Tab 关闭 =====
  // Owner 最终裁决（2026-09-12）：普通 Tab 关闭 = 不弹确认框 + 不持久化 + 直接关闭。
  // 直接走 closeTabNow（其内部先 recordClose 写入 recentlyClosed 内存栈，再完成
  // WebView 生命周期关闭）。不再挂接任何关闭拦截器 / 确认框（M1-9 关闭协议已撤销）。
  async function tabClose(id: string) {
    await closeTabNow(id);
  }
  // 真正执行关闭：先 recordClose 写入 recentlyClosed 内存栈，再完成 WebView 生命周期关闭。
  // 普通 Tab 关闭（tabClose）与手动恢复入口均复用此路径，无可绕过。
  async function closeTabNow(id: string) {
    recordClose(id);
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
    if (!tabs.length) layout.setView("browser");
    await nextTick();
    relocate();
    syncFreeze();
  }
  // Phase 04：记录被关闭的页签（在真正拆除前读取 url/title）
  function recordClose(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    recentlyClosed.value.unshift({ url: t.url, title: t.title || t.url });
    if (recentlyClosed.value.length > RECENTLY_CLOSED_CAP) {
      recentlyClosed.value = recentlyClosed.value.slice(0, RECENTLY_CLOSED_CAP);
    }
  }
  // Phase 04：恢复最近关闭的页签（Ctrl+Shift+T）；栈空则提示
  async function restoreRecent() {
    const item = recentlyClosed.value.shift();
    if (!item) {
      layout.showToast("没有可恢复的页签");
      return;
    }
    await tabNew(item.url);
  }
  async function tabReload(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    await bridge.tabOpen(id, t.url);
    if (activeTabId.value === id) url.value = t.url;
    await nextTick();
    relocate();
  }
  async function tabNavigate(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    const u = t.url.trim() || "about:blank";
    await bridge.tabOpen(id, u);
    if (activeTabId.value === id) url.value = u;
    layout.showToast("页签导航: " + u);
    await nextTick();
    relocate();
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
    relocate();
  }
  function setTitle(t: TabInfo) {
    const existing = tabs.find((x) => x.id === t.id);
    if (existing && t.title) existing.title = t.title;
    if (t.id === activeTabId.value) nextTick(relocate);
  }
  // 子 webview 内导航完成（点链接/前进/后退/刷新）：同步页签 URL 与地址栏
  function setNavigated(id: string, navUrl: string) {
    const t = tabs.find((x) => x.id === id);
    if (t) t.url = navUrl;
    if (id === activeTabId.value) url.value = navUrl;
  }
  function handleTabRecovery(event: TabRecoveryEvent) {
    bridge.debugLog(
      `tabRecovery id=${event.id} status=${event.status} attempt=${event.attempt}/${event.max_attempts} reason=${event.reason}`
    );
    if (event.status === "recovered") {
      if (event.id === activeTabId.value) nextTick(relocate);
      layout.showToast(`页签已自动恢复: ${event.id}`);
    } else if (event.status === "budget-exhausted") {
      layout.showToast(`页签恢复次数已用尽: ${event.id}`);
    } else if (event.status === "failed") {
      layout.showToast(`页签恢复失败: ${event.id}`);
    } else if (event.status === "load-failed" && event.id === activeTabId.value) {
      layout.showToast(`页面加载失败: ${event.url || event.id}`);
    }
  }

  // Browser owns only ordinary tab WebViews. Grid lifecycle moved to capabilities/grid.
  async function closeBrowser() {
    await bridge.closeBrowser();
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
  // Phase 6A：AI 导航面板开关唯一写入点（owner: useBrowserStore；见 ADR-SEM-P6A-1）。
  // 此前 useLayoutStore 曾重复声明 aiNavOpen（死代码），已删除，唯一真源在此。
  function toggleAiNav() {
    aiNavOpen.value = !aiNavOpen.value;
  }
  // Omnibox navigation is intentionally different from explicitly opening a
  // NEW tab (tabNew/openBrowser). A current tab must retain its ID and WebView.
  async function navigateCurrent() {
    const target = url.value.trim() || "https://www.baidu.com";
    recordRecentUrl(target);
    layout.setView("browser");
    const current = tabs.find((t) => t.id === activeTabId.value);
    if (!current) {
      await tabNew(target);
      return;
    }
    await bridge.tabOpen(current.id, target);
    current.url = target;
    await nextTick();
    relocate();
  }

  async function openBrowser() {
    const target = url.value.trim() || "https://www.baidu.com";
    recordRecentUrl(target);
    layout.setView("browser");
    await tabNew(target);
    layout.showToast("已打开。在网页右键 → 保存选区/整页");
  }

  // 定位调度器从 composable 注入（避免 store 直接依赖 DOM）
  let schedulePosition: () => void = () => {};
  function bindPositionScheduler(fn: () => void) {
    schedulePosition = fn;
  }
  // 供编辑器/侧栏切换后强制重新定位
  function relocate() {
    schedulePosition();
  }


  // ===== 隐藏/非激活 webview 冻结（防后台网页吃 CPU 拖垮终端等主界面） =====
  // 子 webview 是独立进程，不会直接让主 webview 的终端崩溃，但隐藏中的网页
  // 仍在跑 JS 定时器/动画/视频，持续消耗 CPU/GPU。冻结=暂停媒体 + 暂停 CSS 动画
  // + requestAnimationFrame 节流至 1fps（页面脚本误以为仍在正常渲染，恢复时无缝续跑）
  // Browser only freezes ordinary tab WebViews; Grid owns grid-* processes.
  function syncFreeze() {
    for (const t of tabs) {
      const visible = layout.mainView === "browser" && t.id === activeTabId.value;
      bridge.evalInTab(t.id, visible ? WEBVIEW_UNFREEZE_JS : WEBVIEW_FREEZE_JS).catch(() => {});
    }
  }

  // ===== 子 webview 显隐同步 =====
  // 子 webview 是独立置顶 GTK 窗口，不受前端 v-if / visibility 控制。
  // 切到非浏览器视图（编辑器/文件/终端等）时，主 UI 的 addrbar/TabBar 会因
  // v-if 不渲染，但子 webview 仍盖在屏幕上（遮住主区、看似"没有地址栏"）。
  // 因此必须在视图切换时显式把子 webview 移出屏幕 / 移回。
  function hideAllWebviews() {
    // 统一走后端 hide_all_webviews：用无去重的 hide_bounds 强制移出屏幕。
    // 之前逐个调 tabPosition/gridPosition 会被 apply_bounds 的 50ms 去重丢弃，
    // 导致刚定位过的宫格/页签没被移出、切视图后仍残留显示。
    bridge.hideAllWebviews().catch(() => {});
  }

  // 按当前视图同步子 webview 显隐：browser/grid 视图重新定位显示，其它视图移出屏幕
  async function syncViewVisibility() {
    bridge.debugLog(`syncViewVisibility view=${layout.mainView} browserVisible=${layout.mainView === "browser"}`);
    if (layout.mainView === "browser") {
      await bridge.hideAllWebviews().catch(() => {});
      relocate();
    } else if (layout.mainView === "grid") {
      if (activeTabId.value) await bridge.hideWebview(activeTabId.value).catch(() => {});
    } else {
      hideAllWebviews();
    }
    syncFreeze();
  }

  // 监听视图切换，自动同步子 webview 显隐
  watch(
    () => layout.mainView,
    () => {
      // 等 Vue 完成 DOM 更新（v-if 切换）后再定位/隐藏
      nextTick(syncViewVisibility);
    }
  );

  registerBrowserLifecycleBinding({
    resume: () => nextTick(syncViewVisibility),
    suspend: () => {
      if (layout.mainView === "browser") layout.setView("home");
      hideAllWebviews();
      syncFreeze();
    },
    deactivate: async () => {
      if (layout.mainView === "browser") layout.setView("home");
      hideAllWebviews();
      await Promise.all(tabs.map(({ id }) => bridge.tabClose(id).catch(() => {})));
      tabs.splice(0);
      activeTabId.value = "";
      resources.value = null;
      url.value = "";
    },
  });

  return {
    url,
    tabs,
    activeTabId,
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
    closeTabNow,
    recentlyClosed,
    recordClose,
    restoreRecent,
    tabReload,
    tabNavigate,
    goBack,
    goForward,
    reloadActive,
    setTitle,
    setNavigated,
    handleTabRecovery,
    closeBrowser,
    setResources,
    clearResources,
    gotoAI,
    toggleAiNav,
    openBrowser,
    navigateCurrent,
    bindPositionScheduler,
    relocate,
  };
});
