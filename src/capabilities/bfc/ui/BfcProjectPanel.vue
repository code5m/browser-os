<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useBfcStore } from "../state/useBfcStore";

const bfc = useBfcStore();

const phaseLabel = computed(() => ({
  idle: "尚未读取",
  checking_service: "检查本地服务",
  checking_login: "检查登录状态",
  logged_out: "需要登录",
  access_limited: "平台访问受限",
  unknown: "无法确认",
  opening_favorites: "打开收藏夹",
  reading_favorites: "读取收藏分页",
  ready_to_import: "快照已读取，等待保存",
  importing: "保存到本地 BFC",
  imported: "已交给本地 BFC",
  outcome_unknown: "保存结果不确定",
  failed: "读取或保存失败",
  cancelled: "已取消",
}[bfc.phase] || "尚未读取"));

const phaseIsError = computed(() =>
  ["access_limited", "unknown", "outcome_unknown", "failed"].includes(bfc.phase),
);

onMounted(() => {
  void bfc.refreshService().catch(() => {});
});
</script>

<template>
  <section class="bfc-panel" aria-labelledby="bfc-title">
    <div class="bfc-content">
      <header class="bfc-head">
        <div>
          <p class="bfc-kicker">BFC · BILIBILI FAVORITES CURATOR</p>
          <h1 id="bfc-title">个人内容库</h1>
          <p class="bfc-subtitle">把 B站收藏读取到本地 BFC，整理成自己的专题视频库。只读取页面实际显示的内容，不读取 Cookie，不修改账号收藏。</p>
        </div>
        <span class="bfc-module">BrowserOS 集成模块</span>
      </header>

      <div class="bfc-links">
        <button type="button" class="bfc-card" @click="bfc.openLocalUpdates">
          <span class="bfc-icon" aria-hidden="true">▦</span>
          <span class="bfc-copy"><strong>本地 BFC 工作台</strong><small>查看专题、视频库与已导入的新增候选</small></span>
          <span aria-hidden="true">↗</span>
        </button>
        <button type="button" class="bfc-card" @click="bfc.openManualImport">
          <span class="bfc-icon" aria-hidden="true">⇧</span>
          <span class="bfc-copy"><strong>安全补采 · 离线导入</strong><small>实时读取受限时，使用你自行保存的 HTML / JSON</small></span>
          <span aria-hidden="true">↗</span>
        </button>
      </div>

      <section class="bfc-section" aria-labelledby="bfc-service-title">
        <div class="bfc-section-head">
          <div>
            <p class="bfc-eyebrow">LOCAL SERVICE</p>
            <h2 id="bfc-service-title">本地服务状态</h2>
          </div>
          <span class="bfc-pill" :class="{danger:!bfc.serviceStatus?.online}">
            {{bfc.serviceStatus?.online ? "本机服务在线" : "本机服务不可用"}}
          </span>
        </div>
        <div class="bfc-service-grid">
          <div><span>本地地址</span><strong>127.0.0.1:4198</strong></div>
          <div><span>本地服务隔离</span><strong>{{bfc.serviceStatus?.local_only === true ? "已确认" : "未确认"}}</strong></div>
          <div><span>读取模式</span><strong>{{bfc.serviceStatus?.read_snapshot_only === true ? "只读快照" : "未确认"}}</strong></div>
          <div><span>B站网络请求</span><strong>{{bfc.serviceStatus?.bilibili_network_requests ?? "—"}}</strong></div>
        </div>
        <p class="bfc-muted" v-if="bfc.serviceStatus?.complete_snapshot === false">
          当前 BFC 服务尚未声明完整快照能力。导入结果会保留为待核实 / 不完整快照，不能据此宣称账号已完整同步。
        </p>
        <div class="bfc-actions">
          <button type="button" class="bfc-secondary" :disabled="bfc.busy" @click="bfc.refreshService">重新检查服务</button>
        </div>
      </section>

      <section class="bfc-section" aria-labelledby="bfc-read-title">
        <div class="bfc-section-head">
          <div>
            <p class="bfc-eyebrow">USER-INITIATED READBACK</p>
            <h2 id="bfc-read-title">读取最新收藏</h2>
          </div>
          <span class="bfc-pill" :class="{danger:phaseIsError}">
            {{phaseLabel}}
          </span>
        </div>
        <p class="bfc-description">
          点击后，BrowserOS 会使用当前 B站页面检查可见登录状态，再读取该账号收藏夹的页面内容和分页。只有总数、分页和 BV 号核对通过后，才会提交到本机 BFC。
        </p>
        <div class="bfc-actions">
          <button type="button" class="bfc-primary" :disabled="bfc.busy || !bfc.serviceStatus?.online" @click="bfc.startReadback">
            {{bfc.busy ? "正在处理…" : "开始读取当前账号收藏"}}
          </button>
          <button type="button" class="bfc-secondary" :disabled="bfc.busy" @click="bfc.openBiliPage">打开 B站 / 登录</button>
          <button v-if="bfc.busy && bfc.ticket && ['checking_login', 'opening_favorites', 'reading_favorites'].includes(bfc.phase)" type="button" class="bfc-danger" @click="bfc.cancelReadback">停止本次读取</button>
          <button v-if="bfc.phase === 'ready_to_import'" type="button" class="bfc-primary" :disabled="bfc.busy" @click="bfc.finishReadback">重试保存到本地 BFC</button>
        </div>
        <div class="bfc-result" :class="{danger:phaseIsError}" role="status" aria-live="polite">
          <strong>{{phaseLabel}}</strong>
          <p>{{bfc.message}}</p>
          <dl>
            <div><dt>状态码</dt><dd>{{bfc.readbackStatus?.message_code || bfc.lastErrorCode || "—"}}</dd></div>
            <div><dt>登录状态</dt><dd>{{bfc.readbackStatus?.login_status || "尚未确认"}}</dd></div>
            <div><dt>账号 MID</dt><dd>{{bfc.readbackStatus?.mid || "—"}}</dd></div>
            <div><dt>收藏夹</dt><dd>{{bfc.readbackStatus?.folder_name || "—"}}</dd></div>
            <div><dt>分页进度</dt><dd>{{bfc.readbackStatus?.pages_captured ?? 0}} / {{bfc.readbackStatus?.expected_pages ?? "—"}}</dd></div>
            <div><dt>页面条目数</dt><dd>{{bfc.readbackStatus?.items_captured ?? 0}}</dd></div>
          </dl>
        </div>
        <div v-if="bfc.importResult" class="bfc-import-result">
          <strong>本机 BFC 接收结果</strong>
          <div class="bfc-service-grid">
            <div><span>读取页数</span><strong>{{bfc.importResult.pages}}</strong></div>
            <div><span>收藏条目</span><strong>{{bfc.importResult.cards}}</strong></div>
            <div><span>唯一 BV</span><strong>{{bfc.importResult.unique}}</strong></div>
            <div><span>差异缺口</span><strong>{{bfc.importResult.gap ?? "待核实"}}</strong></div>
          </div>
        </div>
      </section>

      <section class="bfc-section" aria-labelledby="bfc-safety-title">
        <div class="bfc-section-head">
          <div>
            <p class="bfc-eyebrow">SAFETY & RECOVERY</p>
            <h2 id="bfc-safety-title">访问受限与安全补采</h2>
          </div>
        </div>
        <div class="bfc-guides">
          <article>
            <h3>HTTP 412 / 风控验证</h3>
            <p>立即停止读取并等待平台限制解除。不要轮换代理、导入他人 Cookie、注入 Cookie 或绕过验证码。</p>
          </article>
          <article>
            <h3>登录状态无法确认</h3>
            <p>在 B站官方页面正常登录，确认收藏夹可以显示，再回到这里重新开始。BrowserOS 不保存账号凭据。</p>
          </article>
          <article>
            <h3>页面结构或分页无法识别</h3>
            <p>本次不会导入不完整结果。可将你自行保存的收藏网页通过 BFC 安全补采导入，并在工作台检查快照完整性。</p>
          </article>
          <article>
            <h3>本地保存结果不确定</h3>
            <p>不要立即重复导入。先到本地 BFC 工作台检查本次扫描记录，避免重复写入或误判。</p>
          </article>
        </div>
        <div class="bfc-actions">
          <button type="button" class="bfc-secondary" @click="bfc.openManualImport">打开安全补采</button>
          <button type="button" class="bfc-secondary" @click="bfc.openLocalUpdates">查看本地新增候选</button>
        </div>
      </section>

      <footer class="bfc-footer">
        BFC 读取的是用户当前登录后实际渲染的收藏页面。页面未显示、分页不完整或平台访问受限时，不会声称已同步。
      </footer>
    </div>
  </section>
