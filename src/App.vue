<script setup lang="ts">
import { ref, reactive, computed, onMounted } from "vue";
import { bridge } from "./bridge";
import type {
  Artifact,
  RepoConfig,
  RepoProvider,
  SyncPreview,
  SyncJob,
  AuditEntry,
  WorkspaceTree,
  DomainItem,
  DirEntry,
} from "./types";

const url = ref("");
const tree = ref<WorkspaceTree>({ nodes: [] });
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
const busy = ref(false);

// 左栏 tab 切换：成果库 / 文件浏览器
const leftTab = ref<"artifacts" | "files">("files"); // 默认显示文件浏览器

// 成果编辑
const current = ref<Artifact | null>(null);
const editTitle = ref("");
const editTags = ref("");
const editText = ref("");

// 文件浏览器
const fileEntries = ref<DirEntry[]>([]);
const filePath = ref(""); // 当前浏览路径
const fileContent = ref(""); // 文件内容预览/编辑
const editingFile = ref(false); // 是否处于编辑模式
const startDirs = ref<DirEntry[]>([]);

const flatArtifacts = computed(() =>
  tree.value.nodes.flatMap((n) => n.items)
);

async function refresh() {
  [tree.value, repos.value, audit.value] = await Promise.all([
    bridge.browseWorkspace(),
    bridge.listRepos(),
    bridge.auditLog(),
  ]);
}

function toggle(id: string) {
  if (selected.has(id)) selected.delete(id);
  else selected.add(id);
}

async function openBrowser() {
  const target = url.value.trim() || "https://www.baidu.com";
  try {
    msg.value = "正在打开浏览器…";
    await bridge.openBrowser(target);
    msg.value = "已打开。在网页右键 → 保存选区/整页";
  } catch (e: any) {
    msg.value = "打开失败: " + (e ?? e);
  }
}

// ====== 成果操作 ======
async function openArtifact(item: DomainItem) {
  const art = await bridge.readArtifact(item.id);
  current.value = art;
  editTitle.value = art.title;
  editTags.value = art.tags.join(", ");
  editText.value = art.text;
}

async function saveEdit() {
  if (!current.value) return;
  const tags = editTags.value.split(",").map((s) => s.trim()).filter(Boolean);
  await bridge.updateArtifact({
    id: current.value.id,
    title: editTitle.value,
    text: editText.value,
    tags,
  });
  await refresh();
  msg.value = "已保存编辑";
}

async function removeArtifact(item: DomainItem) {
  if (!confirm(`删除「${item.title}」？`)) return;
  await bridge.deleteArtifact(item.id);
  if (current.value?.id === item.id) current.value = null;
  await refresh();
}

// ====== 文件浏览器 ======
async function loadStartDirs() {
  startDirs.value = await bridge.getStartDirs();
}

async function enterDir(path: string) {
  filePath.value = path;
  fileContent.value = "";
  try {
    fileEntries.value = await bridge.listDir(path);
  } catch (e: any) {
    msg.value = "无法读取目录: " + (e ?? e);
    fileEntries.value = [];
  }
}

async function openFile(entry: DirEntry) {
  if (entry.is_dir) { enterDir(entry.path); return; }
  // 简单判断是否为文本文件
  const ext = entry.name.split(".").pop()?.toLowerCase() || "";
  const textExts = ["txt","md","json","js","ts","vue","rs","html","css","xml","yaml","yml","toml","csv","log","sh","py","java","go","c","cpp","h","sql","env","gitignore"];
  if (!textExts.includes(ext)) {
    msg.value = `非文本文件 (${entry.name})，暂不支持预览`;
    return;
  }
  try {
    fileContent.value = await bridge.readFile(entry.path);
    filePath.value = entry.path; // 记录当前打开的文件路径
    editingFile.value = true; // 自动进入编辑模式
    msg.value = `已加载: ${entry.name} (${(entry.size).toLocaleString()} bytes) — 可直接编辑并保存`;
  } catch (e: any) {
    msg.value = "读取失败: " + (e ?? e);
  }
}

function goUp() {
  const p = filePath.value;
  if (!p) return;
  const parent = p.split("/").slice(0, -1).join("/") || "/";
  enterDir(parent);
}

