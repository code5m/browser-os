<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  useLayoutStore,
  TOP_NAV_ITEMS,
  nextNavIndex,
} from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";
import { useAppsStore } from "../../capabilities/apps/public";
import { useTerminalStore } from "../../capabilities/terminal/public";
import { useWorkspaceStore } from "../../capabilities/workspace/public";
import { useArtifactStore } from "../../capabilities/workspace/public";
import { useFileStore } from "../../capabilities/workspace/public";
import { redactSecrets } from "../../utils/redact";
import { filterOmnibox, isLocalPath, safeRecentDirs, type OmniboxItem } from "../../utils/omniboxSuggestions";
import { bridge } from "../../bridge";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
import { ArrowLeft, ArrowRight, RotateCw, House, Ellipsis, SlidersHorizontal, ChevronDown } from '@lucide/vue';
import { createNativeMenu, menuAnchor, type NativeMenuEntry } from '../../composables/nativeMenu';

const layout = useLayoutStore();
const browser = useBrowserStore();
const appsStore = useAppsStore();
const term = useTerminalStore();
const ws = useWorkspaceStore();
const art = useArtifactStore();
const fs = useFileStore();
const addressFocused=ref(false);
const contextualAddress=computed(()=>layout.mainView==='browser' ? browser.url
  : layout.modTabs.find(t=>t.id===layout.activeModTab)?.path || 'browseros://'+layout.mainView);
const addressText=ref(contextualAddress.value);
watch(contextualAddress,value=>{if(!addressFocused.value)addressText.value=value;});

// 收藏夹等贡献：经通用 Contribution Registry 按 slot 遍历渲染。
// Shell 不持有 Bookmark 专属知识（不 import 其 store / ui），C3 关键（8B.1）。
const addressBarActions = computed(() => contributionRegistry.getNavigationContributions(
  CONTRIBUTION_SLOTS.ADDRESS_BAR_ACTIONS,
));
const omniContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.OMNIBOX_SUGGESTIONS,
));
const activityNavContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_NAV,
));
const activityRowContributions = computed(() => contributionRegistry.getSurfaceContributions(
  CONTRIBUTION_SLOTS.ACTIVITY_BAR_ROWS,
));

// 一级入口与 ☰ 菜单分节统一来自 useLayoutStore（W17 导航真源），
// 窄窗口按 navTopViews 从尾部裁剪，被裁掉的入口在 ☰ 菜单中仍可达。
const topItems = computed(() => TOP_NAV_ITEMS.filter((i) => layout.navTopViews.includes(i.view)));

// ===== W17：窗口宽度上报（窄窗口密度）+ resize 监听清理 =====
const navEl = ref<HTMLElement | null>(null);
const omniAnchor = ref<HTMLElement | null>(null);
// Native Linux browser/grid WebViews sit ABOVE the HTML shell. Only workbench
// surfaces without visible native children may use a CSS anchored overlay.
// Browser/grid use native menus instead; typing must not move their viewport.
const omniOverlayAllowed = computed(() =>
  layout.mainView !== "browser" && layout.mainView !== "grid",
);
const suggestionsGeometry = ref<Record<string, string>>({});
let suggestionsObserver: ResizeObserver | null = null;
function syncSuggestionsGeometry() {
  const anchor = omniAnchor.value;
  if (!anchor || !anchor.offsetWidth) return;
  // .tbar is the positioned ancestor of .omni-wrap: offsetLeft is already
  // relative to the popover container in CSS pixels, including responsive nav.
  suggestionsGeometry.value = {
    "--omni-left": anchor.offsetLeft + "px", width: anchor.offsetWidth + "px",
  };
}
const nativeMenu=createNativeMenu(()=>layout.showToast('无法打开原生菜单，请在桌面客户端重试'));
async function toggleMoreActions(event:MouseEvent) {
  layout.navSection = '';
  const items:NativeMenuEntry[]=[
    {text:'工作区',children:topItems.value.map(it=>({text:it.label,action:()=>onItem(it.view)}))},
    {text:'文件侧栏',enabled:layout.mainView==='browser',action:()=>layout.toggleBrowserDock('files')},
    {text:'终端侧栏',enabled:layout.mainView==='browser',action:()=>layout.toggleBrowserDock('term')},
    {text:'紧凑模式',checked:layout.shellMode==='compact',action:()=>changeShellMode(layout.shellMode === 'compact' ? 'standard' : 'compact')},
    {text:'沉浸全屏 · F11',action:toggleImmersive},
    {text:'AI 导航',enabled:layout.mainView==='browser',action:()=>browser.toggleAiNav()},
    {text:'采集选中内容',action:()=>art.collectSelection()},
    {text:'设置',action:()=>onItem('settings')},
  ];
  await nativeMenu.open(items,menuAnchor(event.currentTarget as Element));
}
function syncWidth() {
  layout.setWindowWidth(window.innerWidth || 0);
  // ResizeObserver tracks the resized address field after responsive nav layout.
}
onMounted(() => {
  syncWidth();
  window.addEventListener("resize", syncWidth);
  // Observe the input width independently of window resize: nav density and
  // contributions can change the address field without resizing the window.
  if (typeof ResizeObserver !== "undefined" && omniAnchor.value) {
    suggestionsObserver = new ResizeObserver(syncSuggestionsGeometry);
    suggestionsObserver.observe(omniAnchor.value);
  }
});
onBeforeUnmount(() => {
  window.removeEventListener("resize", syncWidth);
  suggestionsObserver?.disconnect();
  nativeMenu.dispose();
});
// Omnibox in file/database/tool pages is anchored and floats over HTML.
// On browser/grid, explicit recent-address actions use a native menu.
// layout.navSection remains the sole owner of the open/closed state.
function changeShellMode(mode: "standard" | "compact" | "immersive") {
  window.dispatchEvent(new CustomEvent("browseros:set-shell-mode", { detail: mode }));
}
function toggleImmersive() {
  changeShellMode("immersive");
}

