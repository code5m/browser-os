import { defineStore } from "pinia";
import { ref, reactive } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";
import {
  emptyScriptForm,
  loadFormFromMeta,
  validateScriptForm,
  serializeScriptForm,
  canDeleteScript,
  type ScriptForm,
} from "../utils/scriptUi";
import type { ScriptMeta } from "../types";

// ============================================================
// Phase 8C-0D — Script（脚本库 CRUD）域 owner
//
// 从 useWorkspaceStore 抽出：scripts / scriptForm + CRUD（复用 M2-3 四条命令）。
// 后端持久化（scriptList/scriptAdd/scriptUpdate/scriptRemove）。
// ============================================================

export const useScriptStore = defineStore("script", () => {
  const layout = useLayoutStore();

  const scripts = ref<ScriptMeta[]>([]);
  const scriptForm = reactive<ScriptForm>(emptyScriptForm());

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

  return { scripts, scriptForm, loadScripts, openScriptForm, saveScript, removeScript };
});
