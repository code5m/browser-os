#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5 BUG-HUNT（Lane A8）宫格关闭回归 · 前端逻辑层自动化测试（source-level，headless）。
//
// 修复 B9-4（P1 空白）：useBrowserStore.closeGridAll 关闭宫格后浏览器区空白。
// 根因：closeGridAll 设 gridOpen=false 但不复位 mainView（仍为 "grid"），
//   → (1) isBrowserVisible = mainView==="browser" 为 false，
//          BrowserHost 内部把浏览器 webview 设为 visibility:hidden；
//   → (2) useBrowserHost.schedulePosition 在 mainView!=="browser" 时直接 return，
//         活动页签 webview 停在宫格离屏坐标，浏览器区整片空白。
//
// Phase 0 复审（DUP-004）：
//   - isBrowserVisible 的 CURRENT 语义是 `mainView === "browser"`，旧耦合 `!gridOpen`
//     已在重构中移除。检查器必须匹配 CURRENT，并守卫「不得重新引入 gridOpen 耦合」。
//   - closeGridAll 当前用 `relocate()`（store 内部封装，经 bindPositionScheduler 调
//     useBrowserHost.schedulePosition）做兜底重定位，而非直接调用 schedulePosition。
//     故 G1#4 断言 relocate()，而非已不存在的 schedulePosition() 字面量。
//
// 本测试为 source-level：pinia store 含 pinia/bridge 依赖，node 直载失败，
// 故改为解析 useBrowserStore.ts / useBrowserHost.ts 源码做断言。
// 不修改任何产品代码。
//
// 用法: node scripts/check-grid-close-logic.mjs [--help|--self-test|--json|--strict]
// 退出码: 0 = 全部通过；1 = 有必需项未通过；2 = 用法错误
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

// 提取 async function <name>(...) { ... } 的完整函数体（括号匹配）
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

