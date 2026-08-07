<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { bridge } from "./bridge";
import type {
  Artifact,
  RepoConfig,
  RepoProvider,
  SyncPreview,
  SyncJob,
  AuditEntry,
} from "./types";

const url = ref("");
const artifacts = ref<Artifact[]>([]);
const selected = reactive<Set<string>>(new Set());
const repos = ref<RepoConfig[]>([]);

const form = reactive({
  name: "",
  provider: "git" as RepoProvider,
  remote_url: "",
  branch: "main",
  username: "",
  token: "",
});

const preview = ref<SyncPreview | null>(null);
const job = ref<SyncJob | null>(null);
const audit = ref<AuditEntry[]>([]);
const msg = ref("");
const busy = ref(false); // 推送进行中

async function refresh() {
  [artifacts.value, repos.value, audit.value] = await Promise.all([
    bridge.listArtifacts(),
    bridge.listRepos(),
    bridge.auditLog(),
  ]);
}

function toggle(id: string) {
  if (selected.has(id)) selected.delete(id);
  else selected.add(id);
}

async function openBrowser() {
  if (!url.value) return;
  msg.value = "已在新窗口打开（右键网页选区可“保存到成果库”）";
  await bridge.openBrowser(url.value);
}

async function saveRepo() {
  if (!form.name || !form.remote_url || !form.token) {
    msg.value = "请填写 名称 / 远程地址 / Token";
    return;
  }
  const config: RepoConfig = {
    id: Math.random().toString(36).slice(2),
    provider: form.provider,
    name: form.name,
    remote_url: form.remote_url,
    branch: form.branch || "main",
    username: form.username || "oauth2",
  };
  await bridge.configureRepo({ config, token: form.token });
  form.name = "";
  form.remote_url = "";
  form.branch = "main";
  form.username = "";
  form.token = "";
  await refresh();
  msg.value = "仓库已保存（token 仅存于系统密钥库）";
}

async function requestSync(repoId: string) {
  const ids = [...selected];
  if (!ids.length) {
    msg.value = "请先勾选要同步的成果";
    return;
  }
  preview.value = await bridge.requestSync({ artifactIds: ids, repoId });
  msg.value = "已生成待确认同步任务，请确认后推送";
}

async function confirmSync() {
  if (!preview.value || busy.value) return;
  busy.value = true;
  // confirm_sync 立即返回 Running 任务；真正结果由 sync-completed 事件带回
  job.value = await bridge.confirmSync({ jobId: preview.value.job_id });
  msg.value = "推送中…（token 仅此刻从系统密钥库读取）";
}

// 填入码云(gitee)最小可用配置示例，用户补全用户名/Token 后保存即可
function loadGiteeExample() {
  form.provider = "gitee";
  form.name = "my-gitee-repo";
  form.remote_url = "https://gitee.com/你的用户名/你的仓库.git";
  form.branch = "master";
  form.username = "你的用户名";
  form.token = "";
  msg.value = "已填入码云示例，请补全用户名与 Token 后点「保存仓库」";
}

onMounted(async () => {
  await refresh();
  // 后台推送完成（成功/失败）回调
  bridge.onSyncCompleted((j) => {
    busy.value = false;
    job.value = j;
    preview.value = null;
    refresh();
    msg.value =
      j.status === "success"
        ? "✅ 推送成功"
        : `❌ 推送失败: ${j.error ?? ""}`;
  });
});
</script>

