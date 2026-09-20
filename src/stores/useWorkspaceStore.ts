import { defineStore } from "pinia";
import { ref, reactive } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { useBrowserStore } from "./useBrowserStore";
import { useFileStore } from "./useFileStore";
import { useArtifactStore } from "./useArtifactStore";
import {
  emptyScriptForm,
  loadFormFromMeta,
  validateScriptForm,
  serializeScriptForm,
  canDeleteScript,
  type ScriptForm,
} from "../utils/scriptUi";
import {
  emptySnippetForm,
  loadSnippetForm,
  validateSnippetForm,
  serializeSnippetForm,
  canDeleteSnippet,
  type SnippetForm,
} from "../utils/snippetUi";
import type { CommandSnippet, ScriptMeta } from "../types";

export interface RecentItem {
  type: "url" | "file";
  title: string;
  path: string;
  at: number;
}

// ============================================================
// Phase 8C-0 — Workspace Core（God Store 逐步收敛）
//
//   8C-0A：Files 子域      → useFileStore
//   8C-0B：Artifact 子域   → useArtifactStore
//
// 仍持有：REPO（仓库同步）/ SCRIPT / SNIPPET / AUDIT / recents（跨域）。
// 设计约束（接管指令 §5）：只保留 Workspace 自身语义 + 跨域编排（refresh / openRecent），
// 禁止回存 Files / Artifact / Repo / Script 子域内部状态（WS_OWNER_* 门禁守护）。
// ============================================================

