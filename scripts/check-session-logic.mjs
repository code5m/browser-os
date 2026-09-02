#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M1-9 会话存档与关闭协议前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/stores/useSessionStore.ts` 与 `src/stores/useBrowserStore.ts`，
// 只把 `src/bridge.ts` 的会话/页签相关方法替换为记录型 mock（不 mock store 自身逻辑）：
// 因此下面每一条断言反映的都是**产品代码**的行为，而非测试替身的行为。
//
// 覆盖：关闭协议三分支（保存→落盘+关闭 / 删除→丢弃+关闭 / 取消→什么都不做）、
// 拦截器不可绕过、保存/删除/恢复/导出、策略开关、异常容错。
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
const { useSessionStore } = await import(`${ROOT}src/stores/useSessionStore.ts`);
const { useBrowserStore } = await import(`${ROOT}src/stores/useBrowserStore.ts`);
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

// ================= 1) 关闭协议：requestClose 判定 =================
browser.tabs.push({ id: "tab-1", url: "https://a", title: "页面A" });
browser.tabs.push({ id: "tab-2", url: "https://b", title: "页面B" });

assert(session.requestClose("tab-1") === true, "close_prompt 开启时 requestClose 必须接管");
assert(session.closeDialogOpen === true, "接管后弹窗必须打开");
assert(session.pendingCloseTabId === "tab-1", "待决定 tab 必须是 tab-1");

// ================= 2) 取消分支：什么都不做 =================
await session.resolveClose("cancel");
assert(session.closeDialogOpen === false, "取消后弹窗必须关闭");
assert(!hasCall("sessionSave") && !hasCall("sessionDiscard") && !hasCall("tabClose"),
  "取消分支不得保存/丢弃/关闭");
assert(browser.tabs.find((t) => t.id === "tab-1"), "取消后 tab-1 必须仍在");

// ================= 3) 保存分支：采集预览 → 落盘 → 关闭 =================
calls.length = 0;
session.requestClose("tab-1");
await session.resolveClose("save");
const saveCall = findCall("sessionSave");
assert(saveCall && saveCall[1] === "tab-1", "保存分支必须调用 sessionSave");
assert(saveCall && saveCall[2] === "页面正文片段", "保存分支必须带采集到的预览文本");
assert(findCall("evalInTab") && findCall("evalInTab")[1] === "tab-1", "预览必须经 evalInTab 采集");
assert(hasCall("tabClose"), "保存后必须真正关闭页签");
const order = callOrder();
assert(
  order.indexOf("sessionSave") < order.indexOf("tabClose"),
  `必须先落盘再关闭: ${order}`
);
assert(!browser.tabs.find((t) => t.id === "tab-1"), "保存关闭后 tab-1 必须移除");
assert(session.sessions.length === 1 && session.sessions[0].tab_id === "tab-1",
  "保存后历史会话列表必须刷新");

// ================= 4) 删除分支：丢弃草稿 → 关闭（不落盘） =================
calls.length = 0;
session.requestClose("tab-2");
await session.resolveClose("discard");
assert(hasCall("sessionDiscard") && findCall("sessionDiscard")[1] === "tab-2",
  "删除分支必须调用 sessionDiscard");
assert(!hasCall("sessionSave"), "删除分支不得落盘");
assert(hasCall("tabClose"), "删除后必须真正关闭页签");
assert(!browser.tabs.find((t) => t.id === "tab-2"), "删除关闭后 tab-2 必须移除");

// ================= 5) 拦截器不可绕过：tabClose 统一走协议 =================
calls.length = 0;
browser.tabs.push({ id: "tab-3", url: "https://c", title: "页面C" });
browser.bindCloseInterceptor((id) => session.requestClose(id));
await browser.tabClose("tab-3");
assert(session.pendingCloseTabId === "tab-3", "tabClose 必须被协议拦截");
assert(!hasCall("tabClose"), "协议未决前不得直接关闭");

// 协议关闭时（close_prompt=false）直关
await session.setPolicy(false, undefined);
session.requestClose("tab-3"); // 应返回 false
assert(session.requestClose("tab-3") === false, "close_prompt 关闭时 requestClose 必须放行");
calls.length = 0;
await browser.tabClose("tab-3");
assert(hasCall("tabClose"), "close_prompt 关闭时必须直接关闭");
await session.setPolicy(true, undefined);

// ================= 6) 保存当前页签（面板入口） =================
calls.length = 0;
browser.tabs.push({ id: "tab-9", url: "https://d", title: "页面D" });
const saved = await session.saveTab("tab-9", "预览X");
assert(saved && saved.tab_id === "tab-9", "saveTab 必须返回保存的会话摘要");
assert(session.sessions.some((s) => s.tab_id === "tab-9"), "saveTab 后列表必须包含新会话");

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

// ================= 10) 策略开关 =================
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
