#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5 BUG-HUNT（Lane A8）宫格关闭回归 · 前端逻辑层自动化测试（source-level，headless）。
//
// 修复 B9-4（P1 空白）：useBrowserStore.closeGridAll 关闭宫格后浏览器区空白。
// 根因：closeGridAll 设 gridOpen=false 但不复位 mainView（仍为 "grid"），
//   → (1) isBrowserVisible = !gridOpen && mainView==="browser" 为 false，
//          BrowserHost 内部把浏览器 webview 设为 visibility:hidden；
//   → (2) useBrowserHost.schedulePosition 在 mainView!=="browser" 时直接 return，
//          活动页签 webview 停在宫格离屏坐标，浏览器区整片空白。
//
// 本测试为 source-level：pinia store 含 pinia/bridge 依赖，node 直载失败
// （A9 主页检查同因改走静态解析），故改为解析 useBrowserStore.ts 源码，
// 断言 B9-4 修复点齐全，并用 isBrowserVisible 公式做非空白契约仿真。
// 不修改任何产品代码。
//
// 用法: node scripts/check-grid-close-logic.mjs
// 退出码: 0 = 全部通过；1 = 有必需项未通过
// ---------------------------------------------------------------------------

import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const STORE_PATH = join(ROOT, "src/stores/useBrowserStore.ts");
const HOST_PATH = join(ROOT, "src/composables/useBrowserHost.ts");

let passed = 0;
let failed = 0;
const failures = [];
function ok(name) {
  passed++;
  console.log("  ok   " + name);
}
function fail(name, detail) {
  failed++;
  failures.push(name + (detail ? " :: " + detail : ""));
  console.error("  FAIL " + name + (detail ? " :: " + detail : ""));
}

// 提取 async function <name>(...) { ... } 的完整函数体（括号匹配，箭头函数 {} 已被平衡）
function extractFn(src, name) {
  const sig = `async function ${name}(`;
  const start = src.indexOf(sig);
  if (start < 0) return null;
  let depth = 0;
  for (let i = src.indexOf("{", start); i < src.length; i++) {
    const ch = src[i];
    if (ch === "{") depth++;
    else if (ch === "}") {
      depth--;
      if (depth === 0) return src.slice(start, i + 1);
    }
  }
  return null;
}

if (!existsSync(STORE_PATH)) {
  fail("useBrowserStore.ts 存在", "找不到 " + STORE_PATH);
  console.log(`\n宫格关闭逻辑测试：通过 ${passed}，失败 ${failed}`);
  console.log(`GRID_CLOSE_RESULT=${failed === 0 ? "PASS" : "FAIL"}`);
  process.exit(1);
}
const storeSrc = readFileSync(STORE_PATH, "utf8");

