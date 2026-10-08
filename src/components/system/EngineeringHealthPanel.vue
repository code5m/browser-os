<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { evaluateAll } from "./engineeringHealthModel.mjs";

type Phase = "idle" | "loading" | "ready" | "error";
type Run = { id: number; name: string; event: string; head_sha: string; created_at: string; updated_at: string; status: string; conclusion: string | null; html_url: string };
const phase = ref<Phase>("idle");
const branchSha = ref("");
const runs = ref<Run[]>([]);
const tagMatches = ref<boolean | null>(null);
const errorMessage = ref("");
const loadedAt = ref("");
let activeController: AbortController | null = null;

const governance = ref<any>(null);
const evidencePolicy = ref<any>(null);
const repository = "code5m/browser-os";
const api = "https://api.github.com/repos/" + repository;
const repoUrl = "https://github.com/" + repository;
const checked = computed(() => phase.value === "ready" ? evaluateAll(branchSha.value, runs.value) : null);
const gateCount = computed(() => governance.value?.gateFamilies?.length ?? null);
const native = computed(() => governance.value?.nativeSemantics ?? null);
const evidence = computed(() => evidencePolicy.value ?? null);
const labels: Record<string, string> = {
  PASS: "通过", FAIL: "失败", RUNNING: "运行中", STALE: "已过期", UNKNOWN: "未知",
};
function statusText(state: string) {
  return labels[state] || "未知";
}
function safeRunUrl(run: Run | null) {
  return run && Number.isInteger(run.id) && run.id > 0 ? repoUrl + "/actions/runs/" + run.id : repoUrl + "/actions";
}
async function jsonRequest(path: string, signal: AbortSignal): Promise<any> {
  const response = await fetch(api + path, {
    headers: { Accept: "application/vnd.github+json" },
    signal,
  });
  if (!response.ok) throw new Error("GitHub API HTTP " + response.status);
  return response.json();
}
async function refresh() {
  if (activeController) activeController.abort();
  const controller = new AbortController();
  activeController = controller;
  const timeout = setTimeout(() => controller.abort(), 12000);
  phase.value = "loading";
  errorMessage.value = "";
  branchSha.value = "";
  tagMatches.value = null;
  runs.value = [];
  try {
    const branch = await jsonRequest("/branches/master", controller.signal);
    const sha = branch?.commit?.sha;
    if (typeof sha !== "string" || !/^[a-f0-9]{40}$/i.test(sha)) throw new Error("GitHub 返回的 master SHA 无效");
    const rawUrl = "https://raw.githubusercontent.com/" + repository + "/" + encodeURIComponent(sha) + "/docs/engineering/";
    const [runsData, tagData, govResponse, policyResponse] = await Promise.all([
      jsonRequest("/actions/runs?head_sha=" + encodeURIComponent(sha) + "&per_page=100", controller.signal),
      jsonRequest("/git/ref/tags/capability-platform-v4-stable", controller.signal),
      fetch(rawUrl + "governance.json", { signal: controller.signal }),
      fetch(rawUrl + "evidence-policy.json", { signal: controller.signal }),
    ]);
    if (!govResponse.ok || !policyResponse.ok) throw new Error("无法读取当前 master 对应的工程治理配置");
    const [liveGov, livePolicy] = await Promise.all([govResponse.json(), policyResponse.json()]);
    if (liveGov?.repository !== repository || livePolicy?.schemaVersion !== 1) throw new Error("工程治理配置格式或仓库身份不匹配");
    if (!Array.isArray(runsData?.workflow_runs)) throw new Error("工作流数据格式无法识别");
    governance.value = liveGov;
    evidencePolicy.value = livePolicy;
    branchSha.value = sha;
    runs.value = runsData.workflow_runs;
    tagMatches.value = tagData?.object?.sha === liveGov.stableBaseline.commit;
    loadedAt.value = new Date().toLocaleString("zh-CN", { hour12: false });
    phase.value = "ready";
  } catch (error) {
    if (controller !== activeController) return;
    errorMessage.value = error instanceof Error && error.name === "AbortError"
      ? "连接 GitHub 超时，请检查网络后重试。"
      : "无法读取 GitHub 实时证据：" + String(error);
    phase.value = "error";
  } finally {
    clearTimeout(timeout);
    if (activeController === controller) activeController = null;
  }
}
onMounted(() => void refresh());
onBeforeUnmount(() => activeController?.abort());
</script>

