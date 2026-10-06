#!/usr/bin/env node
// ---------------------------------------------------------------------------
// Phase 6A Core Closure — 语义收敛逻辑层自动化测试（headless）。
//
// 加载**真实** store（pinia 真实实例）验证三项最小语义迁移的功能不回归：
//   6A-1 aiNavOpen 唯一 owner：打开/关闭/读取路径/writer 唯一
//   6A-2 面板边界：bookmark.panelOpen / layout 各面板开关各自独立、bmPanelOpen 为派生
//   6A-3 gridSession 唯一 owner：打开(重建)自增、状态保持(内存,非持久化)、writer 唯一
//
// 同时做源码级静态断言（单一声明 / writer 唯一 / 派生量不存为态），覆盖 negative / false-positive。
// bridge 以宽 Proxy 桩注入（避免引入 Tauri 运行时）。
//
// 用法: node scripts/check-semantic-closure-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------
import * as nodeModule from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// 宽 bridge 桩：任何方法都返回 Promise(undefined)，使 store 生命周期动作可在 headless 下跑通
const BRIDGE_STUB =
  "export const bridge = new Proxy({}, { get: () => async () => undefined });" +
  "export const M0Config = {};";

function resolveWithExt(specifier, context, next) {
  if (specifier === "../bridge" || specifier.endsWith("/bridge")) {
    return { url: "data:text/javascript," + encodeURIComponent(BRIDGE_STUB), shortCircuit: true };
  }
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {
          /* try next */
        }
      }
    }
    throw err;
  }
}
if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," + encodeURIComponent(`export async function resolve(s, c, n){ return globalThis.__scResolve(s,c,n); }`),
  );
  globalThis.__scResolve = resolveWithExt;
}

// 最小浏览器环境桩
globalThis.window = { setTimeout: (fn, ms) => setTimeout(fn, ms), addEventListener: () => {}, __TAURI_INTERNALS__: undefined };
const lsWrites = [];
globalThis.localStorage = {
  _s: {},
  getItem(k) { return Object.prototype.hasOwnProperty.call(this._s, k) ? this._s[k] : null; },
  setItem(k, v) { lsWrites.push([k, String(v)]); this._s[k] = String(v); },
  removeItem(k) { delete this._s[k]; },
};

const ROOT = fileURLToPath(new URL("..", import.meta.url));

let pass = 0, fail = 0;
let useBrowserStore, useGridStore, useLayoutStore, useBookmarkStore;
const failures = [];
function check(name, cond, detail) {
  if (cond) { pass++; console.log(`  ok   ${name}`); }
  else { fail++; failures.push(`${name}${detail ? " — " + detail : ""}`); console.log(`  FAIL ${name}${detail ? " — " + detail : ""}`); }
}

