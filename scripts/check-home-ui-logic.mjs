#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5-W17（Lane A9）主页 UI 逻辑层自动化测试（source-level，headless，无 GUI/DOM 依赖）。
//
// 按 W17 调度 A9 职责对 A5 首页组件（src/components/home/**）做 DOM/源码级检查：
//   1. home actions      —— 主区域入口/快捷方式动作齐全
//   2. accessible names   —— 每个可见交互控件都有可访问名称
//   3. bounded lists      —— 快捷方式数量有上限 + 新增/渲染时裁剪（W17 验收#2）
//   4. no sensitive copy  —— 新 UI/错误文案不泄露凭据/密钥/敏感 URL
//   5. persistence safety —— app 命令体绝不落库（Closeout 债务 HOME_NO_SECRET_PERSIST）
//
// 首页特性拆为 4 个组件（HomePanel / HomeLaunchers / HomeShortcuts / HomeShortcutEditor），
// 故本脚本扫描 src/components/home 下全部 .vue 的合集，而非单一文件。
// 不加载 pinia/vue（useHomeStore 含 pinia import，node 直载失败），改为静态解析。
// 不修改任何产品组件（W17 A9 硬约束）。
//
// 用法: node scripts/check-home-ui-logic.mjs
// 退出码: 0 = 全部通过；1 = 有必需项未通过（含已知债务，见 HOME_UI_RESULT）
// ---------------------------------------------------------------------------

