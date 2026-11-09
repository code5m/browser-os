import { computed, ref } from "vue";
import { defineStore } from "pinia";
import {
  bridge,
  type BfcImportResult,
  type BfcReadbackStatus,
  type BfcServiceStatus,
} from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useBrowserStore } from "../../browser/public";
import {
  buildFavoritesReadbackScript,
  buildLoginProbeScript,
} from "../model/readbackScripts.mjs";

const BILIBILI_HOME = "https://www.bilibili.com/";
const BFC_UPDATES = "http://127.0.0.1:4198/#updates";
const BFC_IMPORT = "http://127.0.0.1:4198/manage#sync";
const LOGIN_TIMEOUT_MS = 25_000;
const READBACK_TIMEOUT_MS = 7 * 60_000;
const POLL_INTERVAL_MS = 750;

type BfcPhase =
  | "idle"
  | "checking_service"
  | "checking_login"
  | "logged_out"
  | "access_limited"
  | "unknown"
  | "opening_favorites"
  | "reading_favorites"
  | "ready_to_import"
  | "importing"
  | "imported"
  | "outcome_unknown"
  | "failed"
  | "cancelled";

const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

function isBilibiliUrl(raw: string): boolean {
  try {
    const url = new URL(raw);
    return url.protocol === "https:"
      && (url.hostname === "www.bilibili.com" || url.hostname === "space.bilibili.com");
  } catch {
    return false;
  }
}

function isFavoritesUrl(raw: string, mid: string): boolean {
  try {
    const url = new URL(raw);
    const segments = url.pathname.split("/").filter(Boolean);
    return url.protocol === "https:"
      && url.hostname === "space.bilibili.com"
      && segments[0] === mid
      && segments[1] === "favlist";
  } catch {
    return false;
  }
}

function messageForCode(code: string | null | undefined): string {
  const messages: Record<string, string> = {
    BFC_BILIBILI_ACCESS_LIMITED: "B站页面显示访问受限或安全验证。已停止翻页，不会重试、轮换代理或绕过验证。",
    BFC_BILIBILI_HTTP_412: "检测到 B站 HTTP 412 / 风控状态。已停止读取，请等待平台限制解除。",
    BFC_TOTAL_UNKNOWN: "页面没有提供可核实的收藏总数，无法证明分页完整；没有导入快照。",
    BFC_PAGINATION_UNKNOWN: "无法识别收藏分页信息，已停止读取。",
    BFC_FAVORITES_CONTAINER_UNRECOGNIZED: "无法识别当前收藏列表结构。可能是页面改版或尚未加载完成；没有导入快照。",
    BFC_READBACK_PAGE_COUNT_MISMATCH: "当前页实际读取数量与页面总数推算不一致；没有导入快照。",
    BFC_READBACK_TOTAL_MISMATCH: "分页合计与页面报告的收藏总数不一致；没有导入快照。",
    BFC_PAGINATION_TIMEOUT: "等待下一页加载超时；没有导入快照。",
    BFC_PAGINATION_INCOMPLETE: "页面没有提供可用的下一页入口；没有导入快照。",
    BFC_START_FROM_FIRST_PAGE: "无法回到收藏列表第一页；没有导入快照。",
    BFC_FAVORITES_PAGE_REQUIRED: "当前不是已登录账号对应的收藏夹页面。",
    BFC_LOGIN_IDENTITY_NOT_FOUND: "页面未能提供可核实的账号主页 ID，不能确认登录身份。",
    BFC_BROWSER_SCRIPT_ERROR: "页面读取脚本执行失败。请确认 B站页面正常显示后再试。",
  };
  return code ? (messages[code] || `读取未完成：${code}`) : "读取未完成。";
}

