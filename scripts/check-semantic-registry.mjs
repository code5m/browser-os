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
import { join, dirname, resolve } from "node:path";
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

function rule2(files, reg, loc) {
  const out = [];
  const states = reg.states.states || {};
  const governedExtra = asList(reg.states.governed_files);
  const governedSet = new Set([...governedExtra, ...Object.values((loc && loc.resolved) || {})]);
  const observed = new Set();
  for (const arr of Object.values(reg.states.observed_not_governed || {})) for (const n of asList(arr)) observed.add(String(n));
  for (const f of files) {
    if (!governedSet.has(f.path)) continue;
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

function rule3(files, reg, loc) {
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
    const isOwner = f.path === (loc.resolved && loc.resolved.useBrowserStore);
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
      const isFrontend = isVue || /\/(stores|composables)\//.test(f.path) || /\/capabilities\/[^\/]+\/(state|services|intents|adapters|lifecycle)\//.test(f.path);
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
      // 仅当该派生量被声明为独立存储态（ref/reactive/shallowRef）时才构成“第二真源”违规；
      // 写穿式 computed({get,set}) 的 .value = 落到真源（如 document.connId），不产生第二真源，属合法，不判。
      const declaredStored = declRe.test(code);
      if (declaredStored)
        out.push({ rule: "R6", code: "SEMANTIC_DERIVED_STATE_STORED", file: f.path, detail: `"${key}" 在 registry 标记为 derived（禁止存储），却被声明为 ref/reactive/shallowRef 存储态`, severity: "fail" });
      if (declaredStored && new RegExp("\\b" + key + "\\.value\\s*=").test(code))
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

// ============================ R8：受治理状态唯一 owner（防第二真源）========================
// 每个受治理「存储态」（derived !== true）必须在 owner 对应的 store 文件中声明一次；
// 若在任何非 owner store 文件中被声明为 ref/reactive/shallowRef，即破坏唯一真源（R8）。
// 另：派生面板（如 bmPanelOpen）严禁在 store 中声明为存储态，必须留在组件 computed。
function rule8(files, reg, loc) {
  const out = [];
  const states = (reg.states && reg.states.states) || {};
  const resolved = (loc && loc.resolved) || {};
  // 存储声明（ref/shallowRef/reactive；**不含 computed** —— computed 正是派生的正确形态）
  const STORED_DECL = (name) =>
    new RegExp("(?:const|let|var)\\s+" + name + "\\s*(?::\\s*[^=;]+)?=\\s*(?:ref|shallowRef|reactive)\\s*(?:<[^>]*>)?\\s*\\(");
  // store-like 文件：/stores/ 或 capability 的 state/services/intents 目录（物理迁移后也能识别）
  const isStoreLikeFile = (p) =>
    /\/(stores)\//.test(p) || /\/capabilities\/[^\/]+\/(state|services|intents)\//.test(p);
  // 派生面板：必须保持组件 computed（= panelOpen && mainView==="browser"），禁止在 store 中存为态
  const DERIVED_ONLY = ["bmPanelOpen"];
  for (const f of files) {
    if (f.path !== resolved.useBookmarkStore) continue; // 派生面板只在该 owner 文件内检查
    const code = stripComments(f.src);
    for (const name of DERIVED_ONLY) {
      if (STORED_DECL(name).test(code))
        out.push({ rule: "R8", code: "SEMANTIC_DERIVED_PANEL_STORED", file: f.path, detail: `"${name}" 必须保持为组件 computed 派生量（= panelOpen && mainView==='browser'），禁止在 store 中声明为存储态（第二真源）`, severity: "fail" });
    }
  }
  for (const [key, def] of Object.entries(states)) {
    if (!def || def.derived === true) continue; // 只查存储态；派生态由 R6 守护
    if (def.single_owner_required !== true) continue; // 仅对显式要求唯一 owner 的状态强制（避免 generic 名域内复用误报）
    const owner = String(def.owner || "");
    const ownerFile = resolved[owner];
    if (!ownerFile) continue; // credential 等 Rust 侧 owner 跳过文件判定
    const re = STORED_DECL(key); // stateless（无 g 标志）
    const outside = files.filter((f) => isStoreLikeFile(f.path) && f.path !== ownerFile && re.test(stripComments(f.src))).map((f) => f.path);
    if (outside.length)
      out.push({ rule: "R8", code: "SEMANTIC_STATE_MULTI_OWNER", file: outside.join(", "), detail: `治理状态 "${key}"（owner=${owner}）在 owner 文件 ${ownerFile} 之外被声明：${outside.join(", ")} —— 破坏唯一真源`, severity: "fail" });
  }
  return out;
}

// ============================ R9：受治理状态唯一写者（Writer Enforcement）========================
// Phase 6B — 把“Owner 唯一”提升为“Owner 唯一 + Writer 唯一 + Checker 可证明”。
// 仅对 registry 中 `single_owner_required === true` 且声明了 `canonical_writer` 的**存储态**强制：
//   a) owner 文件内：写入必须发生在 canonical_writer 列出的函数体内；函数外或错误函数内写入 = FAIL。
//   b) 非 owner 文件（含组件 / 其它 store / composable）：任何 `state.value =` 直写 = FAIL。
// 区分读取与写入：写入 = `.value` 后接 `=` / `+=`；读取（`.value` 后非赋值，如 `===` / 取值）不误报。
// 真源：states.yaml 的 canonical_writer / forbidden_writers / owner；不硬编码 allow-list（遵守 §6 原则）。
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
      else if (c === ")") {
        depth--;
        if (depth === 0) {
          let k = j + 1;
          while (k < code.length && /\s/.test(code[k])) k++;
          if (code[k] === "{") bodyStart = k;
          break;
        }
      }
    }
    if (bodyStart < 0) continue;
    let d = 0, bodyEnd = -1;
    for (let j = bodyStart; j < code.length; j++) {
      const c = code[j];
      if (c === "{") d++;
      else if (c === "}") {
        d--;
        if (d === 0) { bodyEnd = j; break; }
      }
    }
    if (bodyEnd >= 0) fns.push({ name, start: bodyStart, end: bodyEnd });
  }
  return fns;
}
function enclosingFunction(fns, idx) {
  let best = null;
  for (const fn of fns) {
    if (idx >= fn.start && idx <= fn.end) {
      if (!best || (fn.end - fn.start) < (best.end - best.start)) best = fn;
    }
  }
  return best ? best.name : null;
}

