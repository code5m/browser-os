#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-view-intent.mjs — Phase 1 Browser/Grid 单一语义门禁
//
// 守护 Phase 1 冻结的单一语义（真源见
// docs/architecture/semantic-governance/phase1-browser-grid/）：
//
//   ONE STATE → ONE MEANING       mainView / gridOpen 各只承担一个含义
//   ONE INTENT → ONE ENTRYPOINT   组件只调 Intent API，不拼生命周期
//   ONE LIFECYCLE DECISION → ONE OWNER
//   NATIVE VISIBILITY ≠ DOMAIN TRUTH
//   VIEW SWITCH ≠ RESOURCE DESTROY
//   HIDE ≠ CLOSE / CLOSE ≠ SHUTDOWN
//
// 冻结的唯一公式：
//   desiredGridVisibility = gridOpen && mainView === "grid"
//   （mainView==="browser"/"home"/... 时 Grid 资源存活但隐藏）
//
// 用法:
//   node scripts/check-view-intent.mjs              正式门禁（当前仓库）
//   node scripts/check-view-intent.mjs --self-test  自检（好样本 PASS / 坏样本必被抓）
// ---------------------------------------------------------------------------
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const ROOT = process.cwd();
const BROWSER_STORE = "src/capabilities/browser/state/useBrowserStore.ts";
const GRID_STORE = "src/capabilities/grid/state/useGridStore.ts";
const LAYOUT = "src/stores/useLayoutStore.ts";

let failures = 0;
function ok(msg) {
  console.log("  ✓ " + msg);
}
function fail(msg, detail) {
  failures++;
  console.log("  ✗ " + msg + (detail ? " — " + detail : ""));
}

function walk(dir, exts, out) {
  out = out || [];
  if (!existsSync(dir)) return out;
  let names;
  try {
    names = readdirSync(dir);
  } catch {
    return out;
  }
  for (const name of names) {
    if (name === "node_modules" || name === "dist" || name === "target" || name === ".git") continue;
    const p = join(dir, name);
    let st;
    try {
      st = statSync(p);
    } catch {
      continue;
    }
    if (st.isDirectory()) walk(p, exts, out);
    else if (exts.some((e) => name.endsWith(e))) out.push(p);
  }
  return out;
}