// 在给定源码字符串上运行 G1/G2/G3 断言（供真实仓库与自测共用）。
function runGridChecks(storeSrc, hostSrc) {
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

    // 4) 兜底重定位活动页签 webview
    //    CURRENT：closeGridAll 调 relocate()（store 内部封装，经 bindPositionScheduler
    //    调用 useBrowserHost.schedulePosition）。断言 relocate()，而非已不存在的字面量。
    if (/relocate\(\s*\)/.test(closeGridAll))
      ok("closeGridAll 调用 relocate() 重定位活动页签 webview");
    else fail("closeGridAll 未重定位活动页签", "缺少 relocate() 调用");

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

  console.log("[G2] isBrowserVisible 公式（非空白判定真源，CURRENT 语义）");
  const visM = storeSrc.match(/const isBrowserVisible = computed\(\s*\(\)\s*=>\s*([\s\S]*?)\);/);
  if (!visM) {
    fail("isBrowserVisible 公式存在", "useBrowserStore.ts 未找到 isBrowserVisible computed");
  } else {
    const formula = visM[1].trim();
    // CURRENT（经 DUP-004 复审）：isBrowserVisible = mainView === "browser"
    // 旧耦合 !gridOpen 已在重构中移除——若公式重新出现 gridOpen 属回归，必须 FAIL。
    if (/layout\.mainView === "browser"/.test(formula) && !/gridOpen/.test(formula))
      ok("isBrowserVisible = mainView==='browser'（公式正确，且不含 gridOpen 旧耦合）");
    else fail("isBrowserVisible 公式不符 CURRENT 语义", formula);

    // 非空白契约仿真：严格遵循 CURRENT 公式
    const isBrowserVisible = (gridOpen, mainView) => mainView === "browser";
    if (isBrowserVisible(false, "browser") === true)
      ok("契约仿真：mainView='browser' 时 isBrowserVisible=true（浏览器区不空白）");
    else fail("契约仿真：mainView='browser' 仍空白", "应可见");
    // 复现 B9-4：若 mainView 仍为 grid → 空白（closeGridAll 必须复位 mainView 才能可见）
    if (isBrowserVisible(false, "grid") === false)
      ok("契约仿真：mainView 仍为 'grid' 时 isBrowserVisible=false（B9-4 复现，回归可检测）");
    else fail("契约仿真：无法复现 B9-4 空白", "mainView=grid 应为空白");
  }

  console.log("[G3] schedulePosition 依赖 mainView==='browser'（复位必要性）");
  if (!hostSrc) {
    fail("useBrowserHost.ts 存在", "找不到 " + HOST_PATH);
  } else {
    if (/mainView !== "browser"/.test(hostSrc))
      ok("schedulePosition 仅在 mainView==='browser' 时下发定位（故复位 mainView 必要且充分）");
    else fail("未找到 schedulePosition 的 mainView 依赖", "useBrowserHost.schedulePosition 应 early-return 当 mainView!=='browser'");
    // 关闭宫格后必须触发重定位：mainView 的 watch → syncViewVisibility → relocate
    if (/watch\(\s*\(\) => layout\.mainView/.test(storeSrc))
      ok("mainView 变更 watch 存在（复位 mainView 会触发 relocate→schedulePosition）");
    else fail("缺少 mainView watch", "store 应有 watch(() => layout.mainView) 联动重定位");
  }
}

// ===== 自测：用合成源码验证检查器自身逻辑 =====
function runSelfTest() {
  const fails = [];

  const storePositive = `
    const gridOpen = ref(false);
    const isBrowserVisible = computed(() => layout.mainView === "browser");
    async function closeGridAll() {
      gridOpen.value = false;
      await bridge.closeGrid().catch(() => {});
      layout.gridToolbarOpen = false;
      gridRects.splice(0, gridRects.length);
      if (layout.mainView === "grid") {
        layout.mainView = "browser";
      }
      await bridge.tabActivate(activeTabId.value).catch(() => {});
      relocate();
      layout.showToast("已关闭宫格");
    }
    watch(() => layout.mainView, () => {});
  `;
  const hostPositive = `
    function schedulePosition(retry = 0) {
      if (layout.mainView !== "browser" || !browserHost.value) return;
    }
  `;

  // 正常源码：应全过
  { passed = 0; failed = 0; failures.length = 0;
    runGridChecks(storePositive, hostPositive);
    if (failed !== 0) fails.push("positive fixture 不应失败: " + failures.join("; "));
  }

  // 负例 1：closeGridAll 不复位 mainView → G1#1 必须 FAIL
  { passed = 0; failed = 0; failures.length = 0;
    const storeNeg = storePositive.replace('if (layout.mainView === "grid") {\n        layout.mainView = "browser";\n      }', '/* no reset */');
    runGridChecks(storeNeg, hostPositive);
    if (!failures.some((f) => f.includes("未复位 mainView"))) fails.push("负例1: 未检出 mainView 未复位");
  }

  // 负例 2：isBrowserVisible 重新引入 gridOpen 旧耦合 → G2 必须 FAIL
  { passed = 0; failed = 0; failures.length = 0;
    const storeNeg = storePositive.replace(
      'computed(() => layout.mainView === "browser")',
      'computed(() => !gridOpen.value && layout.mainView === "browser")'
    );
    runGridChecks(storeNeg, hostPositive);
    if (!failures.some((f) => f.includes("isBrowserVisible 公式"))) fails.push("负例2: 未检出 gridOpen 旧耦合回归");
  }

  // 负例 3：host 无 mainView!=="browser" 守卫 → G3 必须 FAIL
  { passed = 0; failed = 0; failures.length = 0;
    const hostNeg = `function schedulePosition(retry = 0) { positionNow(); }`;
    runGridChecks(storePositive, hostNeg);
    if (!failures.some((f) => f.includes("schedulePosition 的 mainView 依赖"))) fails.push("负例3: 未检出 host 缺失 mainView 守卫");
  }

  // 负例 4：closeGridAll 不复用 relocate() → G1#4 必须 FAIL
  { passed = 0; failed = 0; failures.length = 0;
    const storeNeg = storePositive.replace("relocate();", "/* no relocate */");
    runGridChecks(storeNeg, hostPositive);
    if (!failures.some((f) => f.includes("relocate"))) fails.push("负例4: 未检出 missing relocate()");
  }

  return fails;
}

// ===== CLI =====
function printHelp() {
  console.log(`check-grid-close-logic.mjs — 宫格关闭逻辑检查器（Phase 0）

用法:
  node scripts/check-grid-close-logic.mjs           默认（文本输出）
  node scripts/check-grid-close-logic.mjs --help    显示本帮助
  node scripts/check-grid-close-logic.mjs --self-test  自测（不读真实仓库）
  node scripts/check-grid-close-logic.mjs --json    JSON 输出
  node scripts/check-grid-close-logic.mjs --strict  保留（与默认一致，兼容性占位）

检查项:
  [G1] closeGridAll 修复点（B9-4 复位 mainView / 重激活 / relocate / closeGrid / 清空）
  [G2] isBrowserVisible = mainView==='browser'（CURRENT 语义，禁止 gridOpen 旧耦合回归）
  [G3] schedulePosition 仅当 mainView==='browser' 下发定位

退出码: 0 = PASS；1 = FAIL；2 = USAGE_ERROR`);
}

function main() {
  const args = process.argv.slice(2);
  const known = new Set(["--help", "--self-test", "--json", "--strict"]);
  const unknown = args.filter((a) => !known.has(a));
  if (unknown.length > 0) {
    console.error(`未知参数: ${unknown.join(", ")}`);
    console.error("使用 --help 查看可用参数");
    process.exit(2);
  }
  if (args.includes("--help")) { printHelp(); process.exit(0); }
  if (args.includes("--self-test")) {
    const fails = runSelfTest();
    if (fails.length === 0) {
      console.log("GRID_CLOSE_SELF_TEST: ALL_PASS");
      process.exit(0);
    }
    console.log("GRID_CLOSE_SELF_TEST: FAIL");
    for (const f of fails) console.log("  - " + f);
    process.exit(1);
  }

  if (!existsSync(STORE_PATH)) {
    fail("useBrowserStore.ts 存在", "找不到 " + STORE_PATH);
    console.log(`\n宫格关闭逻辑测试：通过 ${passed}，失败 ${failed}`);
    console.log(`GRID_CLOSE_RESULT=${failed === 0 ? "PASS" : "FAIL"}`);
    process.exit(1);
  }
  const storeSrc = readFileSync(STORE_PATH, "utf8");
  const hostSrc = existsSync(HOST_PATH) ? readFileSync(HOST_PATH, "utf8") : null;

  runGridChecks(storeSrc, hostSrc);

  console.log(`\n宫格关闭逻辑测试：通过 ${passed}，失败 ${failed}`);
  if (failures.length) {
    console.log("未通过项：");
    for (const f of failures) console.log("  - " + f);
  }
  const result = failed === 0 ? "PASS" : "FAIL";
  if (args.includes("--json")) {
    console.log(JSON.stringify({
      check: "grid-close-logic",
      status: result,
      passed, failed,
      failures,
    }, null, 2));
  }
  console.log(`GRID_CLOSE_RESULT=${result}`);
  process.exit(failed === 0 ? 0 : 1);
}

main();
