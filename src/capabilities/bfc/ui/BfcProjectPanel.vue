<script setup lang="ts">
import { computed, ref } from "vue";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useBrowserStore } from "../../browser/public";

type ResultState = "idle" | "opened" | "blocked" | "error";
const layout = useLayoutStore();
const browser = useBrowserStore();
const latestBusy = ref(false);
const latestState = ref<ResultState>("idle");
const latestNotice = ref("尚未执行检查。BFC 本地快照不等于 B站最新收藏。");
const resultCode = ref("BFC_NOT_CHECKED");
const resultSource = ref("尚无记录");
const resultTime = ref("");
const loginConfirmed = ref(false);
const importConfirmed = ref(false);
const guideVisible = ref(true);
const links = [
  { id:"website", icon:"🌐", label:"BFC Web 版", description:"面向用户的专题视频入口", url:"https://github.com/code5m/bcf-website" },
  { id:"workbench", icon:"🧭", label:"BFC 工作台", description:"本地整理、分类与差异监测", url:"http://127.0.0.1:4198/#topics" },
  { id:"updates", icon:"🆕", label:"新发现的收藏", description:"在 BFC 工作台内查看已授权本地快照的新增 BV 候选", url:"http://127.0.0.1:4198/#updates" }
];
const statusLabel = computed(()=>({idle:"尚未检查",opened:"需在打开的页面继续确认",blocked:"用户报告访问受限",error:"打开页面失败"}[latestState.value]));
function record(state:ResultState, code:string, message:string, source:string) {
  latestState.value=state;
  resultCode.value=code;
  latestNotice.value=message;
  resultSource.value=source;
  resultTime.value=new Date().toLocaleString("zh-CN",{hour12:false});
}
async function openProject(url:string) {
  await browser.tabNew(url);
  layout.activateBrowser();
}
// The local BFC API is same-origin only and blocks account network access on
// HTTP 412. BrowserOS does not pretend navigating a tab proves live sync.
async function navigate(url:string, code:string, message:string) {
  if(latestBusy.value) return;
  latestBusy.value=true;
  try {
    await openProject(url);
    record("opened",code,message,"BrowserOS 导航成功（未校验页面或数据）");
  } catch {
    record("error","BFC_PAGE_NAVIGATION_FAILED","页面打开失败。请检查本地 BFC 工作台是否运行、端口 4198 是否可访问。没有发起实时抓取。","BrowserOS 导航失败");
  } finally { latestBusy.value=false; }
}
function openLatest() {
  void navigate("http://127.0.0.1:4198/#updates","BFC_LOCAL_SNAPSHOT_VIEW_OPENED","已打开「新发现的收藏」。请在该页面点击「刷新本地快照」查看候选 BV；不代表 B站实时同步成功。");
}
function acquireLatest() {
  void navigate("http://127.0.0.1:4198/manage#sync","BFC_MANUAL_IMPORT_OPENED","已请求打开「安全补采 · 导入已保存的网页」。请使用合规保存的 HTML/JSON，选择对应收藏夹并确认暂存；尚未取得实时新收藏。");
}
function openBiliPage() {
  void navigate("https://www.bilibili.com/","BFC_LOGIN_PAGE_OPENED","已请求打开 B站。请在浏览器中自行登录，进入个人收藏夹；不会读取或导出 Cookie。");
}
function confirmLogin() {
  loginConfirmed.value=true;
  record("opened","BFC_LOGIN_CONFIRMED_BY_USER","已记录你手动确认登录。BrowserOS 未读取登录凭据，也没有验证 B站会话。","用户手动确认");
}
function confirmImport() {
  importConfirmed.value=true;
  record("opened","BFC_IMPORT_REPORTED_BY_USER","已记录你在 BFC 页面完成了手动导入。请打开「新发现的收藏」读取本地对比结果；这不是实时同步成功确认。","用户手动确认");
}
function markBlocked() {
  record("blocked","BFC_HTTP_412_REPORTED","你报告 B站访问受限 / HTTP 412。已停止本次实时抓取尝试。请等待限制解除、确认正常登录；期间仅使用已保存的合规网页进行离线补采。","用户手动报告，未重新请求 B站");
}
</script>

