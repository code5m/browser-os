<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useBrowserStore } from "../../capabilities/browser/public";
import { useSystemStore } from "../../stores/useSystemStore";
import { useWorkspaceStore } from "../../capabilities/workspace/public";
import { useRepoStore } from "../../capabilities/workspace/public";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { bridge } from "../../bridge";
import type { ResourceStats } from "../../types";

const browser = useBrowserStore();
const system = useSystemStore();
const ws = useWorkspaceStore();
const rp = useRepoStore();
const layout = useLayoutStore();
const settings = useSettingsStore();

// W17(A7): 当前活动视图的可读名（纯展示，用于状态栏发现性；不引入运行时行为）。
// mainView 为内部枚举键，非路径/URL/凭据，展示无敏感信息泄露风险。
const KNOWN_VIEWS: Record<string, string> = {
  home: "主页", browser: "浏览器", grid: "宫格", files: "文件", arts: "成果库",
  clip: "剪贴板", repo: "仓库", apps: "应用", audit: "审计", scripts: "脚本库",
  commands: "命令库", tools: "工具箱", db: "数据库", tasks: "定时任务",
  plugin: "插件", skills: "技能", agents: "智能体", graph: "知识图谱",
  settings: "设置", term: "终端", editor: "编辑器", vault: "Vault",
};
const viewLabel = computed(() => KNOWN_VIEWS[layout.mainView] ?? "");
const viewUnknown = computed(
  () => !!layout.mainView && !(layout.mainView in KNOWN_VIEWS)
);

const tabCount = computed(() => browser.tabs.length);
const auditCount = computed(() => ws.audit.length);
const termReady = computed(() => system.terminalOpen && system.termPanes.length > 0);
const repoReady = computed(() => rp.repos.length > 0);
const nativeReady = computed(() => Boolean((window as any).__TAURI_INTERNALS__));

// ===== 资源监控（常驻）：系统内存 / 应用占用 / 内存预算 / 页签休眠 =====
const stats = ref<ResourceStats | null>(null);
const detailOpen = ref(false);
let timer: number | null = null;

async function refresh() {
  try {
    stats.value = await bridge.resourceStats();
  } catch {}
}

onMounted(() => {
  refresh();
  timer = window.setInterval(refresh, 3000);
});
onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});

function fmtMb(mb: number): string {
  return mb >= 1024 ? (mb / 1024).toFixed(1) + "G" : Math.round(mb) + "M";
}

// 可用内存占比（<10% 红色告警）
const memLow = computed(() => {
  if (!stats.value || !stats.value.mem_total_mb) return false;
  return stats.value.mem_available_mb / stats.value.mem_total_mb < 0.1;
});

// 内存预算提示：预算 < 当前选择格数时提示会降级
const budgetHint = computed(() => {
  if (!stats.value) return "";
  const want = browser.gridCount;
  const budget = stats.value.grid_budget;
  if (budget < want) return `预算仅 ${budget} 格`;
  return `可开 ${budget} 格`;
});
const budgetLow = computed(
  () => !!stats.value && stats.value.grid_budget < browser.gridCount
);
</script>

<template>
  <footer class="status">
    <span v-if="nativeReady" class="ok">● 已连接</span>
    <span v-else class="dim">● 预览模式</span>
    <span v-if="viewLabel" class="viewchip">当前：{{ viewLabel }}</span>
    <span v-else-if="viewUnknown" class="viewchip warn" role="alert">⚠ 未知视图：{{ layout.mainView }}</span>
    <span>页签 {{ tabCount }}</span>
    <span>· 终端{{ termReady ? "就绪" : "未启" }}</span>
    <span>· 仓库{{ repoReady ? "已配置" : "未配" }}</span>
    <span>· 审计 {{ auditCount }} 条</span>
    <!-- 资源摘要（常驻，点击查看明细） -->
    <span
      v-if="stats"
      class="res"
      :class="{ warn: memLow }"
      title="点击查看资源明细"
      @click="detailOpen = !detailOpen"
    >
      内存 {{ fmtMb(stats.mem_available_mb) }}/{{ fmtMb(stats.mem_total_mb) }}
      · 应用 {{ fmtMb(stats.app_total_mb) }}
      <template v-if="stats.grids.length">
        <template v-for="g in stats.grids" :key="g.pid">
          · {{ g.name }} {{ fmtMb(g.rss_mb) }}
        </template>
      </template>
      <span :class="{ warn: budgetLow }">· {{ budgetHint }}</span>
      <span v-if="settings.tabHibernation" class="hib">
        · 休眠{{ stats.hibernated_count ? ` ${stats.hibernated_count}` : "开" }}
      </span>
      <span v-else class="dim">· 休眠关</span>
    </span>
    <span v-if="layout.msg" class="msg">{{ layout.msg }}</span>
  </footer>
  <!-- 资源明细浮层（点击状态栏资源区展开） -->
  <div v-if="detailOpen && stats" class="res-detail" @click="detailOpen = false">
    <div class="rd-title">资源明细（3s 自动刷新，点击关闭）</div>
    <div class="rd-row">
      系统内存：可用 {{ fmtMb(stats.mem_available_mb) }} / 共
      {{ fmtMb(stats.mem_total_mb) }}
      <span v-if="memLow" class="warn">⚠️ 可用内存偏低，建议关闭其它应用或减少宫格</span>
    </div>
    <div class="rd-row">应用合计：{{ fmtMb(stats.app_total_mb) }}（主进程 {{ fmtMb(stats.main.rss_mb) }} + 宫格子进程）</div>
    <div v-for="g in stats.grids" :key="g.pid" class="rd-row">
      {{ g.name }}（pid {{ g.pid }}）：{{ fmtMb(g.rss_mb) }}
    </div>
    <div class="rd-row">
      内存预算守卫：当前可用内存最多支撑 {{ stats.grid_budget }} 格（每格按 450MB 估算，保留 700MB 系统余量）
    </div>
    <div class="rd-row">
      页签休眠：{{ stats.hibernation_enabled ? `开启（已休眠 ${stats.hibernated_count} 个页签）` : "关闭（可在设置 → 性能中开启）" }}
    </div>
  </div>
  <!-- 浏览器内容是原生子 WebView，会盖住 HTML 内的 fixed 浮层。提示仅在本状态栏
       展示：既不会遮住网页，也不会被原生 WebView 反向遮住。 -->
</template>

<style scoped>
.status {
  height: 24px;
  background: #1f2733;
  color: #9aa4b2;
  font-size: 11px;
  display: flex;
  align-items: center;
  padding: 0 12px;
  gap: 16px;
  flex-shrink: 0;
  user-select: none;
}
.status .ok {
  color: #52c41a;
}
.status .msg {
  margin-left: auto;
  color: #cbd5e0;
}
.viewchip {
  color: #cbd5e0;
}
.viewchip.warn {
  color: #ff7a7a;
}
.res {
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.res:hover {
  color: #cbd5e0;
}
.warn {
  color: #ff7a7a;
}
.hib {
  color: #d4a94e;
}
.dim {
  color: #5a6472;
}
.res-detail {
  position: fixed;
  bottom: 32px;
  left: 12px;
  z-index: 99999;
  background: rgba(15, 23, 42, 0.95);
  color: #cbd5e0;
  font-size: 12px;
  padding: 12px 16px;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  max-width: 70vw;
  cursor: pointer;
  line-height: 1.8;
}
.rd-title {
  font-weight: 600;
  color: #fff;
  margin-bottom: 4px;
}
.rd-row .warn {
  margin-left: 8px;
}
</style>
