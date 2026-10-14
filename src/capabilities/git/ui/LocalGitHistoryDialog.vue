<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { bridge } from "../../../bridge";
import { useGitStore } from "../public";
import { redactSecrets } from "../../../utils/redact";

const git = useGitStore();
type Result = Awaited<ReturnType<typeof bridge.gitLocalHistory>>;
const data = ref<Result | null>(null);
const page = ref(0);
const loading = ref(false);
const error = ref("");
const selectedOid = ref("");
const diff = ref("");
const diffLoading = ref(false);
let requestId = 0;
let diffRequestId = 0;

function describe(error: unknown): string {
  const code = redactSecrets(error);
  if (code.includes("GIT_LOCAL_REPO_NOT_FOUND")) return "此文件或目录不属于本地 Git 仓库。";
  if (code.includes("GIT_LOCAL_PATH_NOT_ALLOWED") || code.includes("GIT_LOCAL_REPO_OUTSIDE_ALLOWED_ROOTS")) {
    return "所选 Git 根目录不在 BrowserOS 授权的本地目录中；请先授权并从项目根目录打开。";
  }
  return code;
}
async function loadPage(nextPage: number) {
  const path = git.localHistoryPath;
  if (!path || nextPage < 0 || nextPage > 20) return;
  const id = ++requestId;
  ++diffRequestId;
  page.value = nextPage;
  data.value = null;
  diff.value = "";
  selectedOid.value = "";
  loading.value = true;
  error.value = "";
  try {
    const result = await bridge.gitLocalHistory(path, nextPage);
    if (id === requestId && git.localHistoryPath === path) data.value = result;
  } catch (e) {
    if (id === requestId && git.localHistoryPath === path) error.value = describe(e);
  } finally {
    if (id === requestId) loading.value = false;
  }
}
async function chooseCommit(oid: string) {
  const path = git.localHistoryPath;
  if (!path || !data.value) return;
  const id = ++diffRequestId;
  selectedOid.value = oid;
  diff.value = "";
  diffLoading.value = true;
  try {
    const result = await bridge.gitLocalCommitDiff(path, oid);
    if (id === diffRequestId && path === git.localHistoryPath) diff.value = result || "此版本在选定范围内没有文本差异。";
  } catch (e) {
    if (id === diffRequestId && path === git.localHistoryPath) diff.value = describe(e);
  } finally {
    if (id === diffRequestId) diffLoading.value = false;
  }
}
function close() {
  requestId++; diffRequestId++;
  git.closeLocalHistory();
}
watch(() => git.localHistoryPath, (path) => {
  if (path) void loadPage(0);
  else { requestId++; diffRequestId++; data.value = null; }
}, { immediate: true });
onUnmounted(() => { requestId++; diffRequestId++; });
</script>

<template>
  <div v-if="git.localHistoryPath" class="modal-mask" @click.self="close" @keydown.esc.stop="close">
    <section class="local-git-history" role="dialog" aria-modal="true" aria-label="本地 Git 文件和目录历史">
      <header class="local-git-head">
        <div class="local-git-heading">
          <strong>Git 提交历史 · 本地仓库</strong>
          <small :title="git.localHistoryPath">{{git.localHistoryPath}}</small>
        </div>
        <button type="button" @click="close" aria-label="关闭本地 Git 提交历史">✕</button>
      </header>
      <p v-if="data" class="local-git-info">仓库：{{data.repository_root}} · 分支：{{data.branch}}
        <span v-if="data.relative_path"> · 路径：{{data.relative_path}}</span>
      </p>
      <p v-if="loading" class="local-git-info" role="status">正在从本地 Git 读取提交记录…</p>
      <p v-if="error" class="local-git-error" role="alert">{{error}}</p>
      <div v-if="data" class="local-git-main">
        <div class="local-git-list">
          <button v-for="c in data.entries" :key="c.oid" class="local-git-row"
            :class="{active:selectedOid===c.oid}" @click="chooseCommit(c.oid)">
            <strong>{{c.summary || "无标题提交"}}</strong>
            <small>{{c.oid.slice(0,9)}} · {{c.author}} · {{new Date(c.time*1000).toLocaleString()}}</small>
          </button>
          <p v-if="!data.entries.length" class="local-git-info">当前页没有相关提交。</p>
          <p v-if="data.scan_limited" class="local-git-info">已达到单次最多扫描 4,000 条提交的资源保护限制；结果可能不完整。</p>
          <div class="local-git-pagination">
            <button :disabled="loading || page===0" @click="loadPage(page-1)">上一页</button>
            <span>第 {{page+1}} 页</span>
            <button :disabled="loading || !data.has_more || page>=20" @click="loadPage(page+1)">下一页</button>
          </div>
        </div>
        <div class="local-git-diff">
          <p v-if="diffLoading" role="status">正在读取此提交在所选路径中的差异…</p>
          <pre v-else>{{selectedOid ? diff : "选择左侧提交查看此目录或文件的真实修改差异。"}}</pre>
        </div>
      </div>
      <footer>只读操作 · 直接访问所选本地仓库，不克隆、不 Fetch、不 Pull、不修改索引或工作区。历史不跨重命名自动追踪。</footer>
    </section>
  </div>
</template>

<style scoped>
.local-git-history{width:min(1060px,92vw);height:min(750px,85vh);display:flex;flex-direction:column;min-height:300px;background:#fff;border-radius:12px;box-shadow:0 16px 60px #0004;overflow:hidden;color:#25374a}
.local-git-head{display:flex;justify-content:space-between;gap:14px;align-items:center;padding:13px 16px;border-bottom:1px solid #dce5f0}
.local-git-heading{min-width:0;display:flex;flex-direction:column;gap:5px}.local-git-heading strong{font-size:15px}.local-git-heading small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#74869a}
.local-git-head>button{font-size:18px;border:0;background:none;cursor:pointer}
.local-git-info{margin:5px 12px;color:#64748b;font-size:12px;overflow-wrap:anywhere}
.local-git-error{margin:10px;color:#a32632}
.local-git-main{display:grid;grid-template-columns:minmax(240px,36%) minmax(0,1fr);gap:0;min-height:0;flex:1}
.local-git-list{display:flex;flex-direction:column;overflow:auto;border-right:1px solid #dce5f0}
.local-git-row{text-align:left;border:0;border-bottom:1px solid #eef2f6;background:#fff;padding:9px 11px;cursor:pointer}
.local-git-row:hover,.local-git-row.active{background:#e9f2ff}
.local-git-row strong{display:block;font-size:12px;overflow-wrap:anywhere}
.local-git-row small{display:block;color:#65758a;font-size:10px;padding-top:4px}
.local-git-pagination{display:flex;align-items:center;justify-content:center;gap:12px;margin-top:auto;padding:10px;font-size:12px}
.local-git-pagination button{border:1px solid #cbd8e8;border-radius:6px;background:white;padding:5px 10px}
.local-git-diff{overflow:auto;min-width:0;background:#f9fbff}
.local-git-diff pre{margin:0;padding:12px;white-space:pre;overflow:auto;font:12px/1.6 ui-monospace,Consolas,monospace}
.local-git-diff>p{padding:12px;font-size:12px}
.local-git-history footer{border-top:1px solid #e5eaf0;padding:9px 14px;color:#627389;font-size:11px}
@media(max-width:700px){.local-git-main{grid-template-columns:minmax(0,1fr)}.local-git-list{max-height:210px;border-bottom:1px solid #e3e7ee}.local-git-history{height:90vh}}
</style>
