<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useBrowserStore } from "../../capabilities/browser/public";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { bridge } from "../../bridge";
import { contributionRegistry } from "../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types";
import type { ResourceStats } from "../../types";

const browser = useBrowserStore();
const layout = useLayoutStore();

// 当前视图名称以动态 Capability 贡献为先，避免插件卸载后状态栏仍显示过期名称。
const KNOWN_VIEWS: Record<string, string> = {
  home: "主页", browser: "浏览器", grid: "宫格", files: "文件", arts: "成果库",
  clip: "剪贴板", repo: "仓库", apps: "应用", audit: "审计", scripts: "脚本库",
  commands: "命令库", tools: "工具箱", db: "数据库", tasks: "定时任务",
  plugin: "插件", skills: "技能", agents: "智能体", graph: "知识图谱",
  settings: "设置", term: "终端", editor: "编辑器", vault: "Vault",
};
const contributedViewLabels = computed(() => Object.fromEntries(
  contributionRegistry.getSurfaceContributions(CONTRIBUTION_SLOTS.WORKBENCH_MAIN)
    .filter((contribution) => contribution.view)
    .map((contribution) => [contribution.view as string, contribution.label ?? contribution.view as string]),
));
const viewLabel = computed(() => contributedViewLabels.value[layout.mainView] ?? KNOWN_VIEWS[layout.mainView] ?? "");
const viewUnknown = computed(() => !!layout.mainView && !viewLabel.value);

// 页签真源是两类独立 store：浏览器网页 + 目录/模块。状态栏必须显示同一条页签栏的总数。
const tabCount = computed(() => browser.tabs.length + layout.modTabs.length);
const nativeReady = computed(() => Boolean((window as any).__TAURI_INTERNALS__));

const stats = ref<ResourceStats | null>(null);
const detailOpen = ref(false);

// 普通浏览降至 15s，宫格/查看详情采用 3s；不可见窗口暂停轮询。
// resource_stats 会扫描 /proc 进程树，不能无条件每 3s 常驻。
// 同一时刻最多一个请求，防止慢请求重叠；卸载后不回写状态。
const IDLE_REFRESH_MS = 15_000;
const ACTIVE_REFRESH_MS = 3_000;
let timer: number | null = null;
let pending = false;
let disposed = false;

async function refresh() {
  if (disposed || pending || !nativeReady.value || document.hidden) return;
  pending = true;
  try {
    const result = await bridge.resourceStats();
    if (!disposed) stats.value = result;
  } catch {
    // 暂态失败保留上一次可信统计；不制造持续错误 Toast。
  } finally {
    pending = false;
  }
}

function schedulePoll() {
  if (timer !== null) window.clearTimeout(timer);
  timer = null;
  if (disposed || !nativeReady.value || document.hidden) return;
  timer = window.setTimeout(async () => {
    timer = null;
    await refresh();
    schedulePoll();
  }, detailOpen.value || layout.mainView === "grid" ? ACTIVE_REFRESH_MS : IDLE_REFRESH_MS);
}

function onVisibilityChange() {
  if (!document.hidden) void refresh();
  schedulePoll();
}

onMounted(() => {
  document.addEventListener("visibilitychange", onVisibilityChange);
  void refresh().finally(schedulePoll);
});
onBeforeUnmount(() => {
  disposed = true;
  document.removeEventListener("visibilitychange", onVisibilityChange);
  if (timer !== null) window.clearTimeout(timer);
  timer = null;
});

function toggleResourceDetails() {
  detailOpen.value = !detailOpen.value;
}
watch(detailOpen, () => {
  // 详情占据真实文档流高度；通知 BrowserHost 重新计算原生 WebView 可见区域。
  void nextTick(() => browser.relocate());
  void refresh();
  schedulePoll();
});
watch(() => layout.mainView, () => {
  if (detailOpen.value) detailOpen.value = false;
  else schedulePoll();
});

function fmtMb(mb: number): string {
  return mb >= 1024 ? (mb / 1024).toFixed(1) + "G" : Math.round(mb) + "M";
}
const memLow = computed(() => {
  if (!stats.value || !stats.value.mem_total_mb) return false;
  return stats.value.mem_available_mb / stats.value.mem_total_mb < 0.1;
});
</script>