function rule9(files, reg, loc) {
  const out = [];
  const states = (reg.states && reg.states.states) || {};
  const resolved = (loc && loc.resolved) || {};
  for (const [key, def] of Object.entries(states)) {
    if (!def) continue;
    if (def.single_owner_required !== true) continue; // 仅对显式要求唯一 owner（Phase 6B 范围）的状态强制
    if (def.derived === true) continue;               // 派生态由 R6 守护，不在此重复
    const cw = asList(def.canonical_writer).map(String).map((s) => String(s).split(".").pop());
    if (!cw.length) continue;                          // 未声明 canonical_writer 则跳过（避免误报，不硬编码）
    const owner = String(def.owner || "");
    const ownerFile = resolved[owner];
    if (!ownerFile) continue;                          // credential 等 Rust 侧 owner 无文件映射，跳过
    const NAME = key;
    const writeRe = new RegExp("\\b" + NAME + "\\.value\\s*(?:\\+=|=(?![=>]))", "g");
    for (const f of files) {
      const code = stripComments(f.src);
      const fns = findFunctionRanges(code);
      let m;
      while ((m = writeRe.exec(code))) {
        const idx = m.index;
        const lineIdx = code.slice(0, idx).split("\n").length - 1;
        const isOwnerFile = f.path === ownerFile;
        if (isOwnerFile) {
          const enc = enclosingFunction(fns, idx);
          if (!enc) {
            out.push({ rule: "R9", code: "SEMANTIC_STATE_WRITER_VIOLATION", file: f.path, line: lineIdx + 1, detail: `"${NAME}"（owner=${owner}）在 owner 文件 ${ownerFile} 顶层（函数外）被写入，必须位于 canonical_writer 函数之一（${cw.join(" / ")}）`, severity: "fail" });
          } else if (!cw.includes(enc)) {
            out.push({ rule: "R9", code: "SEMANTIC_STATE_WRITER_VIOLATION", file: f.path, line: lineIdx + 1, detail: `"${NAME}" 在 owner 文件 ${ownerFile} 内由非 canonical 函数 "${enc}" 写入（允许：${cw.join(" / ")}）`, severity: "fail" });
          }
        } else {
          out.push({ rule: "R9", code: "SEMANTIC_STATE_WRITER_VIOLATION", file: f.path, line: lineIdx + 1, detail: `"${NAME}"（owner=${owner}）在非 owner 文件 ${f.path} 被直接写入 .value=（违反 canonical_writer：${cw.join(" / ")}）；仅 owner 文件内的 canonical_writer 函数可写`, severity: "fail" });
        }
      }
    }
  }
  return out;
}

