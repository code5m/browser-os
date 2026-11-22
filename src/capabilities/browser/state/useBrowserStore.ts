import { defineStore } from "pinia";
import { ref, reactive, computed, nextTick, watch } from "vue";
import { bridge } from "../../../bridge";
import { WEBVIEW_FREEZE_JS, WEBVIEW_UNFREEZE_JS } from "../../../utils/webviewFreeze";
import { isBrowserResourceAllowed } from "../resource/guard";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { classifyNativeTabLoadFailure, recentNativeTabLoadFailure, type NativeTabLoadFailureRecord } from "../model/nativeTabLoadFailure.mjs";
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
  const nativeTabFailures = new Map<string, NativeTabLoadFailureRecord>();
  function getRecentNativeTabFailure(id: string) {
    return recentNativeTabLoadFailure(nativeTabFailures.get(id));
  }

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

  // Serialize the existing owner's native mutations. A slow activation or close
  // must not publish an older selection after a newer request has completed.
  let tabMutationTail: Promise<unknown> = Promise.resolve();
  function mutateTabs<T>(operation: () => Promise<T>): Promise<T> {
    const result = tabMutationTail.then(operation);
    tabMutationTail = result.catch(() => {});
    return result;
  }

  async function tabNew(target?: string): Promise<boolean> {
    if (!isBrowserResourceAllowed()) {
      bridge.debugLog("tabNew refused: browser capability not ACTIVE");
      return false;
    }
    const publish = layout.prepareViewChange("browser");
    if (!publish) return false;
    const u = target === undefined ? "about:blank" : target.trim() || "about:blank";
    return mutateTabs(async () => {
      if (!isBrowserResourceAllowed() || !publish.isCurrent()) return false;
      let t: TabInfo;
      try {
        t = await bridge.tabNew(u);
      } catch (error) {
        await syncViewVisibility();
        throw error;
      }
      if (!isBrowserResourceAllowed()) {
        try { await bridge.tabClose(t.id); }
        catch {
          // A native creation that loses activation still has an Owner until
          // cleanup succeeds. Deactivation/retry must be able to find its id.
          tabs.push(t);
          throw new Error(`BROWSER_CLEANUP_FAILED:${t.id}`);
        }
        return false;
      }
      tabs.push(t);
      // Module navigation may have won while native creation was pending.
      // Retain the new tab without taking that module's view or address away.
      if (publish()) {
        activeTabId.value = t.id;
        url.value = t.url;
        await nextTick();
        relocate();
        syncFreeze();
      } else {
        if (activeTabId.value && tabs.some((tab) => tab.id === activeTabId.value)) {
          await bridge.tabActivate(activeTabId.value);
        }
        await syncViewVisibility();
      }
      return true;
    });
  }
  // A fresh about:blank tab is a disposable placeholder when the user chooses
  // a different surface (directory/Grid). Never discard an occupied web page.
  const blankTabReplacements = new Map<string, Promise<boolean>>();
  async function consumeActiveBlankTab(expectedId?: string): Promise<boolean> {
    const id = expectedId ?? (layout.mainView === "browser" ? activeTabId.value : "");
    if (!id || tabs.find(tab => tab.id === id)?.url !== "about:blank") return false;
    const existing = blankTabReplacements.get(id);
    if (existing) return existing;
    // Directory navigation captures its disposable placeholder before awaiting
    // I/O, but consumes it only after committing the successful new view.
    const pending = closeTabNow(id, true);
    blankTabReplacements.set(id, pending);
    try { return await pending; }
    finally { if (blankTabReplacements.get(id) === pending) blankTabReplacements.delete(id); }
  }

  async function tabSwitch(id: string): Promise<boolean> {
    if (!tabs.some(tab => tab.id === id) || !isBrowserResourceAllowed()) return false;
    const publish = layout.prepareViewChange("browser");
    if (!publish) return false;
    return mutateTabs(async () => {
      const t = tabs.find((x) => x.id === id);
      if (!t || !isBrowserResourceAllowed() || !publish.isCurrent()) return false;
      try {
        await bridge.tabActivate(id);
      } catch (error) {
        // Native failure never publishes a new visible view or activeTabId.
        await syncViewVisibility();
        throw error;
      }
      if (!isBrowserResourceAllowed() || !tabs.some((tab) => tab.id === id) || !publish()) {
        // Another owner changed the visible surface while IPC was in flight.
        if (activeTabId.value && tabs.some((tab) => tab.id === activeTabId.value)) {
          await bridge.tabActivate(activeTabId.value);
        }
        await syncViewVisibility();
        return false;
      }
      activeTabId.value = id;
      url.value = t.url;
      await nextTick();
      relocate();
      syncFreeze();
      return true;
    });
  }
  // ===== 普通 Tab 关闭 =====
  // Owner 最终裁决（2026-09-12）：普通 Tab 关闭 = 不弹确认框 + 不持久化 + 直接关闭。
  // 直接走 closeTabNow（其内部先 recordClose 写入 recentlyClosed 内存栈，再完成
  // WebView 生命周期关闭）。不再挂接任何关闭拦截器 / 确认框（M1-9 关闭协议已撤销）。
  async function tabClose(id: string): Promise<boolean> {
    return closeTabNow(id);
  }
  // 真正执行关闭：先 recordClose 写入 recentlyClosed 内存栈，再完成 WebView 生命周期关闭。
  // 普通 Tab 关闭（tabClose）与手动恢复入口均复用此路径，无可绕过。
  async function closeTabNow(id: string, onlyBlank = false): Promise<boolean> {
    return mutateTabs(async () => {
      const target = tabs.find((t) => t.id === id);
      if (!target || (onlyBlank && target.url !== "about:blank")) return false;
      recordClose(id);
      await bridge.tabClose(id);
      nativeTabFailures.delete(id);
      const idx = tabs.findIndex((t) => t.id === id);
      if (idx < 0) return true;
      tabs.splice(idx, 1);
      let activateNext = "";
      if (activeTabId.value === id) {
        const next = tabs[idx] || tabs[idx - 1];
        activeTabId.value = next ? next.id : "";
        if (layout.mainView === "browser") {
          url.value = next?.url ?? "";
          activateNext = next?.id ?? "";
        }
      }
      // Closing a background web tab (including the last one) never changes
      // the visible module. Ordinary close remains direct, with no save/prompt.
      try {
        if (activateNext) await bridge.tabActivate(activateNext);
      } finally {
        await nextTick();
        relocate();
        syncFreeze();
      }
      return true;
    });
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
    const item = recentlyClosed.value[0];
    if (!item) {
      layout.showToast("没有可恢复的页签");
      return;
    }
    if (await tabNew(item.url)) {
      const index = recentlyClosed.value.indexOf(item);
      if (index >= 0) recentlyClosed.value.splice(index, 1);
    }
  }
  async function tabReload(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    t.title = t.url;
    nativeTabFailures.delete(id);
    await bridge.tabOpen(id, t.url);
    if (activeTabId.value === id && layout.mainView === "browser") url.value = t.url;
    await nextTick();
    relocate();
  }
  async function tabNavigate(id: string) {
    const t = tabs.find((x) => x.id === id);
    if (!t) return;
    const u = t.url.trim() || "about:blank";
    t.title = u;
    nativeTabFailures.delete(id);
    await bridge.tabOpen(id, u);
    if (activeTabId.value === id) url.value = u;
    layout.showToast("页签导航: " + u);
    await nextTick();
    relocate();
  }
  async function goBack() {
    if (!activeTabId.value) return;
    nativeTabFailures.delete(activeTabId.value);
    await bridge.tabGoBack(activeTabId.value);
  }
  async function goForward() {
    if (!activeTabId.value) return;
    nativeTabFailures.delete(activeTabId.value);
    await bridge.tabGoForward(activeTabId.value);
  }
  async function reloadActive() {
    if (!activeTabId.value) return;
    const current = tabs.find((t) => t.id === activeTabId.value);
    if (current) current.title = current.url;
    nativeTabFailures.delete(activeTabId.value);
    await bridge.tabReload(activeTabId.value);
    layout.showToast("已刷新当前页签");
    await nextTick();
    relocate();
  }
  function setTitle(t: TabInfo) {
    const existing = tabs.find((x) => x.id === t.id);
    // Never apply a stale title from an earlier URL or another WebView.
    if (!existing || !t.title || !t.url) return;
    // URL canonicalization accounts for a bare origin acquiring its trailing
    // slash from WebKit while still rejecting a different navigation.
    const canonical = (value: string) => {
      try { return new URL(value).href; } catch { return value; }
    };
    if (canonical(existing.url) !== canonical(t.url)) return;
    existing.title = t.title;
    if (t.id === activeTabId.value) nextTick(relocate);
  }
  // 子 webview 内导航完成（点链接/前进/后退/刷新）：同步页签 URL 与地址栏
  function setNavigated(id: string, navUrl: string) {
    const t = tabs.find((x) => x.id === id);
    // A late event from a closed tab or an empty native URI must not rewrite
    // the visible address. Linux navigation events now come from the native
    // top-level WebKit URI property rather than iframe on_navigation hooks.
    if (!t || !navUrl.trim()) return;
    if (t.url !== navUrl) {
      t.url = navUrl;
      t.title = navUrl; // Placeholder until document.title arrives.
    }
    if (id === activeTabId.value && layout.mainView === "browser") url.value = navUrl;
  }
  function handleTabRecovery(event: TabRecoveryEvent) {
    const failureKind = classifyNativeTabLoadFailure(event.status, event.message);
    if (failureKind && (event.status === "load-failed" || !getRecentNativeTabFailure(event.id))) {
      nativeTabFailures.set(event.id, { kind: failureKind, at: Date.now() });
    } else if (event.status === "recovered") {
      nativeTabFailures.delete(event.id);
    }
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
  async function navigateCurrent(): Promise<boolean> {
    const target = url.value.trim() || "https://www.baidu.com";
    if (!tabs.some((t) => t.id === activeTabId.value)) {
      const created = await tabNew(target);
      if (created) recordRecentUrl(target);
      return created;
    }
    return mutateTabs(async () => {
      const current = tabs.find((t) => t.id === activeTabId.value);
      if (!current || !isBrowserResourceAllowed()) return false;
      const publish = layout.prepareViewChange("browser");
      if (!publish) return false;
      await bridge.tabOpen(current.id, target);
      if (!tabs.some((t) => t.id === current.id)) return false;
      current.title = target;
      current.url = target;
      nativeTabFailures.delete(current.id);
      recordRecentUrl(target);
      if (!publish()) return false;
      url.value = target;
      await nextTick();
      relocate();
      return true;
    });
  }

  async function openBrowserAt(requestedUrl: string): Promise<boolean> {
    const target = requestedUrl.trim() || "https://www.baidu.com";
    const opened = await tabNew(target);
    if (opened) recordRecentUrl(target);
    return opened;
  }
  async function openBrowser(): Promise<boolean> { return openBrowserAt(url.value); }

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
      await mutateTabs(async () => {
        const closing = tabs.map(({ id }) => id);
        const results = await Promise.allSettled(closing.map(id => bridge.tabClose(id)));
        const failed: string[] = [];
        for (let index = 0; index < results.length; index++) {
          const id = closing[index];
          if (results[index].status === "rejected") {
            failed.push(id);
            continue;
          }
          const row = tabs.findIndex(tab => tab.id === id);
          if (row >= 0) tabs.splice(row, 1);
          nativeTabFailures.delete(id);
        }
        if (!tabs.some(tab => tab.id === activeTabId.value)) activeTabId.value = tabs[0]?.id ?? "";
        if (failed.length) {
          // Keep both native failure metadata and logical records for retry.
          // Runtime must not issue a successful Owner cleanup receipt.
          throw new Error(`BROWSER_CLEANUP_FAILED:${failed.join(",")}`);
        }
        nativeTabFailures.clear();
        activeTabId.value = "";
        resources.value = null;
        url.value = "";
      });
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
    consumeActiveBlankTab,
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
    getRecentNativeTabFailure,
    closeBrowser,
    setResources,
    clearResources,
    gotoAI,
    toggleAiNav,
    openBrowser,
    openBrowserAt,
    navigateCurrent,
    bindPositionScheduler,
    relocate,
  };
});