export const useWorkspaceStore = defineStore("workspace", () => {
  const layout = useLayoutStore();

  // ===== 仓库同步（REPO 域）=====
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
  const busy = ref(false);
  const job = ref<SyncJob | null>(null);

  // ===== 审计（AUDIT 域）=====
  const audit = ref<AuditEntry[]>([]);

  // ===== 脚本库 CRUD（M2-5.a，纯前端，复用 M2-3 四条命令） =====
  const scripts = ref<ScriptMeta[]>([]);
  const scriptForm = reactive<ScriptForm>(emptyScriptForm());
  const snippets = ref<CommandSnippet[]>([]);
  const snippetForm = reactive<SnippetForm>(emptySnippetForm());

  async function loadScripts() {
    try {
      scripts.value = await bridge.scriptList();
    } catch (e: any) {
      layout.showToast("加载脚本失败：" + (e?.message ?? e));
    }
  }
  function openScriptForm(m?: ScriptMeta | null) {
    Object.assign(scriptForm, m ? loadFormFromMeta(m) : emptyScriptForm());
  }
  async function saveScript() {
    const issues = validateScriptForm(scriptForm);
    if (issues.length) {
      layout.showToast("表单校验未通过：" + issues[0].message);
      return;
    }
    const ser = serializeScriptForm(scriptForm);
    try {
      if (scriptForm.id) {
        await bridge.scriptUpdate({
          id: scriptForm.id,
          name: ser.name,
          category: ser.category,
          interpreter: ser.interpreter,
          body: ser.body,
          params: ser.params,
          description: ser.description,
          timeoutSecs: ser.timeoutSecs,
          enabled: ser.enabled,
        });
        layout.showToast("已保存：" + ser.name);
      } else {
        await bridge.scriptAdd({
          name: ser.name,
          category: ser.category,
          interpreter: ser.interpreter,
          body: ser.body,
          params: ser.params,
          description: ser.description,
          timeoutSecs: ser.timeoutSecs,
        });
        layout.showToast("已新建：" + ser.name);
      }
      await loadScripts();
    } catch (e: any) {
      layout.showToast("保存失败：" + (e?.message ?? e));
    }
  }
  async function removeScript(m: ScriptMeta) {
    if (!canDeleteScript(m)) {
      layout.showToast("内置脚本不可删除");
      return;
    }
    try {
      await bridge.scriptRemove(m.id);
      layout.showToast("已删除：" + m.name);
      await loadScripts();
    } catch (e: any) {
      layout.showToast("删除失败：" + (e?.message ?? e));
    }
  }

  // ===== 命令片段库 CRUD（M2-6.d，执行通过 run_command 复用脚本运行态） =====
  async function loadSnippets() {
    try {
      snippets.value = await bridge.snippetList();
    } catch (e: any) {
      layout.showToast("加载命令片段失败：" + (e?.message ?? e));
    }
  }
  function openSnippetForm(m?: CommandSnippet | null) {
    Object.assign(snippetForm, m ? loadSnippetForm(m) : emptySnippetForm());
  }
  async function saveSnippet() {
    const issues = validateSnippetForm(snippetForm);
    if (issues.length) {
      layout.showToast("表单校验未通过：" + issues[0].message);
      return;
    }
    const ser = serializeSnippetForm(snippetForm);
    try {
      if (snippetForm.id) {
        await bridge.snippetUpdate({
          id: snippetForm.id,
          name: ser.name,
          category: ser.category,
          interpreter: ser.interpreter,
          argv: ser.argv,
          params: ser.params,
          description: ser.description,
          dangerous: ser.dangerous,
          enabled: ser.enabled,
          timeoutSecs: ser.timeoutSecs,
        });
        layout.showToast("已保存：" + ser.name);
      } else {
        await bridge.snippetAdd({
          name: ser.name,
          category: ser.category,
          interpreter: ser.interpreter,
          argv: ser.argv,
          params: ser.params,
          description: ser.description,
          dangerous: ser.dangerous,
          // M2-6-fix1（复核 F-1）：新建态也要传 enabled，否则表单勾选被丢弃
          enabled: ser.enabled,
          timeoutSecs: ser.timeoutSecs,
        });
        layout.showToast("已新建：" + ser.name);
      }
      await loadSnippets();
    } catch (e: any) {
      layout.showToast("保存失败：" + (e?.message ?? e));
    }
  }
  async function removeSnippet(m: CommandSnippet) {
    if (!canDeleteSnippet(m)) {
      layout.showToast("内置命令不可删除");
      return;
    }
    try {
      await bridge.snippetRemove(m.id);
      layout.showToast("已删除：" + m.name);
      await loadSnippets();
    } catch (e: any) {
      layout.showToast("删除失败：" + (e?.message ?? e));
    }
  }

  const recents = reactive<RecentItem[]>([]);

  // ===== 跨域编排（Workspace Core 职责：聚合各子域 load，不持有子域内部状态）=====
  async function refresh() {
    const [, r, a] = await Promise.all([
      useArtifactStore().loadTree(),
      bridge.listRepos(),
      bridge.auditLog(),
    ]);
    repos.value = r;
    audit.value = a;
  }

  // ===== 同步 =====
  async function saveRepo() {
    if (!form.name || !form.remote_url || !form.token) {
      layout.showToast("请填写 名称 / 远程地址 / Token");
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
    layout.showToast("仓库已保存（token 仅存密钥库）");
  }
  async function requestSync(repoId: string) {
    // 跨域读：同步消费 Artifact 多选（selection 真源归 useArtifactStore）
    const ids = [...useArtifactStore().selected];
    if (!ids.length) {
      layout.showToast("请先勾选要同步的成果");
      return;
    }
    preview.value = await bridge.requestSync({ artifactIds: ids, repoId });
    layout.showToast("已生成待确认同步任务，请确认后推送");
  }
  async function confirmSync() {
    if (!preview.value || busy.value) return;
    busy.value = true;
    job.value = await bridge.confirmSync({ jobId: preview.value.job_id });
    layout.showToast("推送中…");
  }
  function onSyncCompleted(j: SyncJob) {
    busy.value = false;
    job.value = j;
    preview.value = null;
    refresh();
    layout.showToast(j.status === "success" ? "✅ 推送成功" : `❌ 推送失败: ${j.error ?? ""}`);
  }
  function loadGiteeExample() {
    form.provider = "gitee";
    form.name = "my-gitee-repo";
    form.remote_url = "https://gitee.com/你的用户名/你的仓库.git";
    form.branch = "master";
    form.username = "你的用户名";
    form.token = "";
    layout.showToast("已填入码云示例，补全用户名与 Token 后点保存");
  }

  // ===== 最近访问（跨域；文件侧由 useFileStore 调 addRecentFile，url 侧见下） =====
  const RECENTS_KEY = "browser-os-recents";
  function loadRecents() {
    try {
      const raw = localStorage.getItem(RECENTS_KEY);
      if (raw) recents.splice(0, recents.length, ...JSON.parse(raw));
    } catch {}
  }
  function saveRecents() {
    try {
      localStorage.setItem(RECENTS_KEY, JSON.stringify(recents.slice(0, 30)));
    } catch {}
  }
  function addRecentUrl(urlStr: string) {
    const idx = recents.findIndex((r) => r.type === "url" && r.path === urlStr);
    if (idx >= 0) recents.splice(idx, 1);
    recents.unshift({ type: "url", title: urlStr, path: urlStr, at: Date.now() });
    saveRecents();
  }
  function addRecentFile(path: string, name: string) {
    const idx = recents.findIndex((r) => r.type === "file" && r.path === path);
    if (idx >= 0) recents.splice(idx, 1);
    recents.unshift({ type: "file", title: name, path, at: Date.now() });
    saveRecents();
  }
  function openRecent(r: RecentItem) {
    if (r.type === "url") {
      const browser = useBrowserStore();
      browser.url = r.path;
      browser.openBrowser();
    } else {
      // 文件侧打开逻辑已归 Files owner（useFileStore）
      useFileStore().openFile({ name: r.title, path: r.path, is_dir: false, size: 0 });
    }
  }

  return {
    repos,
    form,
    preview,
    busy,
    job,
    audit,
    scripts,
    scriptForm,
    snippets,
    snippetForm,
    loadScripts,
    openScriptForm,
    saveScript,
    removeScript,
    loadSnippets,
    openSnippetForm,
    saveSnippet,
    removeSnippet,
    recents,
    refresh,
    saveRepo,
    requestSync,
    confirmSync,
    onSyncCompleted,
    loadGiteeExample,
    loadRecents,
    addRecentUrl,
    addRecentFile,
    openRecent,
  };
});