<template>
  <section class="bfc-panel" aria-labelledby="bfc-title">
    <header class="bfc-head">
      <div>
        <p class="bfc-kicker">BFC · Bilibili Favorites Curator</p>
        <h1 id="bfc-title">个人内容库</h1>
        <p>在本地工作台整理收藏。若访问受限，明确告知结果并提供人工补采，不绕过平台风控。</p>
      </div>
      <span class="bfc-badge">可装卸模块</span>
    </header>
    <div class="bfc-links">
      <button v-for="link in links" :key="link.id" type="button" class="bfc-card" @click="openProject(link.url)">
        <span class="bfc-icon" aria-hidden="true">{{link.icon}}</span>
        <span class="bfc-copy"><strong>{{link.label}}</strong><small>{{link.description}}</small></span>
        <span aria-hidden="true">↗</span>
      </button>
    </div>

    <section class="bfc-feedback" aria-labelledby="bfc-access-title">
      <div class="bfc-section-head">
        <h2 id="bfc-access-title">最新收藏 · 状态与操作</h2>
        <span class="bfc-status-pill" :class="{danger:latestState==='blocked'||latestState==='error'}">{{statusLabel}}</span>
      </div>
      <p>近期 BFC 工作台曾遇到 HTTP 412 访问限制。这里不会擅自反复请求 B站；未获得真实新数据时不会显示「同步成功」。</p>
      <div class="bfc-actions">
        <button type="button" class="bfc-refresh" :disabled="latestBusy" @click="openLatest">⟳ 查看 / 检查本地新增收藏</button>
        <button type="button" class="bfc-refresh" :disabled="latestBusy" @click="acquireLatest">↗ 打开安全补采</button>
        <button type="button" class="bfc-outline" :disabled="latestBusy" @click="markBlocked">我遇到 HTTP 412 / 访问受限</button>
      </div>
      <div class="bfc-result" :class="{danger:latestState==='blocked'||latestState==='error'}" :role="latestState==='blocked'||latestState==='error'?'alert':'status'" aria-live="polite">
        <strong>本次返回：{{statusLabel}}</strong>
        <p>{{latestNotice}}</p>
        <dl>
          <div><dt>状态码</dt><dd>{{resultCode}}</dd></div>
          <div><dt>结果来源</dt><dd>{{resultSource}}</dd></div>
          <div><dt>操作时间</dt><dd>{{resultTime||'—'}}</dd></div>
          <div><dt>实时同步</dt><dd>未验证成功</dd></div>
        </dl>
      </div>
    </section>

    <section class="bfc-manual" aria-labelledby="bfc-manual-title">
      <div class="bfc-section-head">
        <h2 id="bfc-manual-title">手动补采与恢复</h2>
        <button type="button" class="bfc-text-action" :aria-expanded="guideVisible" aria-controls="bfc-manual-steps" @click="guideVisible=!guideVisible">{{guideVisible?'收起步骤':'展开步骤'}} {{guideVisible?'⌃':'⌄'}}</button>
      </div>
      <ol v-if="guideVisible" id="bfc-manual-steps" class="bfc-manual-steps">
        <li><div><b>打开 B站收藏夹</b><p>使用你平时的浏览器、正常网络和账号；先确认收藏列表在浏览器里可见。</p></div><button type="button" :disabled="latestBusy" @click="openBiliPage">打开 B站 ↗</button></li>
        <li><div><b>手动确认已经登录</b><p>仅记录人工确认，不抓取、保存或传播账号 Cookie。</p></div><button type="button" @click="confirmLogin">{{loginConfirmed?'已人工确认 ✓':'我已登录'}}</button></li>
        <li><div><b>保存网页并离线导入</b><p>在能正常访问的收藏页保存 HTML 或 JSON；到 BFC「安全补采」选择文件、原收藏夹并确认暂存。</p></div><button type="button" :disabled="latestBusy" @click="acquireLatest">打开导入页 →</button></li>
        <li><div><b>导入完成后核对</b><p>先确认导入页面返回成功，再到本地新发现列表重新对比；不要将本地差异标为实时收藏。</p></div><button type="button" @click="confirmImport">{{importConfirmed?'已人工确认 ✓':'我已完成导入'}}</button></li>
        <li><div><b>查看本地对比结果</b><p>检查新增候选、快照完整性和时间；仅当官方真实回读成功后才能宣称同步。</p></div><button type="button" :disabled="latestBusy" @click="openLatest">查看新发现 →</button></li>
      </ol>
      <details class="bfc-solutions">
        <summary>访问受限怎么解决？</summary>
        <p><b>HTTP 412 / 风控：</b>暂停频繁刷新，等待平台解除限制，在常用登录环境和正常网络下再尝试；不要使用代理轮换、Cookie 注入或绕过验证码。</p>
        <p><b>未登录 / 会话到期：</b>在 B站官方页面重新登录后确认收藏可见；不要将 Cookie 粘贴到 BrowserOS。</p>
        <p><b>BFC 页面无法打开：</b>确认本地 BFC 服务已运行并能访问 <code>127.0.0.1:4198</code>，然后重新打开安全补采。</p>
        <p><b>本地没有新增：</b>核实网页保存的分页是否齐全、原收藏夹是否选对；「0 条」不代表账号实时没有新增收藏。</p>
      </details>
    </section>
  </section>
</template>

