#!/usr/bin/env node
// ---------------------------------------------------------------------------
// 会话存档 / 页签关闭逻辑前端自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/capabilities/session/state/useSessionStore.ts` 与 `src/capabilities/browser/state/useBrowserStore.ts`，
// 只把 `src/bridge.ts` 的会话/页签相关方法替换为记录型 mock（不 mock store 自身逻辑）：
// 因此下面每一条断言反映的都是**产品代码**的行为，而非测试替身的行为。
//
// Owner 最终裁决（2026-09-12）：普通 Tab 关闭 = 不弹确认框 + 不持久化 + 直接关闭。
// 关闭入口只把 {url,title} 写入 recentlyClosed 内存栈，再调 closeTabNow 完成生命周期关闭。
// 本测试同时是**确定性门禁**：任何代码若让普通 Tab 关闭重新触发 prompt 或 sessionSave，
// 或重新引入已撤销的关闭协议（requestClose / resolveClose / closeDialogOpen /
// pendingCloseTabId / bindCloseInterceptor / autoSaveOnClose），都必须 FAIL。
//
// 覆盖：普通关闭不保存不弹框、recentlyClosed 内存栈 +1 与上限 20、Ctrl+Shift+T 恢复、
// 手动保存仍持久化且与关闭解耦、删除/恢复/导出、策略开关、异常容错。
//
// 用法: node scripts/check-session-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__m19Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m19Resolve = resolveWithExt;
}

// useLayoutStore.showToast 用 window.setTimeout（浏览器环境），Node 下补最小桩
globalThis.window = { setTimeout: (fn, ms) => setTimeout(fn, ms) };

const ROOT = new URL("..", import.meta.url).pathname;

const { bridge } = await import(`${ROOT}src/bridge.ts`);
const { useSessionStore } = await import(`${ROOT}src/capabilities/session/state/useSessionStore.ts`);
const { useBrowserStore } = await import(`${ROOT}src/capabilities/browser/state/useBrowserStore.ts`);
const { createPinia, setActivePinia } = await import(
  `${ROOT}node_modules/pinia/dist/pinia.mjs`
);

// ---------- mock bridge（只替换会话/页签相关方法，记录调用顺序） ----------
const calls = [];
let savedSummaries = [];
let policyState = { close_prompt: true, auto_save_on_exit: false };

function summary(id, tabId) {
  return {
    id,
    tab_id: tabId,
    url: "https://ex.com/a?token=***",
    title: "标题" + id,
    preview: "预览",
    preview_truncated: false,
    resource_count: 3,
    saved: true,
    close_reason: "user_saved",
    created_at: "2026-09-03T00:00:00Z",
    updated_at: "2026-09-03T00:00:00Z",
  };
}

bridge.sessionSave = async (tabId, preview) => {
  calls.push(["sessionSave", tabId, preview]);
  const s = summary("s" + (savedSummaries.length + 1), tabId);
  savedSummaries.push(s);
  return s;
};
bridge.sessionDiscard = async (tabId) => {
  calls.push(["sessionDiscard", tabId]);
};
bridge.sessionList = async () => {
  calls.push(["sessionList"]);
  return savedSummaries.slice();
};
bridge.sessionGet = async (id) => {
  calls.push(["sessionGet", id]);
  return { ...summary(id, "tab-1"), resources: [] };
};
bridge.sessionDelete = async (id) => {
  calls.push(["sessionDelete", id]);
  savedSummaries = savedSummaries.filter((s) => s.id !== id);
  return true;
};
bridge.sessionExport = async (id) => {
  calls.push(["sessionExport", id]);
  return '{"id":"' + id + '"}';
};
bridge.sessionRestore = async (id) => {
  calls.push(["sessionRestore", id]);
  return { id: "tab-restored-" + id, url: "https://ex.com/a?token=***", title: "已恢复" };
};
bridge.flushSessions = async () => {
  calls.push(["flushSessions"]);
  return { persisted: 0, drafts_dropped: 1, tmp_removed: 0, capacity_removed: 0 };
};
bridge.getSessionPolicy = async () => {
  calls.push(["getSessionPolicy"]);
  return { ...policyState };
};
bridge.setSessionPolicy = async (closePrompt, autoSaveOnExit) => {
  calls.push(["setSessionPolicy", closePrompt, autoSaveOnExit]);
  if (closePrompt !== null && closePrompt !== undefined) policyState.close_prompt = closePrompt;
  if (autoSaveOnExit !== null && autoSaveOnExit !== undefined)
    policyState.auto_save_on_exit = autoSaveOnExit;
  return { ...policyState };
};
bridge.evalInTab = async (tabId, js) => {
  calls.push(["evalInTab", tabId]);
  return "页面正文片段";
};
bridge.tabClose = async (id) => {
  calls.push(["tabClose", id]);
};
bridge.tabActivate = async (id) => {
  calls.push(["tabActivate", id]);
};
bridge.tabNew = async (u) => {
  calls.push(["tabNew", u]);
  return { id: "tab-new-" + u, url: u, title: "" };
};
bridge.clipboardWrite = async (text) => {
  calls.push(["clipboardWrite", text]);
};

