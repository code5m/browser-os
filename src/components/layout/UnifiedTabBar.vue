<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";
import { ref, computed } from "vue";
import { useFileStore } from "../../capabilities/workspace/public";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Globe, Plus, X } from '@lucide/vue';
import { bridge, isDownloadOwnerObservation, type DownloadInventory, type DownloadOwnerObservation } from '../../bridge';
import { useUnifiedTabs } from "../../composables/useUnifiedTabs";
import { documentPathName, relativeDocumentPath } from "../../application/documentPaths.mjs";

// 统一页签条：浏览器网页页签 + 目录页签 + 模块页签（终端/文件等）混排在同一条，
// 不区分类型 —— 点击即切换对应视图，行为与浏览器标签一致。
const layout = useLayoutStore();
const browser = useBrowserStore();
const fs = useFileStore();
const context = ref<{ id: string; kind: 'web' | 'module' } | null>(null);
// Read-only projection and actions are shared with the global shortcuts.
const tabActions = useUnifiedTabs();
const projectedTabs = tabActions.projectedTabs;
const restartTarget = ref<Awaited<ReturnType<typeof browser.inspectNativeDomainRecovery>>>(null);
const rendererTarget = ref<Awaited<ReturnType<typeof browser.inspectRendererRecovery>>>(null);
const reopenTarget = ref<{id: string; address: string} | null>(null);
const reopening = ref(false);
const parkTarget = ref<Awaited<ReturnType<typeof browser.prepareRendererPark>>>(null);
const parking = ref(false);
async function prepareRendererPark() {
  const selected = context.value;
  if (!selected || selected.kind !== "web" || parking.value) return;
  const target = await browser.prepareRendererPark(selected.id);
  if (context.value !== selected) return;
  context.value = null;
  parkTarget.value = target;
}
async function confirmRendererPark() {
  const target = parkTarget.value;
  if (!target || parking.value) return;
  parking.value = true;
  try {
    parkTarget.value = null;
    await browser.parkRenderer(target);
  } finally { parking.value = false; }
}
const downloadPanelOpen = ref(false);
const downloadPanelBusy = ref(false);
const downloadInventory = ref<DownloadInventory | null>(null);
const downloadPanelError = ref("");
let downloadPanelEpoch = 0;
const downloadSampleLabel = ref("");
async function refreshDownloadInventory(sample?: DownloadOwnerObservation) {
  if (downloadPanelBusy.value || !downloadPanelOpen.value) return;
  const epoch = ++downloadPanelEpoch;
  downloadPanelBusy.value = true;
  downloadInventory.value = null;
  downloadPanelError.value = "";
  downloadSampleLabel.value = "";
  try {
    const view = sample ? await bridge.sampleDownload(sample) : await bridge.inspectDownloadInventory();
    if (epoch !== downloadPanelEpoch || !downloadPanelOpen.value) return;
    const data = view.download_inventory;
    if (!data || !Array.isArray(data.live) || !Array.isArray(data.retired)
        || data.retired_limit !== 128 || data.retired.length > 128
        || data.live.some(row => !isDownloadOwnerObservation(row) || row.phase !== "live")
        || data.retired.some(row => !isDownloadOwnerObservation(row) || row.phase !== "owner_retired")
        || new Set(data.live.map(row => row.id)).size !== data.live.length
        || new Set(data.retired.map(row => row.id + ":" + row.pid + ":" + row.spawn_epoch)).size !== data.retired.length) {
      throw new Error("invalid download inventory");
    }
    if (sample && !data.live.some(row => row.id === sample.id && row.pid === sample.pid
        && row.spawn_epoch === sample.spawn_epoch && row.connection_current && row.requested !== null)) {
      throw new Error("sample owner changed");
    }
    downloadInventory.value = data;
    if (sample) downloadSampleLabel.value = sample.id + "：已主动采样本次宿主的累计下载回调；不代表最终下载状态。";
  } catch {
    if (epoch === downloadPanelEpoch) downloadPanelError.value = sample
      ? "指定原生宿主采样未确认；可能已更换、忙碌或退出，请重新查询。"
      : "原生下载观察暂不可核实，请手动重新查询。";
  } finally {
    downloadPanelBusy.value = false;
    if (epoch !== downloadPanelEpoch && downloadPanelOpen.value) void refreshDownloadInventory();
  }
}
function toggleDownloadInventory() {
  downloadPanelOpen.value = !downloadPanelOpen.value;
  if (downloadPanelOpen.value) void refreshDownloadInventory();
  else {
    ++downloadPanelEpoch;
    downloadInventory.value = null;
    // Keep the unresolved native request's single-flight ownership until
    // it actually settles. A reopened panel can refresh after settlement.
    downloadPanelError.value = "";
  }
}
const downloadRows = computed(() => downloadInventory.value
  ? [...downloadInventory.value.live, ...downloadInventory.value.retired] : []);