const RULES = [rule1, rule2, rule3, rule4, rule5, rule6, rule7, rule8, rule9];

// owner 符号 → 首个磁盘存在的候选路径；无命中 = UNRESOLVED，多命中 = DUPLICATE
// 这是 Phase 8A.1 的核心：物理路径可以迁移，但 loader 必须仍能解析到实现，否则 FAIL（防静默失守）。
function resolveOwnerFiles(reg, existingPaths) {
  const impls = (reg.states && reg.states.owner_implementations) || {};
  const resolved = {};
  const errors = [];
  for (const [symbol, def] of Object.entries(impls)) {
    const paths = asList(def && def.paths != null ? def.paths : def);
    const hits = paths.filter((p) => existingPaths.has(String(p)));
    if (hits.length === 0) errors.push({ symbol, kind: "UNRESOLVED" });
    else if (hits.length > 1) errors.push({ symbol, kind: "DUPLICATE", paths: hits });
    else resolved[symbol] = hits[0];
  }
  return { resolved, errors };
}

function ruleImplementationLocator(loc) {
  const out = [];
  for (const e of loc.errors) {
    if (e.kind === "UNRESOLVED")
      out.push({ rule: "RI", code: "SEMANTIC_IMPLEMENTATION_UNRESOLVED", file: "(registry:owner_implementations)", detail: `受治理 owner "${e.symbol}" 无法通过 implementation locator 解析到任何真实文件（候选路径均不存在）；物理迁移必须同步更新 locator，否则治理静默失守`, severity: "fail" });
    else
      out.push({ rule: "RI", code: "SEMANTIC_IMPLEMENTATION_DUPLICATE", file: e.paths.join(", "), detail: `受治理 owner "${e.symbol}" 解析到多份实现：${e.paths.join(", ")}（禁止第二真源）`, severity: "fail" });
  }
  return out;
}

function analyze(files, reg) {
  const loc = resolveOwnerFiles(reg, new Set(files.map((f) => f.path)));
  const findings = [];
  findings.push(...ruleImplementationLocator(loc));
  for (const r of RULES) findings.push(...r(files, reg, loc));
  return findings;
}