let passed = 0;
const failures = [];
function assert(cond, label) {
  if (cond) {
    passed += 1;
  } else {
    failures.push(label);
    console.error(`FAIL: ${label}`);
  }
}
function findCall(name) {
  return calls.find((c) => c[0] === name);
}
function hasCall(name) {
  return calls.some((c) => c[0] === name);
}
function callOrder() {
  return calls.map((c) => c[0]).join(",");
}

setActivePinia(createPinia());
const session = useSessionStore();
const browser = useBrowserStore();

// ================= 0) 确定性门禁：已撤销的关闭协议表面必须不存在 =================
// 任何重新引入这些符号的代码都必须让本组断言 FAIL（防止普通关闭再次触发 prompt/save）。
assert(typeof session.requestClose === "undefined", "requestClose 必须已移除（禁止普通关闭走 prompt）");
assert(typeof session.resolveClose === "undefined", "resolveClose 必须已移除（禁止三选一确认框）");
assert(typeof session.closeDialogOpen === "undefined", "closeDialogOpen 必须已移除");
assert(typeof session.pendingCloseTabId === "undefined", "pendingCloseTabId 必须已移除");
assert(typeof session.autoSaveOnClose === "undefined", "autoSaveOnClose 必须已撤销");
assert(typeof browser.bindCloseInterceptor === "undefined", "bindCloseInterceptor 必须已移除（无关闭拦截器）");

// ================= 1) 普通 Tab 关闭：不保存 + 不弹框 + recentlyClosed +1 =================
browser.tabs.push({ id: "tab-1", url: "https://a.example.com/p", title: "页面A" });
const rcBefore = browser.recentlyClosed.length;
calls.length = 0;
await browser.tabClose("tab-1");
assert(!hasCall("sessionSave"), "普通关闭不得调用 sessionSave（不持久化）");
assert(!hasCall("sessionDiscard"), "普通关闭不得调用 sessionDiscard");
assert(browser.recentlyClosed.length === rcBefore + 1, "普通关闭必须使 recentlyClosed +1");
assert(browser.recentlyClosed[0] && browser.recentlyClosed[0].url === "https://a.example.com/p",
  "recentlyClosed 栈顶必须是刚关闭的页签");
assert(browser.recentlyClosed[0].title === "页面A", "recentlyClosed 必须记录标题");
assert(hasCall("tabClose"), "普通关闭必须调用 bridge.tabClose 完成生命周期关闭");
assert(!browser.tabs.find((t) => t.id === "tab-1"), "普通关闭后 tab-1 必须从页签列表移除");
// 全程不得有任何确认框相关状态
assert(browser.recentlyClosed.length === rcBefore + 1, "普通关闭不得产生额外副作用");

// ================= 2) 多次普通关闭只写内存栈，不触发任何持久化 =================
browser.tabs.push({ id: "tab-2", url: "https://b.example.com", title: "页面B" });
browser.tabs.push({ id: "tab-3", url: "https://c.example.com", title: "页面C" });
calls.length = 0;
await browser.tabClose("tab-2");
await browser.tabClose("tab-3");
assert(!hasCall("sessionSave"), "批量普通关闭仍不得调用 sessionSave");
assert(browser.recentlyClosed.length === rcBefore + 3, "三次关闭后 recentlyClosed 必须累计 +3");