// W17：活动条键盘漫游 —— ←/→ 在入口间环绕移动焦点，Home/End 直达首尾
function onNavKeydown(e: KeyboardEvent) {
  const origin = e.target as HTMLElement;
  if (origin.tagName === "INPUT" || origin.tagName === "TEXTAREA" || origin.isContentEditable) return;
  const nav = navEl.value;
  if (!nav) return;
  const items = Array.from(nav.querySelectorAll<HTMLElement>("[data-nav-item]"));
  if (!items.length) return;
  const cur = items.indexOf(document.activeElement as HTMLElement);
  let next = -1;
  if (e.key === "ArrowRight") next = nextNavIndex(cur < 0 ? -1 : cur, 1, items.length);
  else if (e.key === "ArrowLeft") next = nextNavIndex(cur < 0 ? 0 : cur, -1, items.length);
  else if (e.key === "Home") next = 0;
  else if (e.key === "End") next = items.length - 1;
  else return;
  e.preventDefault();
  items[next]?.focus();
}

// W17：Esc 收起扩展行并把焦点还给触发它的按钮（键盘用户不会丢失焦点位置）
function onEscape() {
  const open = layout.navSection;
  if (!open) return;
  layout.closeNavSection();
  nextTick(() => {
    document.querySelector<HTMLElement>(`[data-nav-toggle="${open}"]`)?.focus();
  });
}

async function onItem(v: string) {
  if (term.m0Cfg?.driver) {
    bridge.debugLog(`[M0] ignore activity item ${v} while driver=${term.m0Cfg.driver}`);
    return;
  }
  layout.navSection = "";
  // 离开宫格视图**不再**自动关闭宫格（新语义 HIDE ≠ CLOSE）：资源存活但隐藏。
  // 历史曾因 gridOpen 悬挂为 true 导致浏览视图定位走错分支（tab 不复位、宫格被拉回
  // 可视区、非浏览器视图空转重试），现已由 syncViewVisibility 的 Visibility Controller
  // 按 desiredGridVisibility 统一收敛，不再依赖"离开即销毁"。
  if (v === "apps") appsStore.loadApps();
  // 浏览器主视图不是模块页签，直接切视图即可
  if (v === "browser") {
    layout.activateBrowser();
    return;
  }
  // 菜单功能与浏览器一致：点一个就新建/复用一个标签
  layout.openModule(v as any);
}

