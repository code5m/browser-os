import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../../../bridge";
import type {
  PluginState,
  PluginSummary,
  PluginDetail,
  TrustedKeyRecord,
  PluginManifest,
} from "../../../types";
import { enabledActionsFor, parseManifestInput } from "../../../utils/pluginUi";

// M5-W14 插件管理器 store。
// 红线（承 W14 Hard Stops）：
//  - 只调用冻结的 src/bridge.ts 方法；**绝不**使用裸 Tauri `invoke`。
//  - 不显示/不持久化：签名原文、公钥原文、资源绝对路径、metadata 正文、凭据、
//    请求/响应体、stdout/stderr。安装用的 manifest 文本与 resourcePath 为**瞬时**
//    入参，调用成功后立即清空，不进入任何展示状态。
//  - 状态变更（enable/disable/uninstall/install/addKey/removeKey）的"显式确认"
//    由 UI 在调用本 store action 之前完成；本 store 只负责执行并刷新。

function describeError(e: unknown): string {
  // 后端错误可能含 manifest 输入或路径；UI 只显示稳定的本地失败文案。
  void e;
  return "插件操作失败，请检查输入或重试。";
}

export const usePluginStore = defineStore("plugin", () => {
  const list = ref<PluginSummary[]>([]);
  const detail = ref<PluginDetail | null>(null);
  const keys = ref<TrustedKeyRecord[]>([]);
  const filterState = ref<PluginState | null>(null);
  const busy = ref(false);
  const error = ref("");
  // 安装表单（瞬时输入，含签名原文；不回显、成功后清空）
  const manifestText = ref("");
  const resourcePath = ref("");

  const backendReady = computed(
    () =>
      typeof bridge.pluginList === "function" &&
      typeof bridge.pluginInstall === "function" &&
      typeof bridge.pluginEnable === "function" &&
      typeof bridge.pluginDisable === "function" &&
      typeof bridge.pluginGet === "function" &&
      typeof bridge.pluginKeysList === "function" &&
      typeof bridge.pluginKeysAdd === "function" &&
      typeof bridge.pluginKeysRemove === "function",
  );

  /** 当前详情视图下，三个状态变更按钮是否可用（按后端状态机门控）。 */
  const actionsFor = computed<{ enable: boolean; disable: boolean; uninstall: boolean }>(() =>
    detail.value ? enabledActionsFor(detail.value.state) : { enable: false, disable: false, uninstall: false },
  );

  async function refreshList() {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return;
    }
    busy.value = true;
    error.value = "";
    try {
      list.value = await bridge.pluginList(filterState.value);
    } catch (e) {
      error.value = describeError(e);
    } finally {
      busy.value = false;
    }
  }

  async function getDetail(id: string) {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return;
    }
    busy.value = true;
    error.value = "";
    try {
      detail.value = await bridge.pluginGet(id);
    } catch (e) {
      error.value = describeError(e);
    } finally {
      busy.value = false;
    }
  }

  /** 安装（校验）一个供予的 manifest。manifestText 为瞬时入参，成功后清空。 */
  async function install(): Promise<boolean> {
    const parsed = parseManifestInput(manifestText.value);
    if (!parsed.ok || !parsed.manifest) {
      error.value = parsed.error || "manifest 解析失败";
      return false;
    }
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return false;
    }
    busy.value = true;
    error.value = "";
    try {
      await bridge.pluginInstall({
        manifest: parsed.manifest as PluginManifest,
        resourcePath: resourcePath.value.trim() || null,
      });
      // 清空瞬时输入（含签名原文），不回显、不持久化。
      manifestText.value = "";
      resourcePath.value = "";
      await refreshList();
      return true;
    } catch (e) {
      error.value = describeError(e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function enable(id: string): Promise<boolean> {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return false;
    }
    busy.value = true;
    error.value = "";
    try {
      await bridge.pluginEnable(id);
      await refreshList();
      if (detail.value?.id === id) await getDetail(id);
      return true;
    } catch (e) {
      error.value = describeError(e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function disable(id: string): Promise<boolean> {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return false;
    }
    busy.value = true;
    error.value = "";
    try {
      await bridge.pluginDisable(id);
      await refreshList();
      if (detail.value?.id === id) await getDetail(id);
      return true;
    } catch (e) {
      error.value = describeError(e);
      return false;
    } finally {
      busy.value = false;
    }
  }

  async function refreshKeys() {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return;
    }
    error.value = "";
    try {
      keys.value = await bridge.pluginKeysList();
    } catch (e) {
      error.value = describeError(e);
    }
  }

  /** 增加受信任密钥。pubkey 为瞬时入参，后端仅回 16-hex 指纹，不持久化原文。 */
  async function addKey(keyId: string, pubkey: string, note: string): Promise<boolean> {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return false;
    }
    error.value = "";
    try {
      keys.value = await bridge.pluginKeysAdd({
        keyId: keyId.trim(),
        pubkey: pubkey.trim(),
        note: note.trim() || null,
      });
      return true;
    } catch (e) {
      error.value = describeError(e);
      return false;
    }
  }

  async function removeKey(keyId: string): Promise<boolean> {
    if (!backendReady.value) {
      error.value = "后端插件命令未就绪";
      return false;
    }
    error.value = "";
    try {
      keys.value = await bridge.pluginKeysRemove(keyId);
      return true;
    } catch (e) {
      error.value = describeError(e);
      return false;
    }
  }

  function selectFromList(id: string) {
    const found = list.value.find((p) => p.id === id);
    if (found) void getDetail(id);
  }

  function clearError() {
    error.value = "";
  }

  return {
    list,
    detail,
    keys,
    filterState,
    busy,
    error,
    manifestText,
    resourcePath,
    backendReady,
    actionsFor,
    refreshList,
    getDetail,
    install,
    enable,
    disable,
    refreshKeys,
    addKey,
    removeKey,
    selectFromList,
    clearError,
  };
});