function downloadLabel(row: DownloadOwnerObservation) {
  if (row.requested === null) return "尚无可信计数；结果待核对";
  const counts = `请求 ${row.requested} / 成功回调 ${row.succeeded} / 失败回调 ${row.failed}`;
  const pending = row.observed_unfinished === null ? "计数无法对账" : `未完成观察 ${row.observed_unfinished}`;
  return counts + "；" + pending + (row.phase === "owner_retired" || !row.connection_current ? "；最终结果未知" : "；非逐项完成回执");
}
async function prepareRendererReopen() {
  const selected = context.value;
  if (!selected || selected.kind !== 'web') return;
  const eligible = await browser.prepareRendererReopen(selected.id);
  if (context.value !== selected) return;
  context.value = null;
  if (eligible) reopenTarget.value = { id: selected.id, address: "about:blank" };
}
async function confirmRendererReopen() {
  const target = reopenTarget.value;
  if (!target || reopening.value) return;
  reopening.value = true;
  try {
    const success = await browser.reopenRenderer(target.id, target.address);
    if (success && reopenTarget.value === target) reopenTarget.value = null;
  } finally {
    reopening.value = false;
  }
}
async function prepareRendererRetire() {
  const selected = context.value;
  if (!selected || selected.kind !== 'web') return;
  const target = await browser.inspectRendererRecovery(selected.id);
  // The menu may have been switched while the native read was outstanding.
  if (context.value === selected) {
    rendererTarget.value = target;
    context.value = null;
  }
}
async function confirmRendererRetire() {
  const target = rendererTarget.value;
  rendererTarget.value = null;
  if (target) await browser.retireRenderer(target);
}
async function prepareDomainRestart() {
  restartTarget.value = await browser.inspectNativeDomainRecovery();
}
async function confirmDomainRestart() {
  const target = restartTarget.value;
  restartTarget.value = null;
  if (target) await browser.requestNativeDomainRecovery(target);
}

// Resolve the native window only after an explicit user action so a browser/dev
// preview can render the shell even when Tauri APIs are unavailable.
async function minimizeWindow() {
  try { await getCurrentWindow().minimize(); } catch { /* browser preview */ }
}

async function toggleWindowMaximize() {
  try { await getCurrentWindow().toggleMaximize(); } catch { /* browser preview */ }
}

async function closeWindow() {
  try { await getCurrentWindow().close(); } catch { /* browser preview */ }
}

async function startWindowDrag(event: MouseEvent) {
  if (event.button !== 0) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, .tab, input, select, textarea, a")) return;
  // One native request per press. Do not also register pointerdown or Tauri's
  // document-level data-tauri-drag-region handler on this titlebar.
  event.preventDefault();
  event.stopPropagation();
  try {
    if (event.detail === 2) await getCurrentWindow().toggleMaximize();
    else await getCurrentWindow().startDragging();
  } catch {
    layout.showToast("窗口拖动失败，请完全退出旧客户端后重新启动");
  }
}
async function closeContext(others = false) {
  const target = context.value;
  context.value = null;
  if (!target) return;
  const key = `${target.kind}:${target.id}`;
  if (others) await tabActions.closeOthers(key);
  else await tabActions.close(key);
}

// 当前右键页签若是本地目录/文件（module 类且带 path），返回其绝对路径，否则 null
const contextPath = computed(() => {
  const c = context.value;
  if (!c || c.kind !== 'module') return null;
  const t = layout.modTabs.find((x) => x.id === c.id);
  return t?.path ?? null;
});

// 相对路径：相对到命中的最长“起始目录”前缀；无匹配时退化为文件名
function relativeOf(abs: string): string {
  let best: string | null = null;
  for (const dir of fs.startDirs || []) {
    try {
      const relative = relativeDocumentPath(dir.path, abs);
      if (relative !== null && (best === null || relative.length < best.length)) best = relative;
    } catch {
      // An unsupported historical start-directory entry cannot match this file.
    }
  }
  return best === null ? documentPathName(abs) : best || ".";
}