// ---------- 静态断言工具 ----------
function countDecl(src, name) {
  const re = new RegExp("(?:const|let|var)\\s+" + name + "\\s*(?::[^=;]+)?=\\s*(?:ref|shallowRef|reactive)\\s*(?:<[^>]*>)?\\s*\\(", "g");
  return (src.match(re) || []).length;
}
function countWrites(src, name) {
  const re = new RegExp("\\b" + name + "\\.value\\s*(\\+=|=)", "g");
  return (src.match(re) || []).length;
}
// 函数作用域分析（与 check-semantic-registry.mjs R9 同口径）：返回各函数体区间，给出写入点的 enclosing 函数名。
function findFunctionRanges(code) {
  const fns = [];
  const declRe = /(?:async\s+)?function\s+([A-Za-z_$][\w$]*)|(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=;{]*?)?\s*=\s*(?:async\s+)?(?:function\s*)?\(/g;
  let m;
  while ((m = declRe.exec(code))) {
    const name = m[1] || m[2];
    const p = code.indexOf("(", m.index);
    if (p < 0) continue;
    let depth = 0, bodyStart = -1;
    for (let j = p; j < code.length; j++) {
      const c = code[j];
      if (c === "(") depth++;
      else if (c === ")") { depth--; if (depth === 0) { let k = j + 1; while (k < code.length && /\s/.test(code[k])) k++; if (code[k] === "{") bodyStart = k; break; } }
    }
    if (bodyStart < 0) continue;
    let d = 0, bodyEnd = -1;
    for (let j = bodyStart; j < code.length; j++) { const c = code[j]; if (c === "{") d++; else if (c === "}") { d--; if (d === 0) { bodyEnd = j; break; } } }
    if (bodyEnd >= 0) fns.push({ name, start: bodyStart, end: bodyEnd });
  }
  return fns;
}
function enclosingFunction(fns, idx) {
  let best = null;
  for (const fn of fns) if (idx >= fn.start && idx <= fn.end) if (!best || (fn.end - fn.start) < (best.end - best.start)) best = fn;
  return best ? best.name : null;
}

const { resolveOwnerFile } = await import(`${ROOT}scripts/check-semantic-registry.mjs`);
const browserPath = resolveOwnerFile("useBrowserStore");
const layoutPath = resolveOwnerFile("useLayoutStore");
const bookmarkPath = resolveOwnerFile("useBookmarkStore");
const gridPath = resolveOwnerFile("useGridStore");
if (!browserPath || !gridPath || !layoutPath || !bookmarkPath) {
  console.error("closure: owner_implementations 无法解析 store 路径（物理迁移后须同步更新 locator）");
  process.exit(1);
}
const browserSrc = readFileSync(`${ROOT}${browserPath}`, "utf8");
const gridSrc = readFileSync(`${ROOT}${gridPath}`, "utf8");
const layoutSrc = readFileSync(`${ROOT}${layoutPath}`, "utf8");
const mainAreaSrc = readFileSync(`${ROOT}src/components/layout/MainArea.vue`, "utf8");
const bookmarkPanelSrc = readFileSync(`${ROOT}src/capabilities/bookmark/ui/BookmarkPanel.vue`, "utf8");

// ============================ 静态：aiNavOpen 唯一 owner ============================
console.log("[static] aiNavOpen 唯一 owner");
check("aiNavOpen 在 useBrowserStore 中恰好声明 1 次", countDecl(browserSrc, "aiNavOpen") === 1, `decl=${countDecl(browserSrc, "aiNavOpen")}`);
check("useLayoutStore 不再声明 aiNavOpen（删除死重复）", !/aiNavOpen/.test(layoutSrc), "layout 仍含 aiNavOpen");
check("源码无 layout.aiNavOpen 第二真源", !/layout\.aiNavOpen/.test(browserSrc) && !/layout\.aiNavOpen/.test(layoutSrc));

// ============================ 静态：gridSession writer 唯一 + 非持久化 ============================
console.log("[static] gridSession 唯一 owner");
check("gridSession 在 useGridStore 中恰好声明 1 次", countDecl(gridSrc, "gridSession") === 1);
check("useBrowserStore 不再持有 gridSession", !/\bgridSession\b/.test(browserSrc));
check("gridSession 仅由 buildGrid/forceGridRelayout 写入（2 处）", countWrites(gridSrc, "gridSession") === 2, `writes=${countWrites(gridSrc, "gridSession")}`);
// Phase 6B：函数级 writer 唯一（Writer Enforcement）——两处写入均在 Grid canonical_writer 函数体内
{
  const ws = [...gridSrc.matchAll(/\bgridSession\.value\s*(?:\+=|=(?![=>]))/g)];
  const ranges = findFunctionRanges(gridSrc);
  const allCanonical = ws.length === 2 && ws.every((mm) => {
    const enc = enclosingFunction(ranges, mm.index);
    return enc === "buildGrid" || enc === "forceGridRelayout";
  });
  check("gridSession 两处写入均在 canonical_writer 函数（buildGrid / forceGridRelayout）内（Writer 唯一）", allCanonical, `enclosing=${ws.map((mm) => enclosingFunction(ranges, mm.index)).join(",")}`);
}
check("gridSession 不入 localStorage（非持久化）", !/gridSession/.test(JSON.stringify(lsWrites.map(([k]) => k))));

// ============================ 静态：面板开关各自单一声明 ============================
console.log("[static] 面板开关各自单一 owner");
for (const p of ["sidebarOpen", "clipOpen", "fileEditorOpen", "browserDockOpen", "browserDockTab"]) {
  check(`面板 ${p} 在 useLayoutStore 中恰好声明 1 次`, countDecl(layoutSrc, p) === 1, `decl=${countDecl(layoutSrc, p)}`);
}
// 派生面板 bmPanelOpen 必须保持组件 derived（= panelOpen && mainView==="browser"），禁止 store 存为态。
// Phase 8B.1：该派生已下沉到 Bookmark 能力自身 UI（BookmarkPanel.vue），Shell(MainArea) 不再持有该知识。
//   - panelOpen owner 仍是 useBookmarkStore（能力包内读取，owner 未迁移，符合 §5-A 冻结）。
//   - Shell 经通用 Contribution Registry 按 slot 渲染，不直接 import 能力内部 store / ui（C3 关键）。
check("bmPanelOpen 派生量已由 BookmarkPanel 自身承载（能力包内读取 panelOpen，不存为态）",
  /bookmarks\.panelOpen/.test(bookmarkPanelSrc) && /mainView\s*===\s*["']browser["']/.test(bookmarkPanelSrc));
check("MainArea 不再持有 Bookmark 专属知识（无 bookmarks. / useBookmarkStore / 直连能力内部 import）",
  !/bookmarks\./.test(mainAreaSrc) &&
  !/useBookmarkStore/.test(mainAreaSrc) &&
  !/capabilities\/bookmark\//.test(mainAreaSrc));

// ============================ 功能：加载真实 store ============================
console.log("[runtime] 加载真实 store 并验证行为");
let browser, grid, layout, bookmark;
try {
  const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);
  setActivePinia(createPinia());
  ({ useBrowserStore } = await import(`${ROOT}${browserPath}`));
  ({ useGridStore } = await import(`${ROOT}${gridPath}`));
  ({ useLayoutStore } = await import(`${ROOT}${layoutPath}`));
  ({ useBookmarkStore } = await import(`${ROOT}${bookmarkPath}`));
  browser = useBrowserStore();
  grid = useGridStore();
  layout = useLayoutStore();
  bookmark = useBookmarkStore();
  check("store 加载成功", !!browser && !!grid && !!layout && !!bookmark);
} catch (e) {
  check("store 加载成功", false, String(e && e.stack || e));
}

if (browser) {
  // 6A-1 aiNavOpen：打开/关闭/读取/唯一 writer
  check("aiNavOpen 初始关闭", browser.aiNavOpen === false);
  browser.toggleAiNav();
  check("toggleAiNav() 打开", browser.aiNavOpen === true);
  browser.toggleAiNav();
  check("toggleAiNav() 再次关闭", browser.aiNavOpen === false);
  const before = browser.aiNavOpen;
  try { browser.gotoAI({ name: "x", url: "https://example.com", region: "国内" }); } catch { /* openBrowser 触发 bridge 桩，aiNavOpen 已在入口同步置 false */ }
  check("gotoAI 关闭 aiNavOpen（writer 唯一）", browser.aiNavOpen === false);

}

if (grid) {
  // 6A-3 gridSession：Grid owner 内重排自增 + 状态保持(内存)
  const sess0 = grid.gridSession;
  check("gridSession 初始为 0", sess0 === 0);
  grid.forceGridRelayout();
  check("forceGridRelayout 使 gridSession 自增 +1（缓存失效纪元）", grid.gridSession === sess0 + 1, `sess=${grid.gridSession}`);
  const persisted = lsWrites.some(([k]) => /gridSession/.test(k));
  check("gridSession 不落 localStorage（内存 runtime）", !persisted);
}

if (layout && bookmark) {
  // 6A-2 面板各自独立、不误合并
  const sb0 = layout.sidebarOpen;
  layout.toggleSidebar();
  check("toggleSidebar 仅切 sidebarOpen，不动 bookmark", layout.sidebarOpen === !sb0 && bookmark.panelOpen === bookmark.panelOpen);

  const clip0 = layout.clipOpen;
  layout.toggleClipboard();
  check("toggleClipboard 仅切 clipOpen 并导航 clip 视图", layout.clipOpen === !clip0 && layout.mainView === "clip");

  const dock0 = layout.browserDockOpen;
  layout.toggleBrowserDock("files");
  check("toggleBrowserDock 切 browserDockOpen 且子页签=files", layout.browserDockOpen === !dock0 && layout.browserDockTab === "files");

  // bookmark 面板独立，不污染 layout 面板
  const bm0 = bookmark.panelOpen;
  bookmark.togglePanel();
  check("togglePanel 仅切 bookmark.panelOpen", bookmark.panelOpen === !bm0 && layout.clipOpen === layout.clipOpen && layout.sidebarOpen === layout.sidebarOpen);
  // bmPanelOpen 派生：bookmark 开 + 浏览器视图 == 真
  layout.setView("browser");
  check("bmPanelOpen 派生 = panelOpen && mainView==='browser'", bookmark.panelOpen === true && layout.mainView === "browser");
}

console.log("");
if (fail === 0) {
  console.log(`SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (${pass}/${pass})`);
  process.exit(0);
} else {
  console.log(`SEMANTIC_CLOSURE_LOGIC_RESULT=FAIL (${fail} failed, ${pass} passed)`);
  for (const f of failures) console.log("  - " + f);
  process.exit(1);
}