async function saveFile() {
  if (!filePath.value || !fileContent.value) return;
  try {
    await bridge.writeFile(filePath.value, fileContent.value);
    msg.value = "已保存: " + filePath.value.split("/").pop();
  } catch (e: any) {
    msg.value = "保存失败: " + (e ?? e);
  }
}

function formatSize(b: number): string {
  if (b < 1024) return b + " B";
  if (b < 1048576) return (b / 1024).toFixed(1) + " KB";
  return (b / 1048576).toFixed(1) + " MB";
}

// ====== 同步 ======
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
  form.name = ""; form.remote_url = ""; form.branch = "main"; form.username = ""; form.token = "";
  await refresh();
  msg.value = "仓库已保存（token 仅存密钥库）";
}

async function requestSync(repoId: string) {
  const ids = [...selected];
  if (!ids.length) { msg.value = "请先勾选要同步的成果"; return; }
  preview.value = await bridge.requestSync({ artifactIds: ids, repoId });
  msg.value = "已生成待确认同步任务，请确认后推送";
}

async function confirmSync() {
  if (!preview.value || busy.value) return;
  busy.value = true;
  job.value = await bridge.confirmSync({ jobId: preview.value.job_id });
  msg.value = "推送中…（token 仅此刻从密钥库读取）";
}

function loadGiteeExample() {
  form.provider = "gitee";
  form.name = "my-gitee-repo";
  form.remote_url = "https://gitee.com/你的用户名/你的仓库.git";
  form.branch = "master";
  form.username = "你的用户名";
  form.token = "";
  msg.value = "已填入码云示例，补全用户名与 Token 后点保存";
}

onMounted(async () => {
  await refresh();
  await loadStartDirs();
  // 默认进入成果工作区目录
  if (startDirs.value.length > 0) {
    const ws = startDirs.value.find((d) => d.name.includes("成果工作区"));
    if (ws) enterDir(ws.path);
    else enterDir(startDirs.value[0].path);
  }
  bridge.onSyncCompleted((j) => {
    busy.value = false; job.value = j; preview.value = null; refresh();
    msg.value = j.status === "success" ? "✅ 推送成功" : `❌ 推送失败: ${j.error ?? ""}`;
  });
});
</script>

