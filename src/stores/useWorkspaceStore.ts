import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { renderMd } from "../utils/markdown";
import { withToast } from "../utils/error";

const TEXT_EXTS = [
  "txt","json","js","ts","vue","rs","html","css","xml","yaml","yml","toml",
  "csv","log","sh","py","java","go","c","cpp","h","sql","env","gitignore",
];

export interface RecentItem {
  type: "url" | "file";
  title: string;
  path: string;
  at: number;
}

export const useWorkspaceStore = defineStore("workspace", () => {
  const layout = useLayoutStore();

  const tree = ref<WorkspaceTree>({ nodes: [] });
  const current = ref<Artifact | null>(null);
  const editTitle = ref("");
  const editTags = ref("");
  const editText = ref("");
  const selected = reactive<Set<string>>(new Set());

  const fileEntries = ref<DirEntry[]>([]);
  const filePath = ref("");
  const pathInput = ref("");
  const fileContent = ref("");
  const editingFile = ref(false);
  const mdPreview = ref(false);
  const mdHtml = ref("");
  const startDirs = ref<DirEntry[]>([]);

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
  const recents = reactive<RecentItem[]>([]);

  // 文件右键菜单状态（集中在 store，组件只渲染）
  const fileCtx = reactive<{
    show: boolean;
    x: number;
    y: number;
    entry: DirEntry | null;
    isDir: boolean;
    newName: string;
    making: "" | "file" | "dir";
  }>({ show: false, x: 0, y: 0, entry: null, isDir: false, newName: "", making: "" });

  // 成果右键菜单状态
  const ctxMenu = reactive<{
    show: boolean;
    x: number;
    y: number;
    item: Artifact | null;
  }>({ show: false, x: 0, y: 0, item: null });

  function onFileContext(e: MouseEvent, entry?: DirEntry) {
    e.preventDefault();
    e.stopPropagation();
    fileCtx.show = true;
    fileCtx.x = e.clientX;
    fileCtx.y = e.clientY;
    fileCtx.entry = entry ?? null;
    fileCtx.isDir = entry?.is_dir ?? false;
    fileCtx.newName = "";
    fileCtx.making = "";
  }
  function closeFileCtx() {
    fileCtx.show = false;
    fileCtx.entry = null;
    fileCtx.making = "";
  }
  async function ctxNewFile() {
    const base = filePath.value || "/";
    const name = (fileCtx.making === "file" ? fileCtx.newName : prompt("新文件名："))?.trim();
    if (!name) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.createFile(base.replace(/\/$/, "") + "/" + name, "");
      layout.showToast("已新建文件: " + name);
      await enterDir(filePath.value);
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  async function ctxNewDir() {
    const base = filePath.value || "/";
    const name = (fileCtx.making === "dir" ? fileCtx.newName : prompt("新目录名："))?.trim();
    if (!name) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.createDir(base.replace(/\/$/, "") + "/" + name);
      layout.showToast("已新建目录: " + name);
      await enterDir(filePath.value);
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  async function ctxDelete(entry: DirEntry) {
    if (!confirm(`删除「${entry.name}」？此操作不可恢复`)) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.deletePath(entry.path);
      layout.showToast("已删除: " + entry.name);
      await enterDir(filePath.value);
    } catch (e: any) {
      layout.showToast("删除失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  async function ctxRename(entry: DirEntry) {
    const name = prompt("重命名为：", entry.name)?.trim();
    if (!name || name === entry.name) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.renamePath(entry.path, name);
      layout.showToast("已重命名 → " + name);
      await enterDir(filePath.value);
    } catch (e: any) {
      layout.showToast("重命名失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }

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

  // ===== 文件浏览器 =====
  async function loadStartDirs() {
    startDirs.value = await bridge.getStartDirs();
  }
  async function enterDir(path: string) {
    filePath.value = path;
    pathInput.value = path;
    fileContent.value = "";
    try {
      fileEntries.value = await bridge.listDir(path);
    } catch (e: any) {
      layout.showToast("无法读取目录: " + (e ?? e));
      fileEntries.value = [];
    }
  }
  function goUp() {
    const p = filePath.value;
    if (!p) return;
    const parent = p.split("/").slice(0, -1).join("/") || "/";
    enterDir(parent);
  }
  function goPath() {
    const p = pathInput.value.trim();
    if (!p) return;
    enterDir(p);
  }
  async function openFile(entry: DirEntry) {
    if (entry.is_dir) {
      enterDir(entry.path);
      return;
    }
    addRecentFile(entry.path, entry.name);
    const ext = entry.name.split(".").pop()?.toLowerCase() || "";
    if (ext === "md" || ext === "markdown") {
      await openMd(entry);
      return;
    }
    if (!TEXT_EXTS.includes(ext)) {
      layout.showToast(`非文本文件 (${entry.name})，暂不支持预览`);
      return;
    }
    fileContent.value = await bridge.readFile(entry.path);
    filePath.value = entry.path;
    editingFile.value = true;
    mdPreview.value = false;
    layout.fileEditorOpen = true;
    layout.mainView = "editor";
    layout.showToast(`已加载: ${entry.name} (${entry.size.toLocaleString()} bytes)`);
  }
  async function openMd(entry: DirEntry) {
    const ext = entry.name.split(".").pop()?.toLowerCase();
    if (ext !== "md" && ext !== "markdown") return false;
    fileContent.value = await bridge.readFile(entry.path);
    mdHtml.value = renderMd(fileContent.value);
    filePath.value = entry.path;
    mdPreview.value = true;
    editingFile.value = false;
    layout.fileEditorOpen = true;
    layout.mainView = "editor";
    return true;
  }
  async function saveFile() {
    if (!filePath.value) return;
    await bridge.writeFile(filePath.value, fileContent.value);
    layout.showToast("已保存: " + filePath.value.split("/").pop());
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

  // ===== 最近访问 =====
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
      openFile({ name: r.title, path: r.path, is_dir: false, size: 0 });
    }
  }

  // 兼容旧名称（App 旧版调用）
  const enterDirCompat = enterDir;

  return {
    tree,
    current,
    editTitle,
    editTags,
    editText,
    selected,
    fileEntries,
    filePath,
    pathInput,
    fileContent,
    editingFile,
    mdPreview,
    mdHtml,
    startDirs,
    repos,
    form,
    preview,
    busy,
    job,
    audit,
    recents,
    fileCtx,
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
    loadStartDirs,
    enterDir,
    onFileContext,
    closeFileCtx,
    ctxNewFile,
    ctxNewDir,
    ctxDelete,
    ctxRename,
    goUp,
    goPath,
    openFile,
    openMd,
    saveFile,
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
