import { defineStore } from "pinia";
import { ref, reactive } from "vue";
import { bridge } from "../../../bridge";
import { navigateBrowser } from "../../../composables/browserNav";
import { useFileStore } from "./useFileStore";
import { useArtifactStore } from "./useArtifactStore";
import { useRepoStore } from "./useRepoStore";

export interface RecentItem {
  type: "url" | "file";
  title: string;
  path: string;
  at: number;
}

// ============================================================
// Phase 8C-0 — Workspace Core（God Store 收敛完成）
//
//   8C-0A：Files     → useFileStore
//   8C-0B：Artifact  → useArtifactStore
//   8C-0C：Repo      → useRepoStore
//   8C-0D：Script    → useScriptStore / Snippet → useSnippetStore
//
// 本 store 现只保留「Workspace 自身语义」：
//   - AUDIT 镜像（Governance 域，8C-0 不动，见 02-DOMAIN-BOUNDARIES.md）
//   - recents（url+file 跨域，暂留 Workspace Core 过渡）
//   - 跨域编排 refresh()（聚合 artifact/repo/audit 的 load，不持有其内部状态）
//
// 禁止回存 Files / Artifact / Repo / Script 子域内部状态（WS_OWNER_* 门禁守护）。
// ============================================================

export const useWorkspaceStore = defineStore("workspace", () => {
  // ===== 审计（AUDIT 域镜像；归 Governance，8C-0 不动）=====
  const audit = ref<AuditEntry[]>([]);

  // ===== 最近访问（跨域；文件侧由 useFileStore 调 addRecentFile，url 侧见下）=====
  const recents = reactive<RecentItem[]>([]);

  // ===== 跨域编排（Workspace Core 职责：聚合各子域 load）=====
  async function refresh() {
    const [, , a] = await Promise.all([
      useArtifactStore().loadTree(),
      useRepoStore().loadRepos(),
      bridge.auditLog(),
    ]);
    audit.value = a;
  }

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
      navigateBrowser(r.path);
    } else {
      // 文件侧打开逻辑已归 Files owner（useFileStore）
      useFileStore().openFile({ name: r.title, path: r.path, is_dir: false, size: 0 });
    }
  }

  return {
    audit,
    recents,
    refresh,
    loadRecents,
    addRecentUrl,
    addRecentFile,
    openRecent,
  };
});