console.log("[G1] closeGridAll 修复点（B9-4）");
const closeGridAll = extractFn(storeSrc, "closeGridAll");
if (!closeGridAll) {
  fail("closeGridAll 函数存在", "useBrowserStore.ts 未找到 async function closeGridAll");
} else {
  // 1) 核心：关闭宫格时把 mainView 复位为浏览器视图（非空白的关键）
  if (/layout\.mainView\s*=\s*"browser"/.test(closeGridAll))
    ok("closeGridAll 将 mainView 复位为 'browser'（消除空白关键）");
  else fail("closeGridAll 未复位 mainView", "缺少 layout.mainView = \"browser\"");

  // 2) 仅在当前确为 grid 视图时复位，不打扰其它视图（files/term 等）
  if (/layout\.mainView\s*===\s*"grid"/.test(closeGridAll))
    ok("复位受 mainView==='grid' 守卫（不误改其它视图）");
  else fail("复位缺少 'grid' 守卫", "缺少 if (layout.mainView === 'grid')");

  // 3) 重激活活动页签（后端聚焦），确保关闭后它是可见页签
  if (/bridge\.tabActivate\(\s*activeTabId\.value/.test(closeGridAll))
    ok("closeGridAll 重激活活动页签（bridge.tabActivate(activeTabId)）");
  else fail("closeGridAll 未重激活活动页签", "缺少 bridge.tabActivate(activeTabId.value)");

  // 4) 兜底重定位活动页签 webview（与 watch 触发互补，真正下发 tabPosition）
  if (/schedulePosition\(/.test(closeGridAll))
    ok("closeGridAll 调用 schedulePosition 重定位活动页签 webview");
  else fail("closeGridAll 未重定位活动页签", "缺少 schedulePosition() 调用");

  // 5) 保留宫格进程边界：仍走 bridge.closeGrid()（不自行重建/另起网格进程）
  if (/bridge\.closeGrid\(/.test(closeGridAll))
    ok("保留宫格进程边界（仍调用 bridge.closeGrid）");
  else fail("closeGridAll 丢失宫格进程清理", "缺少 bridge.closeGrid()");

  // 6) 保留既有清理：gridOpen 翻位 + gridRects 清空
  if (/gridOpen\.value\s*=\s*false/.test(closeGridAll))
    ok("保留 gridOpen=false 标志位翻位");
  else fail("closeGridAll 未翻 gridOpen 标志位", "缺少 gridOpen.value = false");
  if (/gridRects\.splice\(\s*0\s*,\s*gridRects\.length\s*\)/.test(closeGridAll))
    ok("保留 gridRects 清空");
  else fail("closeGridAll 未清空 gridRects", "缺少 gridRects.splice(0, gridRects.length)");
}

console.log("[G2] isBrowserVisible 公式（非空白判定真源）");
const visM = storeSrc.match(/const isBrowserVisible = computed\(\s*\(\)\s*=>\s*([\s\S]*?)\);/);
if (!visM) {
  fail("isBrowserVisible 公式存在", "useBrowserStore.ts 未找到 isBrowserVisible computed");
} else {
  const formula = visM[1].trim();
  if (/!gridOpen\.value/.test(formula) && /layout\.mainView === "browser"/.test(formula))
    ok("isBrowserVisible = !gridOpen && mainView==='browser'（公式正确）");
  else fail("isBrowserVisible 公式不符预期", formula);

  // 非空白契约仿真：与公式严格一致
  const isBrowserVisible = (gridOpen, mainView) => !gridOpen && mainView === "browser";
  // 修复后：gridOpen=false + mainView=browser → 可见（不空白）
  if (isBrowserVisible(false, "browser") === true)
    ok("契约仿真：修复后状态 isBrowserVisible=true（浏览器区不空白）");
  else fail("契约仿真：修复后仍空白", "gridOpen=false,mainView=browser 应可见");
  // 复现 B9-4：若 mainView 仍为 grid → 空白（证明看门狗能抓回退）
  if (isBrowserVisible(false, "grid") === false)
    ok("契约仿真：mainView 仍为 'grid' 时 isBrowserVisible=false（B9-4 复现，回归可检测）");
  else fail("契约仿真：无法复现 B9-4 空白", "gridOpen=false,mainView=grid 应为空白");
}

console.log("[G3] schedulePosition 依赖 mainView==='browser'（复位必要性）");
if (!existsSync(HOST_PATH)) {
  fail("useBrowserHost.ts 存在", "找不到 " + HOST_PATH);
} else {
  const hostSrc = readFileSync(HOST_PATH, "utf8");
  if (/mainView !== "browser"/.test(hostSrc))
    ok("schedulePosition 仅在 mainView==='browser' 时下发定位（故复位 mainView 必要且充分）");
  else fail("未找到 schedulePosition 的 mainView 依赖", "useBrowserHost.schedulePosition 应 early-return 当 mainView!=='browser'");
  // 关闭宫格后必须触发重定位：mainView 的 watch → syncViewVisibility → relocate
  if (/watch\(\s*\(\) => layout\.mainView/.test(storeSrc))
    ok("mainView 变更 watch 存在（复位 mainView 会触发 relocate→schedulePosition）");
  else fail("缺少 mainView watch", "store 应有 watch(() => layout.mainView) 联动重定位");
}

console.log(`\n宫格关闭逻辑测试：通过 ${passed}，失败 ${failed}`);
if (failures.length) {
  console.log("未通过项：");
  for (const f of failures) console.log("  - " + f);
}
const result = failed === 0 ? "PASS" : "FAIL";
console.log(`GRID_CLOSE_RESULT=${result}`);
process.exit(failed === 0 ? 0 : 1);