// ===== 智能地址栏（omnibox）：网址/目录自动识别，目录与网页都在中央主区展示 =====
const RECENT_DIRS_KEY = "browser-os-recent-dirs";
const recentDirs = ref<string[]>(loadRecentDirs());
const omniIndex = ref(-1);
// Only keystrokes trigger filtering: merely focusing an already-open URL must
// show all recent web/file/folder targets (and contributed bookmarks).
const omniTyped = ref(false);
const omniCandidates = computed(() => {
  const items: OmniboxItem[] = [
    ...ws.recents.filter(r => r.type === "url").map(r => ({ value: r.path, label: safeLabel(r.title || r.path), type: isLocalPath(r.path) ? "最近目录" : "最近网页", kind: isLocalPath(r.path) ? "dir" as const : "web" as const })),
    ...ws.recents.filter(r => r.type === "file").map(r => ({ value: r.path, label: safeLabel(r.title || r.path), type: "最近文件", kind: "file" as const })),
    ...recentDirs.value.map(value => ({ value, label: safeLabel(value), type: "最近目录", kind: "dir" as const })),
    ...fs.startDirs.map(d => ({ value: d.path, label: safeLabel(d.name || d.path), type: "常用目录", kind: "dir" as const })),
  ];
  return filterOmnibox(items, addressText.value, omniTyped.value, layout.addrMode);
});
function safeLabel(s: string): string { return redactSecrets(s || ""); }
function loadRecentDirs(): string[] {
  try { return safeRecentDirs(JSON.parse(localStorage.getItem(RECENT_DIRS_KEY) || "[]")); }
  catch { return []; }
}
function recordRecentDir(path: string) {
  recentDirs.value = safeRecentDirs([path, ...recentDirs.value.filter(p => p !== path)]);
  try { localStorage.setItem(RECENT_DIRS_KEY, JSON.stringify(recentDirs.value)); } catch {}
}
function showOmnibox(event:FocusEvent) {
  addressFocused.value=true;
  (event.target as HTMLInputElement).select();
  omniTyped.value = false;
  omniIndex.value = -1;
  layout.navSection = omniOverlayAllowed.value ? 'omni' : '';
  nextTick(syncSuggestionsGeometry);
}
function blurAddress(){addressFocused.value=false;layout.closeNavSection();}
async function showRecentAddresses() {
  layout.closeNavSection();
  const entries=omniCandidates.value.map(s=>({text:s.label,action:()=>chooseSuggestion(s)}));
  await nativeMenu.open(entries,menuAnchor(omniAnchor.value!));
}
function chooseSuggestion(s: OmniboxItem) {
  // Older releases could mistakenly record an absolute directory as a web URL.
  // It must never be sent to a search engine when replayed from recents.
  if (isLocalPath(s.value) || s.kind === "dir") pickDir(s.value);
  else if (s.kind === "file") {
    layout.closeNavSection();
    const record = ws.recents.find(r => r.type === "file" && r.path === s.value);
    if (record) ws.openRecent(record);
  } else pickUrl(s.value);
}
function onOmniKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    if(!omniOverlayAllowed.value){void showRecentAddresses();return;}
    omniIndex.value = (omniIndex.value + (e.key === "ArrowDown" ? 1 : -1) + omniCandidates.value.length) % (omniCandidates.value.length || 1);
  } else if (e.key === "Escape") {
    e.stopPropagation(); layout.closeNavSection(); omniIndex.value = -1; addressText.value=contextualAddress.value;
  } else if (e.key === "Enter") {
    e.preventDefault();
    const match = omniCandidates.value[omniIndex.value];
    omniIndex.value = -1;
    if (match) chooseSuggestion(match); else onAddrGo();
  }
}
function onAddrGo() {
  layout.closeNavSection();
  if(addressText.value===contextualAddress.value && layout.mainView!=='browser')return;
  browser.url=addressText.value;
  addressFocused.value=false;
  // Absolute local paths take precedence over the page/search mode.
  // Even an explicitly selected webpage mode must not upload a local path.
  if (isLocalPath(browser.url) || layout.addrMode === "dir")
    void openDirCenter().catch(() => layout.showToast("无法打开目录，请检查路径"));
  else void browser.navigateCurrent().catch(() => layout.showToast("网页导航失败，请检查网址"));
}
function pickDir(p: string) {
  layout.closeNavSection();
  addressFocused.value=false;addressText.value=p;
  browser.url = p;
  void openDirCenter().catch(() => layout.showToast("无法打开目录，请检查路径"));
}
function pickUrl(u: string) {
  if (isLocalPath(u)) { pickDir(u); return; }
  layout.closeNavSection();
  addressFocused.value=false;addressText.value=u;
  browser.url = u;
  void browser.navigateCurrent().catch(() => layout.showToast("网页导航失败，请检查网址"));
}
// 目录在中央主区展示（与网页同一套展示区域）
async function openDirCenter() {
  const p = browser.url.trim();
  if (!p) {
    layout.showToast("请输入目录路径");
    return;
  }
  await fs.enterDir(p);
  await browser.consumeActiveBlankTab();
  layout.browserDockOpen = false;
  layout.openDirTab(p, true);
  layout.leftTab = "files";
  recordRecentDir(p);
  layout.showToast("📁 " + p);
}
</script>

