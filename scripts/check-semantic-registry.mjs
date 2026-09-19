#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-semantic-registry.mjs — Semantic Gate（Phase 1.5）
//
// 目的：机器防止语义退化（重复状态 / 重复 Intent / Owner 越界 / 副作用误判）。
// 真源是 docs/architecture/semantic-registry/*.yaml —— 本脚本不硬编码语义，
// 只实现"如何依据 registry 判定"。新增/修改语义应改 YAML（并走 SCR），而非改本脚本。
//
// 判定手段（非纯 regex）：
//   1. 自带 YAML 子集解析器（缩进 map / list / 标量），真正读取 registry 结构
//   2. 注释剥离（引号感知），避免注释/字符串里的词被当成代码
//   3. 作用域分析：Rule 2 只在 registry 声明的 governed_files 内生效
//   4. 声明形态识别：区分"存储声明"与"对象字段名"，避免派生量误报
//   5. 副作用认知窗口：调用点附近查找 awareness marker 注释
//
// 规则：
//   R1 SEMANTIC_DUPLICATE_STATE     重复状态（如 gridVisible 存储态）
//   R2 SEMANTIC_UNREGISTERED_STATE  治理域内新增状态未登记
//   R3 SEMANTIC_OWNER_VIOLATION     Owner 越界（组件/非 owner 直调生命周期）
//   R4 SEMANTIC_INTENT_DUPLICATE    重复 Intent（含已否决的 exitGrid）
//   R5 SEMANTIC_SIDE_EFFECT_UNKNOWN 调用带副作用 API 但未声明认知（默认提示，--strict 失败）
//
// 用法: node scripts/check-semantic-registry.mjs [--help|--self-test|--json|--strict]
// ---------------------------------------------------------------------------
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const REG = join(ROOT, "docs/architecture/semantic-registry");