<template>
  <!-- 先渲染资源详情，再渲染底栏：真实占据 Shell 高度，绝不 fixed 覆盖原生网页。 -->
  <section v-if="detailOpen && stats" id="resource-detail-panel" class="res-detail" role="region" aria-label="资源使用详情">
    <div class="rd-header">
      <div>
        <strong>资源使用情况</strong>
        <span class="rd-subtitle">查看详情时每 3 秒刷新；窗口隐藏后暂停</span>
      </div>
      <button type="button" class="rd-close" aria-label="收起资源详情" @click="toggleResourceDetails">收起 ✕</button>
    </div>
    <div class="rd-grid">
      <div><span class="rd-label">系统可用内存</span><strong>{{ fmtMb(stats.mem_available_mb) }} / {{ fmtMb(stats.mem_total_mb) }}</strong></div>
      <div><span class="rd-label">BrowserOS 合计</span><strong>{{ fmtMb(stats.app_total_mb) }}</strong></div>
      <div><span class="rd-label">主进程树</span><strong>{{ fmtMb(stats.main.rss_mb) }}</strong></div>
      <div><span class="rd-label">网页休眠</span><strong>{{ stats.hibernation_enabled ? "已开启（" + stats.hibernated_count + " 个）" : "未开启" }}</strong></div>
    </div>
    <p v-if="memLow" class="rd-alert" role="alert">可用内存偏低。建议收起宫格、减少同时运行的网页或关闭其他占用内存的应用。</p>
    <div v-for="g in stats.grids" :key="g.pid" class="rd-process">{{ g.name }} · PID {{ g.pid }} · {{ fmtMb(g.rss_mb) }}</div>
    <p class="rd-note">
      统计接口的宫格预算估计为 {{ stats.grid_budget }} 格，仅供参考，<strong>不代表实际获准启动</strong>。
      宫格启动另有安全保护（系统预留 1800MB、每格估算 450MB）；实际可启动格数以启动时检查为准。
    </p>
  </section>

  <footer class="status">
    <span v-if="nativeReady" class="connection ok" title="原生客户端已连接">● 已连接</span>
    <span v-else class="connection dim" title="Web 预览，没有原生系统资源数据">● 预览模式</span>
    <span v-if="viewLabel" class="viewchip">当前：{{ viewLabel }}</span>
    <span v-else-if="viewUnknown" class="viewchip warn" role="alert">⚠ 未知视图：{{ layout.mainView }}</span>
    <span class="tabcount" :title="'网页 ' + browser.tabs.length + ' 个；目录与模块 ' + layout.modTabs.length + ' 个'">共 {{ tabCount }} 个页签</span>
    <span v-if="layout.msg" class="msg" role="status">{{ layout.msg }}</span>
    <span v-else class="status-space" aria-hidden="true"></span>
    <button
      v-if="stats"
      type="button"
      class="res"
      :class="{ warn: memLow }"
      aria-controls="resource-detail-panel"
      :aria-expanded="detailOpen"
      title="展开或收起资源详情"
      @click="toggleResourceDetails"
    >
      <span>应用 {{ fmtMb(stats.app_total_mb) }}</span>
      <span class="res-secondary">系统剩余 {{ fmtMb(stats.mem_available_mb) }}</span>
      <span aria-hidden="true">{{ detailOpen ? "⌄" : "⌃" }}</span>
    </button>
  </footer>
  <!-- 操作消息仅在状态栏；资源详情是正常 flex 流中的可收起面板，不在 WebView 上方。 -->
</template>

<style scoped>
.status {
  height: 24px;
  min-height: 24px;
  background: #1f2733;
  color: #aebccc;
  font-size: 11px;
  display: flex;
  align-items: center;
  padding: 0 12px;
  gap: 12px;
  flex-shrink: 0;
  user-select: none;
  min-width: 0;
}
.connection, .viewchip, .tabcount { white-space: nowrap; }
.ok { color: #a4e5bc; }
.dim { color: #a8b2c2; }
.viewchip { color: #e0e8f1; }
.warn { color: #ffcb9e; }
.msg {
  margin-left: auto;
  flex: 1 1 auto;
  min-width: 0;
  color: #e0e8f1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: right;
}
.status-space { flex: 1 1 auto; min-width: 0; }
.res {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  border: 0;
  border-radius: 5px;
  padding: 3px 6px;
  margin: 0 -4px 0 0;
  color: #e2e9f2;
  background: transparent;
  font: inherit;
  white-space: nowrap;
  flex-shrink: 0;
}
.res:hover, .res[aria-expanded="true"] { background: #344355; }
.res:focus-visible, .rd-close:focus-visible { outline: 2px solid #86b7f7; outline-offset: 2px; }
.res-secondary { color: #aebccc; }
.res-detail {
  flex: 0 0 auto;
  max-height: min(33vh, 260px);
  overflow: auto;
  padding: 12px 18px 10px;
  border-top: 1px solid #dbe4ee;
  background: #f7f9fc;
  color: #334155;
  font-size: 12px;
}
.rd-header, .rd-header > div { display: flex; align-items: center; gap: 12px; }
.rd-header { justify-content: space-between; margin-bottom: 10px; }
.rd-header strong { font-size: 13px; color: #1e293b; }
.rd-subtitle, .rd-label, .rd-note { color: #64748b; }
.rd-close {
  border: 1px solid #d7e0ec;
  background: #fff;
  border-radius: 6px;
  color: #334155;
  padding: 4px 9px;
  font-size: 11px;
}
.rd-close:hover { background: #eaf1fb; }
.rd-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
.rd-grid > div {
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 8px 10px;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  background: #fff;
  min-width: 0;
}
.rd-grid strong { color: #1e293b; font-size: 14px; }
.rd-alert { margin: 9px 0 0; color: #9a3412; }
.rd-process { margin-top: 7px; color: #475569; }
.rd-note { margin: 9px 0 0; font-size: 11px; line-height: 1.5; }
@media (max-width: 1000px) {
  .viewchip { display: none; }
  .rd-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 650px) {
  .res-secondary { display: none; }
  .status { gap: 8px; padding: 0 8px; }
  .rd-header > div { flex-direction: column; align-items: flex-start; gap: 3px; }
}
</style>
