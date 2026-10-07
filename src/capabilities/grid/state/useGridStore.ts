import { defineStore } from "pinia";
import { ref, reactive, computed, nextTick, watch } from "vue";
import { bridge } from "../../../bridge";
import { WEBVIEW_FREEZE_JS, WEBVIEW_UNFREEZE_JS } from "../../../utils/webviewFreeze";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useBrowserStore } from "../../browser/public";
import { isGridResourceAllowed } from "../resource/guard";
import { registerGridLifecycleBinding } from "../lifecycle";

export const useGridStore = defineStore("grid", () => {
  const layout = useLayoutStore();
  const browser = useBrowserStore();

  const gridOpen = ref(false);
  const gridSession = ref(0);
  const gridCount = ref(4);
  const gridUrl = ref("");
  const gridUrls = reactive<string[]>([
    "https://www.doubao.com",
    "https://kimi.moonshot.cn",
    "https://tongyi.aliyun.com/qianwen/",
    "https://chat.deepseek.com",
    ...Array(8).fill(""),
  ]);
  const gridLayout = ref<"horizontal" | "quad" | "grid">("quad");
  const gridMode = ref<"browse" | "ai">("ai");
  const gridAiInput = ref("");
  const gridRects = reactive<{ x: number; y: number; w: number; h: number }[]>([]);

  // ===== 各 AI 站点输入框/发送按钮适配表（按 hostname 匹配，数组内按优先级尝试） =====
  // 未命中的站点走通用兜底：textarea → contenteditable → input[type=text]，Enter 提交
  const T = "textarea";
  const CE = "div[contenteditable='true']";
  const SEND = ["button[class*='send']", "button[class*='Send']"];
  const ARIA_SEND = "button[aria-label*='Send']";
  const ARIA_SUBMIT = "button[aria-label*='Submit']";
  const NONE: string[] = [];
  const AI_SITE_ADAPTERS: Record<string, { inputs: string[]; sends: string[] }> = {
    "www.doubao.com": { inputs: ["textarea[data-testid='chat_input_input']", T], sends: ["button[data-testid='chat_input_send_button']"] },
    "kimi.moonshot.cn": { inputs: ["div.chat-input-editor[contenteditable='true']", CE], sends: NONE },
    "www.kimi.com": { inputs: ["div.chat-input-editor[contenteditable='true']", CE], sends: NONE },
    "chat.deepseek.com": { inputs: ["#chat-input", T], sends: NONE },
    "tongyi.aliyun.com": { inputs: [T], sends: SEND },
    "www.tongyi.com": { inputs: [T, CE], sends: SEND },
    "yiyan.baidu.com": { inputs: [CE, T], sends: NONE },
    "chatglm.cn": { inputs: [T], sends: NONE },
    "xinghuo.xfyun.cn": { inputs: [T], sends: NONE },
    "chatgpt.com": { inputs: ["#prompt-textarea", CE], sends: ["button[data-testid='send-button']", ARIA_SEND] },
    "claude.ai": { inputs: ["div.ProseMirror[contenteditable='true']", CE], sends: [ARIA_SEND] },
    "gemini.google.com": { inputs: ["div.ql-editor[contenteditable='true']", CE], sends: ["button.send-button", ARIA_SEND, "button[aria-label*='发送']"] },
    "copilot.microsoft.com": { inputs: ["textarea#userInput", CE, T], sends: ["button[aria-label*='提交']", ARIA_SUBMIT] },
    "www.perplexity.ai": { inputs: [T], sends: [ARIA_SUBMIT] },
    "grok.x.com": { inputs: [T, CE], sends: ["button[type='submit']"] },
    "poe.com": { inputs: [T], sends: ["button[class*='ChatMessageSendButton']", "button[class*='send']"] },
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


  function computeDesiredVisibility(mv: string, go: boolean) {
    return { gridVisible: go && mv === "grid" };
  }
  const desiredGridVisibility = computed(
    () => computeDesiredVisibility(layout.mainView, gridOpen.value).gridVisible
  );

  let scheduleGrid: () => void = () => {};
  function bindGridScheduler(fn: () => void) { scheduleGrid = fn; }

  // Grid 只负责自己的子 WebView 冻结；普通 tab 仍由 Browser owner 管理。
  function syncGridFreeze() {
    if (!gridOpen.value) return;
    for (let i = 0; i < gridCount.value; i++) {
      bridge.evalInTab(`grid-${i}`, layout.mainView === "grid" ? WEBVIEW_UNFREEZE_JS : WEBVIEW_FREEZE_JS).catch(() => {});
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
    // ===== H-G RELEASE BLOCKER 修复：capability-owned 重资源准入闸 =====
    // Grid 资源（宫格子进程 + 原生 WebView）是 Grid capability-owned 重资源，
    // 只能由 Grid Capability 受控的生命周期路径创建。
    // 这是全仓**唯一**的 createGrid 调用点，故在此处收口即可覆盖任何调用来源
    // （已知入口、历史遗留入口、未来新增入口），而不是逐个调用方打补丁。
    //
    // 判定输入只有「能力可用性 + 激活态」一份编排真源：
    //   preference(UI 偏好) ≠ availability ≠ activation ≠ resource existence
    // Browser absent（framework-only）→ 直接拒绝，grid-child 恒为 0。
    // 禁止改写成 profile 名判断 / 环境变量判断 / browser 布尔第二真源。
    if (!isGridResourceAllowed()) {
      bridge.debugLog(
        "buildGrid refused: grid capability not ACTIVE (resource isolation gate)"
      );
      return;
    }
    const n = gridCount.value;
    gridSession.value += 1;
    bridge.debugLog(`buildGrid start n=${n} mainView=${layout.mainView}`);
    // 每格首导航 URL：优先格子自身配置，其次当前地址栏；未配置用 about:blank 占位。
    // 严禁回退第三方站点（如百度）——子 webview 首导航即目标服务，消除"先百度再跳真实服务"双导航。
    const urls: string[] = [];
    for (let i = 0; i < n; i++) {
      const u = (gridUrls[i] || browser.url || "").trim() || "about:blank";
      gridUrls[i] = u;
      urls.push(u);
    }
    // createGrid 内部会先 close_grid 再重建，幂等，可安全重复调用。
    // 返回实际创建格数：内存预算守卫在可用内存不足时自动降级（保底 2 格）
    const created = await bridge.createGrid(n, urls);
    // 降级提示存起来，最后用一条 toast 展示（showToast 单条覆盖，先发的会被吞掉）
    const degraded = created < n;
    if (degraded) gridCount.value = created;
    bridge.debugLog("buildGrid createGrid done");
    gridOpen.value = true;
    // 注意：不要在这里覆盖 mainView。由调用方（onItem/grid 工具条）决定切到 grid 视图，
    // 避免 "grid"->"browser" 的二次覆盖打乱 watch 时序导致宫格不显示/页签残留。
    // 资源已就绪后，若当前不在浏览器/宫格类视图，则切到 grid 视图（经 setView，单一变更入口）。
    if (layout.mainView !== "grid" && layout.mainView !== "browser") {
      layout.setView("grid");
    }
    // 子 webview 已由 create_grid 按上述 urls 完成首次导航，无需再逐个 gridOpen。
    // 等 DOM/子 webview 就绪后重排（scheduleGrid 内部会把激活页签移出屏幕）
    await nextTick();
    bridge.debugLog("buildGrid 导航完成，触发 layoutGrid");
    layoutGrid();
    // 保险：400ms 后再排一次（幂等），覆盖工具条展开/视图切换导致的首次布局时序窗口
    window.setTimeout(() => {
      if (gridOpen.value) layoutGrid();
    }, 400);
    // 宫格打开后激活页签被移出屏幕 → 冻结；格子可见 → 保持运行
    syncGridFreeze();
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
    const u = gridUrls[i] || browser.url || "https://www.baidu.com";
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
      const js = `(function(){try{var q=${JSON.stringify(q)},cfg=${JSON.stringify(cfg)},GI=['textarea','[contenteditable="true"]','input[type="text"]'],GS=['button[type="submit"]','button[class*="send"]','button[class*="Send"]','button[class*="submit"]','button[aria-label*="发送"]','button[aria-label*="Send"]'];function v(e){return e&&e.getClientRects().length>0}function f(a){for(var i=0;i<a.length;i++){var l=document.querySelectorAll(a[i]);for(var j=0;j<l.length;j++)if(v(l[j]))return l[j]}return null}function b(m,o){if(!document.body)return;var d=document.createElement('div');d.textContent=m;d.style.cssText='position:fixed;top:8px;left:50%;transform:translateX(-50%);z-index:2147483647;padding:6px 14px;border-radius:6px;font-size:13px;color:#fff;font-family:sans-serif;background:'+(o?'#2b8a3e':'#c0392b')+';box-shadow:0 2px 8px rgba(0,0,0,.3);pointer-events:none;';document.body.appendChild(d);setTimeout(function(){d.remove()},5000)}function sv(e,t){e.focus();if(e.tagName==='TEXTAREA'||e.tagName==='INPUT'){var p=e.tagName==='TEXTAREA'?window.HTMLTextAreaElement.prototype:window.HTMLInputElement.prototype,x=Object.getOwnPropertyDescriptor(p,'value');x&&x.set?x.set.call(e,t):e.value=t;e.dispatchEvent(new Event('input',{bubbles:true}));e.dispatchEvent(new Event('change',{bubbles:true}))}else{var s=window.getSelection(),r=document.createRange();r.selectNodeContents(e);s.removeAllRanges();s.addRange(r);document.execCommand('insertText',false,t)}}function sub(e){var x=f(cfg.sends.concat(GS));if(x&&!x.disabled){x.click();b('✓ 已点击发送',true);return}['keydown','keypress','keyup'].forEach(function(t){e.dispatchEvent(new KeyboardEvent(t,{key:'Enter',code:'Enter',keyCode:13,which:13,bubbles:true,cancelable:true}))});b('✓ 已填入并回车',true)}var a=0;function go(){a++;if(!document.body){if(a<15)setTimeout(go,600);return}var e=f(cfg.inputs)||f(GI);if(!e){if(a<15){setTimeout(go,600);return}b('✗ 未找到输入框（需登录或站点改版）',false);return}try{sv(e,q);setTimeout(function(){try{sub(e)}catch(x){b('✗ 提交失败: '+x.message,false)}},200)}catch(x){b('✗ 填入失败: '+x.message,false)}}go()}catch(e){try{var d=document.createElement('div');d.textContent='✗ 脚本异常: '+e.message;d.style.cssText='position:fixed;top:8px;left:8px;z-index:2147483647;padding:6px 10px;background:#c0392b;color:#fff;font-size:12px;border-radius:6px;';(document.body||document.documentElement).appendChild(d);setTimeout(function(){d.remove()},8000)}catch(_){}}})();`;
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
    if (browser.activeTabId) {
      await browser.tabSwitch(browser.activeTabId).catch(() => {});
    }
    if (layout.mainView === "grid") {
      layout.setView("browser");
    }
    // 宫格关闭后激活页签重新可见 → 解冻
    syncGridFreeze();
    // 兜底重定位活动页签 webview：此刻 gridOpen 已 false、mainView 若为 browser，
    // relocate 会真正下发 tabPosition；mainView 的 watch 下个 tick 也会再做一次。
    browser.relocate();
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

  // ===== Grid canonical Intent API =====
  // 组件只调以下语义化入口，禁止手拼 buildGrid/closeGridAll/gridCloseOne 与 mainView。
  // openGrid：仅确保 Grid 资源存在（不导航）；已存在则不重建。
  function openGrid() {
    if (!gridOpen.value) return buildGrid();
  }
  // rebuildGrid：按当前 gridCount/gridLayout **重建**资源（格数/布局变更必须走它）。
  // 与 openGrid 的区别：后端 webview 数量须与 gridCount 一致，只重排会对不存在的
  // grid-N 发定位（tab not found），故改格数/切四分必须真重建而非 no-op。
  function rebuildGrid() {
    return buildGrid();
  }
  // closeGrid：仅销毁整个 Grid resource（不导航、不隐藏）。
  // 若当前 mainView==="grid"，视图收敛由下方 invariant guard 自动派生 activateBrowser。
  function closeGrid() {
    return closeGridAll();
  }
  // closeGridCell：关闭单个宫格（销毁该子窗，其余保留）。
  function closeGridCell(i: number) {
    return closeGridOne(i);
  }
  // activateGrid：进入 Grid 主表面（资源已存在则重排，否则创建）。
  // 与历史行为一致：openModule("grid") + (gridOpen ? layoutGrid : buildGrid)。
  function activateGrid() {
    layout.openModule("grid");
  }

  // 状态不变量自动收敛（唯一 reconciliation owner）：
  // mainView === "grid" 必须 ⇒ gridOpen === true。
  // 仅在 Grid 资源**消失**（close，gridOpen true→false）而视图仍在 grid 时收敛。
  // ⚠️ 创建中 mainView 先切到 grid、gridOpen 仍为 false 是**合法中间态**
  // （buildGrid 的 createGrid 是异步 IPC，gridOpen 在其 resolve 后才置 true）；
  // 若仅按当前值收敛，首次打开宫格会被弹回 browser、宫格不显示（须点两次）。
  // 故用 gridOpen 的**前值**区分"正在创建"与"资源已消失"。
  watch(
    () => [layout.mainView, gridOpen.value] as const,
    ([mv, go], prev) => {
      const prevGo = prev ? prev[1] : false;
      if (mv === "grid" && !go && prevGo) layout.setView("browser");
    }
  );

  // ===== Shell UI preference → Capability 资源生命周期（依赖方向纠正）=====
  //
  // 背景（H-G blocker 根因之一）：
  //   旧实现把「Shell 的 gridToolbarOpen 偏好」直接翻译成
  //   `browser.openGrid()` / `browser.closeGrid()`，写在 Core 层 useLayoutStore 里，
  //   形成 Framework Core → Browser internal lifecycle 的反向业务依赖，
  //   并把「持久化 UI 偏好」错误地当成「资源创建许可」。
  //
  // 纠正后的依赖方向：
  //   Shell（useLayoutStore）只持有 gridToolbarOpen 这一份**纯 UI 偏好**，
  //   不再 import / 调用任何 Browser 内部；
  //   Browser（本 store，能力侧）**单向**读取该公开偏好，并由能力自己决定
  //   是否把它翻译成资源生命周期 —— 且必须先过 isGridResourceAllowed() 闸。
  //
  // 语义边界（缺一即复发）：
  //   preference（本 watch 的输入）≠ availability ≠ activation ≠ resource existence
  //   Browser absent → 闸为 false → 偏好再怎么是 true 也不创建资源（CASE A/B）。
  //   Browser present → 沿用原 toggle 语义（CASE C/D）。
  //   能力变不可用 → 闸为 false → 绝不重建（CASE E）。
  watch(
    () => layout.gridToolbarOpen,
    (open) => {
      if (!isGridResourceAllowed()) return;
      if (open && !gridOpen.value) void openGrid();
      else if (!open && gridOpen.value) void closeGrid();
    }
  );



  function forceGridRelayout() {
    gridSession.value += 1;
    layoutGrid();
  }

  async function syncGridVisibility() {
    if (desiredGridVisibility.value) layoutGrid();
    syncGridFreeze();
  }

  watch(
    () => layout.mainView,
    (view) => {
      if (view === "grid" && !gridOpen.value && isGridResourceAllowed()) void openGrid();
      nextTick(syncGridVisibility);
    },
  );

  async function suspendGridResources() {
    if (!gridOpen.value) return;
    for (let i = 0; i < gridCount.value; i++) {
      await bridge.evalInTab(`grid-${i}`, WEBVIEW_FREEZE_JS).catch(() => {});
      await bridge.gridPosition(i, { x: -30000, y: 0, width: 100, height: 100 }).catch(() => {});
    }
    if (layout.mainView === "grid") layout.setView("browser");
  }

  async function resumeGridResources() {
    if (!gridOpen.value) return;
    if (layout.mainView === "grid") {
      await nextTick();
      layoutGrid();
      syncGridFreeze();
    }
  }

  registerGridLifecycleBinding({
    resume: resumeGridResources,
    suspend: suspendGridResources,
    deactivate: closeGridAll,
  });

  return {
    gridOpen,
    gridSession,
    gridCount,
    gridUrl,
    gridUrls,
    gridLayout,
    gridMode,
    gridAiInput,
    gridRects,
    desiredGridVisibility,
    computeDesiredVisibility,
    gridCols,
    buildGrid,
    layoutGrid,
    gridSetUrl,
    gridSendAi,
    closeGridAll,
    closeGridOne,
    openGrid,
    rebuildGrid,
    closeGrid,
    closeGridCell,
    activateGrid,
    bindGridScheduler,
    forceGridRelayout,
  };
});