<template>
  <div class="tbar" :class="'nav-' + layout.navDensity" @keydown.esc="onEscape">
    <nav ref="navEl" class="activity" aria-label="主导航" @keydown="onNavKeydown">

      <!-- 中：智能地址栏 -->
      <div class="addr-mid">
        <button class="tbtn" aria-label="后退" :disabled="!layout.isBrowserView()" @click="browser.goBack" title="后退"><ArrowLeft :size="18"/></button>
        <button class="tbtn" aria-label="前进" :disabled="!layout.isBrowserView()" @click="browser.goForward" title="前进"><ArrowRight :size="18"/></button>
        <button class="tbtn" aria-label="刷新" :disabled="!layout.isBrowserView()" @click="browser.reloadActive" title="刷新"><RotateCw :size="17"/></button>
        <button class="tbtn" data-nav-item aria-label="主页" title="主页" @click="onItem('home')"><House :size="18"/></button>
        <div ref="omniAnchor" class="omni-wrap">
          <button type="button" class="omni-mode" aria-label="切换地址栏模式" :title="'地址栏模式：' + (layout.addrMode === 'auto' ? '自动' : layout.addrMode === 'url' ? '网页' : '文件')"
            @mousedown.prevent @click="layout.toggleAddrMode()"><SlidersHorizontal :size="15"/></button>
          <!-- 聚焦即自动展开最近/常用（无小三角）；选中或回车后自动收起 -->
          <input
            v-model="addressText"
            aria-label="地址栏"
            data-nav-toggle="omni"
            role="combobox" aria-autocomplete="list" :aria-expanded="omniOverlayAllowed && layout.navSection === 'omni'"
            :aria-activedescendant="omniIndex >= 0 ? 'omni-suggestion-' + omniIndex : undefined"
            aria-controls="nav-omni-row"
            placeholder="输入网址或目录路径，回车打开"
            @keydown="onOmniKeydown"
            @input="omniIndex = -1; omniTyped = true; layout.navSection=omniOverlayAllowed ? 'omni' : ''"
            @blur="blurAddress"
            @focus="showOmnibox"
          />
          <button type="button" class="omni-mode" aria-label="最近地址" title="最近地址（方向键也可打开）" aria-haspopup="menu" @mousedown.prevent @click="showRecentAddresses"><ChevronDown :size="15"/></button>
          <!-- Bookmark capability owns the favorite action; Shell only mounts contributions. -->
          <template v-for="c in addressBarActions" :key="c.id">
            <component :is="c.component" />
          </template>
        </div>

      </div>

      <template v-for="c in activityNavContributions" :key="c.id"><component :is="c.component" /></template>
      <button class="tbtn action-overflow" data-nav-item data-nav-toggle="more-actions" aria-label="更多操作" title="更多操作"
        aria-haspopup="menu" @click="toggleMoreActions"><Ellipsis :size="21"/></button>
    </nav>

    <!-- HTML suggestions exist only over HTML workbenches. Native views use the menu adapter. -->
    <div v-if="omniOverlayAllowed && layout.navSection === 'omni'" id="nav-omni-row" class="suggestion-list"
      :class="{ 'suggestion-floating': omniOverlayAllowed }" :style="suggestionsGeometry"
      role="listbox" aria-label="地址建议">
      <button v-for="(suggestion, i) in omniCandidates" :key="suggestion.value" :id="'omni-suggestion-' + i" role="option" :aria-selected="omniIndex === i"
        class="suggestion-item" :class="{ selected: omniIndex === i }" @mousedown.prevent @click="chooseSuggestion(suggestion)">
        <span class="suggestion-type">{{ suggestion.kind === 'web' ? '🌐' : suggestion.kind === 'file' ? '📄' : '📁' }}</span>
        <span class="suggestion-main">{{ suggestion.label }}</span><span class="suggestion-sub">{{ suggestion.type }}</span>
      </button>
      <p v-if="!omniCandidates.length && !omniContributions.length" class="suggestion-empty">输入网址、文件或目录，按 Enter 打开</p>
      <component v-for="c in omniContributions" :key="c.id" :is="c.component"
        :query="omniTyped ? addressText : ''" :mode="layout.addrMode"
        @choose-url="pickUrl" @choose-dir="pickDir" />
    </div>
    <template v-for="c in activityRowContributions" :key="c.id">
      <component :is="c.component" />
    </template>


  </div>