// Phase 8A.1 locator 迁移测试（CASE A–E）。不依赖真实代码，纯 fixture 证明"迁移不绕过治理"。
function runLocatorSelfTest() {
  let bad = 0;
  const buildReg = (impl) => {
    const reg = loadRegistry();
    reg.states = { ...reg.states, owner_implementations: impl };
    return reg;
  };
  const resolveOf = (reg, files) => resolveOwnerFiles(reg, new Set(files.map((f) => f.path)));
  const bm = (paths) => ({ useBookmarkStore: { symbol: "useBookmarkStore", paths } });
  // CASE A: 旧路径存在 → 解析成功（identity 不依赖路径）
  {
    const reg = buildReg(bm(["src/stores/useBookmarkStore.ts", "src/capabilities/bookmark/state/useBookmarkStore.ts"]));
    const loc = resolveOf(reg, [{ path: "src/stores/useBookmarkStore.ts", src: "const panelOpen = ref(false);" }]);
    if (loc.errors.length || loc.resolved.useBookmarkStore !== "src/stores/useBookmarkStore.ts") { bad++; console.log("  ✗ CASE A 旧路径应解析成功"); }
    else console.log("  ✓ CASE A 旧路径解析成功（语义 identity 不依赖物理路径）");
  }
  // CASE B: 新路径存在 + locator 已更新 → 解析成功（迁移后命中新位置）
  {
    const reg = buildReg(bm(["src/capabilities/bookmark/state/useBookmarkStore.ts", "src/stores/useBookmarkStore.ts"]));
    const loc = resolveOf(reg, [{ path: "src/capabilities/bookmark/state/useBookmarkStore.ts", src: "const panelOpen = ref(false);" }]);
    if (loc.errors.length || loc.resolved.useBookmarkStore !== "src/capabilities/bookmark/state/useBookmarkStore.ts") { bad++; console.log("  ✗ CASE B 新路径（locator 已更新）应解析成功"); }
    else console.log("  ✓ CASE B 新路径解析成功（迁移后 locator 命中新位置）");
  }
  // CASE C: 文件移到新路径，但 locator 未更新 → UNRESOLVED（防静默失守）
  {
    const reg = buildReg(bm(["src/stores/useBookmarkStore.ts"]));
    const loc = resolveOf(reg, [{ path: "src/capabilities/bookmark/state/useBookmarkStore.ts", src: "const panelOpen = ref(false);" }]);
    if (!loc.errors.some((e) => e.kind === "UNRESOLVED" && e.symbol === "useBookmarkStore")) { bad++; console.log("  ✗ CASE C 迁移未更新 locator 必须 UNRESOLVED FAIL"); }
    else console.log("  ✓ CASE C 迁移未更新 locator → UNRESOLVED（防静默失守）");
  }
  // CASE D: symbol 对应路径不存在 → UNRESOLVED
  {
    const reg = buildReg(bm(["src/nowhere/useBookmarkStore.ts"]));
    const loc = resolveOf(reg, []);
    if (!loc.errors.some((e) => e.kind === "UNRESOLVED")) { bad++; console.log("  ✗ CASE D symbol 无实现必须 UNRESOLVED FAIL"); }
    else console.log("  ✓ CASE D 实现不存在 → UNRESOLVED");
  }
  // CASE E: 两份实现同时存在 → DUPLICATE（防第二真源）
  {
    const reg = buildReg(bm(["src/stores/useBookmarkStore.ts", "src/capabilities/bookmark/state/useBookmarkStore.ts"]));
    const loc = resolveOf(reg, [
      { path: "src/stores/useBookmarkStore.ts", src: "const panelOpen = ref(false);" },
      { path: "src/capabilities/bookmark/state/useBookmarkStore.ts", src: "const panelOpen = ref(false);" },
    ]);
    if (!loc.errors.some((e) => e.kind === "DUPLICATE" && e.symbol === "useBookmarkStore")) { bad++; console.log("  ✗ CASE E 两份实现必须 DUPLICATE FAIL"); }
    else console.log("  ✓ CASE E 两份实现 → DUPLICATE（防第二真源）");
  }
  return bad;
}