async function revealHere() {
  const p = contextPath.value;
  context.value = null;
  if (!p) return;
  try {
    await bridge.revealPath(p);
  } catch (e: any) {
    layout.showToast("打开目录失败：" + (e?.message ?? e));
  }
}

async function copyAbsPath() {
  const p = contextPath.value;
  context.value = null;
  if (!p) return;
  await bridge.clipboardWrite(p);
  layout.showToast("已复制绝对路径");
}

async function copyRelPath() {
  const p = contextPath.value;
  context.value = null;
  if (!p) return;
  await bridge.clipboardWrite(relativeOf(p));
  layout.showToast("已复制相对路径");
}

function activateProjectedTab(tab: { key: string }) {
  void tabActions.activate(tab.key);
}
function closeProjectedTab(tab: { key: string }) {
  void tabActions.close(tab.key);
}
async function newTab() {
  try { await browser.tabNew(); }
  catch { layout.showToast("新建页签失败，请重试"); }
}
</script>

<template>
  <div class="tabbar unified" @mousedown="startWindowDrag">
    <div class="tab-strip">
    <!-- Unified logical tabs, not a native WebView lifecycle replacement. -->
    <div
      v-for="tab in projectedTabs"
      :key="tab.key"
      :class="['tab', {active:tab.active}]"
      @click="activateProjectedTab(tab)"
      @contextmenu.prevent="context = { id: tab.id, kind: tab.kind === 'web' ? 'web' : 'module' }"
      :title="tab.kind === 'web' ? [tab.title, browser.rendererStateLabel(tab.id), browser.rendererDownloadLabel(tab.id)].filter(Boolean).join(' · ') : (tab.path || tab.title)"
    >
      <Globe v-if="tab.kind === 'web'" class="tab-ic" :size="15"/>
      <span v-else class="tab-ic">{{ tab.icon }}</span>
      <span class="tab-title">{{ tab.title }}</span>
      <span v-if="tab.kind === 'web' && browser.rendererStateLabel(tab.id)" class="renderer-state" role="status">{{ browser.rendererStateLabel(tab.id) }}</span>
      <span v-else-if="tab.kind === 'web' && browser.rendererDownloadLabel(tab.id)" class="renderer-state" role="status">{{ browser.rendererDownloadLabel(tab.id) }}</span>
      <button class="tab-close" @click.stop="closeProjectedTab(tab)" title="关闭"><X :size="14"/></button>
    </div>
    <button class="tab-new" @click="newTab" title="新建页签"><Plus :size="18"/></button>
    </div>
    <button v-if="browser.tabNativeState === 'unknown' || browser.tabRecoveryRequired"
      class="native-recovery" :disabled="browser.tabRecoveryBusy" @click.stop="browser.recoverNativeTabs()"
      :title="browser.tabRecoveryMessage || '查询原生回执并核对页签，不强制释放执行权'">页签恢复</button>
    <button v-if="browser.tabRecoveryRequired && !restartTarget"
      class="native-recovery" :disabled="browser.tabRecoveryBusy" @click.stop="prepareDomainRestart"
      title="先核对卡住的原生操作，确认恢复影响范围">故障域恢复</button>
    <template v-if="browser.tabRecoveryRequired && restartTarget">
      <button class="native-recovery" :disabled="browser.tabRecoveryBusy" @click.stop="confirmDomainRestart"
        :title="restartTarget.scope === 'renderer' ? '仅回收故障页签，页内未保存状态可能丢失；不重放原操作' : '重启整个应用，未保存会话及编辑状态可能丢失；不重放原操作'">{{ restartTarget.scope === 'renderer' ? '确认回收故障页签' : '确认重启应用（未保存状态可能丢失）' }}</button>
      <button class="native-recovery" @click.stop="restartTarget = null">取消</button>
    </template>
    <span v-if="browser.tabRecoveryMessage && (browser.tabRecoveryRequired || browser.tabNativeState === 'unknown')"
      class="native-recovery-status" role="status" :title="browser.tabRecoveryMessage">{{browser.tabRecoveryMessage}}</span>
    <div class="titlebar-drag" aria-hidden="true"></div>
    <div class="window-controls" aria-label="窗口控制">
      <button title="最小化" @click.stop="minimizeWindow">−</button>
      <button title="最大化或还原" @click.stop="toggleWindowMaximize">□</button>
      <button class="window-close" title="关闭窗口" @click.stop="closeWindow">×</button>
    </div>
  </div>
  <div v-if="browser.isolatedRendererMode" class="tab-actions" role="group" aria-label="独立宿主下载观察">
    <button :aria-expanded="downloadPanelOpen" @click="toggleDownloadInventory">下载观察记录</button>
    <template v-if="downloadPanelOpen">
      <button :disabled="downloadPanelBusy" @click="refreshDownloadInventory()">重新查询缓存及回收记录</button>
      <span v-if="downloadPanelBusy" role="status">正在核对原生记录…</span>
      <span v-else-if="downloadPanelError" role="status">{{downloadPanelError}}</span>
      <span v-else-if="downloadInventory">当前宿主 {{downloadInventory.live.length}} / 最近回收 {{downloadInventory.retired.length}}（最多 128）</span>
      <span v-if="downloadSampleLabel" role="status">{{downloadSampleLabel}}</span>
      <div v-if="downloadInventory" class="download-owner-list">
        <div v-if="!downloadRows.length">暂无宿主观察</div>
        <div v-for="row in downloadRows" :key="row.phase + ':' + row.id + ':' + row.pid + ':' + row.spawn_epoch">
          <strong>{{row.id}} · {{row.phase === 'live' ? '当前宿主' : '已回收'}}</strong>
          <span>{{downloadLabel(row)}}</span>
          <button v-if="row.phase === 'live'" :disabled="downloadPanelBusy" @click="refreshDownloadInventory(row)">主动采样</button>
        </div>
      </div>
      <span>仅本次应用会话内的宿主累计观察，不含下载地址、路径或文件操作。</span>
    </template>
  </div>
  <div v-if="context" class="tab-actions" role="menu" @keydown.esc="context = null">
    <button role="menuitem" @click="closeContext()">关闭</button>
    <button role="menuitem" @click="closeContext(true)">关闭其他</button>
    <button v-if="context.kind === 'web' && browser.isolatedRendererMode" role="menuitem" @click="prepareRendererRetire">核对并恢复独立页签</button>
    <button v-if="context.kind === 'web' && browser.isolatedRendererMode" role="menuitem" :disabled="parking" @click="prepareRendererPark">休眠后台页签（保留网址）</button>
    <button v-if="context.kind === 'web' && browser.isolatedRendererMode" role="menuitem" @click="prepareRendererReopen">更换故障页签恢复地址</button>
    <template v-if="contextPath">
      <span class="tab-actions-sep" aria-hidden="true"></span>
      <button role="menuitem" @click="revealHere">在文件管理器中显示</button>
      <button role="menuitem" @click="copyAbsPath">复制绝对路径</button>
      <button role="menuitem" @click="copyRelPath">复制相对路径</button>
    </template>
    <button role="menuitem" @click="context = null">取消</button>
  </div>
  <div v-if="parkTarget" class="tab-actions" role="group" aria-label="独立页签休眠确认">
    <span>休眠 {{parkTarget.target.id}}：仅保留当前网址；表单、历史、滚动及未保存内容不保证恢复。观察到未完成下载时拒绝。</span>
    <span class="park-address">{{parkTarget.address}}</span>
    <button :disabled="parking || browser.tabRecoveryBusy || browser.tabRecoveryRequired" @click="confirmRendererPark">确认休眠</button>
    <button :disabled="parking" @click="parkTarget = null">取消</button>
  </div>
  <form v-if="reopenTarget" class="tab-actions" aria-label="独立页签新地址恢复" @submit.prevent="confirmRendererReopen">
    <span>在原逻辑页签 {{ reopenTarget.id }} 打开新地址，不恢复原页面内容。</span>
    <input v-model="reopenTarget.address" aria-label="完整恢复地址" :disabled="reopening" maxlength="8192" required />
    <button type="submit" :disabled="reopening || browser.tabRecoveryBusy || browser.tabRecoveryRequired || !reopenTarget.address.trim()">确认重开</button>
    <button type="button" :disabled="reopening" @click="reopenTarget = null">取消</button>
  </form>
  <div v-if="rendererTarget" class="tab-actions" role="group" aria-label="独立页签恢复确认">
    <span>回收 {{ rendererTarget.id }} 的原生进程？页内未保存状态可能丢失，不会自动重放原操作。</span>
    <button :disabled="browser.tabRecoveryBusy || browser.tabRecoveryRequired" @click="confirmRendererRetire">确认回收</button>
    <button @click="rendererTarget = null">取消</button>
  </div>