</template>

<style scoped>
.bfc-panel{width:100%;height:100%;min-height:0;box-sizing:border-box;overflow:auto;padding:28px 32px;background:var(--chrome-bg,#f5f7fa);color:var(--chrome-text,#273444)}
.bfc-content{width:100%;max-width:1440px;margin:0 auto}
.bfc-head{display:flex;justify-content:space-between;align-items:flex-start;gap:24px;margin-bottom:22px}
.bfc-kicker,.bfc-eyebrow{margin:0 0 7px;color:var(--bfc-accent,#315ea8);font-size:11px;font-weight:700;letter-spacing:.1em}
.bfc-head h1{margin:0 0 8px;font-size:27px;line-height:1.25;font-weight:700}
.bfc-subtitle{max-width:900px;margin:0;color:var(--chrome-muted,#667085);font-size:13px;line-height:1.7}
.bfc-module{flex:none;padding:6px 10px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:999px;background:var(--chrome-toolbar,#fff);color:var(--chrome-muted,#667085);font-size:11px}
.bfc-links{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;margin-bottom:16px}
.bfc-card{display:flex;align-items:center;gap:14px;min-width:0;padding:17px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:12px;background:var(--chrome-toolbar,#fff);color:inherit;text-align:left;cursor:pointer}
.bfc-card:hover{border-color:var(--bfc-accent,#315ea8);background:var(--chrome-hover,#f8faff)}
.bfc-icon{display:grid;place-items:center;flex:none;width:38px;height:38px;border-radius:10px;background:var(--chrome-address,#eef2f7);color:var(--bfc-accent,#315ea8);font-size:22px}
.bfc-copy{display:grid;gap:4px;flex:1;min-width:0}.bfc-copy strong{font-size:13px}.bfc-copy small{color:var(--chrome-muted,#667085);font-size:11px;line-height:1.5}
.bfc-section{margin-top:15px;padding:19px 20px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:12px;background:var(--chrome-toolbar,#fff)}
.bfc-section-head{display:flex;align-items:center;justify-content:space-between;gap:18px;margin-bottom:12px}
.bfc-section-head h2{margin:0;font-size:17px;line-height:1.4}
.bfc-pill{flex:none;padding:5px 10px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:999px;background:var(--chrome-address,#f5f7fa);color:var(--chrome-text,#273444);font-size:11px}
.bfc-pill.danger{border-color:#d9a1a5;color:#9a3138;background:#fff8f8}
.bfc-service-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px}
.bfc-service-grid>div{display:grid;gap:5px;min-width:0;padding:10px 12px;border-radius:8px;background:var(--chrome-address,#f5f7fa)}
.bfc-service-grid span{color:var(--chrome-muted,#667085);font-size:11px}
.bfc-service-grid strong{overflow-wrap:anywhere;font-size:12px;font-weight:650}
.bfc-muted,.bfc-description{color:var(--chrome-muted,#667085);font-size:12px;line-height:1.75}
.bfc-muted{margin:12px 0 0;padding:10px 12px;border-left:3px solid #c48b36;background:#fffaf1;color:#76541e}
.bfc-description{margin:0 0 14px}
.bfc-actions{display:flex;flex-wrap:wrap;gap:9px;margin-top:14px}
.bfc-primary,.bfc-secondary,.bfc-danger{min-height:34px;padding:8px 13px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:8px;background:var(--chrome-toolbar,#fff);color:var(--chrome-text,#273444);font-size:12px;font-weight:650;cursor:pointer}
.bfc-primary{border-color:var(--bfc-accent,#315ea8);background:var(--bfc-accent,#315ea8);color:#fff}
.bfc-primary:hover{filter:brightness(.96)}
.bfc-secondary:hover{border-color:var(--bfc-accent,#315ea8);color:var(--bfc-accent,#315ea8)}
.bfc-danger{border-color:#d9a1a5;color:#9a3138;background:#fff8f8}
.bfc-panel button:disabled{opacity:.55;cursor:not-allowed}
.bfc-panel button:focus-visible,.bfc-panel summary:focus-visible{outline:2px solid var(--bfc-accent,#315ea8);outline-offset:2px}
.bfc-result{margin-top:14px;padding:13px 14px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:9px;background:var(--chrome-address,#f5f7fa)}
.bfc-result.danger{border-color:#d9a1a5;background:#fff8f8}
.bfc-result>strong,.bfc-import-result>strong{font-size:12px}
.bfc-result p{margin:7px 0 10px;font-size:12px;line-height:1.7}
.bfc-result dl{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px;margin:0}
.bfc-result dl>div{min-width:0}.bfc-result dt{color:var(--chrome-muted,#667085);font-size:10px}.bfc-result dd{margin:3px 0 0;overflow-wrap:anywhere;font-size:11px}
.bfc-import-result{margin-top:12px;padding:13px 14px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:9px}
.bfc-import-result .bfc-service-grid{margin-top:10px}
.bfc-guides{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px}
.bfc-guides article{padding:12px 13px;border:1px solid var(--chrome-border,#d9e0e8);border-radius:9px;background:var(--chrome-address,#f8f9fb)}
.bfc-guides h3{margin:0 0 6px;font-size:12px}.bfc-guides p{margin:0;color:var(--chrome-muted,#667085);font-size:11px;line-height:1.7}
.bfc-footer{padding:18px 2px 4px;color:var(--chrome-muted,#667085);font-size:11px;line-height:1.7}
@media(max-width:850px){.bfc-panel{padding:20px}.bfc-links,.bfc-guides{grid-template-columns:1fr}.bfc-service-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}
@media(max-width:520px){.bfc-panel{padding:14px}.bfc-head{display:block}.bfc-module{display:inline-block;margin-top:10px}.bfc-section{padding:15px}.bfc-result dl{grid-template-columns:repeat(2,minmax(0,1fr))}}
</style>