// R6 写穿式 computed setter 回归：派生量若以 computed({get,set}) 写穿到真源（如 document），其 .value = 不构成第二真源，不误报；
// 但若派生量被声明为 ref/reactive 存储态并写入，仍判 SEMANTIC_DERIVED_STATE_STORED。
function runR6SelfTest() {
  let bad = 0;
  const reg = { states: { states: {
    r6WtComputed: { derived: true, kind: "derived" },
    r6WtRef: { derived: true, kind: "derived" },
  } } };
  // 写穿式 computed：get 读真源、set 写回真源；.value = 合法，不判
  const wt = { path: "src/capabilities/database/state/useDatabaseStore.ts", src:
    `const document = ref({ connId: null, sql: "" });\n` +
    `const r6WtComputed = computed({ get: () => document.value.connId, set: (v) => { document.value.connId = v; } });\n` +
    `r6WtComputed.value = "x";` };
  const wtOut = rule6([wt], reg).filter((x) => x.rule === "R6");
  if (wtOut.length !== 0) { bad++; console.log("  ✗ R6 写穿式 computed setter 误报:", wtOut.map((x) => x.code).join(",")); }
  else console.log("  ✓ R6: 写穿式 computed({get,set}) .value = 落真源，不误报（非第二真源）");
  // 存储态谎报为派生：ref 声明 + .value = 必须判失败
  const refF = { path: "src/stores/useXStore.ts", src:
    `const r6WtRef = ref("");\n` +
    `r6WtRef.value = "x";` };
  const refOut = rule6([refF], reg).filter((x) => x.rule === "R6");
  if (refOut.length < 1) { bad++; console.log("  ✗ R6 存储态谎报为派生（ref + .value=）至少应判 1 失败，实际:", refOut.length); }
  else console.log("  ✓ R6: 派生量被存为 ref 且写入 → 仍判 SEMANTIC_DERIVED_STATE_STORED（" + refOut.length + " 条）");
  return bad;
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
  R8 SEMANTIC_STATE_MULTI_OWNER  受治理存储态在 owner 文件之外被声明（第二真源）；派生面板在 store 中被存为态
  R9 SEMANTIC_STATE_WRITER_VIOLATION  受治理存储态的写入点越权（非 owner 文件直写；或 owner 文件内写在非 canonical_writer 函数）

真源: docs/architecture/semantic-registry/{states,intents,owners,side-effects}.yaml`);
}

function loadRealFiles() {
  // 扫描 src/ 与 packages/*/src（workspace 能力包，如 capability-vault 的 state owner
  // 位于包内而非 src/）。owner_implementations locator 指向包内路径时也必须能被解析，
  // 否则 RI 规则会把已物理迁移到包内的 owner 误报 UNRESOLVED（治理静默失守）。
  const out = [];
  out.push(...walk(join(ROOT, "src"), [".ts", ".vue"]));
  out.push(...walk(join(ROOT, "packages"), [".ts", ".vue"]));
  return out.map((p) => ({ path: rel(p), src: readFileSync(p, "utf8") }));
}

// ---------------------------- fixtures ----------------------------
// false-positive fixtures：合法代码，绝不能报错（专门验证"不误报"）
const FP_FILES = [
  {
    path: "src/capabilities/browser/state/useBrowserStore.ts", // 派生量作为对象字段名（合法，非存储）
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
  // R9 false-positive：读取 .value（含 === 比较）不得误报为写入
  { path: "src/stores/__fx_r9_fp.ts", src: `const x = gridSession.value; if (gridSession.value === 0) { const y = gridSession.value; }` },
];
const POS_FILES = [
  {
    path: "src/capabilities/browser/state/useBrowserStore.ts",
    src: `
const gridOpen = ref(false);
const desiredGridVisibility = computed(() => gridOpen.value && layout.mainView === "grid");
function openGrid() { if (!gridOpen.value) return buildGrid(); }
async function closeGridAll() { gridOpen.value = false; await bridge.closeGrid(); } // side-effect: destroy webviews
`,
  },
  { path: "src/components/x/A.vue", src: `layout.activateBrowser(); browser.activateGrid();` },
  { path: "src/components/browser/CredentialList.vue", src: `bridge.fillBrowserCredential(credentialId, tabId); bridge.listBrowserCredentials();` },
  // R9 positive：owner 文件内 canonical_writer 函数写 gridSession（合法，不误报）
  { path: "src/capabilities/browser/state/useBrowserStore.ts", src: `const gridSession = ref(0);\nasync function buildGrid() { gridSession.value += 1; }\nfunction forceGridRelayout() { gridSession.value += 1; }` },
];
// 每条 negative fixture 声明期望级别：R5 是提示级(warn)，其余是阻断级(fail)
const NEG_FILES = {
  R1: { expect: "fail", files: [{ path: "src/capabilities/browser/state/useBrowserStore.ts", src: `const gridVisible = ref(false);` }] },
  R2: { expect: "fail", files: [{ path: "src/capabilities/browser/state/useBrowserStore.ts", src: `const activeSurface = ref("browser");` }, { path: "src/stores/useBookmarkStore.ts", src: `const extraBookmarks = ref<Bookmark[]>([]);` }, { path: "src/stores/useSystemStore.ts", src: `const extraPanes = ref<{id:string;cwd?:string}[]>([]);` }] },
  R3: { expect: "fail", files: [{ path: "src/components/x/B.vue", src: `browser.closeGrid(); layout.mainView = "browser";` }, { path: "src/components/browser/BadCred.vue", src: `KeyringStore.save_token("repo", token);` }] },
  R4: { expect: "fail", files: [{ path: "src/capabilities/browser/state/useBrowserStore.ts", src: `function showGridView() { setView("grid"); }` }, { path: "src/composables/__fx_term.ts", src: `function newTerm(){ return 0; }` }, { path: "src/composables/__fx_cred.ts", src: `function exposePassword(){ return readKeyring(); }` }] },
  R7: { expect: "fail", files: [{ path: "src/components/workspace/Leak.vue", src: `const password = ref(""); console.log("pw", password.value);` }] },
  R5: { expect: "warn", files: [{ path: "src/composables/useBrowserHost.ts", src: `bridge.tabPosition(id, rect);` }] },
  R6: { expect: "fail", files: [{ path: "src/stores/__fx_ws_derived_bad.ts", src: `const currentLocalPath = ref("");\ncurrentLocalPath.value = "/x";` }] },
  R8: { expect: "fail", files: [
    { path: "src/stores/useLayoutStore.ts", src: `const aiNavOpen = ref(false);` },
    { path: "src/stores/__fx_bookmark_derived_bad.ts", src: `const bmPanelOpen = ref(false);` },
    { path: "src/stores/__fx_owner_bad.ts", src: `const gridSession = ref(0);` },
  ] },
  R9: { expect: "fail", files: [
    // 非 owner store（useLayoutStore）跨域直写 gridSession
    { path: "src/stores/useLayoutStore.ts", src: `function strayGridSession() { gridSession.value = 1; }` },
    // 组件直写（非 owner 文件）
    { path: "src/components/browser/BadGrid.vue", src: `function onX() { browser.gridSession.value += 1; }` },
    // owner 文件内写在非 canonical 函数
    { path: "src/capabilities/browser/state/useBrowserStore.ts", src: `const gridSession = ref(0);\nasync function buildGrid() { gridSession.value += 1; }\nfunction forceGridRelayout() { gridSession.value += 1; }\nfunction rogueWriter() { gridSession.value += 1; }` },
  ] },
};

// owner store 锚点文件：让 implementation locator 在自检中能解析到真实实现，
// 否则 RI 规则会对所有 owner 报 UNRESOLVED 而误 FAIL。内容仅含已登记 minimal 声明，不触发其它规则。
const ANCHOR_FILES = [
  { path: "src/stores/useLayoutStore.ts", src: `const gridToolbarOpen = ref(false); const sidebarOpen = ref(false); const clipOpen = ref(false);` },
  { path: "src/capabilities/browser/state/useBrowserStore.ts", src: `const gridSession = ref(0); const aiNavOpen = ref(false);` },
  { path: "src/capabilities/workspace/state/useWorkspaceStore.ts", src: `const recents = ref([]);` },
  { path: "src/capabilities/workspace/state/useFileStore.ts", src: `const filePath = ref("");` },
  { path: "src/capabilities/workspace/state/useArtifactStore.ts", src: `const current = ref(null);` },
  { path: "src/capabilities/workspace/state/useRepoStore.ts", src: `const preview = ref(null);` },
  { path: "src/capabilities/workspace/state/useScriptStore.ts", src: `const scripts = ref([]);` },
  { path: "src/capabilities/workspace/state/useSnippetStore.ts", src: `const snippets = ref([]);` },
  { path: "src/stores/useBookmarkStore.ts", src: `const panelOpen = ref(false); const items = ref([]);` },
  // Phase 8E/Train D：Terminal owner 已迁入能力包；锚点须跟随 locator，否则 RI 会对
  // useTerminalStore 报 UNRESOLVED（自检误 FAIL）。
  { path: "src/capabilities/terminal/state/useTerminalStore.ts", src: `const terminalOpen = ref(false); const termPanes = ref([]);` },
  // useSystemStore 仍是 Clipboard/Apps 的 owner（locator 候选仍指向该路径）
  { path: "src/stores/useSystemStore.ts", src: `const clipText = ref(""); const apps = ref([]);` },
  // STAGE H：Clipboard/Apps/Tools 已拆为专属 owner（owner_implementations 新增条目）；
  // 锚点须跟随 locator，否则 RI 会对它们报 UNRESOLVED（自检误 FAIL）。
  { path: "packages/capability-clipboard/src/state/useClipboardStore.ts", src: `const clipText = ref(""); const clipHistory = reactive([]);` },
  { path: "src/capabilities/apps/state/useAppsStore.ts", src: `const apps = ref([]); const appFilter = ref("");` },
  { path: "src/capabilities/tools/state/useToolsStore.ts", src: `const tools = ref([]);` },
  // Frontend M2：Vault 升级为 packages/capability-vault（owner_implementations 锚点跟随 locator）。
  { path: "packages/capability-vault/src/state/useVaultStore.ts", src: `const path = ref("");` },
  // Capability Library Expansion v1：Home 域专属 owner（owner_implementations 已登记）；锚点跟随 locator。
  { path: "src/capabilities/home/state/useHomeStore.ts", src: `const shortcuts = reactive([]); const recents = reactive([]);` },
  // SG-C Medium：agent/database/git/skill 专属 owner；锚点跟随 locator。
  // useAgentStore 真实物理位置为 capabilities/agent/state（states.yaml locator 已同步），
  // 锚点须与 locator 一致，否则自检 RI 会误报 UNRESOLVED。
  { path: "src/capabilities/agent/state/useAgentStore.ts", src: `const agents = ref([]);` },
  { path: "src/capabilities/database/state/useDatabaseStore.ts", src: `const connections = ref([]);` },
  { path: "src/capabilities/git/state/useGitStore.ts", src: `const status = ref([]);` },
  // Phase C：Browser 子域 + Settings（owner = browser / settings capability）。锚点跟随 locator。
  { path: "src/capabilities/browser/state/useResourceStore.ts", src: `const resources = ref([]); const filter = ref("all");` },
  { path: "src/capabilities/browser/state/useSessionStore.ts", src: `const sessions = ref([]); const detail = ref(null);` },
  { path: "src/capabilities/browser/state/useGridArchiveStore.ts", src: `const rows = ref([]); const busy = ref(false);` },
  { path: "src/capabilities/browser/state/useImagePreviewStore.ts", src: `const items = ref([]); const open = ref(false);` },
  { path: "src/settings/state/useSettingsStore.ts", src: `const theme = ref("dark"); const keymapScheme = ref("vscode"); const tabHibernation = ref(false); const currentKeymap = computed(() => KEYMAP_SCHEMES[keymapScheme.value]);` },
  { path: "src/capabilities/skill/state/useSkillStore.ts", src: `const skills = ref([]);` },
  // SG-4：task 域专属 owner（owner_implementations 已登记）；锚点跟随 locator，否则 RI 报 UNRESOLVED。
  { path: "src/capabilities/task/state/useTaskStore.ts", src: `const tasks = ref([]); const runs = ref([]); const targets = ref([]);` },
  // Final-3：graph 域 owner；锚点声明全部领域专属变量（loading/error/backendReady 为全局 observed/registered，不重复）。
  { path: "src/capabilities/graph/state/useGraphStore.ts", src: `const nodes = ref(new Map()); const edges = ref(new Map()); const selectedNodeId = ref(null); const selectedEdgeKey = ref(null); const startId = ref(null); const truncated = ref(false); const inFlightRequestId = ref(null); const filter = ref({query:"",kinds:[]}); const nodeList = computed(()=>[]); const edgeList = computed(()=>[]); const visibleNodes = computed(()=>[]); const visibleEdges = computed(()=>[]); const capacity = computed(()=>({pct:0,nodes:0,edges:0})); const capState = computed(()=>({state:"OK",pct:0})); const readOnly = computed(()=>true); const selectionText = computed(()=>""); const selectedNode = computed(()=>null); const selectedEdge = computed(()=>null); const state = computed(()=>({}));` },
  // Final-3：plugin 域 owner；busy 为跨域通用态（useBookmarkStore 键），不在此声明。
  { path: "src/capabilities/plugin/state/usePluginStore.ts", src: `const list = ref([]); const detail = ref(null); const keys = ref([]); const filterState = ref(null); const manifestText = ref(""); const resourcePath = ref(""); const actionsFor = computed(()=>({enable:false,disable:false,uninstall:false}));` },
];
const ALL = (arr) => [...ANCHOR_FILES, ...arr];

function runSelfTest() {
  const reg = loadRegistry();
  let bad = 0;
  // 阻断信号 = fail + warn（info 仅为"已登记未治理"提示，不算问题）
  const blocking = (fs) => analyze(fs, reg).filter((x) => x.severity !== "info");

  const pos = blocking(ALL(POS_FILES));
  if (pos.length) { bad++; console.log("  ✗ positive fixture 应 0 fail/warn，实际:", pos.map((p) => p.code + ":" + p.detail).join(" | ")); }
  else console.log("  ✓ positive fixture: 0 fail/warn（合法语义不报错）");

  for (const [r, spec] of Object.entries(NEG_FILES)) {
    const f = analyze(ALL(spec.files), reg).filter((x) => x.severity === spec.expect && x.rule === r);
    if (!f.length) { bad++; console.log(`  ✗ negative fixture ${r} 未被检出（期望 ${spec.expect}）`); }
    else console.log(`  ✓ negative fixture ${r}: 检出 ${f[0].code} (${spec.expect})`);
  }

  const fp = blocking(ALL(FP_FILES));
  if (fp.length) { bad++; console.log("  ✗ false-positive fixture 误报:", fp.map((p) => p.rule + ":" + p.detail).join(" | ")); }
  else console.log("  ✓ false-positive fixture: 0 fail/warn（派生字段名/注释/治理域外/授权调用均不误报）");

  // Phase 8A.1：implementation locator 迁移测试（CASE A–E）
  bad += runLocatorSelfTest();

  // R6 写穿式 computed setter 回归：derived + computed({get,set}) 写穿真源 不误报；derived + ref 存储态写入 仍判失败。
  bad += runR6SelfTest();

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

// 供其它 checker（如 closure）复用：语义 owner 符号 → 首个磁盘存在的物理路径。
// 物理迁移后只要 owner_implementations 同步更新，调用方即可命中新路径，无需硬编码。
export function resolveOwnerFile(owner) {
  const reg = loadRegistry();
  const impls = (reg.states && reg.states.owner_implementations) || {};
  const def = impls[owner];
  if (!def) return null;
  for (const p of asList(def.paths != null ? def.paths : def)) {
    if (existsSync(join(ROOT, p))) return p;
  }
  return null;
}

// 仅当作为主模块运行时执行 CLI（允许被其它 checker import 复用 resolveOwnerFile）
const isMain = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (isMain) main();
