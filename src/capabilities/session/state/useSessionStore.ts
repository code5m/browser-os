import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../../../bridge";
import { hostServices, type BrowserContextPort } from "../../../capability/platform/host-services";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import type {
  BrowserSession,
  SessionPolicy,
  SessionSummary,
} from "../../../types";

// 历史会话存档状态层（M1-9 会话存档，不含关闭协议）。
//
// Owner 最终裁决（2026-09-12）：普通 Tab 关闭 = 不弹确认框 + 不持久化 + 直接关闭。
// 关闭入口只把 {url,title} 写入 recentlyClosed 内存栈（见 useBrowserStore），再调
// browser.closeTabNow 完成 WebView 生命周期关闭。本 store 不再参与关闭拦截；
// "保存会话"仅为用户主动触发的能力（saveTab），与关闭 Tab 完全解耦。
// 原关闭协议（SessionCloseDialog 三选一：保存/删除/取消）已撤销。
//
// 隐私：预览文本与 URL 均由后端二次脱敏；前端不持久化任何会话数据。

export const useSessionStore = defineStore("session", () => {
  const layout = useLayoutStore();

  const sessions = ref<SessionSummary[]>([]);
  const detail = ref<BrowserSession | null>(null);
  const detailId = ref<string>("");
  // close_prompt 仅保留为后端契约兼容字段，前端关闭路径不再读取（见 types.ts 备注）。
  const policy = ref<SessionPolicy>({ close_prompt: true, auto_save_on_exit: false });
  const loading = ref(false);
  const error = ref("");

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

  // 用户主动保存当前页签为会话（立即落盘）。
  // silent=true 时不弹 Toast（遵循「成功操作不显示提示」约定）。
  async function saveTab(tabId: string, preview?: string, silent = false): Promise<SessionSummary | null> {
    try {
      const summary = await bridge.sessionSave(tabId, preview ?? "");
      await loadSessions();
      if (!silent) layout.showToast("✅ 会话已保存");
      return summary;
    } catch (e) {
      if (!silent) layout.showToast("⚠️ 会话保存失败");
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
      hostServices.require<BrowserContextPort>("browser-context").adoptRestoredTab(tab);
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

  return {
    sessions,
    detail,
    detailId,
    policy,
    loading,
    error,
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
  };
});
