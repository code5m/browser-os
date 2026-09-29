#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-clipboard-boundary.mjs — 根级 Clipboard 包边界门禁（CB-*）。
// 委托运行包内 PKG 门禁（PKG-01..12），并在根视角下补充扫描：
//   CB-02 Host 不 deep-import 包内 / CB-03 Host 通过 public contract 消费
//   CB-04 包不 import Host impl / CB-05 B11-1 / CB-06 单点注册 / CB-07 无旧路径
// 用法: node scripts/check-clipboard-boundary.mjs
// ---------------------------------------------------------------------------
import { execFileSync } from "node:child_process";
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");
const PKG = "packages/capability-clipboard";

let fail = 0;
const failures = [];
function check(name, cond, detail) {
  if (cond) console.log(`  ok   ${name}`);
  else { fail++; failures.push(`${name}${detail ? " — " + detail : ""}`); console.log(`  FAIL ${name}${detail ? " — " + detail : ""}`); }
}

// CB-01 包内 PKG 门禁
try {
  execFileSync(process.execPath, [join(ROOT, PKG, "scripts/check-package.mjs")], { stdio: "inherit", cwd: ROOT });
} catch {
  check("CB-01 package-local PKG 门禁", false, "check-package.mjs 失败");
}

function walk(dir) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const n of readdirSync(dir)) {
    const p = join(dir, n);
    const st = statSync(p);
    if (st.isDirectory()) out.push(...walk(p));
    else if (/\.(ts|vue|mjs|js)$/.test(n) && !/\.d\.ts$/.test(n)) out.push(p);
  }
  return out;
}
function specs(code) {
  const s = [];
  const strip = code.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|\s)\/\/[^\n]*/g, "$1");
  const re = /from\s*["']([^"']+)["']/g;
  let m;
  while ((m = re.exec(strip))) s.push(m[1]);
  return s;
}
const rel = (p) => relative(ROOT, p).split("\\").join("/");
const hostFiles = walk(join(ROOT, "src")).map((p) => [rel(p), readFileSync(p, "utf8")]);
const pkgFiles = walk(join(ROOT, PKG, "src")).map((p) => [rel(p), readFileSync(p, "utf8")]);
const deepRe = /packages\/capability-clipboard\/src\/(state|ui|internal)/;
const nameDeepRe = /@browser-os\/capability-clipboard\/(state|ui|internal)/;

// CB-02 / CB-03
let deep = 0, ok = 0;
for (const [r, code] of hostFiles) {
  if (r.startsWith(PKG)) continue;
  for (const sp of specs(code)) {
    if (deepRe.test(sp) || nameDeepRe.test(sp)) deep++;
    if (sp === "@browser-os/capability-clipboard" || sp === "@browser-os/capability-clipboard/manifest") ok++;
  }
}
check("CB-02 Host 不 deep-import 包内", deep === 0, `deep=${deep}`);
check("CB-03 Host 通过 public contract 消费", ok >= 1 && deep === 0, `ok=${ok} deep=${deep}`);

// CB-04 包不 import Host impl
let hostImpl = 0;
for (const [r, code] of pkgFiles) {
  for (const sp of specs(code)) {
    if (sp.startsWith(".")) {
      const dir = r.split("/").slice(0, -1).join("/");
      const parts = [];
      for (const x of `${dir}/${sp}`.split("/")) {
        if (x === "..") parts.pop();
        else if (x && x !== ".") parts.push(x);
      }
      const res = parts.join("/");
      if (res.startsWith("src/") && !res.startsWith(`${PKG}/src`)) hostImpl++;
    }
    if (/^@browser-os\/capability-(?!clipboard)/.test(sp)) hostImpl++;
  }
}
check("CB-04 包不 import Host impl", hostImpl === 0, `hostImpl=${hostImpl}`);

// CB-05 B11-1（剥离注释后再断言，避免注释中"localStorage"字样误报）
const storeRaw = pkgFiles.find(([r]) => r.endsWith("state/useClipboardStore.ts"));
const storeCode = storeRaw ? storeRaw[1].replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|\s)\/\/[^\n]*/g, "$1") : "";
check(
  "CB-05 B11-1 store 无持久化 API",
  !!storeRaw && !/(localStorage|sessionStorage|indexedDB)/.test(storeCode) && !/(setItem|getItem|removeItem)\(/.test(storeCode)
);

// CB-06 单点注册（按出现次数计）
let pkgReg = 0, hostReg = 0;
for (const [r, code] of pkgFiles) for (const _ of code.match(/contributionRegistry\./g) || []) pkgReg++;
for (const [r, code] of hostFiles) for (const _ of code.match(/register\(\s*clipboardContribution/g) || []) hostReg++;
check("CB-06 单点注册", pkgReg === 0 && hostReg === 1, `pkgReg=${pkgReg} hostReg=${hostReg}`);

// CB-07 无旧路径
check("CB-07 旧 src/capabilities/clipboard 不存在", !existsSync(join(ROOT, "src/capabilities/clipboard")));
let stale = 0;
for (const [r, code] of hostFiles)
  if (!r.startsWith(PKG) && /capabilities\/clipboard\/(state|ui|internal|manifest|index|public)/.test(code)) stale++;
check("CB-07b 无旧路径残留引用", stale === 0, `stale=${stale}`);

console.log("");
if (fail === 0) { console.log("CLIPBOARD_BOUNDARY=PASS"); process.exit(0); }
else { console.log("CLIPBOARD_BOUNDARY=FAIL"); for (const f of failures) console.log("  - " + f); process.exit(1); }
