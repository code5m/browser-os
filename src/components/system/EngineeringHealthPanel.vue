<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { evaluateAll } from "./engineeringHealthModel.mjs";
import { bridge } from "../../bridge";
import type { ResourceStats } from "../../types";
// Baseline: docs/engineering/footprint-baselines/master-7ae6fa86.json.
// These are measured Linux package bytes, NOT live usage or module attribution.
const repo = "https://github.com/code5m/browser-os";
const api = "https://api.github.com/repos/code5m/browser-os";
const loading = ref(false), error = ref(""), sha = ref(""), checkedAt = ref(""), tag = ref<string | null>(null);
const runs = ref<any[]>([]);
const meta = ref<any>(null), policy = ref<any>(null);
const statusText = (s: string) => ({PASS:"通过",FAIL:"失败",RUNNING:"运行中",STALE:"已过期",UNKNOWN:"未知"} as Record<string,string>)[s] || "未知";
const summary = computed(() => sha.value && !error.value ? evaluateAll(sha.value,runs.value) : null);
// ResourceStats is already governed by the native resource_stats command.
// Sample only on request; no second polling loop or invented browser-wide usage.
const nativeSnapshot = ref<ResourceStats | null>(null);
const nativeIssue = ref("");
const nativeMeasuring = ref(false);
async function measureNativeOnce() {
  if (nativeMeasuring.value) return;
  nativeIssue.value = "";
  if (!(window as any).__TAURI_INTERNALS__) {
    nativeIssue.value = "预览模式不提供原生资源数据";
    return;
  }
  nativeMeasuring.value = true;
  try { nativeSnapshot.value = await bridge.resourceStats(); }
  catch { nativeIssue.value = "原生采样失败"; }
  finally { nativeMeasuring.value = false; }
}
const formatMiB = (mb: number | undefined) => mb == null ? "未测量" : mb.toFixed(2) + " MiB";
async function jsonRequest(path: string): Promise<any> {
 const r = await fetch(api + path, {headers:{Accept:"application/vnd.github+json"}});
 if (!r.ok) throw Error("GitHub API HTTP " + r.status);
 return r.json();
}
async function refresh() {
 loading.value = true; error.value = ""; sha.value = ""; runs.value = []; tag.value = null;
 const controller = new AbortController();
 const timeout = setTimeout(() => controller.abort(), 12000);
 try {
   const branch = await jsonRequest("/branches/master");
   const head = branch?.commit?.sha;
   if (!/^[a-f0-9]{40}$/i.test(head || "")) throw Error("master SHA 无效");
   const raw = "https://raw.githubusercontent.com/code5m/browser-os/" + head + "/docs/engineering/";
   const [data, refData, m, p] = await Promise.all([
     jsonRequest("/actions/runs?head_sha=" + head + "&per_page=100"),
     jsonRequest("/git/ref/tags/capability-platform-v4-stable"),
     fetch(raw+"governance.json",{signal:controller.signal}),
     fetch(raw+"evidence-policy.json",{signal:controller.signal}),
   ]);
   if (!m.ok || !p.ok || !Array.isArray(data.workflow_runs)) throw Error("验证数据不完整");
   meta.value = await m.json(); policy.value = await p.json();
   if (meta.value.repository !== "code5m/browser-os") throw Error("治理清单身份不匹配");
   tag.value = refData?.object?.sha === meta.value.stableBaseline?.commit ? "校验一致" : "SHA 不一致";
   runs.value = data.workflow_runs; sha.value = head;
   checkedAt.value = new Date().toLocaleString("zh-CN");
 } catch(e) { error.value = "实时状态未知：" + String(e); }
 finally { clearTimeout(timeout); loading.value = false; }
}
onMounted(() => void refresh());
</script>
<template>
<section class="eng-health" data-engineering-health>
 <header><h2>工程健康中心</h2><button type="button" :disabled="loading" @click="refresh">{{loading?"检查中":"刷新状态"}}</button></header>
 <p>只认当前 GitHub master SHA 的运行证据；过期、失败、执行中和无法联网均不算通过。</p>
 <p v-if="error" role="alert">{{error}}　<a :href="repo+'/actions'" target="_blank" rel="noopener noreferrer">打开 Actions</a></p>
 <p v-if="loading" role="status">正在获取 GitHub 验证记录…</p>
 <div class="health-summary">
   <strong>总状态：{{statusText(summary?.overall||"UNKNOWN")}}</strong>
   <span>master：{{sha?sha.slice(0,12):"未知"}}</span>
   <span>v4 stable：{{tag||"未核对"}}</span>
   <small>查询时间：{{checkedAt||"未查询"}}</small>
 </div>
 <h3>五项阻断工作流</h3>
 <div v-for="check in summary?.checks||[]" :key="check.id" class="health-row">
   <span><b>{{check.label}}</b> <small>{{check.reason}}</small></span>
   <span :class="'state-'+check.state.toLowerCase()">{{statusText(check.state)}}</span>
   <a :href="repo+'/actions/runs/'+(check.run?.id||'')" target="_blank" rel="noopener noreferrer">证据 ↗</a>
 </div>
 <h3>资源与体积</h3>
 <p>Linux 构建基线（不是当前设备内存）：前端 995,732 B；.deb 5,161,532 B；安装后声明 11,941,888 B。<a :href="repo+'/actions/runs/38098492736'" target="_blank" rel="noopener noreferrer">验收证据 7ae6fa86 ↗</a></p>
 <p>原生按需采样（不新增后台轮询）：应用 {{formatMiB(nativeSnapshot?.app_total_mb)}}；主进程树 RSS {{formatMiB(nativeSnapshot?.main.rss_mb)}}。</p>
 <button type="button" :disabled="nativeMeasuring" @click="measureNativeOnce">{{nativeMeasuring?"采样中…":"采样进程内存"}}</button>
 <p v-if="nativeIssue" role="status">{{nativeIssue}}</p>
 <p>首次加载与 CPU/p95 暂无实测；共享 Chunk 不归为某一个模块。<a :href="repo+'/actions'" target="_blank" rel="noopener noreferrer">构建对比与 CI 证据 ↗</a></p>
 <h3>工程资产与债务</h3>
 <p>Rust 原生语义：{{meta?.nativeSemantics?.expectedRegisteredCommands??"未知"}} 命令 / {{meta?.nativeSemantics?.expectedAppStateFields??"未知"}} AppState 字段（登记基线，非实时通过）。</p>
 <p>证据预算：{{policy?.trackedHistoryBudgetBytes??"未知"}} bytes；CI Artifact 保留 {{policy?.classes?.ciArtifact?.retentionDays??"未知"}} 天。</p>
 <p>供应链存在已登记的历史漏洞债务；SBOM 校验和不等于已发布制品签名。Gitee 自动镜像已退役。</p>
 <a :href="repo+'/actions'" target="_blank" rel="noopener noreferrer">全部工作流与 GUI 验收 →</a>
</section>
</template>
<style scoped>
.eng-health{margin:14px 0;padding:14px;border:1px solid #d8e0e4;border-radius:8px;line-height:1.5;font-size:12px}
header,.health-row,.health-summary{display:flex;justify-content:space-between;gap:12px;align-items:center}h2{font-size:17px;margin:0}h3{font-size:13px;margin:18px 0 8px}p{color:#62717c}button{padding:6px 12px;cursor:pointer}.health-summary{flex-wrap:wrap;background:#f0f4f5;padding:12px}.health-row{padding:10px 4px;border-bottom:1px solid #e2e7e9}.health-row small{display:block;color:#777}.health-row span{min-width:0}a{color:#116e88}.state-pass{color:#16865c}.state-fail{color:#b52733}.state-stale,.state-running{color:#9f7300}
</style>
