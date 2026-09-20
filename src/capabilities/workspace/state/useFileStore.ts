import { defineStore } from "pinia";
import { ref, reactive, computed, watch, nextTick } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useSystemStore } from "../../../stores/useSystemStore";
import { renderMd } from "../../../utils/markdown";

// ============================================================
// Phase 8C-0A — Files Domain Owner
//
// 从 useWorkspaceStore（God Store）抽出的「文件系统」子域 canonical owner。
// 范围：文件浏览器 / overlay+inline 编辑器 / 文件夹树 / 目录图片预览 /
//       行内编辑 / 文件右键菜单 / 拖拽移动。
// 不含：知识库(笔记 Vault) / 脚本库 / 片段库 / 仓库同步 / 审计（仍留 Workspace Core 或后续切片）。
// 证据：docs/architecture/capability-modularization/phase8c0/01-WORKSPACE-STORE-INVENTORY.md
//
// 设计约束（接管指令 §7/§17）：
//   - 本 store 是 Files 状态唯一真源；useWorkspaceStore 不再持有同义 Files state。
//   - 不创建第二真源；组件经本 store 的 action 写状态，禁止直写（registry 守护）。
//   - 不保存业务编排（refresh/recents 等跨域逻辑仍在 Workspace Core）。
// ============================================================

const TEXT_EXTS = [
  "txt","json","js","ts","vue","rs","html","css","xml","yaml","yml","toml",
  "csv","log","sh","py","java","go","c","cpp","h","sql","env","gitignore",
];
const IMAGE_EXTS = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "svg"];
const PREVIEW_IMAGE_CONCURRENCY = 4;
const PREVIEW_IMAGE_CACHE_LIMIT = 80;
const COMPARE_IMAGE_LIMIT = 12;