<template>
  <section class="engineering-health" aria-label="工程健康中心" data-engineering-health>
    <header class="health-header">
      <div><h2>工程健康中心</h2><p>直接读取 GitHub 当前 master 的真实验证结果；无数据绝不显示通过。</p></div>
      <button type="button" :disabled="phase === 'loading'" @click="refresh">{{ phase === "loading" ? "读取中…" : "刷新状态" }}</button>
    </header>
    <div v-if="phase === 'loading'" role="status" class="health-note">正在核对 master、工作流与稳定基线…</div>
    <div v-if="phase === 'error'" role="alert" class="health-error">{{ errorMessage }}
      <a :href="repoUrl + '/actions'" target="_blank" rel="noopener noreferrer">打开 GitHub Actions</a>
    </div>

    <div class="health-overview">
      <div class="health-card">
        <small>整体验收</small>
        <strong :class="'health-' + (checked?.overall || 'UNKNOWN').toLowerCase()">
          {{ phase === 'loading' ? '检查中' : statusText(checked?.overall || 'UNKNOWN') }}
        </strong>
        <span>仅本次读取、仅当前 SHA，超过 48 小时标记过期</span>
      </div>
      <div class="health-card">
        <small>当前 master</small>
        <strong class="health-sha">{{ branchSha ? branchSha.slice(0, 12) : '尚未获取' }}</strong>
        <span>{{ loadedAt ? '获取时间：' + loadedAt : '未读取到可信快照' }}</span>
      </div>
      <div class="health-card">
        <small>稳定基线 v4</small>
        <strong :class="tagMatches === true ? 'health-pass' : tagMatches === false ? 'health-fail' : 'health-unknown'">
          {{ tagMatches === true ? '校验一致' : tagMatches === false ? 'SHA 不一致' : '尚未核对' }}
        </strong>
        <span>{{ governance?.stableBaseline?.tag || "尚未获取" }}</span>
      </div>
    </div>

    <h3>自动化门禁</h3>
    <div class="health-checks">
      <article v-for="row in checked?.checks || []" :key="row.id" class="health-check">
        <div><strong>{{ row.label }}</strong><p>{{ row.explanation }}</p><small>{{ row.reason }}</small></div>
        <div class="health-check-side"><span :class="'health-' + row.state.toLowerCase()">{{ statusText(row.state) }}</span>
          <a :href="safeRunUrl(row.run)" target="_blank" rel="noopener noreferrer">{{ row.run ? '查看运行与证据' : '查看工作流' }}</a>
        </div>
      </article>
    </div>
    <div v-if="phase !== 'ready'" role="status" class="health-note">当前未取得完整实时数据。请检查网络或前往 GitHub Actions，不能把历史通过当成现在通过。</div>
    <h3>语义与工程资产</h3>
    <div class="health-details">
      <div><strong>Rust 原生语义</strong><p>登记目标：{{ native?.expectedRegisteredCommands ?? "未知" }} 个命令 / {{ native?.expectedAppStateFields ?? "未知" }} 个 AppState 字段。由完整验收中的机器门禁核对，静态计数不代表实时通过。</p></div>
      <div><strong>治理与证据生命周期</strong><p>{{ gateCount }} 个登记门禁；Git 历史证据预算 {{ evidence ? (evidence.trackedHistoryBudgetBytes / 1000000).toFixed(0) : "未知" }} MB；CI 证据保留 {{ evidence?.classes?.ciArtifact?.retentionDays ?? "未知" }} 天。历史记录不自动删除。</p></div>
      <div><strong>供应链与发布债务</strong><p>历史依赖漏洞仍按明确基线受控，并不代表零漏洞。软件物料清单和校验和不等于发布安装包签名；独立发布证明仍需要对应的真实制品。</p></div>
    </div>
    <p class="health-footnote">数据源：GitHub Actions、GitHub refs、仓库内治理机器清单。网络不可用时显示未知；Gitee 自动镜像已退役，不再作为验收门禁。</p>
    <a class="health-link" :href="repoUrl + '/actions'" target="_blank" rel="noopener noreferrer">在 GitHub 查看所有门禁与 GUI 证据 ↗</a>
  </section>
</template>

<style scoped>
.engineering-health{padding:18px 0 24px;max-width:1024px;color:var(--ui-text,#243349)}
.health-header{display:flex;justify-content:space-between;align-items:flex-start;gap:16px;margin-bottom:16px}.health-header h2{margin:0;font-size:19px}.health-header p{margin:6px 0 0;color:var(--ui-muted,#667);font-size:12px;line-height:1.6}
.health-header button{flex:0 0 auto;background:#176b52;color:white;padding:8px 12px;border:0;border-radius:6px;cursor:pointer}.health-header button:disabled{opacity:.6}
.health-overview{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:12px}.health-card{border:1px solid var(--ui-border,#dbe3e8);border-radius:9px;padding:15px;background:var(--ui-surface,#fff);display:flex;flex-direction:column;gap:9px}.health-card small{color:var(--ui-muted,#667)}.health-card strong{font-size:20px}.health-card span{font-size:11px;color:var(--ui-muted,#667);word-break:break-word}.health-sha{font-family:ui-monospace,monospace;letter-spacing:.2px}
.engineering-health h3{font-size:14px;margin:22px 0 10px}.health-checks{display:grid;gap:8px}.health-check{border:1px solid var(--ui-border,#dbe3e8);padding:13px;border-radius:8px;display:flex;justify-content:space-between;align-items:center;gap:12px}.health-check p{margin:5px 0;font-size:12px;color:var(--ui-muted,#667)}.health-check small{font-size:11px;color:var(--ui-muted,#667)}.health-check-side{display:grid;gap:8px;text-align:right;flex:0 0 105px;font-size:13px}.health-check-side a,.health-link,.health-error a{color:var(--ui-accent,#2178a9)}.health-details{display:grid;gap:10px}.health-details>div{border-left:3px solid #a4b4c0;padding:3px 12px}.health-details strong{font-size:13px}.health-details p{font-size:12px;color:var(--ui-muted,#667);line-height:1.65;margin:6px 0}.health-footnote{font-size:11px;color:var(--ui-muted,#667);line-height:1.5}.health-link{font-size:13px}
.health-pass{color:#16845b}.health-fail{color:#c23b45}.health-running{color:#a37100}.health-stale{color:#a37100}.health-unknown{color:var(--ui-muted,#778)}.health-note{font-size:12px;padding:12px;border:1px solid var(--ui-border,#ddd);margin:10px 0}.health-error{font-size:12px;background:#fff0f0;color:#a53030;padding:12px;margin:10px 0;display:grid;gap:8px;border-radius:5px}
@media(max-width:650px){.health-header,.health-check{align-items:flex-start}.health-check{gap:8px}.health-header p{max-width:250px}.health-card strong{font-size:16px}}
</style>
