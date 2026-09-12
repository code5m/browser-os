import { defineStore } from "pinia";
import { ref, reactive, computed, nextTick, watch } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { useWorkspaceStore } from "./useWorkspaceStore";
import type { RecentlyClosedEntry, TabRecoveryEvent } from "../types";

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
  // 宫格会话号：buildGrid 每次重建 +1，供定位层识别"同一批 webview 已销毁重建，
  // 上次发送缓存作废必须重发"（重建后 rect 可能与新 webview 的 1x1 初始态相同）
  const gridSession = ref(0);
  const gridCount = ref(4);
  const gridUrl = ref("");
  // 默认四分展示国内四大 AI 站点：豆包 / Kimi / 通义千问 / DeepSeek
  const gridUrls = reactive<string[]>([
    "https://www.doubao.com",
    "https://kimi.moonshot.cn",
    "https://tongyi.aliyun.com/qianwen/",
    "https://chat.deepseek.com",
    ...Array(8).fill(""),
  ]);
  // 宫格布局模式：horizontal 横向 / quad 四分 / grid 宫格（精简后只留三种好用的）
  const gridLayout = ref<"horizontal" | "quad" | "grid">("quad");
  // 宫格使用模式：browse 对比浏览 / ai 多 AI 同时提问（默认 AI：打开宫格即可底部群发）
  const gridMode = ref<"browse" | "ai">("ai");
  // AI 模式统一输入框内容
  const gridAiInput = ref("");
  // 每格相对 host 的 rect（供关闭按钮覆盖层定位），scheduleGrid 时填充
  const gridRects = reactive<{ x: number; y: number; w: number; h: number }[]>([]);
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

  // ===== 各 AI 站点输入框/发送按钮适配表（按 hostname 匹配，数组内按优先级尝试） =====
  // 未命中的站点走通用兜底：textarea → contenteditable → input[type=text]，Enter 提交
  const AI_SITE_ADAPTERS: Record<string, { inputs: string[]; sends: string[] }> = {
    "www.doubao.com": {
      inputs: ["textarea[data-testid='chat_input_input']", "textarea"],
      sends: ["button[data-testid='chat_input_send_button']"],
    },
    "kimi.moonshot.cn": {
      inputs: ["div.chat-input-editor[contenteditable='true']", "div[contenteditable='true']"],
      sends: [], // 回车即发送
    },
    "www.kimi.com": {
      inputs: ["div.chat-input-editor[contenteditable='true']", "div[contenteditable='true']"],
      sends: [],
    },
    "chat.deepseek.com": {
      inputs: ["#chat-input", "textarea"],
      sends: [], // 回车即发送
    },
    "tongyi.aliyun.com": {
      inputs: ["textarea"],
      sends: ["button[class*='send']", "button[class*='Send']"],
    },
    "www.tongyi.com": {
      inputs: ["textarea", "div[contenteditable='true']"],
      sends: ["button[class*='send']", "button[class*='Send']"],
    },
    "yiyan.baidu.com": {
      inputs: ["div[contenteditable='true']", "textarea"],
      sends: [],
    },
    "chatglm.cn": {
      inputs: ["textarea"],
      sends: [],
    },
    "xinghuo.xfyun.cn": {
      inputs: ["textarea"],
      sends: [],
    },
    "chatgpt.com": {
      inputs: ["#prompt-textarea", "div[contenteditable='true']"],
      sends: ["button[data-testid='send-button']", "button[aria-label*='Send']"],
    },
    "claude.ai": {
      inputs: ["div.ProseMirror[contenteditable='true']", "div[contenteditable='true']"],
      sends: ["button[aria-label*='Send']"],
    },
    "gemini.google.com": {
      inputs: ["div.ql-editor[contenteditable='true']", "div[contenteditable='true']"],
      sends: ["button.send-button", "button[aria-label*='Send']", "button[aria-label*='发送']"],
    },
    "copilot.microsoft.com": {
      inputs: ["textarea#userInput", "div[contenteditable='true']", "textarea"],
      sends: ["button[aria-label*='提交']", "button[aria-label*='Submit']"],
    },
    "www.perplexity.ai": {
      inputs: ["textarea"],
      sends: ["button[aria-label*='Submit']"],
    },
    "grok.x.com": {
      inputs: ["textarea", "div[contenteditable='true']"],
      sends: ["button[type='submit']"],
    },
    "poe.com": {
      inputs: ["textarea"],
      sends: ["button[class*='ChatMessageSendButton']", "button[class*='send']"],
    },
  };
  function aiAdapterFor(u: string): { inputs: string[]; sends: string[] } {
    try {
      const host = new URL(u).hostname;
      if (AI_SITE_ADAPTERS[host]) return AI_SITE_ADAPTERS[host];
      // 子域名兜底：如 kimi.com / m.doubao.com
      for (const k of Object.keys(AI_SITE_ADAPTERS)) {
        if (host === k || host.endsWith("." + k) || k.endsWith("." + host)) {
          return AI_SITE_ADAPTERS[k];
        }
      }
    } catch {}
    return { inputs: [], sends: [] };
  }

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
    const u = target ?? url.value.trim() ?? "";
    // 网页内 target=_blank / window.open 触发的新页签：若当前不在浏览器视图则切过去
    if (!layout.isBrowserView()) layout.mainView = "browser";
    const t = await bridge.tabNew(u);
    tabs.push(t);
    activeTabId.value = t.id;
    url.value = t.url;
    await nextTick();
    schedulePosition();
    syncFreeze();
  }
  async function tabSwitch(id: string) {
    activeTabId.value = id;
    const t = tabs.find((x) => x.id === id);
    if (t) url.value = t.url;
    await bridge.tabActivate(id);
    await nextTick();
    schedulePosition();
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
    if (!tabs.length) layout.mainView = "browser";
    await nextTick();
    schedulePosition();
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
  function handleTabRecovery(event: TabRecoveryEvent) {
    bridge.debugLog(
      `tabRecovery id=${event.id} status=${event.status} attempt=${event.attempt}/${event.max_attempts} reason=${event.reason}`
    );
    if (event.status === "recovered") {
      if (event.id === activeTabId.value) nextTick(schedulePosition);
      layout.showToast(`页签已自动恢复: ${event.id}`);
    } else if (event.status === "budget-exhausted") {
      layout.showToast(`页签恢复次数已用尽: ${event.id}`);
    } else if (event.status === "failed") {
      layout.showToast(`页签恢复失败: ${event.id}`);
    } else if (event.status === "load-failed" && event.id === activeTabId.value) {
      layout.showToast(`页面加载失败: ${event.url || event.id}`);
    }
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
    gridSession.value += 1;
    bridge.debugLog(`buildGrid start n=${n} mainView=${layout.mainView}`);
    // createGrid 内部会先 close_grid 再重建，幂等，可安全重复调用。
    // 返回实际创建格数：内存预算守卫在可用内存不足时自动降级（保底 2 格）
    const created = await bridge.createGrid(n);
    // 降级提示存起来，最后用一条 toast 展示（showToast 单条覆盖，先发的会被吞掉）
    const degraded = created < n;
    if (degraded) gridCount.value = created;
    bridge.debugLog("buildGrid createGrid done");
    gridOpen.value = true;
    // 注意：不要在这里覆盖 mainView。由调用方（onItem/grid 工具条）决定切到 grid 视图，
    // 避免 "grid"->"browser" 的二次覆盖打乱 watch 时序导致宫格不显示/页签残留。
    if (layout.mainView !== "grid" && layout.mainView !== "browser") {
      layout.mainView = "grid";
    }
    for (let i = 0; i < created; i++) {
      const u = gridUrls[i] || url.value || "https://www.baidu.com";
      gridUrls[i] = u;
      await bridge.gridOpen(i, u);
    }
    // 等 DOM/子 webview 就绪后重排（scheduleGrid 内部会把激活页签移出屏幕）
    await nextTick();
    bridge.debugLog("buildGrid 导航完成，触发 layoutGrid");
    layoutGrid();
    // 保险：400ms 后再排一次（幂等），覆盖工具条展开/视图切换导致的首次布局时序窗口
    window.setTimeout(() => {
      if (gridOpen.value) layoutGrid();
    }, 400);
    // 宫格打开后激活页签被移出屏幕 → 冻结；格子可见 → 保持运行
    syncFreeze();
    if (degraded) {
      layout.showToast(
        `⚠️ 可用内存不足，已降级为 ${created} 格（每格约需 450MB；可关闭其它应用后重试）`
      );
    } else {
      layout.showToast(`已打开 ${created} 宫格对比`);
    }
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
  // AI 模式：向所有宫格 webview 注入问题并提交
  // 按各格网址匹配站点适配 selector；注入脚本带重试（AI 页面输入框常懒加载），
  // textarea/input 用原生 value setter 触发 React 受控更新，contenteditable 用
  // execCommand('insertText')（ProseMirror/Lexical 均兼容）
  async function gridSendAi() {
    const q = gridAiInput.value.trim();
    if (!q) {
      layout.showToast("请输入问题");
      return;
    }
    const n = gridCount.value;
    layout.showToast(`正在向 ${n} 个宫格注入…`);
    bridge.debugLog(`gridSendAi start n=${n} q=${q}`);
    const fails: string[] = [];
    // 串行错峰提交：每格提交后延迟 GRID_STAGGER_MS 再发下一格，让各 AI 的流式响应
    // 错峰启动，避免 4 个子 webview 同时进入高强度流式渲染导致 WebKitGTK 并发崩溃
    const GRID_STAGGER_MS = 1500;
    for (let i = 0; i < n; i++) {
      const label = `grid-${i}`;
      const cfg = aiAdapterFor(gridUrls[i] || "");
      const js = `
        (function(){
          try {
          var q = ${JSON.stringify(q)};
          var cfg = ${JSON.stringify(cfg)};
          var GENERIC_INPUTS = ['textarea', '[contenteditable="true"]', 'input[type="text"]'];
          var GENERIC_SENDS = ['button[type="submit"]', 'button[class*="send"]', 'button[class*="Send"]',
                               'button[class*="submit"]', 'button[aria-label*="发送"]', 'button[aria-label*="Send"]'];
          function visible(el){ return el && el.getClientRects().length > 0; }
          function findEl(sels){
            for (var i = 0; i < sels.length; i++) {
              var list = document.querySelectorAll(sels[i]);
              for (var j = 0; j < list.length; j++) { if (visible(list[j])) return list[j]; }
            }
            return null;
          }
          // 页内可视反馈：win.eval 无返回值，用徽标把结果画在格子网页顶部
          function badge(msg, ok){
            if (!document.body) return;
            var d = document.createElement('div');
            d.textContent = msg;
            d.style.cssText = 'position:fixed;top:8px;left:50%;transform:translateX(-50%);z-index:2147483647;'
              + 'padding:6px 14px;border-radius:6px;font-size:13px;color:#fff;font-family:sans-serif;'
              + 'background:' + (ok ? '#2b8a3e' : '#c0392b') + ';box-shadow:0 2px 8px rgba(0,0,0,.3);pointer-events:none;';
            document.body.appendChild(d);
            setTimeout(function(){ d.remove(); }, 5000);
          }
          function setValue(el, text){
            el.focus();
            if (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') {
              var proto = el.tagName === 'TEXTAREA' ? window.HTMLTextAreaElement.prototype : window.HTMLInputElement.prototype;
              var desc = Object.getOwnPropertyDescriptor(proto, 'value');
              if (desc && desc.set) desc.set.call(el, text); else el.value = text;
              el.dispatchEvent(new Event('input', { bubbles: true }));
              el.dispatchEvent(new Event('change', { bubbles: true }));
            } else {
              // 全选后插入：触发框架受控更新并清空旧内容
              var sel = window.getSelection();
              var range = document.createRange();
              range.selectNodeContents(el);
              sel.removeAllRanges();
              sel.addRange(range);
              document.execCommand('insertText', false, text);
            }
          }
          function trySubmit(el){
            var btn = findEl(cfg.sends.concat(GENERIC_SENDS));
            if (btn && !btn.disabled) { btn.click(); badge('✓ 已点击发送', true); return; }
            ['keydown', 'keypress', 'keyup'].forEach(function(type){
              el.dispatchEvent(new KeyboardEvent(type, { key: 'Enter', code: 'Enter', keyCode: 13, which: 13, bubbles: true, cancelable: true }));
            });
            badge('✓ 已填入并回车', true);
          }
          var attempts = 0;
          function attempt(){
            attempts++;
            if (!document.body) {
              if (attempts < 15) setTimeout(attempt, 600); // 页面还没加载出来
              return;
            }
            var el = findEl(cfg.inputs) || findEl(GENERIC_INPUTS);
            if (!el) {
              if (attempts < 15) { setTimeout(attempt, 600); return; } // 输入框懒加载，最多等 9 秒
              badge('✗ 未找到输入框（需登录或站点改版）', false);
              return;
            }
            try {
              setValue(el, q);
              setTimeout(function(){
                try { trySubmit(el); } catch(e) { badge('✗ 提交失败: ' + e.message, false); }
              }, 200);
            } catch(e) {
              badge('✗ 填入失败: ' + e.message, false);
            }
          }
          attempt();
          } catch(e) {
            // 注入脚本自身异常：尽量画徽标，页面加载极早期 document 不可用时静默
            try {
              var d = document.createElement('div');
              d.textContent = '✗ 脚本异常: ' + e.message;
              d.style.cssText = 'position:fixed;top:8px;left:8px;z-index:2147483647;padding:6px 10px;background:#c0392b;color:#fff;font-size:12px;border-radius:6px;';
              (document.body || document.documentElement).appendChild(d);
              setTimeout(function(){ d.remove(); }, 8000);
            } catch(_) {}
          }
        })();
      `;
      try {
        bridge.debugLog(`gridSendAi evalInTab[${label}] begin`);
        await bridge.evalInTab(label, js);
        bridge.debugLog(`gridSendAi evalInTab[${label}] done`);
      } catch (e: any) {
        bridge.debugLog(`gridSendAi evalInTab[${label}] fail: ${String(e).slice(0, 60)}`);
        fails.push(`${i + 1}格(${String(e).slice(0, 40)})`);
      }
      // 串行错峰：最后一格无需再等；前面的格子提交后错峰，让流式响应错开启动
      if (i < n - 1) {
        bridge.debugLog(`gridSendAi stagger wait ${GRID_STAGGER_MS}ms before grid-${i + 1}`);
        await new Promise((r) => setTimeout(r, GRID_STAGGER_MS));
      }
    }
    bridge.debugLog(`gridSendAi loop end fails=${fails.length}`);
    if (fails.length) {
      layout.showToast(`⚠️ ${fails.length}/${n} 格注入失败: ${fails[0]}`);
    } else {
      layout.showToast(`已注入 ${n} 格，看各格网页顶部的绿/红徽标`);
    }
    gridAiInput.value = "";
  }
  async function closeGridAll() {
    // 先翻标志位：立刻阻断 scheduleGrid/schedulePosition 走宫格分支，
    // 不等后端 IPC 往返（否则视图已切走、gridOpen 仍为 true 导致重试风暴）
    gridOpen.value = false;
    await bridge.closeGrid().catch(() => {});
    layout.gridToolbarOpen = false;
    gridRects.splice(0, gridRects.length);
    // B9-4 修复：宫格关闭后必须把视图切回浏览器视图，否则
    // (1) isBrowserVisible = !gridOpen && mainView==="browser" 为 false → BrowserHost
    //     内部把浏览器 webview 设为 visibility:hidden；
    // (2) schedulePosition 在 mainView!=="browser" 时直接 return（useBrowserHost.ts）→
    //     活动页签 webview 停在宫格离屏坐标，浏览器区整片空白，须手动切视图才恢复。
    // 切回 browser 会触发 mainView 的 watch → syncViewVisibility → relocate →
    // schedulePosition 把活动页签 webview 重定位回浏览器区（原生 webview 布局不空白）。
    // 仅当当前确处于宫格视图时复位，不打扰其它视图（如正在看 files/term）。
    // 重激活活动页签（后端聚焦），确保关闭宫格后它就是可见页签；无活动页签则跳过。
    if (activeTabId.value) {
      await bridge.tabActivate(activeTabId.value).catch(() => {});
    }
    if (layout.mainView === "grid") {
      layout.mainView = "browser";
    }
    // 宫格关闭后激活页签重新可见 → 解冻
    syncFreeze();
    // 兜底重定位活动页签 webview：此刻 gridOpen 已 false、mainView 若为 browser，
    // schedulePosition 会真正下发 tabPosition；mainView 的 watch 下个 tick 也会再做一次。
    schedulePosition();
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

  // 强制重排宫格（绕过定位层的 rect 去重缓存）。
  // 场景：下拉菜单打开时宫格 webview 被临时移出屏幕，关闭菜单后必须重发定位，
  // 但 rect 没变、会被去重缓存跳过 —— 会话号 +1 即作废缓存触发重发。
  function forceGridRelayout() {
    gridSession.value += 1;
    layoutGrid();
  }

  // ===== 隐藏/非激活 webview 冻结（防后台网页吃 CPU 拖垮终端等主界面） =====
  // 子 webview 是独立进程，不会直接让主 webview 的终端崩溃，但隐藏中的网页
  // 仍在跑 JS 定时器/动画/视频，持续消耗 CPU/GPU。冻结=暂停媒体 + 暂停 CSS 动画
  // + requestAnimationFrame 节流至 1fps（页面脚本误以为仍在正常渲染，恢复时无缝续跑）
  const FREEZE_JS = `
    (function(){
      if (window.__vibeFrozen) return;
      window.__vibeFrozen = true;
      try {
        document.querySelectorAll('video,audio').forEach(function(m){
          if (!m.paused) m.setAttribute('data-vibe-resume', '1');
          m.pause();
        });
        if (!document.getElementById('__vibe-freeze-style')) {
          var st = document.createElement('style');
          st.id = '__vibe-freeze-style';
          st.textContent = '*,*::before,*::after{animation-play-state:paused!important}';
          (document.head || document.documentElement).appendChild(st);
        }
        if (!window.__vibeOrigRAF) {
          window.__vibeOrigRAF = window.requestAnimationFrame.bind(window);
          window.requestAnimationFrame = function(cb){
            return setTimeout(function(){ try { cb(performance.now()); } catch(e){} }, 1000);
          };
        }
      } catch(e) {}
    })();
  `;
  const UNFREEZE_JS = `
    (function(){
      if (!window.__vibeFrozen) return;
      window.__vibeFrozen = false;
      try {
        var st = document.getElementById('__vibe-freeze-style');
        if (st) st.remove();
        if (window.__vibeOrigRAF) {
          window.requestAnimationFrame = window.__vibeOrigRAF;
          window.__vibeOrigRAF = null;
        }
        document.querySelectorAll('video[data-vibe-resume],audio[data-vibe-resume]').forEach(function(m){
          m.removeAttribute('data-vibe-resume');
          m.play().catch(function(){});
        });
      } catch(e) {}
    })();
  `;
  // 按当前视图/激活状态同步冻结：只有"看得见的" webview 保持运行
  function syncFreeze() {
    const inBrowserView = layout.mainView === "browser" || layout.mainView === "grid";
    for (const t of tabs) {
      const visible = layout.mainView === "browser" && t.id === activeTabId.value;
      bridge.evalInTab(t.id, visible ? UNFREEZE_JS : FREEZE_JS).catch(() => {});
    }
    if (gridOpen.value) {
      // 宫格打开时所有格子都可见
      for (let i = 0; i < gridCount.value; i++) {
        bridge.evalInTab(`grid-${i}`, layout.mainView === 'grid' ? UNFREEZE_JS : FREEZE_JS).catch(() => {});
      }
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
    bridge.debugLog(`syncViewVisibility view=${layout.mainView}`);
    if (layout.mainView === "browser" || layout.mainView === "grid") {
      await bridge.hideAllWebviews().catch(() => {});
      relocate();
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

  return {
    url,
    tabs,
    activeTabId,
    gridOpen,
    gridSession,
    gridCount,
    gridUrl,
    gridUrls,
    gridLayout,
    gridMode,
    gridAiInput,
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
    buildGrid,
    layoutGrid,
    gridSetUrl,
    gridSendAi,
    closeGridAll,
    closeGridOne,
    gridCols,
    setResources,
    clearResources,
    gotoAI,
    openBrowser,
    bindPositionScheduler,
    bindGridScheduler,
    relocate,
    forceGridRelayout,
  };
});