<style scoped>
.bfc-panel{min-height:100%;padding:28px;background:var(--bfc-background,linear-gradient(135deg,#f4f7ff,#fff));color:var(--chrome-text,#3c4043)}
.bfc-head{display:flex;justify-content:space-between;gap:20px;align-items:flex-start;margin-bottom:24px}
.bfc-kicker{margin:0 0 8px;color:var(--bfc-accent,#315ea8);font-size:12px;letter-spacing:.08em}
h1{margin:0 0 10px;font-size:28px}.bfc-head p:not(.bfc-kicker){max-width:620px;margin:0;color:var(--chrome-text,#3c4043);line-height:1.7}
.bfc-badge{border:1px solid var(--chrome-border,#dadce0);border-radius:999px;padding:6px 10px;color:var(--bfc-accent,#315ea8);background:var(--chrome-address,#f1f3f4);white-space:nowrap;font-size:12px}
.bfc-links{display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:14px;max-width:860px}
.bfc-card{display:flex;align-items:center;gap:14px;padding:18px;border:1px solid var(--chrome-border,#dadce0);border-radius:14px;background:var(--chrome-toolbar,#fff);color:var(--chrome-text,#3c4043);text-align:left;cursor:pointer;box-shadow:0 4px 18px #315ea80d}
.bfc-card:hover{border-color:var(--bfc-accent,#315ea8)}.bfc-icon{font-size:28px}.bfc-copy{display:grid;gap:5px;flex:1}.bfc-copy small{color:var(--chrome-muted,#70757a)}
.bfc-feedback,.bfc-manual{max-width:1000px;margin-top:18px;background:var(--chrome-toolbar,#fff);border:1px solid var(--chrome-border,#dadce0);border-radius:12px;padding:18px}
.bfc-feedback>p{font-size:13px;line-height:1.7;color:var(--chrome-text,#3c4043)}
.bfc-section-head{display:flex;align-items:center;justify-content:space-between;gap:14px}.bfc-section-head h2{margin:0;font-size:17px}
.bfc-status-pill{border:1px solid var(--chrome-border,#dadce0);background:var(--chrome-address,#f1f3f4);border-radius:20px;padding:5px 12px;color:var(--chrome-text,#3c4043);font-size:12px}
.bfc-status-pill.danger,.bfc-result.danger{border-color:#b85e66;color:var(--chrome-text,#3c4043)}
.bfc-actions{display:flex;flex-wrap:wrap;gap:9px;margin:15px 0}
.bfc-refresh,.bfc-outline,.bfc-manual-steps button{background:var(--chrome-toolbar,#fff);border:1px solid var(--bfc-accent,#315ea8);border-radius:8px;padding:9px 12px;color:var(--bfc-accent,#315ea8);font-weight:600;cursor:pointer;font-size:12px}
.bfc-outline{border-color:var(--chrome-border,#dadce0);color:var(--chrome-text,#3c4043)}
.bfc-panel button:disabled{opacity:.6;cursor:wait}.bfc-panel button:focus-visible,.bfc-panel summary:focus-visible{outline:2px solid var(--bfc-accent,#315ea8);outline-offset:2px}
.bfc-result{border:1px solid var(--chrome-border,#dadce0);border-radius:9px;background:var(--chrome-address,#f1f3f4);padding:12px}
.bfc-result strong{font-size:13px}.bfc-result p{font-size:13px;line-height:1.6;margin:7px 0}.bfc-result dl{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px;margin:10px 0 0}.bfc-result dl>div{min-width:0}.bfc-result dt{color:var(--chrome-muted,#70757a);font-size:11px}.bfc-result dd{margin:2px 0 0;overflow-wrap:anywhere;font-size:12px}
.bfc-text-action{border:0;background:none;color:var(--bfc-accent,#315ea8);cursor:pointer;font-size:12px}
.bfc-manual-steps{counter-reset:step;list-style:none;padding:0;margin:14px 0;display:grid;gap:8px}
.bfc-manual-steps li{counter-increment:step;display:flex;align-items:center;gap:12px;border:1px solid var(--chrome-border,#dadce0);border-radius:8px;padding:12px}
.bfc-manual-steps li:before{content:counter(step);display:grid;place-items:center;flex-shrink:0;width:27px;height:27px;border:1px solid var(--bfc-accent,#315ea8);border-radius:50%;color:var(--bfc-accent,#315ea8);font-weight:700}
.bfc-manual-steps li>div{flex:1;min-width:0}.bfc-manual-steps b{font-size:13px}.bfc-manual-steps p{margin:4px 0 0;color:var(--chrome-muted,#70757a);line-height:1.6;font-size:12px}
.bfc-solutions{margin-top:12px;padding:12px;border:1px solid var(--chrome-border,#dadce0);border-radius:8px}.bfc-solutions summary{cursor:pointer;font-size:13px;font-weight:700}.bfc-solutions p{line-height:1.8;font-size:12px}.bfc-solutions code{font-size:11px;overflow-wrap:anywhere}
@media(max-width:720px){.bfc-panel{padding:18px}.bfc-head{display:block}.bfc-badge{display:inline-block;margin-top:14px}.bfc-manual-steps li{flex-wrap:wrap}.bfc-result dl{grid-template-columns:1fr}}
</style>