export const useBfcStore = defineStore("bfc", () => {
  const layout = useLayoutStore();
  const browser = useBrowserStore();

  const phase = ref<BfcPhase>("idle");
  const busy = ref(false);
  const message = ref("尚未读取 B站收藏。只有用户主动开始后，才会检查当前已打开的 B站页面。");
  const serviceStatus = ref<BfcServiceStatus | null>(null);
  const readbackStatus = ref<BfcReadbackStatus | null>(null);
  const importResult = ref<BfcImportResult | null>(null);
  const activeTabId = ref("");
  const ticket = ref("");
  const cancelRequested = ref(false);
  const lastErrorCode = ref("");
  const partialSnapshotWarning = computed(() => serviceStatus.value?.complete_snapshot === false);

  async function refreshService(): Promise<BfcServiceStatus> {
    const result = await bridge.bfcServiceStatus();
    serviceStatus.value = result;
    return result;
  }

  async function ensureBilibiliTab(): Promise<string> {
    const active = browser.tabs.find((tab) => tab.id === browser.activeTabId);
    const existing = active && isBilibiliUrl(active.url)
      ? active
      : browser.tabs.find((tab) => isBilibiliUrl(tab.url));

    layout.activateView("browser");
    if (existing) {
      if (browser.activeTabId !== existing.id) await browser.tabSwitch(existing.id);
      activeTabId.value = existing.id;
      return existing.id;
    }

    await browser.tabNew(BILIBILI_HOME);
    const id = browser.activeTabId;
    if (!id || !browser.tabs.some((tab) => tab.id === id)) {
      throw new Error("BFC_BILIBILI_TAB_CREATE_FAILED");
    }
    activeTabId.value = id;
    return id;
  }

  async function navigateTab(tabId: string, target: string): Promise<void> {
    const tab = browser.tabs.find((item) => item.id === tabId);
    if (!tab) throw new Error("BFC_BILIBILI_TAB_NOT_FOUND");
    tab.url = target;
    tab.title = target;
    await browser.tabNavigate(tabId);
  }

  async function waitForStatus(
    readbackTicket: string,
    terminalStates: string[],
    timeoutMs: number,
  ): Promise<BfcReadbackStatus> {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      if (cancelRequested.value) throw new Error("BFC_READBACK_CANCELLED");
      const next = await bridge.bfcReadbackStatus(readbackTicket);
      readbackStatus.value = next;
      if (terminalStates.includes(next.state)) return next;
      await wait(POLL_INTERVAL_MS);
    }
    throw new Error("BFC_READBACK_TIMEOUT");
  }

  async function stopAndShow(phaseValue: BfcPhase, text: string): Promise<void> {
    phase.value = phaseValue;
    message.value = text;
    layout.activateView("bfc");
  }

  async function openBiliPage(): Promise<void> {
    try {
      const tabId = await ensureBilibiliTab();
      const tab = browser.tabs.find((item) => item.id === tabId);
      if (!tab || !isBilibiliUrl(tab.url)) await navigateTab(tabId, BILIBILI_HOME);
      layout.activateView("browser");
      message.value = "已打开 B站官方页面。请正常登录并确认收藏夹可以在页面中显示，然后回到 BFC 点击「开始读取」。";
    } catch (error) {
      phase.value = "failed";
      message.value = String(error);
      layout.activateView("bfc");
    }
  }

  async function openLocalUpdates(): Promise<void> {
    layout.activateView("browser");
    await browser.tabNew(BFC_UPDATES);
  }

  async function openManualImport(): Promise<void> {
    layout.activateView("browser");
    await browser.tabNew(BFC_IMPORT);
  }

  async function startReadback(): Promise<void> {
    if (busy.value) return;
    busy.value = true;
    cancelRequested.value = false;
    phase.value = "checking_service";
    message.value = "正在检查本地 BFC 服务。尚未向 B站读取任何内容。";
    readbackStatus.value = null;
    importResult.value = null;
    lastErrorCode.value = "";
    let activeTicket = "";

    try {
      const service = await refreshService();
      if (!service.online || service.local_only !== true || service.read_snapshot_only !== true
          || service.bilibili_network_requests !== 0) {
        throw new Error(service.message_code || "BFC_LOCAL_SERVICE_UNSAFE_OR_UNAVAILABLE");
      }

      const tabId = await ensureBilibiliTab();
      const created = await bridge.bfcReadbackBegin(tabId);
      activeTicket = created.ticket;
      ticket.value = activeTicket;

      phase.value = "checking_login";
      message.value = "正在检查当前 B站官方页面的可见登录状态；不会读取 Cookie 或账号凭据。";
      await bridge.evalInTab(tabId, "delete window.__browserosBfcCancelScan;");
      await wait(1800);
      await bridge.evalInTab(tabId, buildLoginProbeScript(activeTicket));

      const login = await waitForStatus(
        activeTicket,
        ["logged_in", "logged_out", "access_limited", "unknown", "failed"],
        LOGIN_TIMEOUT_MS,
      );
      if (login.state === "logged_out") {
        await stopAndShow("logged_out", "当前页面没有检测到已登录账号。请打开 B站正常登录，再回到这里重新开始；不会读取或导出 Cookie。");
        return;
      }
      if (login.state === "access_limited") {
        await stopAndShow("access_limited", messageForCode(login.message_code));
        return;
      }
      if (login.state !== "logged_in" || !login.mid) {
        await stopAndShow("unknown", messageForCode(login.message_code || "BFC_LOGIN_IDENTITY_NOT_FOUND"));
        return;
      }

      const mid = login.mid;
      const current = browser.tabs.find((tab) => tab.id === tabId);
      if (!current) throw new Error("BFC_BILIBILI_TAB_NOT_FOUND");
      if (!isFavoritesUrl(current.url, mid)) {
        phase.value = "opening_favorites";
        message.value = "已确认页面存在登录态，正在打开该账号的官方收藏夹页面。";
        await navigateTab(tabId, `https://space.bilibili.com/${encodeURIComponent(mid)}/favlist`);
        await wait(2200);
      }

      phase.value = "reading_favorites";
      message.value = "正在按页面显示顺序读取收藏夹分页。若页面出现 HTTP 412、安全验证或数量不一致，将立即停止。";
      await bridge.evalInTab(tabId, buildFavoritesReadbackScript(activeTicket, mid));

      const captured = await waitForStatus(
        activeTicket,
        ["ready", "access_limited", "failed", "outcome_unknown"],
        READBACK_TIMEOUT_MS,
      );
      if (captured.state === "access_limited") {
        await stopAndShow("access_limited", messageForCode(captured.message_code));
        return;
      }
      if (captured.state !== "ready") {
        await stopAndShow(
          captured.state === "outcome_unknown" ? "outcome_unknown" : "failed",
          messageForCode(captured.message_code || "BFC_BROWSER_SCRIPT_ERROR"),
        );
        return;
      }

      if (cancelRequested.value) return;
      phase.value = "ready_to_import";
      message.value = `页面读取完成：${captured.items_captured} 条，${captured.pages_captured}/${captured.expected_pages} 页。正在提交到本机 BFC 服务。`;
      busy.value = false;
      await finishReadback();
    } catch (error) {
      if (cancelRequested.value) {
        phase.value = "cancelled";
        message.value = "已取消本次读取。不会继续翻页或导入快照。";
        layout.activateView("bfc");
        return;
      }
      const code = String(error instanceof Error ? error.message : error);
      lastErrorCode.value = code;
      if (activeTicket) {
        try {
          const current = await bridge.bfcReadbackStatus(activeTicket);
          readbackStatus.value = current;
          if (current.state === "outcome_unknown") {
            await stopAndShow("outcome_unknown", "本地 BFC 服务的写入结果不确定。为避免重复导入，系统没有自动重试；请先到 BFC 工作台检查本次扫描记录。");
            return;
          }
          if (current.state === "ready") {
            await stopAndShow("ready_to_import", `快照已在内存中完整读取，但尚未确认写入本地 BFC：${code}。检查本地服务后，可手动重试一次保存。`);
            return;
          }
          await bridge.bfcReadbackCancel(activeTicket).catch(() => {});
        } catch {
          // The session may have expired or already been removed.
        }
      }
      await stopAndShow("failed", code === "BFC_READBACK_TIMEOUT"
        ? "等待 B站页面读取超时。已停止本次流程；请确认页面可正常打开，之后可重新开始。"
        : `读取失败：${code}`);
    } finally {
      busy.value = false;
    }
  }

  async function finishReadback(): Promise<void> {
    if (busy.value || !ticket.value) return;
    busy.value = true;
    phase.value = "importing";
    message.value = "正在使用本机 BFC 的 CSRF 会话保存快照。不会执行账号收藏写入。";
    try {
      const result = await bridge.bfcReadbackFinish(ticket.value);
      importResult.value = result;
      readbackStatus.value = await bridge.bfcReadbackStatus(ticket.value);
      phase.value = "imported";
      message.value = serviceStatus.value?.complete_snapshot === false
        ? `BFC 已接收 ${result.cards} 条收藏（${result.pages} 页，${result.unique} 个唯一 BV）。注意：当前 BFC 服务声明 completeSnapshot=false，因此该快照在工作台中仍应视为待核实/不完整，不代表完整同步。`
        : `BFC 已接收 ${result.cards} 条收藏（${result.pages} 页，${result.unique} 个唯一 BV）。`;
      layout.activateView("bfc");
    } catch (error) {
      const code = String(error instanceof Error ? error.message : error);
      lastErrorCode.value = code;
      const current = await bridge.bfcReadbackStatus(ticket.value).catch(() => null);
      if (current) readbackStatus.value = current;
      if (current?.state === "ready") {
        phase.value = "ready_to_import";
        message.value = `本地 BFC 未确认收到本次快照（${code}）。读取结果仍保留在内存中；检查本地服务后可手动重试保存。`;
      } else if (current?.state === "outcome_unknown") {
        phase.value = "outcome_unknown";
        message.value = "本地 BFC 写入结果不确定。为避免重复导入，系统没有自动重试；请先检查本次扫描记录。";
      } else {
        phase.value = "failed";
        message.value = `保存到本地 BFC 失败：${code}`;
      }
      layout.activateView("bfc");
    } finally {
      busy.value = false;
    }
  }

  async function cancelReadback(): Promise<void> {
    const currentTicket = ticket.value;
    if (!currentTicket) return;
    if (["importing", "imported", "outcome_unknown"].includes(phase.value)) {
      message.value = "快照已开始提交到本机 BFC，不能撤销可能已经成功的写入；请先核对本次扫描记录。";
      return;
    }
    cancelRequested.value = true;
    const tabId = activeTabId.value || browser.activeTabId;
    if (tabId) {
      await bridge.evalInTab(
        tabId,
        `window.__browserosBfcCancelScan = ${JSON.stringify(currentTicket)};`,
      ).catch(() => {});
    }
    try {
      await bridge.bfcReadbackCancel(currentTicket);
      phase.value = "cancelled";
      message.value = "已取消本次读取。不会继续翻页或导入快照。";
      layout.activateView("bfc");
    } catch {
      cancelRequested.value = false;
      const current = await bridge.bfcReadbackStatus(currentTicket).catch(() => null);
      if (current) readbackStatus.value = current;
      phase.value = current?.state === "outcome_unknown" ? "outcome_unknown"
        : current?.state === "importing" ? "importing" : "failed";
      message.value = "无法确认读取已取消，可能已经开始本机导入。请先检查本次扫描记录，不要重复导入。";
      layout.activateView("bfc");
    }
  }

  return {
    phase,
    busy,
    message,
    serviceStatus,
    readbackStatus,
    importResult,
    activeTabId,
    ticket,
    lastErrorCode,
    partialSnapshotWarning,
    refreshService,
    openBiliPage,
    openLocalUpdates,
    openManualImport,
    startReadback,
    finishReadback,
    cancelReadback,
  };
});