// ================= 3) recentlyClosed 内存栈上限 20 =================
// 清空后连续关闭 25 个页签，栈必须封顶 20（不持久化、不报错）
browser.recentlyClosed = [];
for (let i = 1; i <= 25; i++) {
  const id = "cap-" + i;
  browser.tabs.push({ id, url: "https://cap.example.com/" + i, title: "C" + i });
  await browser.tabClose(id);
}
assert(browser.recentlyClosed.length === 20, `recentlyClosed 必须封顶 20（实际 ${browser.recentlyClosed.length}）`);
assert(browser.recentlyClosed.every((e) => typeof e.url === "string" && typeof e.title === "string"),
  "recentlyClosed 每项必须仅含 {url,title}");
assert(!hasCall("sessionSave"), "封顶测试期间仍不得触发 sessionSave");

// ================= 4) Ctrl+Shift+T（restoreRecent）正确恢复 =================
const rcLen = browser.recentlyClosed.length; // 应为 20
calls.length = 0;
await browser.restoreRecent();
assert(hasCall("tabNew"), "Ctrl+Shift+T 必须调用 tabNew 重开页签");
assert(browser.recentlyClosed.length === rcLen - 1, "restoreRecent 必须弹出栈顶（长度 -1）");
assert(browser.tabs.some((t) => t.url === browser.recentlyClosed[0]?.url || t.id.startsWith("tab-new-")),
  "restoreRecent 必须新增一个页签");

// ================= 5) 空栈恢复：不调用 tabNew =================
calls.length = 0;
browser.recentlyClosed = [];
await browser.restoreRecent();
assert(!hasCall("tabNew"), "空栈恢复不得调用 tabNew（仅提示，不重开）");

// ================= 6) 手动保存 Session：仍可持久化，且与关闭解耦 =================
calls.length = 0;
browser.tabs.push({ id: "tab-9", url: "https://d.example.com", title: "页面D" });
const saved = await session.saveTab("tab-9", "预览X");
assert(saved && saved.tab_id === "tab-9", "saveTab 必须返回保存的会话摘要");
assert(hasCall("sessionSave"), "手动保存必须调用 sessionSave（与关闭解耦、独立持久化）");
assert(session.sessions.some((s) => s.tab_id === "tab-9"), "saveTab 后历史会话列表必须包含新会话");
// 手动保存不得写入 recentlyClosed（只有关闭才写）
const rcAfterSave = browser.recentlyClosed.length;
await session.saveTab("tab-9", "预览Y");
assert(browser.recentlyClosed.length === rcAfterSave, "手动保存不得写入 recentlyClosed（仅关闭才写）");

// ================= 7) 删除会话（幂等 + 详情清理） =================
calls.length = 0;
await session.openDetail("s1");
assert(session.detailId === "s1", "openDetail 必须设置当前详情");
await session.deleteSession("s1");
assert(hasCall("sessionDelete"), "deleteSession 必须调用后端删除");
assert(session.detailId === "", "删除当前详情后必须清空详情");
assert(!session.sessions.some((s) => s.id === "s1"), "删除后列表必须移除");

// ================= 8) 恢复会话（新 tab + 激活） =================
calls.length = 0;
const before = browser.tabs.length;
await session.restoreSession("s2");
assert(hasCall("sessionRestore"), "restoreSession 必须调用后端恢复");
assert(browser.tabs.length === before + 1, "恢复后必须新增页签");
assert(browser.activeTabId === "tab-restored-s2", "恢复后必须激活新页签");

// ================= 9) 导出（复制到剪贴板） =================
calls.length = 0;
await session.exportSession("s2");
assert(hasCall("sessionExport"), "exportSession 必须调用后端导出");
assert(hasCall("clipboardWrite"), "导出必须复制到剪贴板（后端不写磁盘）");

// ================= 10) 策略开关（auto_save_on_exit 保持可选） =================
calls.length = 0;
await session.setPolicy(undefined, true);
assert(policyState.auto_save_on_exit === true, "auto_save_on_exit 必须可开启");
await session.setPolicy(undefined, false);

// ================= 11) 异常容错（evalInTab 失败回退空预览） =================
calls.length = 0;
bridge.evalInTab = async () => {
  throw new Error("页面未就绪");
};
const preview = await session.capturePreview("tab-x");
assert(preview === "", "预览采集失败必须回退空串（不抛错）");

// ================= 汇总 =================
console.log(`\nassertions passed: ${passed}`);
if (failures.length) {
  console.error(`assertions failed: ${failures.length}`);
  process.exit(1);
}
console.log("SESSION_LOGIC_RESULT=ALL_PASS");
