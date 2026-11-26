import { defineStore } from "pinia";
import { ref, reactive, computed, watch, nextTick } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore, type ViewChangeCommit } from "../../../stores/useLayoutStore";
import { openTerminalAt } from "../../../composables/terminalNav";
import { renderMd } from "../../../utils/markdown";
import { documentOwnership } from "../../../application/documentOwnership.mjs";
import { documentPathKey, documentPathName, documentPathParent, joinDocumentPath, sameDocumentPath, containsDocumentPath, relativeDocumentPath, replaceDocumentPathPrefix } from "../../../application/documentPaths.mjs";
import { persistDraftAfterSave } from "../model/draftRecovery.mjs";

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
  // Directory navigation must not own editor identity. This is a separate document snapshot.
  const editorFilePath = ref("");
  const editorSavedContent = ref("");
  type RecoveryEditSession = { id: string; revision: string };
  const editorRecoverySession = ref<RecoveryEditSession | null>(null);
  const editorRecoveryId = computed(() => editorRecoverySession.value?.id ?? "");
  const editorDirty = computed(() => !!editorFilePath.value &&
    (!!editorRecoverySession.value || fileContent.value !== editorSavedContent.value));
  let editorReadRequest = 0;
  let editorDocumentRevision = 0;
  function canDiscardEditor(): boolean {
    return !editorDirty.value || confirm("主编辑器有未保存的修改，确定放弃修改吗？");
  }
  layout.registerBeforeViewChange("workspace-editor-drafts", (next, previous) => {
    if (previous === "editor" && next !== "editor" && !canDiscardEditor()) return false;
    // Switching between filesystem modules can abandon an inline draft too.
    if (previous === "files" && (next !== "files" || inlineFile.value) && !canDiscardInline()) return false;
    return true;
  });
  // System window exit may bypass the in-app editor close button.
  if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
    window.addEventListener("beforeunload", (event: BeforeUnloadEvent) => {
      if (!editorDirty.value && (!inlineFile.value || inlineText.value === inlineSavedText.value)) return;
      event.preventDefault();
      event.returnValue = "";
    });
  }
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
  const inlineSavedText = ref("");
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
  let locateRequest = 0;
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
  let inlineReadRequest = 0;
  let inlineDocumentRevision = 0;
  // Each editor surface gets its own encrypted journal. BrowserOS does not
  // persist user text in localStorage or share one journal between processes.
  const editorDraftSession = crypto.randomUUID().replaceAll("-", "");
  const inlineDraftSession = crypto.randomUUID().replaceAll("-", "");
  // Recovered source journals remain owned by their original session. Normal
  // saves update only this surface's journal; catalog deletion is version-bound.
  let editorJournalTimer: ReturnType<typeof setTimeout> | null = null;
  let inlineJournalTimer: ReturnType<typeof setTimeout> | null = null;
  let journalChain: Promise<void> = Promise.resolve();
  let pendingEditorJournal: (() => void) | null = null;
  let pendingInlineJournal: (() => void) | null = null;
  // An attempted save has three independent facts: the original baseline,
  // the user's local text, and the most recently observed disk contents.
  // Keep them separate even when the native write may have succeeded.
  type SaveConflict = { path: string; revision: number; reason: string; disk: string | null; diskError: boolean; savedCopy: string };
  const editorConflict = ref<SaveConflict | null>(null);
  const inlineConflict = ref<SaveConflict | null>(null);
  const conflictCopyBusy = ref(false);
  const activeEditorConflict = computed(() => editorConflict.value?.revision === editorDocumentRevision
    && editorConflict.value.path === editorFilePath.value ? editorConflict.value : null);
  const activeInlineConflict = computed(() => inlineConflict.value?.revision === inlineDocumentRevision
    && inlineConflict.value.path === inlineFile.value ? inlineConflict.value : null);
  // Save As is independent of the original path; never rebase or overwrite
  // the conflicted original merely because the user created a copy.
  async function saveConflictCopy(surface: "editor" | "inline", destination: string): Promise<boolean> {
    const conflict = surface === "editor" ? activeEditorConflict.value : activeInlineConflict.value;
    const target = destination.trim();
    if (!conflict || !target || conflictCopyBusy.value) return false;
    const text = surface === "editor" ? fileContent.value : inlineText.value;
    const baseline = surface === "editor" ? editorSavedContent.value : inlineSavedText.value;
    const session = surface === "editor" ? editorDraftSession : inlineDraftSession;
    let release: (() => void) | undefined;
    conflictCopyBusy.value = true;
    try {
      documentPathKey("", target);
      if (sameDocumentPath(conflict.path, target)) throw Error("FILE_COPY_MUST_BE_NEW");
      release = documentOwnership.beginSave(surface === "editor" ? "workspace-overlay" : "workspace-inline", target);
      // Native create_file uses pinned-parent, no-replace publication. Keep
      // the original encrypted journal even after the new file is published.
      await queueJournalStrict(() => bridge.editorDraftSave(conflict.path, session, baseline, text));
      await bridge.createFile(target, text);
      if ((surface === "editor" ? activeEditorConflict.value : activeInlineConflict.value) === conflict) {
        conflict.savedCopy = target;
      }
      return true;
    } catch (error) {
      layout.showToast("冲突副本未确认保存，原草稿保留；请检查目标路径，不会自动覆盖: " + String(error).slice(0, 80));
      return false;
    } finally { release?.(); conflictCopyBusy.value = false; }
  }

  async function inspectSaveConflict(surface: "editor" | "inline", path: string, revision: number, reason: string) {
    const owns = () => surface === "editor"
      ? editorDocumentRevision === revision && editorFilePath.value === path
      : inlineDocumentRevision === revision && inlineFile.value === path;
    if (!owns()) return;
    const conflict: SaveConflict = { path, revision, reason, disk: null, diskError: false, savedCopy: "" };
    const state = surface === "editor" ? editorConflict : inlineConflict;
    state.value = conflict;
    const observation = state.value;
    try {
      const disk = await bridge.readFile(path);
      if (owns() && state.value === observation) state.value.disk = disk;
    } catch {
      if (owns() && state.value === observation) state.value.diskError = true;
    }
  }

  async function saveWithDraft(path: string, session: string, baseline: string, text: string) {
    // Persist the exact intended bytes before asking the native owner to
    // mutate a shared file. A crash, missing reply or external conflict must
    // not rely solely on the debounced autosave having happened.
    if (baseline !== text) await queueJournalStrict(() => bridge.editorDraftSave(path, session, baseline, text));
    await bridge.writeFileCas(path, baseline, text);
    // The original operation is best-effort CAS against external writers.
    // Observe the publication before clearing the encrypted draft. This
    // catches some late external writes, not all arbitrary-writer races.
    if (await bridge.readFile(path) !== text) throw Error("FILE_POST_SAVE_CONFLICT");
  }
  function queueJournal(action: () => Promise<void>): Promise<void> {
    journalChain = journalChain.then(action).catch((error) => {
      layout.showToast("草稿加密保存失败，当前修改仍保留在内存中: " + String(error).slice(0, 90));
    });
    return journalChain;
  }
  function queueJournalStrict(action: () => Promise<void>): Promise<void> {
    const task = journalChain.then(action);
    journalChain = task.catch((error) => {
      layout.showToast("草稿加密保存失败，当前修改仍保留在内存中: " + String(error).slice(0, 90));
    });
    return task;
  }
  type RecoverableDraft = {
    recovery_id: string; path: string; session_id: string; updated_at_ms: number; revision?: string; error?: string;
  };
  const recoverableDrafts = ref<RecoverableDraft[]>([]);
  const recoveryNextCursor = ref<string | null>(null);
  const recoveryLoading = ref(false);
  const recoveryActionBusy = ref(false);
  const recoverySaving = ref(false);
  const recoveryError = ref("");
  const recoveryPanelOpen = ref(false);
  async function loadRecoverableDrafts(more = false) {
    if (recoveryLoading.value || (more && !recoveryNextCursor.value)) return;
    recoveryLoading.value = true;
    recoveryError.value = "";
    try {
      const page = await bridge.editorDraftCatalog(more ? recoveryNextCursor.value! : undefined);
      recoverableDrafts.value = more
        ? [...recoverableDrafts.value, ...page.items.filter(item =>
            !recoverableDrafts.value.some(existing => existing.recovery_id === item.recovery_id))]
        : page.items;
      recoveryNextCursor.value = page.next_cursor;
    } catch (error) {
      recoveryError.value = "无法读取加密草稿目录: " + String(error).slice(0, 100);
    } finally { recoveryLoading.value = false; }
  }
  async function toggleRecoveryPanel() {
    recoveryPanelOpen.value = !recoveryPanelOpen.value;
    if (recoveryPanelOpen.value) await loadRecoverableDrafts();
  }
  async function openRecoveredDraft(recoveryId: string): Promise<boolean> {
    if (recoveryActionBusy.value || !canDiscardEditor()) return false;
    const publish = layout.prepareViewChange("editor", true);
    if (!publish) return false;
    const request = ++editorReadRequest;
    const oldPath = editorFilePath.value, oldContent = fileContent.value;
    recoveryActionBusy.value = true;
    let fork: Awaited<ReturnType<typeof bridge.editorDraftRead>> | null = null;
    let published = false;
    try {
      // The native fork is isolated from an original process still autosaving.
      // Neither this read nor catalog enumeration requires the source to exist.
      fork = await bridge.editorDraftRead(recoveryId, true);
      documentPathKey("", fork.path);
      if (request !== editorReadRequest || editorFilePath.value !== oldPath ||
          fileContent.value !== oldContent || !publish()) return false;
      ++editorDocumentRevision;
      editorConflict.value = null;
      editorFilePath.value = fork.path;
      editorSavedContent.value = fork.baseline;
      fileContent.value = fork.content;
      editorRecoverySession.value = { id: fork.recovery_id, revision: fork.revision };
      mdPreview.value = false;
      mdHtml.value = "";
      editingFile.value = true;
      published = true;
      return true;
    } catch (error) {
      recoveryError.value = "草稿无法恢复，原加密记录仍保留: " + String(error).slice(0, 100);
      return false;
    } finally {
      recoveryActionBusy.value = false;
      // An unpublished fork has no user edits. Conditional cleanup cannot
      // remove an updated record or the original source journal.
      if (fork && !published) {
        try { await bridge.editorDraftRemove(fork.recovery_id, fork.revision); }
        catch { recoveryError.value = "未打开的恢复副本仍保留在草稿目录，可稍后处理"; }
      }
    }
  }
  async function discardRecoverableDraft(recoveryId: string) {
    if (recoveryActionBusy.value) return;
    if (editorRecoveryId.value === recoveryId) {
      recoveryError.value = "请先关闭正在编辑的恢复副本，再删除该记录";
      return;
    }
    const revision = recoverableDrafts.value.find(item => item.recovery_id === recoveryId)?.revision;
    if (!revision) { recoveryError.value = "请刷新草稿目录后再删除该记录"; return; }
    if (!confirm("永久删除这份加密草稿？此操作不会删除原文件。")) return;
    recoveryActionBusy.value = true;
    try {
      await journalChain;
      // Bind deletion to the exact catalog version the user selected. Native
      // ownership/version checks also allow explicit removal of corrupt records.
      await bridge.editorDraftRemove(recoveryId, revision);
      recoverableDrafts.value = recoverableDrafts.value.filter(item => item.recovery_id !== recoveryId);
      await loadRecoverableDrafts();
    } catch (error) {
      recoveryError.value = "草稿未删除: " + String(error).slice(0, 100);
    } finally { recoveryActionBusy.value = false; }
  }
  async function saveRecoveredDraftAs(targetPath: string): Promise<boolean> {
    const recovery = editorRecoverySession.value;
    const target = targetPath.trim();
    if (!recovery || recoverySaving.value) return false;
    const text = fileContent.value;
    let release: (() => void) | undefined;
    recoverySaving.value = true;
    try {
      documentPathKey("", target);
      release = documentOwnership.beginSave("workspace-overlay", target);
      if (editorJournalTimer) { clearTimeout(editorJournalTimer); editorJournalTimer = null; pendingEditorJournal = null; }
      await queueJournalStrict(async () => {
        const saved = await bridge.editorDraftUpdateRecovered(recovery.id, recovery.revision, text);
        recovery.revision = saved.revision;
      });
      // Native no-replace publication rejects an existing destination. It never
      // reuses the missing source's baseline to authorize another file.
      const saved = await bridge.editorDraftSaveAs(recovery.id, target, text);
      if (editorRecoverySession.value === recovery) {
        ++editorDocumentRevision;
      editorConflict.value = null;
        editorRecoverySession.value = null;
        editorFilePath.value = saved.path;
        editorSavedContent.value = saved.content;
      }
      // Keep the encrypted recovery source even after publishing a new file.
      if (recoveryPanelOpen.value) void loadRecoverableDrafts();
      return true;
    } catch (error) {
      layout.showToast("另存未完成，恢复草稿仍保留；目标须为尚不存在的新文件: " + String(error).slice(0, 80));
      return false;
    } finally { release?.(); recoverySaving.value = false; }
  }
  // Serialize same-process saves and clears; otherwise a late debounced save
  // could recreate a discarded or already-saved draft after successful save.
  watch([editorFilePath, fileContent, editorSavedContent, editorRecoverySession], (_current, previous) => {
    documentOwnership.replace("workspace-overlay", editorFilePath.value
      ? [{path:editorFilePath.value, dirty:editorDirty.value}] : []);
    if (editorJournalTimer) clearTimeout(editorJournalTimer);
    editorJournalTimer = null;
    // Changing documents must enqueue the old captured draft before the
    // new document can replace its debounce. Never read the new buffer here.
    if (previous && previous[0] !== editorFilePath.value) pendingEditorJournal?.();
    pendingEditorJournal = null;
    if (!editorDirty.value) return;
    const path = editorFilePath.value;
    const content = fileContent.value;
    const baseline = editorSavedContent.value;
    // Capture the recovery edit session as well as the content. A later file
    // switch cannot redirect this queued write to another encrypted journal.
    const recovery = editorRecoverySession.value;
    pendingEditorJournal = () => {
      editorJournalTimer = null;
      pendingEditorJournal = null;
      void queueJournal(async () => {
        if (recovery) {
          const saved = await bridge.editorDraftUpdateRecovered(recovery.id, recovery.revision, content);
          recovery.revision = saved.revision;
        } else {
          await bridge.editorDraftSave(path, editorDraftSession, baseline, content);
        }
      });
    };
    editorJournalTimer = setTimeout(pendingEditorJournal, 650);
  }, { flush: "sync" });
  watch([inlineFile, inlineText, inlineSavedText], (_current, previous) => {
    documentOwnership.replace("workspace-inline", inlineFile.value
      ? [{path:inlineFile.value, dirty:inlineText.value !== inlineSavedText.value}] : []);
    if (inlineJournalTimer) clearTimeout(inlineJournalTimer);
    inlineJournalTimer = null;
    if (previous && previous[0] !== inlineFile.value) pendingInlineJournal?.();
    pendingInlineJournal = null;
    if (!inlineFile.value || inlineText.value === inlineSavedText.value) return;
    const path = inlineFile.value;
    const content = inlineText.value;
    const baseline = inlineSavedText.value;
    pendingInlineJournal = () => {
      inlineJournalTimer = null;
      pendingInlineJournal = null;
      void queueJournal(() => bridge.editorDraftSave(path, inlineDraftSession, baseline, content));
    };
    inlineJournalTimer = setTimeout(pendingInlineJournal, 650);
  }, { flush: "sync" });
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
      return documentPathParent(e.path);
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
    if (name === "." || name === ".." || /[\/\\\0]/.test(name)) {
      layout.showToast("名称无效：请输入单个文件或目录名称");
      closeFileCtx();
      return;
    }
    try {
      await bridge.createFile(joinDocumentPath(base, name), "");
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
    if (name === "." || name === ".." || /[\/\\\0]/.test(name)) {
      layout.showToast("名称无效：请输入单个文件或目录名称");
      closeFileCtx();
      return;
    }
    try {
      await bridge.createDir(joinDocumentPath(base, name));
      layout.showToast("已新建目录: " + name);
      treeExpanded.add(base);
      await refreshTree();
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
    closeFileCtx();
  }
  // 目录预览以目录本身为目标；编辑文件时使用文件的父目录。
  function currentBaseDir(): string {
    if (layout.fileEditorOpen && editorFilePath.value) {
      return documentPathParent(editorFilePath.value);
    }
    if (inlineFile.value) {
      return documentPathParent(inlineFile.value);
    }
    if (previewDir.value) return previewDir.value;
    if (filePath.value) return filePath.value;
    return treeRoots.value[0]?.path || "/";
  }
  async function quickNew(kind: "file" | "dir") {
    const base = currentBaseDir();
    const def = kind === "file" ? "新建文件.txt" : "新建文件夹";
    const label = kind === "file" ? "新文件名" : "新目录名";
    const name = (prompt(`${label}（将创建于 ${base}）：`, def) || "").trim();
    if (!name) return;
    if (name === '.' || name === '..' || /[\/\\\0]/.test(name)) {
      layout.showToast('名称无效：请输入单个文件或目录名称');
      return;
    }
    try {
      if (kind === "file") {
        await bridge.createFile(joinDocumentPath(base, name), "");
        layout.showToast("已新建文件: " + name);
      } else {
        await bridge.createDir(joinDocumentPath(base, name));
        layout.showToast("已新建目录: " + name);
      }
      treeExpanded.add(base);
      await refreshTree();
    } catch (e: any) {
      layout.showToast("新建失败: " + (e?.message ?? e));
    }
  }
  // Refuse path mutations that could orphan or silently replace an active
  // unsaved document, including deleting/moving its containing directory.
  function containsDocument(target: string, openPath: string, isDir: boolean): boolean {
    return !!openPath && (sameDocumentPath(target, openPath) || (isDir && containsDocumentPath(target, openPath)));
  }
  function blocksDirtyPathMutation(path: string, isDir: boolean): boolean {
    try { documentPathKey("", path); }
    catch { layout.showToast("文件路径无效或不受支持"); return true; }
    // Coding Workbench may own an independently dirty buffer for the same
    // filesystem path. Directory mutation must respect every open editor.
    if (documentOwnership.dirtyUnder(path, isDir) || documentOwnership.writingUnder(path, isDir)) {
      layout.showToast("其他工作台仍有未保存的文档，请先保存后重试");
      return true;
    }
    const overlayBlocked = editorDirty.value && containsDocument(path, editorFilePath.value, isDir);
    const inlineBlocked = !!inlineFile.value && inlineText.value !== inlineSavedText.value && containsDocument(path, inlineFile.value, isDir);
    if (overlayBlocked || inlineBlocked) {
      layout.showToast("当前文件或目录含未保存的编辑草稿，请先保存或关闭编辑器后重试");
      return true;
    }
    return false;
  }
  function lockPathMutation(paths: { path: string; isDirectory: boolean }[]): (() => void) | null {
    try { return documentOwnership.beginMutation(paths); }
    catch { layout.showToast('文件或目录正在被编辑、保存或移动，请处理后重试'); return null; }
  }
  function replacePathPrefix(oldPath: string, from: string, to: string): string {
    return replaceDocumentPathPrefix(oldPath, from, to);
  }
  function updateOpenPaths(from: string, to: string) {
    editorFilePath.value = replacePathPrefix(editorFilePath.value, from, to);
    inlineFile.value = replacePathPrefix(inlineFile.value, from, to);
    previewDir.value = replacePathPrefix(previewDir.value, from, to);
    const updatedDir = replacePathPrefix(filePath.value, from, to);
    if (updatedDir !== filePath.value) { filePath.value = updatedDir; pathInput.value = updatedDir; }
    const expanded = [...treeExpanded].map(path => replacePathPrefix(path, from, to));
    treeExpanded.clear();
    for (const path of expanded) treeExpanded.add(path);
    locateTarget.value = replacePathPrefix(locateTarget.value, from, to);
    layout.relocateDirectoryTabs(from, to);
  }
  async function ctxDelete(entry: DirEntry) {
    if (blocksDirtyPathMutation(entry.path, entry.is_dir)) return;
    const target = entry.path;
    const openPath = inlineFile.value;
    const containsOpenFile = containsDocument(target, openPath, entry.is_dir);
    if (containsOpenFile && !canDiscardInline()) return;
    if (!confirm(`删除「${entry.name}」？此操作不可恢复`)) {
      closeFileCtx();
      return;
    }
    const releaseMutation = lockPathMutation([{path: entry.path, isDirectory: entry.is_dir}]);
    if (!releaseMutation) return;
    try {
      await bridge.deletePath(entry.path);
      documentOwnership.invalidatePath(entry.path, entry.is_dir);
      const overlayRemoved = containsDocument(target, editorFilePath.value, entry.is_dir);
      const preserveOverlay = overlayRemoved && editorDirty.value;
      const preserveInline = containsOpenFile && inlineFile.value === openPath && inlineText.value !== inlineSavedText.value;
      const preserveOtherDraft = documentOwnership.dirtyUnder(target, entry.is_dir);
      if (overlayRemoved && !preserveOverlay) {
        editorFilePath.value = "";
        editorSavedContent.value = "";
        fileContent.value = "";
        layout.setView("files");
      }
      layout.showToast(preserveOverlay || preserveInline || preserveOtherDraft
        ? "已删除磁盘文件；删除期间产生的编辑草稿仍保留，请复制或另存。"
        : "已删除: " + entry.name);
      if (containsOpenFile && inlineFile.value === openPath && !preserveInline) {
        ++inlineReadRequest;
        inlineFile.value = "";
      }
      await refreshTree();
    } catch (e: any) {
      layout.showToast("删除失败: " + (e?.message ?? e));
    } finally { releaseMutation(); }
    closeFileCtx();
  }
  async function ctxRename(entry: DirEntry) {
    if (blocksDirtyPathMutation(entry.path, entry.is_dir)) return;
    const name = prompt("重命名为：", entry.name)?.trim();
    if (!name || name === entry.name) {
      closeFileCtx();
      return;
    }
    if (name === "." || name === ".." || /[\/\\\0]/.test(name)) {
      layout.showToast("名称无效：请输入单个文件或目录名称");
      closeFileCtx();
      return;
    }
    let renamed: string;
    try { renamed = joinDocumentPath(documentPathParent(entry.path), name); }
    catch { layout.showToast("名称或路径无效，请输入单个文件或目录名称"); closeFileCtx(); return; }
    const releaseMutation = lockPathMutation([{path: entry.path, isDirectory: entry.is_dir}, {path: renamed, isDirectory: entry.is_dir}]);
    if (!releaseMutation) return;
    try {
      await bridge.renamePath(entry.path, name);
      documentOwnership.invalidatePath(entry.path, entry.is_dir, ["workspace-overlay", "workspace-inline"]);
      const editedDuringMutation = documentOwnership.dirtyUnder(entry.path, entry.is_dir);
      updateOpenPaths(entry.path, renamed);
      layout.showToast(editedDuringMutation
        ? "已重命名；操作期间产生了编辑草稿，请检查原路径及新路径后再保存。"
        : "已重命名 → " + name);
      await refreshTree();
    } catch (e: any) {
      layout.showToast("重命名失败: " + (e?.message ?? e));
    } finally { releaseMutation(); }
    closeFileCtx();
  }
  async function ctxFavorite(entry: DirEntry) {
    if (!entry.is_dir) return;
    // STAGE I-B：经 shared 窄缝 src/composables/homeNav.ts 触发 Home 公开意图，
    // 避免 workspace→home 的必须依赖（会与 home→workspace 构成 CB-04 必须依赖环）。
    const { favoriteHomeDirectory } = await import("../../../composables/homeNav");
    favoriteHomeDirectory(entry.path);
    closeFileCtx();
  }
  async function copyPath(entry: DirEntry, relative: boolean) {
    let value = entry.path;
    if (relative) {
      const root = treeRoots.value.find((item) => containsDocumentPath(item.path, entry.path));
      value = root ? relativeDocumentPath(root.path, entry.path) || "." : entry.name;
    }
    await bridge.clipboardWrite(value);
    layout.showToast(relative ? "已复制相对路径" : "已复制绝对路径");
    closeFileCtx();
  }
  // 文件树右键：新标签打开（目录→目录页签；文件→行内编辑/预览）
  async function ctxOpenInNewTab(entry: DirEntry) {
    closeFileCtx();
    if (entry.is_dir) {
      try {
        const publish = layout.prepareDirTab(entry.path);
        if (!publish || !await enterDir(entry.path, publish)) return;
      } catch { layout.showToast("目录路径无效或目录切换未完成"); }
    } else {
      await openFileInline(entry);
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
    const dir = entry.is_dir ? entry.path : documentPathParent(entry.path);
    await openTerminalAt(dir);
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
      const name = documentPathName(filePath.value);
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
      const name = documentPathName(filePath.value);
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
    const request = ++locateRequest;
    if (!targetPath) return;
    if (!startDirs.value.length) {
      try {
        await loadStartDirs();
        if (request !== locateRequest) return;
      } catch {
        return;
      }
    }
    const dirs = [...treeRoots.value, ...startDirs.value].map((d) => d.path).filter(Boolean);
    let root = "";
    for (const dir of dirs) {
      if (containsDocumentPath(dir, targetPath) &&
          (!root || documentPathKey("", dir).length > documentPathKey("", root).length)) root = dir;
    }
    if (!root) {
      locateTarget.value = targetPath;
      return;
    }
    if (treeRoots.value.length === 1 && filePath.value &&
        sameDocumentPath(treeRoots.value[0].path, filePath.value) &&
        !sameDocumentPath(filePath.value, root)) treeRoots.value = startDirs.value;
    // Follow the actual native entries so case or separator aliases never
    // become a second Map key and a file is never queried as a directory.
    let current = root;
    const relative = relativeDocumentPath(root, targetPath) || "";
    let directory = true;
    for (const segment of relative.split("/").filter(Boolean)) {
      treeExpanded.add(current);
      await ensureTreeChildren(current);
      if (request !== locateRequest) return;
      const next = (treeChildren.get(current) || []).find(entry =>
        sameDocumentPath(entry.path, joinDocumentPath(current, segment)));
      if (!next) break;
      current = next.path;
      directory = next.is_dir;
      if (!directory) break;
    }
    if (directory && sameDocumentPath(current, targetPath)) {
      treeExpanded.add(current);
      await ensureTreeChildren(current);
      if (request !== locateRequest) return;
    }
    locateTarget.value = sameDocumentPath(current, targetPath) ? current : targetPath;
    await nextTick();
    if (request !== locateRequest) return;
    // Attribute comparison avoids CSS escaping differences for backslashes.
    for (const node of document.querySelectorAll<HTMLElement>("[data-path]")) {
      if (node.dataset.path === locateTarget.value) {
        node.scrollIntoView({ block: "center" });
        break;
      }
    }
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
    try { documentPathKey("", src); documentPathKey("", path); }
    catch { layout.showToast("移动路径无效或不受支持"); return; }
    if (sameDocumentPath(src, path)) return;
    if (containsDocumentPath(src, path)) {
      layout.showToast("不能移动到自身子目录内");
      return;
    }
    requestMove(src, path);
  }

  function requestMove(src: string, dstDir: string) {
    try { documentPathKey("", src); documentPathKey("", dstDir); }
    catch { layout.showToast("移动路径无效或不受支持"); return; }
    const name = documentPathName(src);
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
    if (blocksDirtyPathMutation(src, true)) return;
    const target = joinDocumentPath(dst, documentPathName(src));
    const releaseMutation = lockPathMutation([{path: src, isDirectory: true}, {path: target, isDirectory: true}]);
    if (!releaseMutation) return;
    try {
      await bridge.movePath(src, dst);
      documentOwnership.invalidatePath(src, true, ["workspace-overlay", "workspace-inline"]);
      const editedDuringMutation = documentOwnership.dirtyUnder(src, true);
      updateOpenPaths(src, target);
      layout.showToast(editedDuringMutation
        ? "已移动；操作期间产生了编辑草稿，请检查原路径及新路径后再保存。"
        : "已移动: " + documentPathName(dst));
      await refreshTree();
    } catch (e: any) {
      layout.showToast("移动失败: " + (e?.message ?? e));
    } finally { releaseMutation(); }
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
  function canDiscardInline(): boolean {
    return !inlineFile.value || inlineText.value === inlineSavedText.value ||
      confirm('当前文件有未保存的修改，放弃修改并继续吗？');
  }
  async function openFileInline(entry: DirEntry) {
    if (entry.is_dir) {
      toggleTreeDir(entry.path);
      return openDirPreview(entry);
    }
    const ext = entry.name.split(".").pop()?.toLowerCase() || "";
    const isMd = ext === "md" || ext === "markdown";
    if (!isMd && !TEXT_EXTS.includes(ext)) {
      layout.showToast(`非文本文件 (${entry.name})，暂不支持预览`);
      return false;
    }
    if (layout.mainView !== "files" && !canDiscardInline()) return false;
    const publish = layout.prepareViewChange("files", true);
    if (!publish) return false;
    const request = ++inlineReadRequest;
    ++previewRequest;
    const oldFile = inlineFile.value;
    const oldText = inlineText.value;
    const ownsRead = () => request === inlineReadRequest && inlineFile.value === oldFile &&
      inlineText.value === oldText && publish.isCurrent();
    try {
      documentPathKey("", entry.path);
      const text = await bridge.readFile(entry.path);
      if (!ownsRead()) return false;
      let recovered: { session_id: string; baseline: string; content: string } | null = null;
      try {
        const drafts = await bridge.editorDraftList(entry.path);
        if (!ownsRead()) return false;
        if (drafts.length && confirm("检测到未完成的加密行内草稿，是否恢复？")) recovered = drafts[0];
      } catch (error) {
        if (ownsRead()) layout.showToast("草稿恢复检查失败，请勿删除原有草稿: " + String(error).slice(0, 70));
      }
      if (!ownsRead() || !publish()) return false;
      ++inlineDocumentRevision;
      inlineConflict.value = null;
      inlineFile.value = entry.path;
      filePath.value = documentPathParent(entry.path);
      pathInput.value = filePath.value;
      inlineText.value = recovered?.content ?? text;
      inlineSavedText.value = recovered?.baseline ?? text;
      inlineIsMd.value = isMd;
      inlineEdit.value = !isMd;
      inlineHtml.value = isMd ? renderMd(inlineText.value) : "";
      previewLoading.value = false;
      await rememberOpenedFile(entry);
      return true;
    } catch (e: any) {
      if (ownsRead()) layout.showToast("读取失败: " + (e?.message ?? e));
      return false;
    }
  }
  function inlineToggleEdit() {
    inlineEdit.value = !inlineEdit.value;
    if (!inlineEdit.value && inlineIsMd.value) inlineHtml.value = renderMd(inlineText.value);
  }
  async function saveInline() {
    if (!inlineFile.value || inlineText.value === inlineSavedText.value) return;
    const path = inlineFile.value, text = inlineText.value, baseline = inlineSavedText.value;
    const documentRevision = inlineDocumentRevision;
    let releaseSave: (() => void) | undefined;
    try {
      releaseSave = documentOwnership.beginSave("workspace-inline", path);
      await saveWithDraft(path, inlineDraftSession, baseline, text);
    } catch (error: any) {
      void inspectSaveConflict("inline", path, documentRevision, String(error).slice(0, 160));
      layout.showToast(!releaseSave && String(error).includes("DOCUMENT_PATH_CHANGED")
        ? "原文件已移动或删除；保留本地草稿，请另存为新文件。"
        : "保存结果未确认；保留本地编辑，请使用冲突恢复区检查磁盘版本。");
      return;
    } finally {
      releaseSave?.();
    }
    const stillSameInlineFile = inlineFile.value === path && inlineDocumentRevision === documentRevision;
    if (stillSameInlineFile) {
      inlineConflict.value = null;
      inlineSavedText.value = text;
      if (inlineJournalTimer) { clearTimeout(inlineJournalTimer); inlineJournalTimer = null; pendingInlineJournal = null; }
    }
    // Snapshot before enqueueing; global inlineText can belong to another file
    // by the time an earlier encrypted journal operation finishes.
    // A different document now owns the surface. Preserve its predecessor's
    // queued recovery record rather than clearing bytes from a stale completion.
    if (!stillSameInlineFile) return;
    const currentText = inlineText.value;
    try {
      await queueJournalStrict(() => {
        // A reopened instance of the same path owns the surface journal now.
        // Older CAS completion must not clear that instance's recovery record.
        if (inlineFile.value === path && inlineDocumentRevision !== documentRevision) return Promise.resolve();
        return persistDraftAfterSave({
          path,
          currentSessionId: inlineDraftSession,
          savedText: text,
          currentText,
          save: (filePath, sessionId, baseline, content) => bridge.editorDraftSave(filePath, sessionId, baseline, content),
          clear: (filePath, sessionId) => bridge.editorDraftClear(filePath, sessionId),
        });
      });
    } catch (error) {
      layout.showToast("文件已写入，但草稿恢复记录未能安全更新；请勿关闭当前编辑器: " + String(error).slice(0, 70));
      return;
    }
    if (inlineIsMd.value && !inlineEdit.value) inlineHtml.value = renderMd(inlineText.value);
    layout.showToast("已保存: " + documentPathName(inlineFile.value));
  }
  function closeInline() {
    if (!canDiscardInline()) return;
    ++inlineReadRequest;
    ++inlineDocumentRevision;
      inlineConflict.value = null;
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
    if (!entry.is_dir) return false;
    try { documentPathKey("", entry.path); }
    catch { layout.showToast("目录路径无效或不受支持"); return false; }
    if (layout.mainView !== "files" && !canDiscardInline()) return false;
    const publish = layout.prepareViewChange("files", true);
    if (!publish) return false;
    ++inlineReadRequest;
    const request = ++previewRequest;
    const oldFile = inlineFile.value, oldText = inlineText.value;
    previewLoading.value = true;
    try {
      const entries = await bridge.listDir(entry.path);
      if (request !== previewRequest || inlineFile.value !== oldFile ||
          inlineText.value !== oldText || !publish()) return false;
      clearPreviewImageQueue();
      previewDir.value = entry.path;
      previewEntries.value = entries;
      previewImages.value = {};
      previewImageErrors.value = {};
      compareImages.value = [];
      previewError.value = "";
      ++inlineDocumentRevision;
      inlineConflict.value = null;
      inlineFile.value = "";
      return true;
    } catch (e: any) {
      if (request === previewRequest && publish.isCurrent()) previewError.value = e?.message ?? String(e);
      return false;
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
  async function initializeDirectory(path: string): Promise<boolean> {
    if (filePath.value || editorFilePath.value || inlineFile.value || previewDir.value) return false;
    const isCurrent = layout.captureNavigation();
    const editorRequest = editorReadRequest, inlineRequest = inlineReadRequest, preview = previewRequest;
    try {
      documentPathKey("", path);
      const entries = await bridge.listDir(path);
      if (!isCurrent() || editorRequest !== editorReadRequest || inlineRequest !== inlineReadRequest ||
          preview !== previewRequest || filePath.value || editorFilePath.value || inlineFile.value || previewDir.value) return false;
      // Startup warms the Files owner only. It never creates a tab or changes
      // Home/Browser/Terminal selection and cannot replace a user's directory.
      filePath.value = path;
      pathInput.value = path;
      fileEntries.value = entries;
      treeChildren.set(path, entries);
      treeErrors.delete(path);
      treeRoots.value = [{ name: documentPathName(path), path, is_dir: true, size: 0 }];
      treeExpanded.add(path);
      return true;
    } catch (error) {
      if (isCurrent()) layout.showToast("初始目录未载入，可在文件面板重新打开: " + String(error).slice(0, 80));
      return false;
    }
  }
  async function enterDir(path: string, navigation?: ViewChangeCommit): Promise<boolean> {
    try { documentPathKey("", path); }
    catch { layout.showToast("目录路径无效或不受支持"); return false; }
    const publish = navigation ?? layout.prepareDirTab(path, true);
    if (!publish || !publish.isCurrent()) return false;
    // The Layout permit already guarded the visible editor. Directory changes
    // also replace retained editors that may currently be behind another view.
    if (layout.mainView !== "editor" && !canDiscardEditor()) return false;
    if (layout.mainView !== "files" && !canDiscardInline()) return false;
    const request = ++editorReadRequest;
    ++inlineReadRequest;
    ++previewRequest;
    const previousEditor = editorFilePath.value;
    const previousContent = fileContent.value;
    const previousInline = inlineFile.value;
    const previousInlineText = inlineText.value;
    let entries: DirEntry[];
    try {
      entries = await bridge.listDir(path);
    } catch (error: any) {
      if (request === editorReadRequest && publish.isCurrent()) {
        layout.showToast("无法读取目录: " + (error?.message ?? error));
      }
      return false;
    }
    // Commit the visible intent and all directory state in one synchronous
    // turn. No caller publishes a tab after an already-mutated FileStore.
    if (request !== editorReadRequest || editorFilePath.value !== previousEditor ||
        fileContent.value !== previousContent || inlineFile.value !== previousInline ||
        inlineText.value !== previousInlineText || !publish()) return false;
    ++editorDocumentRevision;
      editorConflict.value = null;
    ++inlineDocumentRevision;
      inlineConflict.value = null;
    editorRecoverySession.value = null;
    editorFilePath.value = "";
    editorSavedContent.value = "";
    fileContent.value = "";
    inlineFile.value = "";
    clearPreviewImageQueue();
    previewDir.value = "";
    previewEntries.value = [];
    previewImages.value = {};
    previewImageErrors.value = {};
    compareImages.value = [];
    previewLoading.value = false;
    previewError.value = "";
    filePath.value = path;
    pathInput.value = path;
    fileEntries.value = entries;
    treeChildren.set(path, entries);
    treeErrors.delete(path);
    treeRoots.value = [{ name: documentPathName(path), path, is_dir: true, size: 0 }];
    treeExpanded.add(path);
    return true;
  }
  function goUp() {
    const p = filePath.value;
    if (!p) return;
    const parent = documentPathParent(p);
    enterDir(parent);
  }
  function goPath() {
    const p = pathInput.value.trim();
    if (!p) return;
    enterDir(p);
  }
  async function rememberOpenedFile(entry: DirEntry) {
    try {
      const { useWorkspaceStore } = await import("./useWorkspaceStore");
      useWorkspaceStore().addRecentFile(entry.path, entry.name);
    } catch {
      layout.showToast("文件已打开，但最近文件记录未能更新");
    }
  }
  async function openFile(entry: DirEntry) {
    if (entry.is_dir) return enterDir(entry.path);
    const ext = entry.name.split(".").pop()?.toLowerCase() || "";
    const markdown = ext === "md" || ext === "markdown";
    if (!markdown && !TEXT_EXTS.includes(ext)) {
      layout.showToast(`非文本文件 (${entry.name})，暂不支持预览`);
      return false;
    }
    // Reserve navigation before the first await, including lazy recent-file
    // imports; a later Home/Browser intent must supersede this file opening.
    const opened = await openEditorDocument(entry.path, markdown, entry.name, entry.size);
    if (opened) await rememberOpenedFile(entry);
    return opened;
  }
  async function openMd(entry: DirEntry) {
    const ext = entry.name.split(".").pop()?.toLowerCase();
    if (ext !== "md" && ext !== "markdown") return false;
    return openEditorDocument(entry.path, true, entry.name, entry.size);
  }
  async function openEditorDocument(path: string, markdown: boolean, name: string, size: number) {
    if (!canDiscardEditor()) return false;
    const publish = layout.prepareViewChange("editor", true);
    if (!publish) return false;
    const request = ++editorReadRequest;
    const currentPath = editorFilePath.value;
    const currentContent = fileContent.value;
    const ownsRead = () => request === editorReadRequest && editorFilePath.value === currentPath &&
      fileContent.value === currentContent && publish.isCurrent();
    try {
      documentPathKey("", path);
      const text = await bridge.readFile(path);
      if (!ownsRead()) return false;
      let recovered: { session_id: string; baseline: string; content: string } | null = null;
      try {
        const drafts = await bridge.editorDraftList(path);
        if (!ownsRead()) return false;
        if (drafts.length && confirm("检测到未完成的加密编辑草稿，是否恢复？外部磁盘修改不会被覆盖。")) {
          recovered = drafts[0];
        }
      } catch (error) {
        if (ownsRead()) layout.showToast("草稿恢复检查失败，请勿删除原有草稿: " + String(error).slice(0, 70));
      }
      if (!ownsRead() || !publish()) return false;
      ++editorDocumentRevision;
      editorConflict.value = null;
      editorRecoverySession.value = null;
      editorFilePath.value = path;
      editorSavedContent.value = recovered?.baseline ?? text;
      fileContent.value = recovered?.content ?? text;
      mdHtml.value = markdown ? renderMd(fileContent.value) : "";
      mdPreview.value = markdown;
      editingFile.value = !markdown;
      return true;
    } catch (error: any) {
      if (ownsRead()) layout.showToast("读取失败: " + (error?.message ?? error));
      return false;
    }
  }
  async function saveFile() {
    if (editorRecoverySession.value) {
      layout.showToast("恢复草稿请使用“另存为新文件”；原文件不会被重新创建或覆盖");
      return;
    }
    const path = editorFilePath.value;
    if (!path) return;
    const text = fileContent.value;
    const baseline = editorSavedContent.value;
    if (baseline === text) return;
    const documentRevision = editorDocumentRevision;
    let releaseSave: (() => void) | undefined;
    try {
      releaseSave = documentOwnership.beginSave("workspace-overlay", path);
      // Do not clear the recovery record on an uncertain native outcome.
      await saveWithDraft(path, editorDraftSession, baseline, text);
      const stillSameEditorFile = editorFilePath.value === path && editorDocumentRevision === documentRevision;
      if (stillSameEditorFile) {
        editorConflict.value = null;
        editorSavedContent.value = text;
        if (editorJournalTimer) { clearTimeout(editorJournalTimer); editorJournalTimer = null; pendingEditorJournal = null; }
      }
      // Preserve the departed document's recovery journal when a late save
      // finishes after navigation. Its baseline may need explicit reconciliation.
      if (!stillSameEditorFile) return;
      const currentText = fileContent.value;
      try {
        await queueJournalStrict(() => {
          if (editorFilePath.value === path && editorDocumentRevision !== documentRevision) return Promise.resolve();
          return persistDraftAfterSave({
            path,
            currentSessionId: editorDraftSession,
            savedText: text,
            currentText,
            save: (filePath, sessionId, baseline, content) => bridge.editorDraftSave(filePath, sessionId, baseline, content),
            clear: (filePath, sessionId) => bridge.editorDraftClear(filePath, sessionId),
          });
        });
      } catch (journalError) {
        layout.showToast("文件已写入，但草稿恢复记录未能安全更新；请勿关闭当前编辑器: " + String(journalError).slice(0, 70));
        return;
      }
      layout.showToast("已保存: " + documentPathName(path));
    } catch (error: any) {
      void inspectSaveConflict("editor", path, documentRevision, String(error).slice(0, 160));
      layout.showToast(!releaseSave && String(error).includes("DOCUMENT_PATH_CHANGED")
        ? "原文件已移动或删除；保留本地草稿，请另存为新文件。"
        : "保存结果未确认；保留本地编辑，请使用冲突恢复区检查磁盘版本。");
    } finally {
      releaseSave?.();
    }
  }

  // ===== Files canonical writers（组件经此写状态，禁止直写，FILE-03 / WS-OWNER-02）=====
  function setFileContent(v: string) { fileContent.value = v; }
  function setInlineText(v: string) { inlineText.value = v; }
  function setEditorView(isPreview: boolean) {
    if (isPreview) mdHtml.value = renderMd(fileContent.value);
    mdPreview.value = isPreview;
    editingFile.value = !isPreview;
  }
  function clearDragSource() { dragSource.value = ""; }

  return {
    fileEntries,
    filePath,
    pathInput,
    fileContent,
    editorFilePath,
    editorRecoveryId,
    editorDirty,
    activeEditorConflict,
    activeInlineConflict,
    conflictCopyBusy,
    saveConflictCopy,
    recoverableDrafts,
    recoveryNextCursor,
    recoveryLoading,
    recoveryActionBusy,
    recoverySaving,
    recoveryError,
    recoveryPanelOpen,
    loadRecoverableDrafts,
    toggleRecoveryPanel,
    openRecoveredDraft,
    discardRecoverableDraft,
    saveRecoveredDraftAs,
    canDiscardEditor,
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
    initializeDirectory,
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