function rel(p) {
  return p.startsWith(ROOT) ? p.slice(ROOT.length).replace(/^\//, "") : p;
}

// 抽取 `function name(...) { ... }`（大括号配平），失败返回 null。
function extractFn(src, name) {
  const re = new RegExp("(?:async\\s+)?function\\s+" + name + "\\s*\\(");
  const m = re.exec(src);
  if (!m) return null;
  const start = src.indexOf("{", m.index + m[0].length);
  if (start < 0) return null;
  let depth = 0;
  for (let i = start; i < src.length; i++) {
    if (src[i] === "{") depth++;
    else if (src[i] === "}") {
      depth--;
      if (depth === 0) return src.slice(m.index, i + 1);
    }
  }
  return null;
}

// ============================ 检查项（返回 {pass,msg,detail}） ============================

// C1 组件不得直接调 Grid 生命周期原语
function c1(ctx) {
  const bad = [];
  for (const f of ctx.vueFiles) {
    const m = f.src.match(/\b(buildGrid|closeGridAll|gridCloseOne)\s*\(/g);
    if (m) bad.push(`${f.path}: ${[...new Set(m)].join(",")}`);
  }
  return {
    pass: bad.length === 0,
    msg: "C1 组件不直接调 Grid 生命周期原语（buildGrid/closeGridAll/gridCloseOne）",
    detail: bad.join(" | "),
  };
}

// C2 mainView 无直写（唯一写入点是 useLayoutStore.setView）
function c2(ctx) {
  const bad = [];
  for (const f of ctx.codeFiles) {
    if (f.path === LAYOUT) continue;
    f.src.split("\n").forEach((l, i) => {
      // 排除 `===` 比较（只禁赋值）；同时覆盖 `layout.mainView = v` 这类非字面量直写
      if (/\.mainView\s*=\s*[^=]/.test(l)) bad.push(`${f.path}:${i + 1}`);
    });
  }
  return {
    pass: bad.length === 0,
    msg: "C2 mainView 无直写（仅 useLayoutStore.setView 内部赋值）",
    detail: bad.join(" | "),
  };
}

// C3 视图切换路径禁止销毁资源（VIEW SWITCH ≠ RESOURCE DESTROY / HIDE ≠ CLOSE）
function c3(ctx) {
  const setView = extractFn(ctx.layoutSrc, "setView");
  if (!setView) {
    return { pass: false, msg: "C3 setView 存在", detail: "useLayoutStore.ts 未找到 setView" };
  }
  if (/\bcloseGrid|gridOpen\.value\s*=\s*false|bridge\.closeGrid/.test(setView)) {
    return {
      pass: false,
      msg: "C3 setView 内销毁 Grid 资源（VIEW SWITCH ≠ RESOURCE DESTROY）",
      detail: "setView 含 closeGrid/gridOpen=false/bridge.closeGrid",
    };
  }
  const sync = extractFn(ctx.browserStoreSrc, "syncViewVisibility");
  if (!sync) {
    return { pass: false, msg: "C3 syncViewVisibility 存在", detail: "useBrowserStore.ts 未找到 syncViewVisibility" };
  }
  if (/bridge\.closeGrid/.test(sync)) {
    return { pass: false, msg: "C3 syncViewVisibility 销毁 Grid（应只 hide）", detail: "含 bridge.closeGrid" };
  }
  return { pass: true, msg: "C3 视图切换/显隐同步只 hide 不 destroy（VIEW SWITCH ≠ DESTROY, HIDE ≠ CLOSE）" };
}

// C4 禁止新增 gridVisible 存储状态（visibility 必须是纯派生，不得成为第二真源）
// 覆盖 ref/shallowRef/computed/reactive，含泛型写法，且扫全 src（不只两个 store）。
function c4(ctx) {
  const decl = /\bgridVisible\s*(?:=|:)\s*(?:ref|shallowRef|computed|reactive)\s*(?:<[^>]*>)?\s*\(/;
  const bad = [];
  for (const f of ctx.codeFiles) {
    if (decl.test(f.src)) bad.push(f.path);
  }
  return {
    pass: bad.length === 0,
    msg: "C4 无 gridVisible 存储状态（visibility 为纯派生，非第二真源）",
    detail: bad.join(", "),
  };
}

// C5 desiredGridVisibility 公式已冻结
// 不只要"声明存在"，还要求：computed 实际消费纯派生 helper，且 helper 内为冻结公式。
// （否则把 computed 改成 computed(() => gridOpen.value) 也能骗过旧版检查）
function c5(ctx) {
  const hasComputed = /const\s+desiredGridVisibility\s*=\s*computed\(/.test(ctx.gridStoreSrc);
  const usesHelper =
    /const\s+desiredGridVisibility[\s\S]{0,200}?computeDesiredVisibility/.test(ctx.gridStoreSrc) ||
    /computeDesiredVisibility\([\s\S]{0,160}?\.gridVisible/.test(ctx.gridStoreSrc);
  const formula =
    /gridVisible:\s*go\s*&&\s*mv\s*===\s*["']grid["']/.test(ctx.gridStoreSrc) ||
    /gridVisible:\s*gridOpen\.value\s*&&\s*layout\.mainView\s*===\s*["']grid["']/.test(ctx.gridStoreSrc);
  return {
    pass: hasComputed && usesHelper && formula,
    msg: "C5 desiredGridVisibility = gridOpen && mainView === \"grid\"（公式已冻结且确由纯派生 helper 产出）",
    detail: `computed=${hasComputed} usesHelper=${usesHelper} formula=${formula}`,
  };
}

// C6 closeGrid 销毁资源 + owner 收敛视图 + 状态不变量守卫
function c6(ctx) {
  const cg = extractFn(ctx.gridStoreSrc, "closeGridAll");
  if (!cg) return { pass: false, msg: "C6 closeGridAll 存在", detail: "未找到" };
  if (!/gridOpen\.value\s*=\s*false/.test(cg)) {
    return { pass: false, msg: "C6 closeGrid 确实销毁资源", detail: "缺 gridOpen.value = false" };
  }
  if (!/layout\.(?:mainView\s*=\s*"browser"|setView\(\s*"browser"\s*\))/.test(cg)) {
    return { pass: false, msg: "C6 closeGrid 由 owner 收敛视图", detail: "缺 setView(\"browser\") 复位" };
  }
  if (!/mv\s*===\s*["']grid["']\s*&&\s*!go/.test(ctx.gridStoreSrc)) {
    return { pass: false, msg: "C6 存在状态不变量守卫", detail: "缺 invariant watch（mainView===\"grid\" ⇒ gridOpen）" };
  }
  return { pass: true, msg: "C6 closeGrid 销毁资源 + owner 收敛视图 + 不变量守卫齐备" };
}

// C7 isBrowserVisible 公式（不得重新耦合 gridOpen）
function c7(ctx) {
  const m = ctx.browserStoreSrc.match(/const isBrowserVisible = computed\(\s*\(\)\s*=>\s*([\s\S]*?)\);/);
  if (!m) return { pass: false, msg: "C7 isBrowserVisible 公式存在", detail: "未找到" };
  const f = m[1].trim();
  return {
    pass: /layout\.mainView === "browser"/.test(f) && !/gridOpen/.test(f),
    msg: "C7 isBrowserVisible = mainView === \"browser\"（不含 gridOpen 旧耦合）",
    detail: f,
  };
}

// C8 不得无证据新增 Rust 宫格 show/hide 命令
function c8(ctx) {
  const bad = [];
  for (const f of ctx.rustFiles) {
    const m = f.src.match(/\bfn\s+(show_grid|hide_grid|showGrid|hideGrid)\s*\(/g);
    if (m) bad.push(`${f.path}: ${m.join(",")}`);
  }
  return {
    pass: bad.length === 0,
    msg: "C8 无新增 Rust 宫格 show/hide 命令（复用既有 hideWebview/hideAllWebviews/GridCmd::HideWindow）",
    detail: bad.join(" | "),
  };
}

// C9 canonical Intent API 齐备
function c9(ctx) {
  const need = [
    ["openGrid", ctx.gridStoreSrc],
    ["closeGrid", ctx.gridStoreSrc],
    ["closeGridCell", ctx.gridStoreSrc],
    ["activateGrid", ctx.gridStoreSrc],
    ["activateBrowser", ctx.layoutSrc],
    ["activateHome", ctx.layoutSrc],
    ["activateFiles", ctx.layoutSrc],
    ["activateTerm", ctx.layoutSrc],
  ];
  const missing = need
    .filter(([n, s]) => !new RegExp("function\\s+" + n + "\\s*\\(").test(s))
    .map(([n]) => n);
  return {
    pass: missing.length === 0,
    msg: "C9 canonical Intent API 齐备（openGrid/closeGrid/closeGridCell/activateGrid + activateBrowser/Home/Files/Term）",
    detail: missing.join(", "),
  };
}

// C10 禁止 exitGrid(mode) 之类多义万能 API
function c10(ctx) {
  const bad = [];
  for (const f of ctx.codeFiles) {
    if (/\bexitGrid\s*\(/.test(f.src)) bad.push(f.path);
  }
  return {
    pass: bad.length === 0,
    msg: "C10 无 exitGrid(mode) 多义万能 API（一意图一入口）",
    detail: bad.join(", "),
  };
}

// C11 销毁原语只由 owner 调用（CLOSE 不得散落）
function c11(ctx) {
  const bad = [];
  for (const f of ctx.codeFiles) {
    if (f.path === GRID_STORE) continue;
    if (/bridge\.closeGrid\s*\(/.test(f.src)) bad.push(f.path);
  }
  return {
    pass: bad.length === 0,
    msg: "C11 bridge.closeGrid 只由 owner（useGridStore）调用",
    detail: bad.join(", "),
  };
}

const CHECKS = [c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11];

// ============================ 契约仿真（T1–T12 静态可判部分） ============================
function runSims() {
  console.log("[SIM] 契约仿真：desiredGridVisibility / isBrowserVisible / 状态不变量");
  // 与源码冻结公式同源的纯函数镜像
  const desiredGridVisibility = (gridOpen, mainView) => gridOpen && mainView === "grid";
  const isBrowserVisible = (gridOpen, mainView) => mainView === "browser";

  // T11：gridOpen=true 但 mainView=browser → Grid 必须隐藏（资源存活）
  if (desiredGridVisibility(true, "browser") === false)
    ok("T11 gridOpen=true + mainView='browser' → Grid 隐藏（资源存活，不销毁）");
  else fail("T11 gridOpen=true + mainView='browser' 时 Grid 不应可见");

  if (isBrowserVisible(true, "browser") === true)
    ok("T11 gridOpen=true 时 mainView='browser' → Browser 可见（Grid 存活但隐藏）");
  else fail("T11 gridOpen=true 时 Browser 应可见");

  if (desiredGridVisibility(true, "grid") === true) ok("T11 gridOpen=true + mainView='grid' → Grid 可见");
  else fail("T11 gridOpen=true + mainView='grid' 时 Grid 应可见");

  if (desiredGridVisibility(true, "home") === false)
    ok("T11 mainView='home' → Grid 隐藏（grid→home 不销毁资源）");
  else fail("T11 mainView='home' 时 Grid 不应可见");

  if (desiredGridVisibility(false, "grid") === false) ok("T11 gridOpen=false → Grid 不可见");
  else fail("T11 gridOpen=false 时 Grid 不应可见");

  // T12：不变量收敛规则（与源码 guard 一致：仅 gridOpen 由 true→false 且仍在 grid 时收敛）。
  // 注意"创建中"（gridOpen 恒为 false、mainView 已切 grid）是合法中间态，不参与收敛，
  // 否则首次打开宫格会被弹回 browser —— 此处显式固化该规则，防止回归。
  const converge = (mainView, gridOpen) => (mainView === "grid" && !gridOpen ? "browser" : mainView);
  if (converge("grid", false) === "browser")
    ok("T12 收敛规则：mainView='grid' 且资源已消失 → 收敛为 'browser'");
  else fail("T12 收敛规则不成立", "grid + 无资源 应收敛为 browser");
  if (converge("browser", false) === "browser" && converge("home", true) === "home")
    ok("T12 收敛规则：合法态（browser / home）不被误改");
  else fail("T12 收敛误伤合法视图");
  if (converge("grid", true) === "grid")
    ok("T12 收敛规则：mainView='grid' 且资源存在 → 保持 grid（不误收）");
  else fail("T12 收敛误收合法 grid 态");
}

// 信息性告警（不计入门禁）：canonical Intent API 定义后若零引用，提示为"待接线/冗余"。
function reportApiUsage(ctx) {
  const api = ["openGrid", "rebuildGrid", "closeGrid", "closeGridCell", "activateGrid", "desiredGridVisibility"];
  // 统计 store 之外（组件 / 其它 store）的引用，判断是否真的被接线
  const outside = ctx.codeFiles
    .filter((f) => f.path !== GRID_STORE)
    .map((f) => f.src)
    .join("\n");
  const unused = api.filter((n) => !new RegExp("\\b" + n + "\\b").test(outside));
  if (unused.length) console.log(`  ! WARN 定义但未被引用（Intent API 表面 / 待接线）: ${unused.join(", ")}`);
  else console.log("  · canonical Intent API 均已被引用");
}

function runChecks(ctx, label) {
  console.log(label);
  for (const c of CHECKS) {
    const r = c(ctx);
    if (r.pass) ok(r.msg);
    else fail(r.msg, r.detail);
  }
}

// ============================ 真实仓库 ============================
function buildCtx() {
  const browserStorePath = join(ROOT, BROWSER_STORE);
  const gridStorePath = join(ROOT, GRID_STORE);
  const layoutPath = join(ROOT, LAYOUT);
  const browserStoreSrc = existsSync(browserStorePath) ? readFileSync(browserStorePath, "utf8") : "";
  const gridStoreSrc = existsSync(gridStorePath) ? readFileSync(gridStorePath, "utf8") : "";
  const layoutSrc = existsSync(layoutPath) ? readFileSync(layoutPath, "utf8") : "";
  const vueFiles = walk(join(ROOT, "src"), [".vue"]).map((p) => ({ path: rel(p), src: readFileSync(p, "utf8") }));
  const codeFiles = walk(join(ROOT, "src"), [".ts", ".vue"]).map((p) => ({ path: rel(p), src: readFileSync(p, "utf8") }));
  const rustFiles = walk(join(ROOT, "src-tauri", "src"), [".rs"]).map((p) => ({ path: rel(p), src: readFileSync(p, "utf8") }));
  return { browserStoreSrc, gridStoreSrc, layoutSrc, vueFiles, codeFiles, rustFiles };
}

function runReal() {
  if (!existsSync(join(ROOT, BROWSER_STORE)) || !existsSync(join(ROOT, GRID_STORE)) || !existsSync(join(ROOT, LAYOUT))) {
    fail("源码文件存在", "未找到 useBrowserStore.ts / useGridStore.ts / useLayoutStore.ts（需在仓库根运行）");
    return;
  }
  const ctx = buildCtx();
  runChecks(ctx, "[Phase 1] Browser/Grid 单一语义门禁");
  reportApiUsage(ctx);
  runSims();
  console.log("");
  console.log(failures === 0 ? "VIEW_INTENT_RESULT=PASS" : `VIEW_INTENT_RESULT=FAIL(${failures})`);
  process.exit(failures === 0 ? 0 : 1);
}

// ============================ 自检（双向：好样本 PASS / 坏样本必被抓） ============================
function goodCtx() {
  return {
    browserStoreSrc: `
const isBrowserVisible = computed(() => layout.mainView === "browser");
async function syncViewVisibility() { hideAllWebviews(); }
`,
    gridStoreSrc: `
function computeDesiredVisibility(mv, go) {
  return { browserVisible: mv === "browser", gridVisible: go && mv === "grid" };
}
const desiredGridVisibility = computed(() => computeDesiredVisibility(layout.mainView, gridOpen.value).gridVisible);
async function closeGridAll() {
  gridOpen.value = false;
  if (layout.mainView === "grid") { layout.setView("browser"); }
  await bridge.closeGrid();
}
function openGrid() {}
function closeGrid() {}
function closeGridCell(i) {}
function activateGrid() {}
watch(() => [layout.mainView, gridOpen.value], ([mv, go]) => { if (mv === "grid" && !go) layout.setView("browser"); });
`,
    layoutSrc: `
function setView(v) { mainView.value = v; navSection.value = ""; }
function activateBrowser() { setView("browser"); }
function activateHome() { setView("home"); }
function activateFiles() { setView("files"); }
function activateTerm() { setView("term"); }
`,
    vueFiles: [],
    codeFiles: [],
    rustFiles: [],
  };
}

function runSelfTest() {
  console.log("[self-test] 好样本：全部检查应 PASS");
  const g = goodCtx();
  let badGood = 0;
  for (const c of CHECKS) {
    const r = c(g);
    if (!r.pass) {
      badGood++;
      console.log(`  ✗ 好样本误报: ${r.msg} (${r.detail})`);
    }
  }
  if (badGood === 0) console.log("  ✓ 好样本 11/11 PASS");
  else console.log(`  ✗ 好样本误报 ${badGood} 项`);

  console.log("[self-test] 坏样本：每类违规必须被对应检查抓到");
  const mutations = [
    ["C1 组件直调 buildGrid", (c) => ({ ...c, vueFiles: [{ path: "X.vue", src: "browser.buildGrid();" }] }), 0],
    ["C2 mainView 直写", (c) => ({ ...c, codeFiles: [{ path: "Y.ts", src: 'layout.mainView = "browser";' }] }), 1],
    [
      "C3 setView 内销毁资源",
      (c) => ({ ...c, layoutSrc: c.layoutSrc.replace("function setView(v) { mainView.value = v; navSection.value = \"\"; }", "function setView(v) { mainView.value = v; gridOpen.value = false; }") }),
      2,
    ],
    ["C4 新增 gridVisible 存储态", (c) => ({ ...c, codeFiles: [{ path: "G.ts", src: "const gridVisible = ref(false);" }] }), 3],
    ["C5 公式被改动", (c) => ({ ...c, gridStoreSrc: c.gridStoreSrc.replace('gridVisible: go && mv === "grid"', 'gridVisible: go') }), 4],
    ["C6 closeGrid 不销毁", (c) => ({ ...c, gridStoreSrc: c.gridStoreSrc.replace("gridOpen.value = false;", "") }), 5],
    ["C7 isBrowserVisible 耦合 gridOpen", (c) => ({ ...c, browserStoreSrc: c.browserStoreSrc.replace('computed(() => layout.mainView === "browser")', 'computed(() => layout.mainView === "browser" && !gridOpen.value)') }), 6],
    ["C8 新增 Rust show_grid", (c) => ({ ...c, rustFiles: [{ path: "z.rs", src: "#[tauri::command]\nfn show_grid() {}" }] }), 7],
    ["C9 Intent API 缺失", (c) => ({ ...c, gridStoreSrc: c.gridStoreSrc.replace("function activateGrid() {}", "") }), 8],
    ["C10 exitGrid(mode)", (c) => ({ ...c, codeFiles: [{ path: "Z.ts", src: 'exitGrid("hide");' }] }), 9],
    ["C11 非 owner 调销毁原语", (c) => ({ ...c, codeFiles: [{ path: "W.ts", src: "bridge.closeGrid();" }] }), 10],
  ];

  let missed = 0;
  for (const [label, mutate, idx] of mutations) {
    const bad = mutate(goodCtx());
    const r = CHECKS[idx](bad);
    if (r.pass) {
      missed++;
      console.log(`  ✗ 漏检: ${label}`);
    } else {
      console.log(`  ✓ 抓到: ${label}`);
    }
  }

  console.log("");
  const pass = badGood === 0 && missed === 0;
  console.log(pass ? "SELF_TEST_RESULT=ALL_PASS" : `SELF_TEST_RESULT=FAIL(good=${badGood} missed=${missed})`);
  process.exit(pass ? 0 : 1);
}

const arg = process.argv[2];
if (arg === "--self-test") runSelfTest();
else if (arg === "--help" || arg === "-h") {
  console.log("用法: node scripts/check-view-intent.mjs [--self-test|--help]");
  process.exit(0);
} else runReal();
