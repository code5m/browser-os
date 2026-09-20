import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import { useBrowserStore } from "./useBrowserStore";

// ============================================================
// Phase 8C-0B — Artifact（Knowledge / 笔记 Vault）域 owner
//
// 从 useWorkspaceStore（God Store）抽出：成果树 + 当前编辑 + 多选 + 右键菜单 + 采集。
// 后端持久化（browseWorkspace / readArtifact / updateArtifact / deleteArtifact / collectSelection）。
// Workspace Core 不再持有 artifact 状态；跨域编排 refresh() 改调本 store.loadTree()。
//
// 边界：artifact 状态唯一真源 = 本 store。组件只渲染 + 调 action；禁止直写。
// ============================================================

export const useArtifactStore = defineStore("artifact", () => {
  const layout = useLayoutStore();

  const tree = ref<WorkspaceTree>({ nodes: [] });
  const current = ref<Artifact | null>(null);
  const editTitle = ref("");
  const editTags = ref("");
  const editText = ref("");
  const selected = reactive<Set<string>>(new Set());

  // 成果右键菜单状态
  const ctxMenu = reactive<{
    show: boolean;
    x: number;
    y: number;
    item: Artifact | null;
  }>({ show: false, x: 0, y: 0, item: null });

  const flatArtifacts = computed(() => tree.value.nodes.flatMap((n) => n.items));

  async function loadTree() {
    tree.value = await bridge.browseWorkspace();
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
    await loadTree();
    layout.showToast("已保存编辑");
  }
  async function removeArtifact(item: DomainItem) {
    if (!confirm(`删除「${item.title}」？`)) return;
    await bridge.deleteArtifact(item.id);
    if (current.value?.id === item.id) current.value = null;
    await loadTree();
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

  // 采集当前网页选中内容（对应 prototype 的「＋ 采集选中内容」）
  async function collectSelection() {
    try {
      const sel = (window.getSelection?.()?.toString?.() || "").trim();
      const url = useBrowserStore().url || "about:blank";
      const title = useBrowserStore().activeTab?.title || url;
      const text = sel || "";
      const art = await bridge.collectSelection({ url, title, html: "", text });
      await loadTree();
      layout.showToast("✅ 已保存: " + (art.title || text.slice(0, 18) || "选中内容"));
    } catch (e: any) {
      layout.showToast("采集失败: " + (e?.message ?? e));
    }
  }

  // ===== canonical writers（组件经此写编辑缓冲，禁止直写；WS-OWNER-03 守护）=====
  function setEditTitle(v: string) { editTitle.value = v; }
  function setEditTags(v: string) { editTags.value = v; }
  function setEditText(v: string) { editText.value = v; }

  return {
    tree,
    current,
    editTitle,
    editTags,
    editText,
    selected,
    setEditTitle,
    setEditTags,
    setEditText,
    ctxMenu,
    flatArtifacts,
    loadTree,
    openArtifact,
    saveEdit,
    removeArtifact,
    onArtifactContext,
    closeCtx,
    ctxReveal,
    ctxOpenSource,
    ctxRemove,
    toggle,
    collectSelection,
  };
});