export const useFileStore = defineStore("files", () => {
  const layout = useLayoutStore();

  // ===== 文件浏览器 / overlay 编辑器 =====
  const fileEntries = ref<DirEntry[]>([]);
  const filePath = ref("");
  const pathInput = ref("");
  const fileContent = ref("");
  const editingFile = ref(false);
  const mdPreview = ref(false);
  const mdHtml = ref("");
  const startDirs = ref<DirEntry[]>([]);
  const previewDir = ref("");
  const previewEntries = ref<DirEntry[]>([]);
  const previewImages = ref<Record<string, string>>({});
  const previewImageErrors = ref<Record<string, string>>({});
  const previewLoading = ref(false);
  const previewError = ref("");
  const previewTileSize = ref(132);
  const compareImages = ref<DirEntry[]>([]);
  const inlineFile = ref("");
  const inlineText = ref("");
  const inlineIsMd = ref(false);
  const inlineEdit = ref(false);
  const inlineHtml = ref("");

  // ===== 文件夹树（左树右编辑） =====
  const treeRoots = ref<DirEntry[]>([]);
  const treeChildren = reactive<Map<string, DirEntry[]>>(new Map());
  const treeExpanded = reactive<Set<string>>(new Set());
  const treeLoading = reactive<Set<string>>(new Set());
  const treeErrors = reactive<Map<string, string>>(new Map());

  // ===== 文件树：定位 / 拖拽移动 =====
  const locateTarget = ref("");
  const dragSource = ref("");
  const dropTarget = ref("");
  const moveConfirm = reactive({ show: false, src: "", dst: "", name: "" });

  // ===== 文件右键菜单状态（集中在 store，组件只渲染） =====
  const fileCtx = reactive<{
    show: boolean;
    x: number;
    y: number;
    entry: DirEntry | null;
    isDir: boolean;
    newName: string;
    making: "" | "file" | "dir";
  }>({ show: false, x: 0, y: 0, entry: null, isDir: false, newName: "", making: "" });

  // ===== 目录图片预览：加载内部状态（实现细节，非公开治理态） =====
  let previewRequest = 0;
  let previewImageGeneration = 0;
  let previewImageActive = 0;
  const previewImageQueue: string[] = [];
  const previewImagePending = new Set<string>();
  const previewImageCache = new Map<string, string>();

  // ===== 文件右键菜单动作 =====
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
  // 新建的基准目录：优先右键目标（目录本身 / 文件的父目录），兜底当前目录
  function ctxBaseDir(): string {
    const e = fileCtx.entry;
    if (e) {
      if (e.is_dir) return e.path;
      const i = e.path.lastIndexOf("/");
      return i > 0 ? e.path.slice(0, i) : "/";
    }
    return filePath.value || "/";
  }
  async function ctxNewFile() {
    const base = ctxBaseDir();
    const name = (fileCtx.making === "file" ? fileCtx.newName : prompt("新文件名："))?.trim();
    if (!name) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.createFile(base.replace(/\/$/, "") + "/" + name, "");
      layout.showToast("已新建文件: " + name);
      treeExpanded.add(base); // 展开目标目录让新文件可见
      await refreshTree();
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  async function ctxNewDir() {
    const base = ctxBaseDir();
    const name = (fileCtx.making === "dir" ? fileCtx.newName : prompt("新目录名："))?.trim();
    if (!name) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.createDir(base.replace(/\/$/, "") + "/" + name);
      layout.showToast("已新建目录: " + name);
      treeExpanded.add(base);
      await refreshTree();
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  // 头部快捷新建：基于当前打开文件所在目录（无打开文件则用首根目录）
  function currentBaseDir(): string {
    if (filePath.value) {
      const i = filePath.value.lastIndexOf("/");
      return i > 0 ? filePath.value.slice(0, i) : "/";
    }
    return treeRoots.value[0]?.path || "/";
  }
  async function quickNew(kind: "file" | "dir") {
    const base = currentBaseDir();
    const def = kind === "file" ? "新建文件.txt" : "新建文件夹";
    const label = kind === "file" ? "新文件名" : "新目录名";
    const name = (prompt(`${label}（将创建于 ${base}）：`, def) || "").trim();
    if (!name) return;
    try {
      if (kind === "file") {
        await bridge.createFile(base.replace(/\/$/, "") + "/" + name, "");
        layout.showToast("已新建文件: " + name);
      } else {
        await bridge.createDir(base.replace(/\/$/, "") + "/" + name);
        layout.showToast("已新建目录: " + name);
      }
      treeExpanded.add(base);
      await refreshTree();
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
  }
  async function ctxDelete(entry: DirEntry) {
    if (!confirm(`删除「${entry.name}」？此操作不可恢复`)) {
      closeFileCtx();
      return;
    }
    try {
      await bridge.deletePath(entry.path);
      layout.showToast("已删除: " + entry.name);
      if (inlineFile.value === entry.path) closeInline();
      await refreshTree();
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
      await refreshTree();
    } catch (e: any) {
      layout.showToast("重命名失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  async function ctxFavorite(entry: DirEntry) {
    if (!entry.is_dir) return;
    const { useHomeStore } = await import("../../../stores/useHomeStore");
    useHomeStore().favoriteDirectory(entry.path);
    closeFileCtx();
  }
  async function copyPath(entry: DirEntry, relative: boolean) {
    let value = entry.path;
    if (relative) {
      const root = treeRoots.value.find((item) => entry.path === item.path || entry.path.startsWith(item.path.replace(/\/$/, "") + "/"));
      value = root ? entry.path.slice(root.path.length).replace(/^\//, "") || "." : entry.name;
    }
    await bridge.clipboardWrite(value);
    layout.showToast(relative ? "已复制相对路径" : "已复制绝对路径");
    closeFileCtx();
  }
  // 文件树右键：新标签打开（目录→目录页签；文件→行内编辑/预览）
  function ctxOpenInNewTab(entry: DirEntry) {
    closeFileCtx();
    if (entry.is_dir) {
      layout.openDirTab(entry.path);
    } else {
      openFileInline(entry);
    }
  }
  // 文件树右键：系统文件管理器定位（资源管理器打开）
  async function ctxOpenInExplorer(entry: DirEntry) {
    closeFileCtx();
    try {
      await bridge.revealPath(entry.path);
    } catch (e: any) {
      layout.showToast("打开失败: " + (e?.message ?? e));
    }
  }
  // 文件树右键：终端打开并 cd 到该路径
  async function ctxOpenInTerminal(entry: DirEntry) {
    closeFileCtx();
    const dir = entry.is_dir
      ? entry.path
      : entry.path.lastIndexOf("/") > 0
        ? entry.path.slice(0, entry.path.lastIndexOf("/"))
        : "/";
    await useSystemStore().openTerminalAt(dir);
  }

  // ===== IDE 文件树（左树右编辑） =====
  async function ensureTreeChildren(path: string) {
    if (treeChildren.has(path) || treeLoading.has(path)) return;
    treeLoading.add(path);
    treeErrors.delete(path);
    try {
      treeChildren.set(path, await bridge.listDir(path));
    } catch (e: any) {
      treeChildren.delete(path);
      treeErrors.set(path, e?.message ?? String(e));
    } finally {
      treeLoading.delete(path);
    }
  }
  // 树根：已进入某目录则以它为根（自动展开），否则展示起始目录集合
  async function loadTree() {
    if (filePath.value) {
      const name = filePath.value.split("/").filter(Boolean).pop() || "/";
      treeRoots.value = [{ name, path: filePath.value, is_dir: true, size: 0 }];
      treeExpanded.add(filePath.value);
      await ensureTreeChildren(filePath.value);
    } else {
      treeRoots.value = await bridge.getStartDirs();
    }
  }
  async function toggleTreeDir(path: string) {
    if (treeExpanded.has(path)) treeExpanded.delete(path);
    else {
      treeExpanded.add(path);
      await ensureTreeChildren(path);
    }
  }
  // 刷新：保留展开状态，重新拉取所有已展开目录
  async function refreshTree() {
    treeChildren.clear();
    treeErrors.clear();
    if (filePath.value) {
      const name = filePath.value.split("/").filter(Boolean).pop() || "/";
      treeRoots.value = [{ name, path: filePath.value, is_dir: true, size: 0 }];
    } else {
      treeRoots.value = await bridge.getStartDirs();
    }
    for (const p of [...treeExpanded]) await ensureTreeChildren(p);
  }
  function collapseAllTree() {
    treeExpanded.clear();
    locateTarget.value = "";
    layout.showToast("已全部折叠");
  }
  async function expandTreeEntry(entry: DirEntry, depth = 0) {
    if (!entry.is_dir || depth > 4) return;
    treeExpanded.add(entry.path);
    await ensureTreeChildren(entry.path);
    const children = treeChildren.get(entry.path) || [];
    for (const child of children) await expandTreeEntry(child, depth + 1);
  }
  async function expandAllTree() {
    if (!treeRoots.value.length) await loadTree();
    for (const root of treeRoots.value) await expandTreeEntry(root);
    layout.showToast("已全部展开");
  }

  // ===== 文件树：定位（IDEA 式“在项目中定位到当前打开位置”）=====
  // 当前“打开的本地位置”：优先激活的目录/文件页签，其次行内打开的文件，再次当前浏览目录
  const currentLocalPath = computed(() => {
    if (inlineFile.value) return inlineFile.value;
    if (previewDir.value) return previewDir.value;
    const mod = layout.modTabs.find((t) => t.id === layout.activeModTab);
    if (mod?.path) return mod.path;
    if (filePath.value) return filePath.value;
    return "";
  });

  // 展开 root → targetPath 的祖先链并高亮该节点
  async function locateTo(targetPath: string) {
    if (!targetPath) return;
    if (!startDirs.value.length) {
      try {
        await loadStartDirs();
      } catch {
        return;
      }
    }
    const dirs = (startDirs.value || []).map((d) => d.path).filter(Boolean);
    let root = "";
    for (const d of dirs) {
      if (targetPath.startsWith(d) && d.length > root.length) root = d;
    }
    if (!root) {
      // 不在任何起始目录内：仅高亮（若树中已存在）
      locateTarget.value = targetPath;
      return;
    }
    // 若当前是单根(filePath)模式且根不是目标所在 root，切回 startDirs 多根
    if (
      treeRoots.value.length === 1 &&
      filePath.value &&
      treeRoots.value[0].path === filePath.value &&
      filePath.value !== root
    ) {
      treeRoots.value = startDirs.value;
    }
    const segs = targetPath.split("/").filter(Boolean);
    let cur = "";
    for (const s of segs) {
      cur += "/" + s;
      treeExpanded.add(cur);
      await ensureTreeChildren(cur);
    }
    locateTarget.value = targetPath;
    await nextTick();
    const selectorPath = typeof CSS !== "undefined" && CSS.escape ? CSS.escape(targetPath) : targetPath.replace(/"/g, '\\"');
    document.querySelector(`[data-path="${selectorPath}"]`)?.scrollIntoView({ block: "center" });
  }

  // 准星按钮：定位到“当前打开位置”
  function locateCurrent() {
    const p = currentLocalPath.value;
    if (p) locateTo(p);
  }

  // ===== 文件树：拖拽移动 =====
  function startDrag(path: string, ev: DragEvent) {
    dragSource.value = path;
    if (ev.dataTransfer) {
      ev.dataTransfer.setData("text/plain", path);
      ev.dataTransfer.effectAllowed = "move";
    }
  }
  function onDirDragOver(path: string, ev: DragEvent) {
    if (!dragSource.value) return;
    dropTarget.value = path;
    if (ev.dataTransfer) ev.dataTransfer.dropEffect = "move";
  }
  function onDirDragLeave(path: string) {
    if (dropTarget.value === path) dropTarget.value = "";
  }
  function onDirDrop(path: string, ev: DragEvent) {
    const src =
      dragSource.value ||
      (ev.dataTransfer ? ev.dataTransfer.getData("text/plain") : "");
    dropTarget.value = "";
    dragSource.value = "";
    if (!src || !path) return;
    if (src === path) return;
    if (path.startsWith(src + "/")) {
      layout.showToast("不能移动到自身子目录内");
      return;
    }
    requestMove(src, path);
  }

  function requestMove(src: string, dstDir: string) {
    const name = src.split("/").filter(Boolean).pop() || src;
    moveConfirm.src = src;
    moveConfirm.dst = dstDir;
    moveConfirm.name = name;
    moveConfirm.show = true;
  }
  function cancelMove() {
    moveConfirm.show = false;
    moveConfirm.src = "";
    moveConfirm.dst = "";
  }
  async function confirmMove() {
    const src = moveConfirm.src;
    const dst = moveConfirm.dst;
    moveConfirm.show = false;
    try {
      await bridge.movePath(src, dst);
      layout.showToast("已移动: " + (dst.split("/").filter(Boolean).pop() || dst));
      await refreshTree();
    } catch (e: any) {
      layout.showToast("移动失败: " + (e?.message ?? e));
    }
  }

  // 打开软件 / 切换本地位置后，侧边栏树自动定位到当前打开位置
  watch(
    currentLocalPath,
    (p) => {
      if (p) locateTo(p);
    },
    { immediate: true }
  );

  // ===== 行内文件预览/编辑（IDE 右栏；与 overlay 编辑器的 filePath/fileContent 完全隔离） =====
  async function openFileInline(entry: DirEntry) {
    if (entry.is_dir) {
      toggleTreeDir(entry.path);
      openDirPreview(entry);
      return;
    }
    const ext = entry.name.split(".").pop()?.toLowerCase() || "";
    const isMd = ext === "md" || ext === "markdown";
    if (!isMd && !TEXT_EXTS.includes(ext)) {
      layout.showToast(`非文本文件 (${entry.name})，暂不支持预览`);
      return;
    }
    try {
      const text = await bridge.readFile(entry.path);
      inlineFile.value = entry.path;
      filePath.value = entry.path.lastIndexOf("/") > 0 ? entry.path.slice(0, entry.path.lastIndexOf("/")) : "/";
      pathInput.value = filePath.value;
      inlineText.value = text;
      inlineIsMd.value = isMd;
      inlineEdit.value = !isMd; // md 默认预览，其它文本直接编辑
      inlineHtml.value = isMd ? renderMd(text) : "";
      const { useWorkspaceStore } = await import("./useWorkspaceStore");
      useWorkspaceStore().addRecentFile(entry.path, entry.name);
    } catch (e: any) {
      layout.showToast("读取失败: " + (e?.message ?? e));
    }
  }
  function inlineToggleEdit() {
    inlineEdit.value = !inlineEdit.value;
    if (!inlineEdit.value && inlineIsMd.value) inlineHtml.value = renderMd(inlineText.value);
  }
  async function saveInline() {
    if (!inlineFile.value) return;
    await bridge.writeFile(inlineFile.value, inlineText.value);
    if (inlineIsMd.value && !inlineEdit.value) inlineHtml.value = renderMd(inlineText.value);
    layout.showToast("已保存: " + inlineFile.value.split("/").pop());
  }
  function closeInline() {
    inlineFile.value = "";
  }

  function isImageEntry(entry: DirEntry) {
    return !entry.is_dir && IMAGE_EXTS.includes(entry.name.split(".").pop()?.toLowerCase() || "");
  }
  function rememberPreviewImage(path: string, src: string) {
    previewImageCache.delete(path);
    previewImageCache.set(path, src);
    while (previewImageCache.size > PREVIEW_IMAGE_CACHE_LIMIT) {
      const oldest = previewImageCache.keys().next().value;
      if (!oldest) break;
      previewImageCache.delete(oldest);
    }
  }
  function putPreviewImage(path: string, src: string) {
    rememberPreviewImage(path, src);
    previewImages.value = { ...previewImages.value, [path]: src };
  }
  function putPreviewImageError(path: string, message: string) {
    previewImageErrors.value = { ...previewImageErrors.value, [path]: message };
  }
  function clearPreviewImageQueue() {
    previewImageGeneration += 1;
    previewImageQueue.splice(0);
    previewImagePending.clear();
    previewImageActive = 0;
  }
  function drainPreviewImageQueue(generation = previewImageGeneration) {
    if (generation !== previewImageGeneration) return;
    while (previewImageActive < PREVIEW_IMAGE_CONCURRENCY && previewImageQueue.length) {
      const path = previewImageQueue.shift()!;
      previewImagePending.delete(path);
      if (previewImages.value[path] || previewImageErrors.value[path]) continue;
      previewImageActive += 1;
      bridge.readImageDataUrl(path)
        .then((src) => {
          if (generation === previewImageGeneration) putPreviewImage(path, src);
        })
        .catch((e: any) => {
          if (generation === previewImageGeneration) putPreviewImageError(path, e?.message ?? String(e));
        })
        .finally(() => {
          if (generation !== previewImageGeneration) return;
          previewImageActive = Math.max(0, previewImageActive - 1);
          drainPreviewImageQueue(generation);
        });
    }
  }
  function loadPreviewImage(entry: DirEntry) {
    if (!isImageEntry(entry)) return;
    if (previewImages.value[entry.path] || previewImageErrors.value[entry.path] || previewImagePending.has(entry.path)) return;
    const cached = previewImageCache.get(entry.path);
    if (cached) {
      previewImages.value = { ...previewImages.value, [entry.path]: cached };
      return;
    }
    previewImagePending.add(entry.path);
    previewImageQueue.push(entry.path);
    drainPreviewImageQueue();
  }
  async function openDirPreview(entry: DirEntry) {
    if (!entry.is_dir) return;
    const request = ++previewRequest;
    clearPreviewImageQueue();
    previewDir.value = entry.path;
    previewEntries.value = [];
    previewImages.value = {};
    previewImageErrors.value = {};
    compareImages.value = [];
    previewError.value = "";
    previewLoading.value = true;
    inlineFile.value = "";
    try {
      const entries = await bridge.listDir(entry.path);
      if (request !== previewRequest) return;
      previewEntries.value = entries;
    } catch (e: any) {
      if (request === previewRequest) previewError.value = e?.message ?? String(e);
    } finally {
      if (request === previewRequest) previewLoading.value = false;
    }
  }
  function setPreviewTileSize(size: number) {
    previewTileSize.value = Math.min(260, Math.max(72, Math.round(size)));
  }
  function toggleCompareImage(entry: DirEntry) {
    const i = compareImages.value.findIndex((img) => img.path === entry.path);
    if (i >= 0) {
      compareImages.value.splice(i, 1);
      return;
    }
    compareImages.value.push(entry);
    if (compareImages.value.length > COMPARE_IMAGE_LIMIT) compareImages.value.shift();
    loadPreviewImage(entry);
  }
  function moveCompareImage(path: string, delta: number) {
    const i = compareImages.value.findIndex((img) => img.path === path);
    const j = i + delta;
    if (i < 0 || j < 0 || j >= compareImages.value.length) return;
    const [item] = compareImages.value.splice(i, 1);
    compareImages.value.splice(j, 0, item);
  }
  function clearCompareImages() {
    compareImages.value = [];
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
    // 地址栏打开目录时同步树：以该目录为根
    await loadTree();
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
    const { useWorkspaceStore } = await import("./useWorkspaceStore");
    useWorkspaceStore().addRecentFile(entry.path, entry.name);
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
    layout.setView("editor");
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
    layout.setView("editor");
    return true;
  }
  async function saveFile() {
    if (!filePath.value) return;
    await bridge.writeFile(filePath.value, fileContent.value);
    layout.showToast("已保存: " + filePath.value.split("/").pop());
  }

  // ===== Files canonical writers（组件经此写状态，禁止直写，FILE-03 / WS-OWNER-02）=====
  function setFileContent(v: string) { fileContent.value = v; }
  function setInlineText(v: string) { inlineText.value = v; }
  function setEditorView(isPreview: boolean) { mdPreview.value = isPreview; editingFile.value = !isPreview; }
  function clearDragSource() { dragSource.value = ""; }

  return {
    fileEntries,
    filePath,
    pathInput,
    fileContent,
    editingFile,
    mdPreview,
    mdHtml,
    startDirs,
    previewDir,
    previewEntries,
    previewImages,
    previewImageErrors,
    previewLoading,
    previewError,
    previewTileSize,
    compareImages,
    inlineFile,
    inlineText,
    inlineIsMd,
    inlineEdit,
    inlineHtml,
    treeRoots,
    treeChildren,
    treeExpanded,
    treeLoading,
    treeErrors,
    locateTarget,
    dragSource,
    dropTarget,
    moveConfirm,
    fileCtx,
    currentLocalPath,
    onFileContext,
    closeFileCtx,
    ctxNewFile,
    ctxNewDir,
    quickNew,
    ctxDelete,
    ctxRename,
    ctxFavorite,
    copyPath,
    ctxOpenInNewTab,
    ctxOpenInExplorer,
    ctxOpenInTerminal,
    ensureTreeChildren,
    loadTree,
    toggleTreeDir,
    refreshTree,
    expandAllTree,
    collapseAllTree,
    locateTo,
    locateCurrent,
    startDrag,
    onDirDragOver,
    onDirDragLeave,
    onDirDrop,
    requestMove,
    confirmMove,
    cancelMove,
    openFileInline,
    inlineToggleEdit,
    saveInline,
    closeInline,
    isImageEntry,
    loadPreviewImage,
    setPreviewTileSize,
    toggleCompareImage,
    moveCompareImage,
    clearCompareImages,
    openDirPreview,
    loadStartDirs,
    enterDir,
    goUp,
    goPath,
    openFile,
    openMd,
    saveFile,
    setFileContent,
    setInlineText,
    setEditorView,
    clearDragSource,
  };
});