// ============================ YAML 子集解析 ============================
// 支持：缩进 map、`key: value`、行列表 `- item`、`#` 注释、引号标量。
function parseYaml(text) {
  const items = [];
  text.split(/\r?\n/).forEach((raw, i) => {
    if (!raw.trim()) return;
    if (/^\s*#/.test(raw)) return;
    const indent = raw.match(/^\s*/)[0].length;
    let content = raw.trim();
    // 去行尾注释（仅当 # 前有空白且不在引号内）
    const q = content.indexOf("#");
    if (q > 0 && /[\s]/.test(content[q - 1] || "")) {
      const before = content.slice(0, q);
      if ((before.match(/"/g) || []).length % 2 === 0) content = before.trim();
    }
    items.push({ indent, content, line: i + 1 });
  });
  let pos = 0;
  const scalar = (s) => {
    s = String(s).trim();
    if ((s.startsWith('"') && s.endsWith('"')) || (s.startsWith("'") && s.endsWith("'"))) s = s.slice(1, -1);
    if (s === "true") return true;
    if (s === "false") return false;
    if (s === "null" || s === "~" || s === "") return null;
    if (/^-?\d+(\.\d+)?$/.test(s)) return Number(s);
    return s;
  };
  function parseList(indent) {
    const out = [];
    while (pos < items.length && items[pos].indent === indent && items[pos].content.startsWith("- ")) {
      out.push(scalar(items[pos].content.slice(2)));
      pos++;
    }
    return out;
  }
  function parseNode(indent) {
    if (pos >= items.length) return null;
    if (items[pos].indent < indent) return null;
    if (items[pos].content.startsWith("- ")) return parseList(items[pos].indent);
    return parseMap(items[pos].indent);
  }
  function parseMap(indent) {
    const out = {};
    while (pos < items.length && items[pos].indent === indent && !items[pos].content.startsWith("- ")) {
      const m = items[pos].content.match(/^([^:]+):\s*(.*)$/);
      if (!m) { pos++; continue; }
      const key = m[1].trim();
      const val = m[2].trim();
      pos++;
      if (val === "") {
        out[key] = pos < items.length && items[pos].indent > indent ? parseNode(indent) : null;
      } else {
        out[key] = scalar(val);
      }
    }
    return out;
  }
  return parseMap(items.length ? items[0].indent : 0);
}

function loadRegistry() {
  const read = (f) => (existsSync(join(REG, f)) ? parseYaml(readFileSync(join(REG, f), "utf8")) : {});
  return {
    states: read("states.yaml"),
    intents: read("intents.yaml"),
    owners: read("owners.yaml"),
    sideEffects: read("side-effects.yaml"),
  };
}

// ============================ 文件收集 / 预处理 ============================
function walk(dir, exts, out = []) {
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    if (["node_modules", "dist", "target", ".git"].includes(name)) continue;
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, exts, out);
    else if (exts.some((e) => name.endsWith(e))) out.push(p);
  }
  return out;
}
const rel = (p) => (p.startsWith(ROOT) ? p.slice(ROOT.length).replace(/^\//, "") : p);

// 引号感知地移除 // 行注释与 /* */ 块注释，避免把注释里的词当代码。
function stripComments(src) {
  let out = "";
  let i = 0;
  let inS = null; // ' " `
  while (i < src.length) {
    const c = src[i];
    const n = src[i + 1];
    if (inS) {
      out += c;
      if (c === "\\") { out += n || ""; i += 2; continue; }
      if (c === inS) inS = null;
      i++; continue;
    }
    if (c === '"' || c === "'" || c === "`") { inS = c; out += c; i++; continue; }
    if (c === "/" && n === "/") { while (i < src.length && src[i] !== "\n") i++; continue; }
    if (c === "/" && n === "*") { i += 2; while (i < src.length && !(src[i] === "*" && src[i + 1] === "/")) i++; i += 2; continue; }
    out += c; i++;
  }
  return out;
}

// YAML 内联 `[]` 会被本解析器读成字符串 "[]"，统一归一化为数组
const asList = (v) => (Array.isArray(v) ? v : v == null || v === "[]" ? [] : [String(v)]);

// 存储形态声明：const X = ref( / computed( / reactive( / shallowRef(
const DECL = (name) =>
  new RegExp("(?:const|let|var)\\s+" + name + "\\s*(?::\\s*[^=;]+)?=\\s*(?:ref|shallowRef|computed|reactive)\\s*(?:<[^>]*>)?\\s*\\(", "g");

// ============================ 规则 ============================
function rule1(files, reg) {
  const out = [];
  const states = reg.states.states || {};
  for (const f of files) {
    const code = stripComments(f.src);
    for (const [key, def] of Object.entries(states)) {
      for (const dup of asList(def.duplicate_names)) {
        const name = String(dup);
        if (new RegExp(DECL(name).source).test(code)) {
          out.push({ rule: "R1", code: "SEMANTIC_DUPLICATE_STATE", file: f.path, detail: `"${name}" 被声明为存储态，重复已注册状态 "${key}"（派生量禁止存储）`, severity: "fail" });
        }
        if (new RegExp("\\b" + name + "\\.value\\s*=").test(code)) {
          out.push({ rule: "R1", code: "SEMANTIC_DUPLICATE_STATE", file: f.path, detail: `"${name}.value =" 写入派生量，重复 "${key}"`, severity: "fail" });
        }
      }
    }
  }
  return out;
}

function rule2(files, reg) {
  const out = [];
  const states = reg.states.states || {};
  const governed = asList(reg.states.governed_files);
  const observed = new Set();
  for (const arr of Object.values(reg.states.observed_not_governed || {})) for (const n of asList(arr)) observed.add(String(n));
  for (const f of files) {
    if (!governed.some((g) => f.path === String(g))) continue;
    const code = stripComments(f.src);
    const re = /(?:const|let|var)\s+([A-Za-z_]\w*)\s*(?::\s*[^=;]+)?=\s*(?:ref|shallowRef|computed|reactive)\s*(?:<[^>]*>)?\s*\(/g;
    let m;
    while ((m = re.exec(code))) {
      const name = m[1];
      if (Object.prototype.hasOwnProperty.call(states, name)) continue;
      if (observed.has(name)) {
        out.push({ rule: "R2", code: "SEMANTIC_STATE_NOT_GOVERNED", file: f.path, detail: `"${name}" 已登记但未治理（observed_not_governed），纳入治理需走 SCR`, severity: "info" });
      } else {
        out.push({ rule: "R2", code: "SEMANTIC_UNREGISTERED_STATE", file: f.path, detail: `"${name}" 在治理文件内但未登记于 states.yaml`, severity: "fail" });
      }
    }
  }
  return out;
}

function rule3(files, reg) {
  const out = [];
  const owners = reg.owners.owners || {};
  const lifecycle = owners.browser_grid_lifecycle || {};
  const api = asList(lifecycle.owner_only_api);
  const authorized = asList(lifecycle.authorized_callers);
  const isAuthorized = (path) =>
    authorized.some((a) => {
      const n = String(a).split("（")[0].trim();
      return path.includes(n + ".ts") || path.includes(n + ".vue") || path.includes(n);
    });
  const primitives = ["buildGrid", "closeGridAll", "closeGridOne"];
  for (const f of files) {
    const code = stripComments(f.src);
    const isVue = f.path.endsWith(".vue");
    const isOwner = f.path.includes("useBrowserStore.ts");
    // a) 组件直写 mainView
    if (isVue && /\.mainView\s*=\s*[^=]/.test(code))
      out.push({ rule: "R3", code: "SEMANTIC_OWNER_VIOLATION", file: f.path, detail: "组件直写 mainView（owner: useLayoutStore，唯一写入口 setView）", severity: "fail" });
    // b) 组件直调生命周期原语
    for (const p of primitives)
      if (isVue && new RegExp("\\b" + p + "\\s*\\(").test(code))
        out.push({ rule: "R3", code: "SEMANTIC_OWNER_VIOLATION", file: f.path, detail: `组件直调生命周期原语 ${p}()（owner: useBrowserStore）`, severity: "fail" });
    // c) 非 owner / 非授权文件调生命周期 intent
    if (!isOwner && !isAuthorized(f.path))
      for (const a of api)
        if (new RegExp("\\b" + a + "\\s*\\(").test(code))
          out.push({ rule: "R3", code: "SEMANTIC_OWNER_VIOLATION", file: f.path, detail: `非授权文件调用 owner-only API ${a}()（owner: useBrowserStore）`, severity: "fail" });
    // d) 非 owner 调 bridge.closeGrid（native destroy）
    if (!isOwner && /bridge\.closeGrid\s*\(/.test(code))
      out.push({ rule: "R3", code: "SEMANTIC_OWNER_VIOLATION", file: f.path, detail: "非 owner 调用 bridge.closeGrid（native destroy 仅 useBrowserStore 可调）", severity: "fail" });

    // --- Phase 5.1-A：Credential Owner 通用化（registry 驱动，不削弱既有 Browser 检查）---
    const cred = owners.credential;
    if (cred) {
      const credApi = asList(cred.owner_only_api); // save_token / get_token / delete_token
      const isFrontend = isVue || f.path.includes("/stores/") || f.path.includes("/composables/");
      if (isFrontend) {
        for (const m of credApi) {
          if (new RegExp("\\b" + m + "\\s*\\(").test(code))
            out.push({ rule: "R3", code: "SEMANTIC_OWNER_VIOLATION", file: f.path, detail: `前端直接调用凭据原语 ${m}()（owner: ${String(cred.owner).split("（")[0]}；凭据值不得进入前端，须经 bridge 意图入口）`, severity: "fail" });
        }
        if (/\bKeyringStore\s*[.(]/.test(code))
          out.push({ rule: "R3", code: "SEMANTIC_OWNER_VIOLATION", file: f.path, detail: "前端直接引用 KeyringStore（凭据真源=系统密钥库，组件不得触碰；须经 bridge 意图入口）", severity: "fail" });
      }
    }
  }
  return out;
}

function rule4(files, reg) {
  const out = [];
  const intents = reg.intents.intents || {};
  for (const f of files) {
    const code = stripComments(f.src);
    for (const [key, def] of Object.entries(intents)) {
      for (const dup of asList(def.duplicate_names)) {
        const name = String(dup);
        const defined = new RegExp("(?:async\\s+)?function\\s+" + name + "\\s*\\(").test(code) ||
          new RegExp("const\\s+" + name + "\\s*=\\s*(?:async\\s*)?\\(").test(code) ||
          new RegExp("const\\s+" + name + "\\s*=\\s*(?:async\\s+)?function").test(code);
        if (defined)
          out.push({ rule: "R4", code: "SEMANTIC_INTENT_DUPLICATE", file: f.path, detail: `定义 "${name}()"，与已注册 intent "${key}" 语义重复（一个意图一个入口）`, severity: "fail" });
      }
    }
    const rejected = reg.intents.rejected_intents || {};
    for (const [name, def] of Object.entries(rejected)) {
      if (new RegExp("\\b" + name + "\\s*\\(").test(code))
        out.push({ rule: "R4", code: "SEMANTIC_INTENT_DUPLICATE", file: f.path, detail: `出现已否决 intent "${name}()"：${def.reason || "已被 ADR 否决"}`, severity: "fail" });
    }
  }
  return out;
}

function rule5(files, reg) {
  const out = [];
  const marker = String((reg.sideEffects && reg.sideEffects.awareness_marker) || "side-effect");
  const win = Number((reg.sideEffects && reg.sideEffects.awareness_window_lines) || 6);
  const ses = (reg.sideEffects && reg.sideEffects.side_effects) || {};
  for (const f of files) {
    const code = stripComments(f.src);
    const rawLines = f.src.split(/\r?\n/);
    for (const [key, def] of Object.entries(ses)) {
      if (!def.requires_declaration) continue;
      const cs = String(def.call_sites || key);
      // 若 call_sites 是限定形式（bridge.closeGrid）则按"对象.方法("精确匹配，
      // 避免把 intent 调用（browser.closeGrid）误判成 native 调用。
      let method = cs;
      let re;
      if (cs.includes(".")) {
        const parts = cs.split(".");
        method = parts[parts.length - 1];
        re = new RegExp("\\b" + parts[parts.length - 2] + "\\." + method + "\\s*\\(", "g");
      } else {
        re = new RegExp("\\b" + cs + "\\s*\\(", "g");
      }
      let m;
      while ((m = re.exec(code))) {
        const lineIdx = code.slice(0, m.index).split(/\r?\n/).length - 1;
        let aware = false;
        for (let i = Math.max(0, lineIdx - win); i <= Math.min(rawLines.length - 1, lineIdx + win); i++) {
          if (rawLines[i].includes(marker)) { aware = true; break; }
        }
        if (!aware)
          out.push({ rule: "R5", code: "SEMANTIC_SIDE_EFFECT_UNKNOWN", file: f.path, line: lineIdx + 1, detail: `调用 ${method}() 未声明副作用认知：${def.warning || ""}（在附近注释写 "${marker}: ..." 可消除）`, severity: "warn" });
      }
    }
  }
  return out;
}

// ============================ R6：派生状态禁止存储 ============================
// 任何 registry 中 derived: true 的状态（Phase 1: desiredGridVisibility / isBrowserVisible；
// Phase 2: currentLocalPath）禁止：a) 被声明为 ref/reactive/shallowRef 存储态；b) 被 .value = 赋值。
// 直接固化“派生量禁止成为第二真源”的红线（类比 ADR-P1A-3）。
function rule6(files, reg) {
  const out = [];
  const states = (reg.states && reg.states.states) || {};
  const derived = Object.entries(states).filter(([, d]) => d && d.derived === true);
  for (const f of files) {
    const code = stripComments(f.src);
    for (const [key] of derived) {
      const declRe = new RegExp(
        "(?:const|let|var)\\s+" + key + "\\s*(?::[^=;]+)?=\\s*(?:ref|shallowRef|reactive)\\s*(?:<[^>]*>)?\\s*\\("
      );
      if (declRe.test(code))
        out.push({ rule: "R6", code: "SEMANTIC_DERIVED_STATE_STORED", file: f.path, detail: `"${key}" 在 registry 标记为 derived（禁止存储），却被声明为 ref/reactive/shallowRef 存储态`, severity: "fail" });
      if (new RegExp("\\b" + key + "\\.value\\s*=").test(code))
        out.push({ rule: "R6", code: "SEMANTIC_DERIVED_STATE_STORED", file: f.path, detail: `"${key}" 是派生状态，禁止写入 .value =（第二真源）`, severity: "fail" });
    }
  }
  return out;
}

// ============================ R7：敏感输入不得泄露 ============================
// 依据 states.yaml 中 sensitive: true 的状态（如 databaseCredentialInput，frontend_ident: password）。
// 任何前端文件若把该敏感标识符流入泄露汇（console.log/error/warn/debug/info、
// localStorage/sessionStorage.setItem、或 export 出原始值）即阻断。
// 允许：db.connect(password.value)（安全凭据流）、password.value=""（清空）、redactSecrets(password)（脱敏）。
function rule7(files, reg) {
  const out = [];
  const states = (reg.states && reg.states.states) || {};
  const idents = [];
  for (const [, d] of Object.entries(states)) {
    if (d && d.sensitive === true) idents.push(...asList(d.frontend_ident).map(String));
  }
  if (!idents.length) return out;
  for (const f of files) {
    if (!f.path.endsWith(".ts") && !f.path.endsWith(".vue")) continue;
    const code = stripComments(f.src);
    const rawLines = f.src.split(/\r?\n/);
    for (const ident of idents) {
      const word = new RegExp("\\b" + ident + "\\b");
      if (!word.test(code)) continue;
      for (const line of rawLines) {
        const s = stripComments(line);
        if (!word.test(s)) continue;
        const isConsole = /\bconsole\.(?:log|error|warn|debug|info)\s*\(/.test(s);
        const isStorage = /(?:\blocalStorage|\bsessionStorage)\s*(?:\.\s*setItem\s*\(|\[)/.test(s);
        const isExport = /\bexport\b\s+(?:const|let|var)\s+\w+\s*=\s*[^;]*\b/.test(s) && word.test(s);
        if (!isConsole && !isStorage && !isExport) continue;
        if (/db\.connect\s*\(|redactSecrets\s*\(|password\.value\s*=\s*""/.test(s)) continue;
        out.push({ rule: "R7", code: "SENSITIVE_INPUT_LEAK", file: f.path, detail: `敏感输入「${ident}」流入泄露汇（console/localStorage/export），违反 sensitiveInput 契约（须走安全凭据流 db.connect）`, severity: "fail" });
        break;
      }
    }
  }
  return out;
}

const RULES = [rule1, rule2, rule3, rule4, rule5, rule6, rule7];

function analyze(files, reg) {
  const findings = [];
  for (const r of RULES) findings.push(...r(files, reg));
  return findings;
}

// ============================ CLI ============================
function printHelp() {
  console.log(`check-semantic-registry.mjs — Semantic Gate (Phase 1.5)

用法:
  node scripts/check-semantic-registry.mjs              扫描真实仓库
  node scripts/check-semantic-registry.mjs --self-test   自检（positive/negative/false-positive fixtures）
  node scripts/check-semantic-registry.mjs --json        JSON 输出
  node scripts/check-semantic-registry.mjs --strict      提示级(R5)也判失败
  node scripts/check-semantic-registry.mjs --help

规则:
  R1 SEMANTIC_DUPLICATE_STATE      重复状态（派生量被存成第二真源）
  R2 SEMANTIC_UNREGISTERED_STATE   治理域内新增状态未登记
  R3 SEMANTIC_OWNER_VIOLATION      Owner 越界（组件/非授权直调生命周期）
  R4 SEMANTIC_INTENT_DUPLICATE     重复 Intent（含已否决 exitGrid）
  R5 SEMANTIC_SIDE_EFFECT_UNKNOWN  调用带副作用 API 未声明认知（默认提示）
  R6 SEMANTIC_DERIVED_STATE_STORED 派生状态（derived:true）被存为 ref/reactive 或被 .value= 赋值（第二真源）
  R7 SENSITIVE_INPUT_LEAK        敏感输入（sensitive:true 状态）流入 console/localStorage/export 泄露汇（须走安全凭据流 db.connect）

真源: docs/architecture/semantic-registry/{states,intents,owners,side-effects}.yaml`);
}

function loadRealFiles() {
  return walk(join(ROOT, "src"), [".ts", ".vue"]).map((p) => ({ path: rel(p), src: readFileSync(p, "utf8") }));
}

// ---------------------------- fixtures ----------------------------
// false-positive fixtures：合法代码，绝不能报错（专门验证"不误报"）
const FP_FILES = [
  {
    path: "src/stores/useBrowserStore.ts", // 派生量作为对象字段名（合法，非存储）
    src: `
function computeDesiredVisibility(mv: string, go: boolean) {
  return { browserVisible: mv === "browser", gridVisible: go && mv === "grid" };
}
const desiredGridVisibility = computed(() => computeDesiredVisibility(layout.mainView, gridOpen.value).gridVisible);
// gridVisible is a derived field name, not stored state
async function rebuildGrid() { /* side-effect: destroy+create webviews */ await bridge.gridPosition(0, r); }
`,
  },
  { path: "src/stores/useLayoutStore.ts", src: `function toggleGridToolbar() { browser.openGrid(); browser.closeGrid(); }` },
  { path: "src/components/x/Other.vue", src: `const activeSurface = ref("browser"); // 治理域外，不算未登记` },
  { path: "src/composables/useBrowserHost.ts", src: `function scheduleGrid(){ bridge.gridPosition(i, rect); } // side-effect: bounds+show` },
  { path: "src/stores/__fx_ws_derived.ts", src: `const currentLocalPath = computed(() => inlineFile.value || filePath.value);` },
  { path: "src/stores/__fx_bm_derived.ts", src: `const sorted = computed(() => items.value.slice());` },
  { path: "src/components/workspace/DatabasePanel.vue", src: `const password = ref(""); await db.connect(password.value); password.value = "";` },
];
const POS_FILES = [
  {
    path: "src/stores/useBrowserStore.ts",
    src: `
const gridOpen = ref(false);
const desiredGridVisibility = computed(() => gridOpen.value && layout.mainView === "grid");
function openGrid() { if (!gridOpen.value) return buildGrid(); }
async function closeGridAll() { gridOpen.value = false; await bridge.closeGrid(); } // side-effect: destroy webviews
`,
  },
  { path: "src/components/x/A.vue", src: `layout.activateBrowser(); browser.activateGrid();` },
  { path: "src/components/browser/CredentialList.vue", src: `bridge.fillBrowserCredential(credentialId, tabId); bridge.listBrowserCredentials();` },
];
// 每条 negative fixture 声明期望级别：R5 是提示级(warn)，其余是阻断级(fail)
const NEG_FILES = {
  R1: { expect: "fail", files: [{ path: "src/stores/useBrowserStore.ts", src: `const gridVisible = ref(false);` }] },
  R2: { expect: "fail", files: [{ path: "src/stores/useBrowserStore.ts", src: `const activeSurface = ref("browser");` }, { path: "src/stores/useBookmarkStore.ts", src: `const extraBookmarks = ref<Bookmark[]>([]);` }, { path: "src/stores/useSystemStore.ts", src: `const extraPanes = ref<{id:string;cwd?:string}[]>([]);` }] },
  R3: { expect: "fail", files: [{ path: "src/components/x/B.vue", src: `browser.closeGrid(); layout.mainView = "browser";` }, { path: "src/components/browser/BadCred.vue", src: `KeyringStore.save_token("repo", token);` }] },
  R4: { expect: "fail", files: [{ path: "src/stores/useBrowserStore.ts", src: `function showGridView() { setView("grid"); }` }, { path: "src/composables/__fx_term.ts", src: `function newTerm(){ return 0; }` }, { path: "src/composables/__fx_cred.ts", src: `function exposePassword(){ return readKeyring(); }` }] },
  R7: { expect: "fail", files: [{ path: "src/components/workspace/Leak.vue", src: `const password = ref(""); console.log("pw", password.value);` }] },
  R5: { expect: "warn", files: [{ path: "src/composables/useBrowserHost.ts", src: `bridge.tabPosition(id, rect);` }] },
  R6: { expect: "fail", files: [{ path: "src/stores/__fx_ws_derived_bad.ts", src: `const currentLocalPath = ref("");\ncurrentLocalPath.value = "/x";` }] },
};

function runSelfTest() {
  const reg = loadRegistry();
  let bad = 0;
  // 阻断信号 = fail + warn（info 仅为"已登记未治理"提示，不算问题）
  const blocking = (fs) => analyze(fs, reg).filter((x) => x.severity !== "info");

  const pos = blocking(POS_FILES);
  if (pos.length) { bad++; console.log("  ✗ positive fixture 应 0 fail/warn，实际:", pos.map((p) => p.code + ":" + p.detail).join(" | ")); }
  else console.log("  ✓ positive fixture: 0 fail/warn（合法语义不报错）");

  for (const [r, spec] of Object.entries(NEG_FILES)) {
    const f = analyze(spec.files, reg).filter((x) => x.severity === spec.expect && x.rule === r);
    if (!f.length) { bad++; console.log(`  ✗ negative fixture ${r} 未被检出（期望 ${spec.expect}）`); }
    else console.log(`  ✓ negative fixture ${r}: 检出 ${f[0].code} (${spec.expect})`);
  }

  const fp = blocking(FP_FILES);
  if (fp.length) { bad++; console.log("  ✗ false-positive fixture 误报:", fp.map((p) => p.rule + ":" + p.detail).join(" | ")); }
  else console.log("  ✓ false-positive fixture: 0 fail/warn（派生字段名/注释/治理域外/授权调用均不误报）");

  console.log("");
  console.log(bad === 0 ? "SELF_TEST_RESULT=ALL_PASS" : `SELF_TEST_RESULT=FAIL(${bad})`);
  process.exit(bad === 0 ? 0 : 1);
}

function main() {
  const args = process.argv.slice(2);
  const known = new Set(["--help", "--self-test", "--json", "--strict"]);
  const unknown = args.filter((a) => !known.has(a));
  if (unknown.length) { console.error("未知参数: " + unknown.join(",")); process.exit(2); }
  if (args.includes("--help")) { printHelp(); process.exit(0); }
  if (args.includes("--self-test")) return runSelfTest();

  if (!existsSync(REG)) { console.error("缺少 registry: " + REG); process.exit(1); }
  const reg = loadRegistry();
  const files = loadRealFiles();
  const findings = analyze(files, reg);
  const strict = args.includes("--strict");
  const fails = findings.filter((f) => f.severity === "fail");
  const warns = findings.filter((f) => f.severity === "warn");
  const infos = findings.filter((f) => f.severity === "info");
  const blocking = strict ? [...fails, ...warns] : fails;

  if (args.includes("--json")) {
    console.log(JSON.stringify({ check: "semantic-registry", rules: RULES.length, files_scanned: files.length, fails: fails.length, warns: warns.length, infos: infos.length, status: blocking.length ? "FAIL" : "PASS", findings }, null, 2));
  } else {
    console.log("=== Semantic Registry Gate ===");
    console.log(`files scanned: ${files.length} | rules: ${RULES.length}`);
    for (const f of findings) console.log(`  [${f.severity.toUpperCase()}] ${f.code} ${f.file}${f.line ? ":" + f.line : ""} — ${f.detail}`);
    console.log(`\nfail=${fails.length} warn=${warns.length} info=${infos.length}${strict ? " (strict: warn 计入失败)" : ""}`);
    console.log(`SEMANTIC_REGISTRY_RESULT=${blocking.length ? "FAIL" : "PASS"}`);
  }
  process.exit(blocking.length ? 1 : 0);
}

main();
