import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { useBrowserStore } from "./useBrowserStore";
import { useFileStore } from "./useFileStore";
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
// Phase 8C-0A — Workspace Core（God Store 收敛）
//
// 本 store 不再拥有 Files 子域状态（已迁至 useFileStore，见 docs/.../phase8c0/01-...）。
// 仍持有：知识库(Vault/Artifact)、脚本库、片段库、仓库同步、审计、最近访问(跨域)。
// 设计约束（接管指令 §15）：只保留 Workspace 自身语义 + 跨域编排，禁止回存文件树/编辑器/预览内部状态。
// ============================================================

export const useWorkspaceStore = defineStore("workspace", () => {
  const layout = useLayoutStore();

  const tree = ref<WorkspaceTree>({ nodes: [] });
  const current = ref<Artifact | null>(null);
  const editTitle = ref("");
  const editTags = ref("");
  const editText = ref("");
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
  const busy = ref(false);
  const job = ref<SyncJob | null>(null);

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

  // 成果右键菜单状态
  const ctxMenu = reactive<{
    show: boolean;
    x: number;
    y: number;
    item: Artifact | null;
  }>({ show: false, x: 0, y: 0, item: null });

  // 采集当前网页选中内容（对应 prototype 的「＋ 采集选中内容」）
  async function collectSelection() {
    try {
      const sel = (window.getSelection?.()?.toString?.() || "").trim();
      const url = useBrowserStore().url || "about:blank";
      const title = useBrowserStore().activeTab?.title || url;
      const text = sel || "";
      const art = await bridge.collectSelection({ url, title, html: "", text });
      await refresh();
      layout.showToast("✅ 已保存: " + (art.title || text.slice(0, 18) || "选中内容"));
    } catch (e: any) {
      layout.showToast("采集失败: " + (e?.message ?? e));
    }
  }

  const flatArtifacts = computed(() => tree.value.nodes.flatMap((n) => n.items));

  async function refresh() {
    const [t, r, a] = await Promise.all([
      bridge.browseWorkspace(),
      bridge.listRepos(),
      bridge.auditLog(),
    ]);
    tree.value = t;
    repos.value = r;
    audit.value = a;
  }

  // ===== 成果 =====
  async function openArtifact(item: DomainItem) {
    const art = await bridge.readArtifact(item.id);
    current.value = art;
    editTitle.value = art.title;
    editTags.value = art.tags.join(", ");
    editText.value = art.text;
  }
  async function saveEdit() {
    if (!current.value) return;
    const tags = editTags.value
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    await bridge.updateArtifact({
      id: current.value.id,
      title: editTitle.value,
      text: editText.value,
      tags,
    });
    await refresh();
    layout.showToast("已保存编辑");
  }
  async function removeArtifact(item: DomainItem) {
    if (!confirm(`删除「${item.title}」？`)) return;
    await bridge.deleteArtifact(item.id);
    if (current.value?.id === item.id) current.value = null;
    await refresh();
  }
  function onArtifactContext(e: MouseEvent, art: Artifact) {
    e.preventDefault();
    ctxMenu.show = true;
    ctxMenu.x = e.clientX;
    ctxMenu.y = e.clientY;
    ctxMenu.item = art;
  }
  function closeCtx() {
    ctxMenu.show = false;
    ctxMenu.item = null;
  }
  async function ctxReveal() {
    if (!ctxMenu.item) return;
    try {
      await bridge.revealArtifact(ctxMenu.item.id);
      layout.showToast("已在文件管理器中定位");
    } catch (e: any) {
      layout.showToast("打开目录失败: " + (e?.message ?? e));
    }
    closeCtx();
  }
  async function ctxOpenSource() {
    if (!ctxMenu.item) return;
    try {
      await bridge.openSource(ctxMenu.item.source_url);
    } catch (e: any) {
      layout.showToast("打开链接失败: " + (e?.message ?? e));
    }
    closeCtx();
  }
  async function ctxRemove() {
    if (!ctxMenu.item) return;
    await removeArtifact({ id: ctxMenu.item.id, title: ctxMenu.item.title } as DomainItem);
    closeCtx();
  }
  function toggle(id: string) {
    if (selected.has(id)) selected.delete(id);
    else selected.add(id);
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
    const ids = [...selected];
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
    tree,
    current,
    editTitle,
    editTags,
    editText,
    selected,
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
    ctxMenu,
    flatArtifacts,
    refresh,
    openArtifact,
    saveEdit,
    removeArtifact,
    toggle,
    onArtifactContext,
    closeCtx,
    ctxReveal,
    ctxOpenSource,
    ctxRemove,
    saveRepo,
    requestSync,
    confirmSync,
    onSyncCompleted,
    loadGiteeExample,
    loadRecents,
    addRecentUrl,
    addRecentFile,
    openRecent,
    collectSelection,
  };
});
