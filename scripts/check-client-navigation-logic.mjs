#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5-W17（Lane A6）客户端导航逻辑层自动化测试（headless，无 GUI 依赖）。
//
// 加载**真实**的 src/stores/useLayoutStore.ts（pinia 真实实例，不 mock store 逻辑），
// 并对真实的 src/components/layout/ActivityBar.vue 做源码级断言：
// 因此每条断言反映的都是**产品代码**的行为，而不是测试替身的行为。
//
// 覆盖：
//   1) 导航真源（一级入口 / ☰ 菜单）—— 窄窗口裁剪后入口不丢、无孤儿模块；
//   2) 窗口密度与键盘漫游索引（纯函数，含环绕与非法输入）；
//   3) store 行为 —— 扩展行互斥、视图切换自动收起、宽度钳位；
//   4) ActivityBar 源码 —— 可访问名称/展开语义/键盘钩子/无 raw invoke/无敏感字面量。
//
// 用法: node scripts/check-client-navigation-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

function resolveWithExt(specifier, context, next) {
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
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__clientNavResolve(specifier, context, next);
}`
      )
  );
  globalThis.__clientNavResolve = resolveWithExt;
}

// useLayoutStore.showToast 用 window.setTimeout（浏览器环境），Node 下补最小桩
globalThis.window = { setTimeout: (fn, ms) => setTimeout(fn, ms) };

// 用 fileURLToPath 而非 URL.pathname：仓库路径含中文，pathname 会百分号编码
const ROOT = fileURLToPath(new URL("..", import.meta.url));

const nav = await import(`${ROOT}src/stores/useLayoutStore.ts`);
const { createPinia, setActivePinia } = await import(
  `${ROOT}node_modules/pinia/dist/pinia.mjs`
);
setActivePinia(createPinia());

const {
  useLayoutStore,
  TOP_NAV_ITEMS,
  NAV_MENU_SECTIONS,
  MODULE_META,
  navDensityForWidth,
  navTopViewsForWidth,
  isNavActive,
  nextNavIndex,
} = nav;

const storeSrc = readFileSync(`${ROOT}src/stores/useLayoutStore.ts`, "utf8");
const barSrc = readFileSync(`${ROOT}src/components/layout/ActivityBar.vue`, "utf8");

let failures = 0;
let total = 0;
function check(name, cond) {
  total++;
  if (cond) {
    console.log("  ok   " + name);
  } else {
    console.log("  FAIL " + name);
    failures++;
  }
}

const TOP_VIEWS = TOP_NAV_ITEMS.map((i) => i.view);
const MENU_VIEWS = NAV_MENU_SECTIONS.flatMap((s) => s.items.map((i) => i.view));
// 宫格有独立的常驻按钮（不受窄窗口裁剪），其可达性由源码断言覆盖
const ALWAYS_VIEWS = ["grid"];

console.log("--- 1. 导航真源 ---");
check(
  "一级入口每项都有 view/icon/label",
  TOP_NAV_ITEMS.length > 0 &&
    TOP_NAV_ITEMS.every((i) => !!i.view && !!i.icon && !!i.label)
);
check(
  "一级入口 view 无重复",
  new Set(TOP_VIEWS).size === TOP_VIEWS.length
);
check(
  "☰ 菜单分节均含标题与条目",
  NAV_MENU_SECTIONS.length > 0 &&
    NAV_MENU_SECTIONS.every((s) => !!s.title && s.items.length > 0)
);
check(
  "☰ 菜单条目 view 无重复",
  new Set(MENU_VIEWS).size === MENU_VIEWS.length
);

// 从真实源码解析 MainView 联合类型，避免测试里再抄一份而漂移
const mainViewBlock = /export type MainView =([\s\S]*?);\n/.exec(storeSrc);
const declaredViews = mainViewBlock
  ? [...mainViewBlock[1].matchAll(/"([a-z]+)"/g)].map((m) => m[1])
  : [];
check("源码可解析 MainView 联合类型", declaredViews.length >= 15);

const reachable = new Set([...TOP_VIEWS, ...MENU_VIEWS, ...ALWAYS_VIEWS]);
// editor 是文件编辑器覆盖层，不是导航目标；settings 走 ⚙️ 常驻按钮
const orphans = declaredViews.filter(
  (v) => v !== "editor" && v !== "settings" && !reachable.has(v)
);
check(
  `无孤儿模块（每个 MainView 都有导航入口）${orphans.length ? " 缺失:" + orphans : ""}`,
  orphans.length === 0
);
check(
  "MODULE_META 已导出且含主页/插件/设置",
  !!MODULE_META.home && !!MODULE_META.plugin && !!MODULE_META.settings
);

console.log("--- 2. 窄窗口裁剪不丢功能 ---");
const widths = [1920, 1280, 1180, 1179, 1000, 900, 899, 720, 480, 320];
const lostAt = [];
for (const w of widths) {
  const visible = navTopViewsForWidth(w);
  for (const v of TOP_VIEWS) {
    if (!visible.includes(v) && !MENU_VIEWS.includes(v)) lostAt.push(`${w}px:${v}`);
  }
}
check(
  `任意宽度下被裁掉的一级入口都能在 ☰ 菜单找到${lostAt.length ? " 丢失:" + lostAt : ""}`,
  lostAt.length === 0
);
check("宽窗口全部一级入口可见", navTopViewsForWidth(1280).length === TOP_NAV_ITEMS.length);
check(
  "compact 保留主页/浏览/终端",
  (() => {
    const v = navTopViewsForWidth(1000);
    return v.length === 3 && v.includes("home") && v.includes("browser") && v.includes("term");
  })()
);
check(
  "icon 只保留主页/浏览",
  (() => {
    const v = navTopViewsForWidth(600);
    return v.length === 2 && v.includes("home") && v.includes("browser");
  })()
);
check(
  "裁剪单调：icon ⊆ compact ⊆ full",
  navTopViewsForWidth(600).every((v) => navTopViewsForWidth(1000).includes(v)) &&
    navTopViewsForWidth(1000).every((v) => navTopViewsForWidth(1280).includes(v))
);

console.log("--- 3. 窗口密度 ---");
check("1280 -> full", navDensityForWidth(1280) === "full");
check("1179 -> compact", navDensityForWidth(1179) === "compact");
check("900 -> compact", navDensityForWidth(900) === "compact");
check("899 -> icon", navDensityForWidth(899) === "icon");
check("0/NaN -> icon（不抛异常）", navDensityForWidth(0) === "icon" && navDensityForWidth(NaN) === "icon");

console.log("--- 4. 激活态判定 ---");
check("同名激活", isNavActive("home", "home") === true);
check("异名不激活", isNavActive("home", "browser") === false);
check("覆盖层 editor 不误激活主页", isNavActive("editor", "home") === false);
check("空视图不激活", isNavActive("", "home") === false);

console.log("--- 5. 键盘漫游索引 ---");
check("右移", nextNavIndex(0, 1, 5) === 1);
check("末位右移环绕到首位", nextNavIndex(4, 1, 5) === 0);
check("首位左移环绕到末位", nextNavIndex(0, -1, 5) === 4);
check("单元素不自移越界", nextNavIndex(0, 1, 1) === 0 && nextNavIndex(0, -1, 1) === 0);
check("非法长度返回 0", nextNavIndex(2, 1, 0) === 0 && nextNavIndex(2, 1, -1) === 0);
check("越界索引被归一", nextNavIndex(9, 1, 5) === 0);

console.log("--- 6. store 行为（真实 pinia） ---");
const layout = useLayoutStore();

layout.setWindowWidth(1440);
check("宽度 1440 -> full", layout.navDensity === "full");
layout.setWindowWidth(1000);
check("宽度 1000 -> compact", layout.navDensity === "compact");
layout.setWindowWidth(600);
check("宽度 600 -> icon", layout.navDensity === "icon");
layout.setWindowWidth(10);
check("宽度下限钳位到 320", layout.windowWidth === 320);
layout.setWindowWidth(NaN);
check("非法宽度钳位到 320", layout.windowWidth === 320);
layout.setWindowWidth(99999);
check("宽度上限钳位到 4096", layout.windowWidth === 4096);
check(
  "navTopViews 与宽度同步",
  (() => {
    layout.setWindowWidth(1280);
    const full = layout.navTopViews.length;
    layout.setWindowWidth(600);
    return layout.navTopViews.length < full && layout.navTopViews.includes("home");
  })()
);

layout.toggleNavSection("more");
check("展开 ☰ 菜单", layout.navSection === "more");
layout.toggleNavSection("more");
check("再点同一入口收起", layout.navSection === "");
layout.toggleNavSection("more");
layout.toggleNavSection("grid");
check("扩展行互斥（同时只开一个）", layout.navSection === "grid");
layout.closeNavSection();
check("closeNavSection 收起", layout.navSection === "");

layout.toggleNavSection("more");
layout.setView("tools");
check("切换视图自动收起扩展行", layout.navSection === "" && layout.mainView === "tools");
check("激活态与真实 mainView 一致", isNavActive(layout.mainView, "tools") === true);

layout.openModule("db");
check("openModule 打开数据库模块", layout.mainView === "db" && layout.modTabs.length === 1);
const dbTabId = layout.modTabs[0].id;
layout.closeModTab(dbTabId);
check("关闭最后一个页签回落到主页", layout.mainView === "home" && layout.modTabs.length === 0);

console.log("--- 7. ActivityBar 源码约束 ---");
check("消费 store 导航真源", barSrc.includes("TOP_NAV_ITEMS") && barSrc.includes("NAV_MENU_SECTIONS"));
check("不再本地重复定义一级入口", !barSrc.includes("const topItems = ["));
check("主导航有可访问名称", barSrc.includes('aria-label="主导航"'));
check("导航容器绑定键盘漫游", barSrc.includes("@keydown=\"onNavKeydown\"") && barSrc.includes("data-nav-item"));
check("扩展行触发按钮带 aria-expanded", barSrc.includes("aria-expanded"));
check("扩展行有 aria-controls 关联", barSrc.includes("aria-controls"));
check("当前项带 aria-current", barSrc.includes("aria-current"));
check("Esc 可收起扩展行", barSrc.includes("@keydown.esc"));
check("Esc 后焦点回到触发按钮", barSrc.includes("data-nav-toggle="));
check("宫格常驻按钮存在（不随宽度裁剪）", barSrc.includes("onItem('grid')"));
check("窄窗口密度样式存在", barSrc.includes(".nav-compact") && barSrc.includes(".nav-icon"));
check("展示脱敏（不回显凭据查询串）", barSrc.includes("safeLabel") && barSrc.includes("redactSecrets"));
check("无 raw Tauri invoke", !/\binvoke\(/.test(barSrc) && !barSrc.includes("@tauri-apps"));
check(
  "未引入新依赖（仅相对路径/vue/pinia）",
  [...barSrc.matchAll(/^import[^"']*["']([^"']+)["']/gm)]
    .map((m) => m[1])
    .every((s) => s.startsWith(".") || s === "vue" || s.startsWith("vue/"))
);
check(
  "无敏感字面量",
  !/sk-[A-Za-z0-9]{8,}|AKIA[0-9A-Z]{8,}|Bearer\s+[A-Za-z0-9]{8,}|password=|api_key=/.test(barSrc)
);
check("未触碰 home 组件", !barSrc.includes("components/home") && !barSrc.includes("capabilities/home"));
check("未触碰 MainArea", !barSrc.includes("MainArea"));
check("store 不依赖 bridge/后端", !storeSrc.includes("../../bridge") && !/bridge\./.test(storeSrc));

if (failures) {
  console.log(`CLIENT_NAV_RESULT=FAIL (${failures}/${total})`);
  process.exit(1);
}
console.log(`CLIENT_NAV_RESULT=PASS (${total}/${total})`);
process.exit(0);