</template>

<style scoped>
.tbar {
  flex-shrink: 0;
  position: relative;
  z-index: 4;
}
.activity{height:46px;box-sizing:border-box;background:var(--chrome-toolbar,#fff);color:var(--chrome-text,#3c4043);border-bottom:0;gap:4px;padding:5px 9px;font-family:system-ui,sans-serif}
.activity button{color:var(--chrome-text,#3c4043);border-radius:50%;flex:none}

.more-actions { display:flex; gap:6px; flex-wrap:wrap; align-items:center; padding:7px 12px; background:#f8fafc; border-bottom:1px solid #e2e8f0; }
.more-actions button { border:1px solid #d8e1eb; background:#fff; color:#344054; border-radius:7px; padding:5px 10px; font-size:12px; cursor:pointer; }
.more-actions button:hover { background:#eaf2fb; }
.more-heading { color:#667085; font-size:11px; font-weight:600; }
.more-divider { width:1px; height:18px; background:#dce4ee; margin:0 3px; }
.action-overflow { font-weight:700; font-size:17px; }
.activity button:hover { background:var(--chrome-hover,#e8eaed); color:var(--chrome-text,#3c4043); }
.activity button.active { background:#e3eefb; color:#17548f; }
.suggestion-list { display:flex; flex-direction:column; box-sizing:border-box; margin-left:var(--omni-left,0px); max-height:min(220px,50vh); overflow-y:auto; background:#fff; border:1px solid #dce3eb; box-shadow:0 8px 20px #13233b12; border-radius:0 0 12px 12px; padding:5px; }
.suggestion-list.suggestion-floating { position: absolute; top:100%; left:var(--omni-left,0px); margin-left:0; }
.suggestion-item { display:flex; align-items:center; gap:12px; width:100%; padding:8px 12px; border:0; background:transparent; text-align:left; color:#243449; cursor:pointer; border-radius:7px; }
.suggestion-item:hover,.suggestion-item.selected { background:#eef4fb; }
.suggestion-type { width:20px; text-align:center; }
.suggestion-main { flex:1; min-width:0; overflow:hidden; white-space:nowrap; text-overflow:ellipsis; font-size:12px; }
.suggestion-sub { flex:none; font-size:11px; color:#667085; }
.suggestion-empty { padding:5px 12px; color:#667085; font-size:12px; margin:0; }
/* 中部智能地址栏 */
.addr-mid {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  margin: 0;
}
.omni-wrap {
  position: relative;
  flex: 1;
  display: flex;
  min-width: 0;
  align-items:center;
  border:1px solid transparent;
  border-radius:22px;
  height:34px;
  background:var(--chrome-address,#f1f3f4);
  margin:0 6px;
  padding:0 5px;
}
.omni-wrap:focus-within{border-color:#1a73e8;background:var(--chrome-toolbar,#fff);box-shadow:0 0 0 1px #1a73e8}
.omni-leading{display:flex;align-items:center;justify-content:center;width:28px;flex:none;color:var(--chrome-muted,#70757a)}
.omni-wrap input {
  flex: 1;
  height: 32px;
  border: 0;
  border-radius: 0;
  padding: 0 4px;
  font: 13px system-ui,sans-serif;
  min-width: 0;
  background: transparent;
  color: var(--chrome-text,#3c4043);
}
.omni-mode { display:flex;align-items:center;justify-content:center;flex:none;border:0;background:transparent;width:28px;height:28px;padding:0;cursor:pointer; }
.omni-wrap input:focus {
  border:0;
  background:transparent;
  outline:none;
  box-shadow:none;
}
:global(:root[data-theme="dark"]) .omni-wrap input{background:transparent;color:var(--chrome-text)}
.tbtn {
  min-width: 32px;
  width:32px;
  height: 32px;
  border: none;
  cursor: pointer;
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
}

/* W17 窄窗口密度：compact 先收文字标签，icon 再收次要按钮。
   裁剪掉的入口不会消失——它们仍在 ☰ 菜单里（由 store 的 navTopViews 保证）。 */
.nav-compact .lab,
.nav-icon .lab {
  display: none;
}
.nav-compact .activity button,
.nav-icon .activity button {
  padding: 2px 6px;
}

</style>

<style scoped>
.activity :focus-visible { outline: 2px solid #3983c9; outline-offset: -2px; }
</style>