<template>
  <div class="app">
    <header>
      <h1>极智简单 · 浏览器OS融合</h1>
      <div class="addr">
        <input v-model="url" placeholder="输入网址，回车后可在网页右键保存" />
        <button @click="openBrowser">打开浏览</button>
      </div>
      <div class="msg">{{ msg }}</div>
    </header>

    <main>
      <section class="col">
        <h2>本地成果（带溯源）</h2>
        <button @click="refresh">刷新</button>
        <ul>
          <li v-for="a in artifacts" :key="a.id">
            <input type="checkbox" :checked="selected.has(a.id)" @change="toggle(a.id)" />
            <div>
              <div class="title">{{ a.title }}</div>
              <div class="meta">{{ a.source_url }} · {{ a.hash.slice(0, 12) }}</div>
            </div>
          </li>
          <li v-if="!artifacts.length" class="empty">暂无成果，去网页右键保存吧</li>
        </ul>
      </section>

      <section class="col">
        <h2>自有仓库（git / 码云）</h2>
        <div class="form">
          <input v-model="form.name" placeholder="仓库名" />
          <select v-model="form.provider">
            <option value="git">git</option>
            <option value="gitee">码云 gitee</option>
          </select>
          <input v-model="form.remote_url" placeholder="https://.../repo.git" />
          <input v-model="form.branch" placeholder="分支" />
          <input v-model="form.username" placeholder="用户名(可留空)" />
          <input v-model="form.token" type="password" placeholder="Token(仅存密钥库)" />
          <div class="form-actions">
            <button @click="saveRepo">保存仓库</button>
            <button class="ghost" @click="loadGiteeExample">填入码云示例</button>
          </div>
        </div>
        <ul>
          <li v-for="r in repos" :key="r.id" class="repo">
            <div>
              <div class="title">{{ r.name }} ({{ r.provider }})</div>
              <div class="meta">{{ r.remote_url }}</div>
            </div>
            <button @click="requestSync(r.id)">推送到此</button>
          </li>
          <li v-if="!repos.length" class="empty">还没配置仓库</li>
        </ul>
      </section>

      <section class="col">
        <h2>审计日志</h2>
        <ul class="audit">
          <li v-for="(e, i) in audit.slice().reverse()" :key="i">
            <span class="at">{{ e.at.slice(0, 19) }}</span>
            <span class="act">{{ e.action }}</span>
            <span class="det">{{ e.detail }}</span>
          </li>
          <li v-if="!audit.length" class="empty">暂无记录</li>
        </ul>
      </section>
    </main>

    <!-- 确认闸门 -->
    <div v-if="preview" class="modal-mask" @click.self="preview = null">
      <div class="modal">
        <h3>确认推送到「{{ preview.repo_name }}」？</h3>
        <p class="mono">{{ preview.remote_url }}</p>
        <p>将同步 {{ preview.artifact_count }} 个成果：</p>
        <ul>
          <li v-for="t in preview.artifact_titles" :key="t">· {{ t }}</li>
        </ul>
        <div v-if="busy" class="progress">推送中…（token 仅此刻从密钥库读取）</div>
        <div class="actions">
          <button :disabled="busy" @click="preview = null">取消</button>
          <button class="primary" :disabled="busy" @click="confirmSync">确认推送</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
* { box-sizing: border-box; }
body { margin: 0; font-family: system-ui, "PingFang SC", sans-serif; }
.app { display: flex; flex-direction: column; height: 100vh; }
header { padding: 12px 16px; border-bottom: 1px solid #eee; }
header h1 { font-size: 16px; margin: 0 0 8px; }
.addr { display: flex; gap: 8px; }
.addr input { flex: 1; padding: 6px 10px; border: 1px solid #ccc; border-radius: 6px; }
.msg { color: #666; font-size: 12px; margin-top: 6px; min-height: 14px; }
main { flex: 1; display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; padding: 12px; overflow: hidden; }
.col { border: 1px solid #eee; border-radius: 8px; padding: 10px; overflow: auto; }
.col h2 { font-size: 14px; margin: 0 0 8px; }
ul { list-style: none; padding: 0; margin: 0; }
li { padding: 8px; border-bottom: 1px solid #f2f2f2; display: flex; gap: 8px; align-items: center; }
.title { font-size: 13px; }
.meta { font-size: 11px; color: #999; word-break: break-all; }
.empty { color: #bbb; justify-content: center; }
.form { display: flex; flex-direction: column; gap: 6px; margin-bottom: 10px; }
.form input, .form select { padding: 6px 8px; border: 1px solid #ccc; border-radius: 6px; }
.form-actions { display: flex; gap: 8px; }
.form-actions button { flex: 1; }
button.ghost { background: #fff; }
.progress { color: #2b6; font-size: 13px; margin: 8px 0; }
.repo { justify-content: space-between; }
.audit .at { color: #999; font-size: 11px; width: 90px; }
.audit .act { color: #2b6; font-size: 12px; width: 90px; }
.audit .det { font-size: 12px; flex: 1; word-break: break-all; }
button { cursor: pointer; border: 1px solid #ccc; background: #fafafa; border-radius: 6px; padding: 6px 12px; }
button.primary { background: #2b6; color: #fff; border-color: #2b6; }
.modal-mask { position: fixed; inset: 0; background: rgba(0,0,0,.4); display: flex; align-items: center; justify-content: center; }
.modal { background: #fff; border-radius: 10px; padding: 20px; width: 420px; }
.modal .mono { font-family: monospace; font-size: 12px; color: #666; word-break: break-all; }
.modal .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
</style>