</template>

<style scoped>
.park-address {max-width:100%;overflow-wrap:anywhere;font-size:12px;color:#667085}
.download-owner-list {flex-basis:100%;max-height:200px;overflow:auto;font-size:12px;display:grid;gap:4px}
.download-owner-list>div {display:flex;flex-wrap:wrap;gap:8px;padding:3px 0;border-bottom:1px solid #e5e6eb}
.tab-actions { display:flex; flex-wrap:wrap; align-items:center; flex:none; gap:6px; padding:4px 8px; background:#f2f4f6; border-bottom:1px solid #e5e6eb; }
.tab-actions button { border:1px solid #d5dbe7; background:#fff; color:#4e5969; font-size:12px; padding:3px 8px; border-radius:5px; cursor:pointer; }
.tab-actions button:hover { background:#eef1f6; color:#2b6cb0; }
.tab-actions button:disabled { color:#bbb; cursor:not-allowed; }
.tab-actions-sep { width:1px; align-self:stretch; background:#d5dbe7; margin:0 2px; }
.unified {
  display: flex;
  align-items: flex-end;
  gap: 3px;
  padding: 3px 6px 0;
  background:var(--chrome-frame,#e8eaed);
  border-bottom: 0;
  flex-shrink: 0;
  overflow-x: hidden;
  overflow-y: hidden;
  /* 页签超出时仍支持滚轮/触控板横向滚动，但不显示系统横向轨道。 */
  scrollbar-width: none;
  -ms-overflow-style: none;
  min-height: 42px;
  cursor: default;
}
.unified::-webkit-scrollbar { display: none; }
.tab-strip{display:flex;align-items:flex-end;gap:3px;flex:0 1 auto;min-width:0;overflow-x:auto;overflow-y:hidden;scrollbar-width:none}
.tab-strip::-webkit-scrollbar{display:none}
.native-recovery{flex:none;margin:4px;padding:4px 8px;border:1px solid #a97927;border-radius:4px;background:transparent;color:var(--chrome-text,#3c4043);cursor:pointer}
.renderer-state{flex:none;font-size:10px;color:#a97927;white-space:nowrap}
.native-recovery-status{font-size:11px;max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;align-self:center;color:var(--chrome-text,#3c4043)}
.titlebar-drag { flex: 1; min-width: 24px; align-self: stretch; cursor: default; }
.window-controls { display: flex; flex: none; align-items: stretch; align-self:stretch; margin: -3px -6px 0 0; }
.window-controls button { width: 42px; border: 0; border-radius: 0; background: transparent; display: grid; place-items: center; color:var(--chrome-text,#3c4043); cursor: pointer; }
.window-controls button:hover { background: #dfe5ed; color: #1d2939; }
.window-controls .window-close:hover { background: #d64545; color: #fff; }
.tab {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  width:210px;
  max-width:240px;
  min-width:96px;
  height:35px;
  box-sizing:border-box;
  border:0;
  background:transparent;
  color:var(--chrome-text,#3c4043);
  border-radius:10px 10px 0 0;
  padding:0 10px 0 14px;
  font:13px system-ui,sans-serif;
  cursor: pointer;
  white-space: nowrap;
}
.tab.active {
  background:var(--chrome-toolbar,#fff);
  color:var(--chrome-text,#202124);
  font-weight:400;
}
.tab:not(.active):hover{background:var(--chrome-hover,#dfe3e8)}
.tab-ic {
  font-size: 11px;
  flex-shrink: 0;
}
.tab-title {
  flex:1;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.tab-close {
  border: none;
  background: transparent;
  color:var(--chrome-text,#5f6368);
  font-size: 10px;
  padding: 0;
  width:20px;height:20px;display:grid;place-items:center;
  border-radius:50%;
  cursor: pointer;
  flex-shrink: 0;
}
.tab-close:hover {
  color: #c33;
  background: #f0f0f0;
}
.tab-new {
  border: none;
  background: transparent;
  color: #4e5969;
  font-size: 14px;
  padding: 2px 8px;
  border-radius: 5px;
  cursor: pointer;
  flex-shrink: 0;
}
.tab-new:hover {
  background: #e3e8f0;
}
</style>
