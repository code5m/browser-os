import { defineStore } from "pinia";
import { ref, reactive } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import {
  emptySnippetForm,
  loadSnippetForm,
  validateSnippetForm,
  serializeSnippetForm,
  canDeleteSnippet,
  type SnippetForm,
} from "../../../utils/snippetUi";
import type { CommandSnippet } from "../../../types";

// ============================================================
// Phase 8C-0D — Snippet（命令片段库 CRUD）域 owner
//
// 从 useWorkspaceStore 抽出：snippets / snippetForm + CRUD（执行经 run_command 复用脚本运行态）。
// 后端持久化（snippetList/snippetAdd/snippetUpdate/snippetRemove）。
// ============================================================

export const useSnippetStore = defineStore("snippet", () => {
  const layout = useLayoutStore();

  const snippets = ref<CommandSnippet[]>([]);
  const snippetForm = reactive<SnippetForm>(emptySnippetForm());

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

  return { snippets, snippetForm, loadSnippets, openSnippetForm, saveSnippet, removeSnippet };
});