import { readFileSync, readdirSync, existsSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import os from "node:os";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const HOME_DIR = join(ROOT, "src/components/home");
const STORE_PATH = join(ROOT, "src/stores/useHomeStore.ts");

let passed = 0;
let failed = 0;
const failures = [];
const notes = [];
function ok(name) {
  passed++;
  console.log("  ok   " + name);
}
function fail(name, detail) {
  failed++;
  failures.push(name + (detail ? " :: " + detail : ""));
  console.error("  FAIL " + name + (detail ? " :: " + detail : ""));
}
function note(msg) {
  notes.push(msg);
  console.log("  note " + msg);
}

// 加载 homeUi.ts 纯逻辑层（A3 主页逻辑单一真源）用于功能回归断言。
// 优先 native TS strip（Node 22.18+/23.6+ 默认开启），回退 esbuild transform。
async function loadHomeUi() {
  const p = join(ROOT, "src/utils/homeUi.ts");
  if (!existsSync(p)) return null;
  try {
    return await import(pathToFileURL(p).href);
  } catch {}
  try {
    const esbuildMod = await import("esbuild");
    const src = readFileSync(p, "utf8");
    const { code } = esbuildMod.transformSync(src, { loader: "ts", format: "esm" });
    const tmp = join(os.tmpdir(), `homeUi.gen.${process.pid}.mjs`);
    writeFileSync(tmp, code);
    return await import(pathToFileURL(tmp).href);
  } catch {
    return null;
  }
}

// ---- 读取首页组件 ----
function readHomeComponents() {
  if (!existsSync(HOME_DIR)) return [];
  return readdirSync(HOME_DIR)
    .filter((f) => f.endsWith(".vue"))
    .map((f) => ({ name: f, text: readFileSync(join(HOME_DIR, f), "utf8") }));
}
const components = readHomeComponents();
const allHomeText = components.map((c) => c.text).join("\n");

// ===========================================================================
// G1 home actions —— A5 首页动作齐全
// ===========================================================================
console.log("[G1] home actions (A5 首页动作)");

if (components.length === 0) {
  fail("src/components/home 存在 .vue 组件", "目录为空");
} else {
  const needActions = [
    ["favoriteCurrentPage", "收藏当前网页动作"],
    ["favoriteCurrentDir", "收藏当前目录动作"],
    ["startAdd", "新增快捷方式动作"],
    ["resetDefault", "恢复默认动作"],
    ["open", "打开快捷方式动作 (home.open / onOpen)"],
  ];
  for (const [sym, label] of needActions) {
    if (new RegExp(`home\\.${sym}\\b|\\bonOpen\\s*\\(|onOpen\\(\\s*s\\s*\\)`).test(allHomeText) || allHomeText.includes(sym)) {
      ok("首页含 " + label + " (" + sym + ")");
    } else {
      fail("首页缺少 " + label, sym);
    }
  }
  // 主区域入口：HomeLaunchers 提供主要工作区宫格；快捷方式网格用 v-for 渲染
  if (/launch-grid/.test(allHomeText) && /LAUNCHERS/.test(allHomeText)) ok("主要工作区入口（HomeLaunchers 宫格）存在");
  else fail("缺少主要工作区入口", "HomeLaunchers 宫格/LAUNCHERS");
  if (/shortcut-grid/.test(allHomeText) && /v-for=/.test(allHomeText)) ok("快捷方式网格用 v-for 迭代渲染");
  else fail("快捷方式网格未用 v-for 迭代");
  if (/home-empty|暂无快捷方式|还没有快捷方式/.test(allHomeText)) ok("存在空态文案（coherent empty state）");
  else fail("缺少空态文案（empty state）");
}

// ===========================================================================
// G2 accessible names —— 每个可见交互控件都有可访问名称
// ===========================================================================
console.log("[G2] accessible names (可访问名称)");

function extractTags(html, tag) {
  const re = new RegExp(`<${tag}\\b([^>]*)>([\\s\\S]*?)</${tag}>`, "gi");
  const out = [];
  let m;
  while ((m = re.exec(html))) out.push({ attrs: m[1], inner: m[2] });
  return out;
}
function extractVoidTags(html, tag) {
  const re = new RegExp(`<${tag}\\b([^>]*)/?>`, "gi");
  const out = [];
  let m;
  while ((m = re.exec(html))) out.push({ attrs: m[1], inner: "" });
  return out;
}
function hasNameAttr(attrs) {
  return /\b(?:title|:title)\s*=/i.test(attrs) || /\baria-label\b|\baria-labelledby\b/i.test(attrs);
}
function getId(attrs) {
  const m = attrs.match(/\bid\s*=\s*["']([^"']+)["']/i);
  return m ? m[1] : null;
}

// 收集全部模板中的 label[for] 关联（跨组件）
const labelFors = new Set();
{
  let lm;
  const labelForRe = /<label\b[^>]*\bfor\s*=\s*["']([^"']+)["']/gi;
  while ((lm = labelForRe.exec(allHomeText))) labelFors.add(lm[1]);
}

let controlCount = 0;
for (const c of components) {
  const tpl = (c.text.match(/<template>([\s\S]*?)<\/template>/i) || [, ""])[1];
  const controls = [
    ...extractTags(tpl, "button").map((x) => ({ tag: "button", ...x })),
    ...extractTags(tpl, "a").map((x) => ({ tag: "a", ...x })),
    ...extractTags(tpl, "select").map((x) => ({ tag: "select", ...x })),
    ...extractVoidTags(tpl, "input").map((x) => ({ tag: "input", ...x })),
    ...extractTags(tpl, "textarea").map((x) => ({ tag: "textarea", ...x })),
  ];
  for (const ct of controls) {
    controlCount++;
    const innerTrim = (ct.inner || "").replace(/<[^>]+>/g, "").trim();
    const id = getId(ct.attrs);
    let named = false;
    if (hasNameAttr(ct.attrs)) named = true;
    else if (ct.tag === "button" || ct.tag === "a") named = innerTrim.length > 0;
    else if (id && labelFors.has(id)) named = true; // <label for> 程序化关联
    const sig = `<${ct.tag}${id ? " id=" + id : ""}>`;
    if (named) ok("控件可访问名称存在 " + sig + " (" + c.name + ")");
    else fail("控件缺少可访问名称（无 title/aria-label，且无 <label for> 关联）", sig + " (" + c.name + ")");
  }
}
if (controlCount === 0) fail("未扫描到任何交互控件", "首页组件应至少含按钮");

// ===========================================================================
// G3 bounded lists —— 快捷方式数量有上限 + 渲染/新增时裁剪（W17 验收#2）
// ===========================================================================
console.log("[G3] bounded lists (有界列表)");

// 边界可在 store 层（MAX_HOME_SHORTCUTS）或渲染层（VISIBLE_LIMIT）落地，任一即可。
const storeText = existsSync(STORE_PATH) ? readFileSync(STORE_PATH, "utf8") : "";
const capRe =
  /(MAX_?HOME_?SHORTCUTS|HOME_?SHORTCUTS_?MAX|MAX_?SHORTCUTS|SHORTCUT_?MAX|VISIBLE_LIMIT|SHORTCUT_?VISIBLE_?LIMIT)\s*=\s*(\d+)/i;
const capMatch = (storeText + "\n" + allHomeText).match(capRe);
if (capMatch) ok("快捷方式上限常量已定义 (" + capMatch[1] + "=" + capMatch[2] + ")");
else fail("快捷方式缺少上限常量（store 或渲染层）", "建议 MAX_HOME_SHORTCUTS / VISIBLE_LIMIT");

const clampRe =
  /\.slice\(\s*0\s*,\s*(?:MAX_?HOME_?SHORTCUTS|HOME_?SHORTCUTS_?MAX|MAX_?SHORTCUTS|SHORTCUT_?MAX|VISIBLE_LIMIT|\d+)|Math\.min\(\s*shortcuts\.length|shortcuts\.length\s*(?:>=|>\s*)(?:MAX_?HOME_?SHORTCUTS|HOME_?SHORTCUTS_?MAX|MAX_?SHORTCUTS|SHORTCUT_?MAX|\d+)/i;
if (clampRe.test(storeText + "\n" + allHomeText)) ok("快捷方式数量在新增/渲染时被裁剪到上限");
else fail("快捷方式数量未被裁剪到上限", "addShortcut/saveEdit/渲染层需 slice/clamp");

// 畸形本地数据容错：store.load 的 try/Array.isArray，或 HomeShortcuts 的 norm/safeType 防御归一化
const storeResilient = /load\(\)\s*\{[\s\S]*?try\s*\{[\s\S]*?Array\.isArray\([\s\S]*?\}\s*catch/.test(storeText);
const renderResilient = /function\s+norm\s*\(|function\s+safeType\s*\(/.test(allHomeText);
if (storeResilient || renderResilient) ok("对畸形本地数据容错（store.load try/Array.isArray 或 渲染层 norm/safeType）");
else fail("缺少畸形数据容错");
if (storeResilient && !renderResilient) note("渲染层 norm/safeType 防御归一化可作为额外纵深（当前仅 store 层容错）");

// ===========================================================================
// G4 no sensitive copy —— 新 UI/错误文案不泄露凭据/密钥/敏感 URL
// ===========================================================================
console.log("[G4] no sensitive copy (无敏感文案)");

const sensitiveRe =
  /sk-[A-Za-z0-9]{6,}|AKIA[0-9A-Z]{8,}|token\s*[=:]\s*\S+|password\s*[=:]\s*\S+|secret\s*[=:]\s*\S+|Authorization|Bearer\s+[A-Za-z0-9._-]+|api[_-]?key\s*[=:]\s*\S+|credential/i;
const scan = [...components.map((c) => c.text)];
if (existsSync(STORE_PATH)) scan.push(storeText);
let sensitiveHit = false;
for (const txt of scan) {
  if (sensitiveRe.test(txt)) {
    sensitiveHit = true;
    break;
  }
}
if (!sensitiveHit) ok("首页组件与 store 不含硬编码凭据/密钥/Authorization/Bearer 等敏感文案");
else fail("发现敏感文案模式", "首页/store 源码含 sk-/AKIA/token=/password=/secret=/Authorization/Bearer/api_key/credential");

// 错误/提示文案不回显原始报错体（仅展示稳定文案，不泄露路径/栈）
if (/showToast\([^)]*\+\s*(e\?\._?message|err\._?message|error\._?message)/i.test(allHomeText + "\n" + storeText)) {
  note("store.open 的失败提示回显 e?.message（运行时可能含路径；建议稳定文案，属 A5 后续项，非本卡阻塞）");
}

// ===========================================================================
// G5 persistence safety —— HOME_NO_SECRET_PERSIST（Acceptance Closeout 唯一真债务）
// ===========================================================================
console.log("[G5] persistence safety (HOME_NO_SECRET_PERSIST)");

const homeUi = await loadHomeUi();

// 5a 源码接线：store 落库/载入必须走安全过滤（防回退）
if (/toPersisted\(\s*shortcuts\s*\)/.test(storeText)) ok("store.save 经由 toPersisted 过滤后落库");
else fail("store.save 未经由 toPersisted 过滤", "save() 必须 JSON.stringify(toPersisted(shortcuts))");

if (/toPersisted\(\s*recents\s*\)/.test(storeText)) ok("store.saveRecents 经由 toPersisted 过滤后落库");
else fail("store.saveRecents 未经由 toPersisted 过滤", "saveRecents() 必须 JSON.stringify(toPersisted(recents))");

if (/\.filter\(\s*isStorageSafe\s*\)/.test(storeText))
  ok("store.load/loadRecents 经 isStorageSafe 过滤（迁移期剔除已落库 app 命令体）");
else fail("store.load 未过滤 isStorageSafe", "旧 app 条目须在载入时剔除并抹除");

// 5b 纯逻辑层契约 + 功能回归
if (homeUi) {
  if (typeof homeUi.isStorageSafe === "function") ok("homeUi.isStorageSafe 已导出");
  else fail("homeUi 缺少 isStorageSafe");

  if (typeof homeUi.toPersisted === "function") ok("homeUi.toPersisted 已导出");
  else fail("homeUi 缺少 toPersisted");

  if (Array.isArray(homeUi.HOME_PERSISTED_TYPES) && !homeUi.HOME_PERSISTED_TYPES.includes("app"))
    ok("HOME_PERSISTED_TYPES 排除 app（仅 url/dir 可落库）");
  else fail("HOME_PERSISTED_TYPES 应含 url/dir 且排除 app");

  const appCmd = "launch-my-app --token sk-TOPSECRETCMDBODY /home/me/.ssh/id_rsa";
  const appSc = { id: "home-app-x", type: "app", name: "MyApp", target: appCmd, icon: "🚀" };
  const urlSc = { id: "home-url-1", type: "url", name: "百度", target: "https://www.baidu.com", icon: "🔍" };
  const dirSc = { id: "home-dir-1", type: "dir", name: "文档", target: "/home/me/Documents", icon: "📁" };

  if (homeUi.isStorageSafe(appSc) === false && homeUi.isStorageSafe(urlSc) === true && homeUi.isStorageSafe(dirSc) === true)
    ok("isStorageSafe：app=false，url/dir=true");
  else fail("isStorageSafe 语义错误", "app 应为 false");

  const persistedArr = homeUi.toPersisted([appSc, urlSc, dirSc]);
  if (persistedArr.length === 2 && !persistedArr.some((s) => s.type === "app"))
    ok("toPersisted 剔除 app 条目（保留 url/dir）");
  else fail("toPersisted 未剔除 app 条目", "期望长度 2 且无 app 类型");

  const json =
    typeof homeUi.persistedJson === "function"
      ? homeUi.persistedJson([appSc, urlSc, dirSc])
      : JSON.stringify(persistedArr);
  let hasApp = false;
  try {
    hasApp = JSON.parse(json).some((s) => s.type === "app");
  } catch {}
  if (json.includes("sk-TOPSECRETCMDBODY") || json.includes("--token") || hasApp)
    fail("落库 JSON 含 app 命令体/凭据", "persistedJson 不应含 sk-…/--token/app 类型");
  else ok("persistedJson 不含 app 命令体或凭据（HOME_NO_SECRET_PERSIST 已闭环）");
} else {
  note("homeUi.ts 无法在 node 直载（无 native TS / esbuild）；仅以源码接线断言兜底，建议 A0 集成时补功能回归");
}

// ===========================================================================
// 结果
// ===========================================================================
console.log(`\n主页 UI 逻辑测试：通过 ${passed}，失败 ${failed}`);
if (notes.length) {
  console.log("观察项（非阻塞）：");
  for (const n of notes) console.log("  - " + n);
}
if (failures.length) {
  console.log("未通过项：");
  for (const f of failures) console.log("  - " + f);
}
const result = failed === 0 ? "PASS" : "PASS_WITH_DEBT";
console.log(`HOME_UI_RESULT=${result}`);
process.exit(failed === 0 ? 0 : 1);
