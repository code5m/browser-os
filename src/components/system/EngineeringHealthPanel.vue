<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { evaluateAll } from "./engineeringHealthModel.mjs";
const repo = "https://github.com/code5m/browser-os";
const api = "https://api.github.com/repos/code5m/browser-os";
const loading = ref(false), error = ref(""), sha = ref(""), checkedAt = ref(""), tag = ref<string | null>(null);
const runs = ref<any[]>([]);
const meta = ref<any>(null), policy = ref<any>(null);
const statusText = (s: string) => ({PASS:"通过",FAIL:"失败",RUNNING:"运行中",STALE:"已过期",UNKNOWN:"未知"} as Record<string,string>)[s] || "未知";
const summary = computed(() => sha.value && !error.value ? evaluateAll(sha.value,runs.value) : null);
// Browser WebView reports only resources visible to its own Performance API.
// Other WebViews/native subprocesses, OS installation bytes and CPU remain unknown.
const resourceSnapshot = ref<{observedJsBytes:number|null,observedJsCount:number,usedHeapBytes:number|null,resourceCount:number}|null>(null);
function captureResourceSnapshot() {
  const entries = performance.getEntriesByType("resource") as PerformanceResourceTiming[];
  const scripts = entries.filter(e => /\.m?js(?:[?#]|$)/.test(e.name));
  const nonzero = scripts.filter(e => e.transferSize > 0 || e.encodedBodySize > 0);
  const heap = (performance as Performance & {memory?:{usedJSHeapSize?:number}}).memory?.usedJSHeapSize;
  resourceSnapshot.value = {
    observedJsBytes: nonzero.length ? nonzero.reduce((n,e)=>n+(e.transferSize||e.encodedBodySize),0) : null,
    observedJsCount: nonzero.length,
    usedHeapBytes: Number.isFinite(heap) ? heap! : null,
    resourceCount: entries.length,
  };
}
const formatMiB = (bytes:number|null|undefined) => bytes == null ? "未测量" : (bytes/1048576).toFixed(2)+" MiB";
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
onMounted(() => { captureResourceSnapshot(); void refresh(); });
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
 <h3>资源与体积诊断（当前 WebView 实测 / 未测量分开展示）</h3>
 <p>以下浏览器指标只覆盖当前工程健康页面的 WebView，不代表全部 WebView、原生应用、安装包或操作系统资源。</p>
 <div class="health-summary">
   <span>已观察 JS 网络传输：{{formatMiB(resourceSnapshot?.observedJsBytes)}} <small>已获得大小的 JS 资源 {{resourceSnapshot?.observedJsCount??0}} 项</small></span>
   <span>当前 JS 堆：{{formatMiB(resourceSnapshot?.usedHeapBytes)}} <small>WebKit 不提供时显示未测量</small></span>
   <span>安装包 / 安装后体积：未测量 <small>仅能从同版本打包 CI 产物获取</small></span>
   <span>空闲 CPU / 全进程内存：未测量 <small>需目标机器 /proc 与隔离基准测试</small></span>
 </div>
 <p>首次加载与安装体积不能从 Resource Timing 直接推断。模块源码字节数也不等于模块安装体积。构建证据见 <a :href="repo+'/actions'" target="_blank" rel="noopener noreferrer">Actions 中的 footprint artifact ↗</a>。</p>
 <button type="button" @click="captureResourceSnapshot">重新采样本页</button>
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