<template>
  <div class="app">
    <header>
      <h1>浏览器OS融合</h1>
      <div class="addr">
        <input v-model="url" placeholder="输入网址或搜索词（如 baidu.com、天气）" @keyup.enter="openBrowser" />
        <button @click="openBrowser">打开浏览</button>
      </div>
      <div class="msg">{{ msg }}</div>
    </header>

    <main>
      <!-- 左栏：成果库 / 文件浏览器 -->
      <section class="col">
        <!-- Tab 切换 -->
        <div class="tabs">
          <button :class="{ active: leftTab === 'files' }" @click="leftTab = 'files'">📂 本地文件</button>
          <button :class="{ active: leftTab === 'artifacts' }" @click="leftTab = 'artifacts'">📦 成果库</button>
        </div>

        <!-- === 文件浏览器 === -->
        <div v-if="leftTab === 'files'" class="file-browser">
          <!-- 路径栏 -->
          <div class="path-bar">
            <button v-if="filePath" @click="goUp" title="上级">⬆</button>
            <span class="path" @click="loadStartDirs" title="回到起始">{{ filePath || "选择起始目录" }}</span>
          </div>
          <!-- 快捷入口 -->
          <div v-if="!filePath && startDirs.length" class="quick-dirs">
            <div v-for="d in startDirs" :key="d.path" class="qdir" @click="enterDir(d.path)">
              {{ d.name }}
            </div>
          </div>
          <!-- 目录列表 -->
          <ul class="file-list">
            <li v-for="f in fileEntries" :key="f.path"
                :class="{ 'is-dir': f.is_dir }"
                @click="openFile(f)">
              <span class="icon">{{ f.is_dir ? "📁" : "📄" }}</span>
              <span class="fname">{{ f.name }}</span>
              <span class="fsize">{{ formatSize(f.size) }}</span>
            </li>
            <li v-if="!fileEntries.length && filePath" class="empty">空目录</li>
          </ul>
          <!-- 文件预览/编辑 -->
          <div v-if="fileContent" class="preview">
            <div class="preview-header">
              <h4>📝 {{ filePath?.split("/").pop() }}</h4>
              <div class="preview-actions">
                <button class="primary" @click="saveFile" title="保存文件">💾 保存</button>
                <button @click="fileContent = ''; editingFile = false" title="关闭">✕ 关闭</button>
              </div>
            </div>
            <textarea v-model="fileContent" class="file-editor"
              spellcheck="false"
              placeholder="文件内容..."></textarea>
          </div>
        </div>

        <!-- === 成果库 === -->
        <div v-else class="tree">
          <div v-for="node in tree.nodes" :key="node.host" class="domain">
            <div class="domain-name">📁 {{ node.host }} <span class="count">{{ node.items.length }}</span></div>
            <div v-for="item in node.items" :key="item.id"
                 class="item" :class="{ active: current && current.id === item.id, sel: selected.has(item.id) }">
              <input type="checkbox" :checked="selected.has(item.id)" @click.stop="toggle(item.id)" />
              <div class="item-body" @click="openArtifact(item)">
                <div class="title">{{ item.title }}</div>
                <div class="meta">{{ item.created_at.slice(0,10) }} <span v-for="t in item.tags" :key="t" class="tag">#{{ t }}</span></div>
              </div>
              <button class="del" @click.stop="removeArtifact(item)">✕</button>
            </div>
          </div>
          <div v-if="!flatArtifacts.length" class="empty">暂无成果。打开浏览器 → 右键 → 保存选区/整页</div>

          <!-- 编辑面板 -->
          <div v-if="current" class="editor">
            <h3>编辑成果</h3>
            <input v-model="editTitle" placeholder="标题" />
            <input v-model="editTags" placeholder="标签（逗号分隔）" />
            <textarea v-model="editText" placeholder="正文"></textarea>
            <div class="src">来源：<a :href="current.source_url" target="_blank">{{ current.source_url }}</a></div>
            <div class="src">溯源哈希：{{ current.hash.slice(0,16) }}</div>
            <button class="primary" @click="saveEdit">保存编辑</button>
          </div>
        </div>
      </section>

      <!-- 中：浏览器 / 采集说明 -->
      <section class="col">
        <h2>浏览器 / 采集</h2>
        <ol class="guide">
          <li>点「打开浏览」在右侧开网页。</li>
          <li><b>在网页任意位置右键</b> → 弹出自定义菜单 → 选「保存选区」或「保存整页」。</li>
          <li>左侧「成果库」tab 会即时出现采集结果，可预览/编辑/打标签。</li>
          <li>勾选成果 → 右侧「推送到此」备份到自有仓库。</li>
        </ol>
        <div class="note">
          💡 右键菜单会覆盖系统原生菜单（Back/Forward 等）。<br>
          🖼 图片自动转 base64 内联，离线可看。<br>
          📄 整页保存含富文本 HTML 保真。
        </div>
      </section>

      <!-- 右：仓库 + 审计 -->
      <section class="col">
        <h2>自有仓库（git / 码云）</h2>
        <div class="form">
          <input v-model="form.name" placeholder="仓库名" />
          <select v-model="form.provider"><option value="git">git</option><option value="gitee">码云 gitee</option></select>
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
            <div><div class="title">{{ r.name }} ({{ r.provider }})</div><div class="meta">{{ r.remote_url }}</div></div>
            <button @click="requestSync(r.id)">推送到此</button>
          </li>
          <li v-if="!repos.length" class="empty">还没配置仓库</li>
        </ul>

        <h2 style="margin-top:12px">审计日志</h2>
        <ul class="audit">
          <li v-for="(e,i) in audit.slice().reverse()" :key="i">
            <span class="at">{{ e.at.slice(0,19) }}</span>
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
        <ul><li v-for="t in preview.artifact_titles" :key="t">· {{ t }}</li></ul>
        <div v-if="busy" class="progress">推送中…</div>
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
main { flex: 1; display: grid; grid-template-columns: 1.3fr 1fr 1fr; gap: 12px; padding: 12px; overflow: hidden; }
.col { border: 1px solid #eee; border-radius: 8px; padding: 10px; overflow: auto; }
.col h2 { font-size: 14px; margin: 0 0 8px; }

/* Tabs */
.tabs { display: flex; gap: 0; margin-bottom: 8px; border-bottom: 1px solid #eee; }
.tabs button { background: none; border: none; border-bottom: 2px solid transparent; padding: 6px 14px; cursor: pointer; font-size: 13px; color: #888; }
.tabs button.active { color: #2b6; border-bottom-color: #2b6; font-weight: 600; }

/* File browser */
.file-browser { display: flex; flex-direction: column; height: calc(100% - 40px); }
.path-bar { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
.path-bar button { padding: 2px 8px; font-size: 12px; }
.path-bar .path { font-size: 11px; color: #666; cursor: pointer; word-break: break-all; flex: 1; }
.quick-dirs { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
.qdir { padding: 5px 10px; background: #f0f4ff; border-radius: 4px; font-size: 12px; cursor: pointer; color: #2b6; }
.qdir:hover { background: #dce8ff; }
.file-list { list-style: none; padding: 0; margin: 0; flex: 1; overflow: auto; }
.file-list li { display: flex; align-items: center; gap: 6px; padding: 5px 6px; border-bottom: 1px solid #f6f6f6; cursor: pointer; border-radius: 3px; }
.file-list li:hover { background: #f9f9f9; }
.file-list .icon { font-size: 14px; width: 20px; text-align: center; }
.file-list .fname { flex: 1; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-list .fsize { font-size: 10px; color: #bbb; }
.preview { margin-top: 8px; border-top: 1px dashed #ddd; padding-top: 6px; }
.preview h4 { font-size: 12px; margin: 0 0 4px; }
.preview-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 4px; }
.preview-actions { display: flex; gap: 4px; }
.preview-actions button { font-size: 11px; padding: 3px 10px; }
.file-editor { width: 100%; min-height: 200px; max-height: 400px; font-family: monospace; font-size: 12px; padding: 8px; border: 1px solid #ccc; border-radius: 4px; resize: vertical; background: #f6f6f6; overflow: auto; white-space: pre; word-break: keep-all; line-height: 1.5; }

/* Artifacts */
.tree { margin-bottom: 8px; }
.domain { margin-bottom: 8px; }
.domain-name { font-size: 12px; color: #666; font-weight: 600; padding: 4px 0; }
.count { color: #bbb; font-weight: 400; }
.item { display: flex; gap: 6px; align-items: center; padding: 6px; border-bottom: 1px solid #f4f4f4; border-radius: 4px; }
.item.active { background: #eafaf0; }
.item .item-body { flex: 1; cursor: pointer; min-width: 0; }
.item .title { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.item .meta { font-size: 11px; color: #999; }
.tag { color: #2b6; margin-right: 4px; }
.del { border: none; background: transparent; color: #c33; cursor: pointer; }
.empty { color: #bbb; text-align: center; padding: 16px 0; }
.editor { border-top: 1px dashed #ddd; padding-top: 8px; }
.editor input, .editor textarea { width: 100%; margin-bottom: 6px; padding: 6px 8px; border: 1px solid #ccc; border-radius: 6px; }
.editor textarea { min-height: 120px; font-family: monospace; font-size: 12px; }
.editor .src { font-size: 11px; color: #999; margin: 4px 0; word-break: break-all; }

/* Guide & Form */
.guide { font-size: 12px; color: #444; line-height: 1.7; padding-left: 18px; }
.note { font-size: 11px; color: #555; margin-top: 8px; background: #f6f9ff; padding: 8px; border-radius: 6px; line-height: 1.6; }
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
.modal-mask { position: fixed; inset: 0; background: rgba(0,0,0,.4); display: flex; align-items: center; justify-content: center; z-index: 999; }
.modal { background: #fff; border-radius: 10px; padding: 20px; width: 420px; }
.modal .mono { font-family: monospace; font-size: 12px; color: #666; word-break: break-all; }
.modal .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
</style>
