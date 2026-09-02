import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../bridge";
import { useBrowserStore } from "./useBrowserStore";
import { useLayoutStore } from "./useLayoutStore";
import type {
  BrowserSession,
  SessionCloseChoice,
  SessionPolicy,
  SessionSummary,
} from "../types";

// M1-9 会话存档与关闭协议状态层。
//
// 关闭协议（不可静默丢）：关闭 tab 时不直接关闭，先弹「保存 / 删除 / 取消」。
// 通过 browser.bindCloseInterceptor 挂接（单向依赖：本 store 依赖 browser store，
// browser store 不反向依赖本 store，避免循环 import）。
//
// 隐私：预览文本与 URL 均由后端二次脱敏；前端不持久化任何会话数据。

export const useSessionStore = defineStore("session", () => {
  const layout = useLayoutStore();

  const sessions = ref<SessionSummary[]>([]);
  const detail = ref<BrowserSession | null>(null);
  const detailId = ref<string>("");
  const policy = ref<SessionPolicy>({ close_prompt: true, auto_save_on_exit: false });
  const loading = ref(false);
  const error = ref("");

  // 关闭协议：待用户决定的 tab（非空 = 弹窗打开）
  const pendingCloseTabId = ref<string>("");
  const pendingCloseTitle = ref<string>("");
  const closeDialogOpen = computed(() => pendingCloseTabId.value !== "");

  async function loadPolicy() {
    try {
      policy.value = await bridge.getSessionPolicy();
    } catch {
      // 保持默认（close_prompt=true / auto_save_on_exit=false）
    }
  }

  async function setPolicy(closePrompt?: boolean, autoSaveOnExit?: boolean) {
    try {
      policy.value = await bridge.setSessionPolicy(closePrompt, autoSaveOnExit);
    } catch {
      layout.showToast("⚠️ 会话策略设置失败");
    }
  }

  async function loadSessions() {
    loading.value = true;
    error.value = "";
    try {
      sessions.value = await bridge.sessionList();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function openDetail(id: string) {
    try {
      detail.value = await bridge.sessionGet(id);
      detailId.value = id;
    } catch (e) {
      layout.showToast("⚠️ 读取会话失败");
    }
  }

  function closeDetail() {
    detail.value = null;
    detailId.value = "";
  }

  // 保存当前页签为会话（立即落盘）
  async function saveTab(tabId: string, preview?: string): Promise<SessionSummary | null> {
    try {
      const summary = await bridge.sessionSave(tabId, preview ?? "");
      await loadSessions();
      layout.showToast("✅ 会话已保存");
      return summary;
    } catch (e) {
      layout.showToast("⚠️ 会话保存失败");
      return null;
    }
  }

  // 采集页面最小文本预览（失败回退空串，后端兜底脱敏与截断）
  async function capturePreview(tabId: string): Promise<string> {
    try {
      const text = await bridge.evalInTab(
        tabId,
        "(document.body && document.body.innerText ? document.body.innerText.slice(0, 2000) : '')"
      );
      return typeof text === "string" ? text : "";
    } catch {
      return "";
    }
  }

  async function deleteSession(id: string) {
    try {
      await bridge.sessionDelete(id);
      if (detailId.value === id) closeDetail();
      await loadSessions();
      layout.showToast("已删除会话存档");
    } catch (e) {
      layout.showToast("⚠️ 删除会话失败");
    }
  }

  // 恢复：用已脱敏 URL 新建页签（登录态/一次性 token 不会恢复，后端如实返回）
  async function restoreSession(id: string) {
    try {
      const tab = await bridge.sessionRestore(id);
      const browser = useBrowserStore();
      browser.tabs.push(tab);
      browser.activeTabId = tab.id;
      layout.mainView = "browser";
      layout.showToast("已恢复会话（部分页面可能需要重新登录）");
    } catch (e) {
      layout.showToast("⚠️ 会话恢复失败");
    }
  }

  // 导出：复制脱敏 JSON 到剪贴板（后端不落盘，由用户决定去处）
  async function exportSession(id: string) {
    try {
      const json = await bridge.sessionExport(id);
      await bridge.clipboardWrite(json);
      layout.showToast("会话 JSON 已复制到剪贴板");
    } catch (e) {
      layout.showToast("⚠️ 会话导出失败");
    }
  }

  // ===== 关闭协议 =====
  // 返回 true 表示已接管（弹窗等待用户决定），false 表示走默认直接关闭。
  function requestClose(tabId: string): boolean {
    if (!policy.value.close_prompt) return false;
    const browser = useBrowserStore();
    const tab = browser.tabs.find((t) => t.id === tabId);
    if (!tab) return false;
    pendingCloseTabId.value = tabId;
    pendingCloseTitle.value = tab.title || tab.url;
    return true;
  }

  async function resolveClose(choice: SessionCloseChoice) {
    const tabId = pendingCloseTabId.value;
    pendingCloseTabId.value = "";
    pendingCloseTitle.value = "";
    if (!tabId || choice === "cancel") return;
    const browser = useBrowserStore();
    if (choice === "save") {
      const preview = await capturePreview(tabId);
      await saveTab(tabId, preview);
    } else if (choice === "discard") {
      try {
        await bridge.sessionDiscard(tabId);
      } catch {
        // 丢弃失败不阻断关闭
      }
    }
    await browser.closeTabNow(tabId);
  }

  return {
    sessions,
    detail,
    detailId,
    policy,
    loading,
    error,
    pendingCloseTabId,
    pendingCloseTitle,
    closeDialogOpen,
    loadPolicy,
    setPolicy,
    loadSessions,
    openDetail,
    closeDetail,
    saveTab,
    capturePreview,
    deleteSession,
    restoreSession,
    exportSession,
    requestClose,
    resolveClose,
  };
});
