#!/usr/bin/env node
// check-sensitive-side-effects.mjs — Phase 5.1-C Sensitive Side Effect Contract gate.
//
// 强制凭据 keyring 访问的副作用契约（docs/architecture/semantic-registry/side-effects.yaml
// 的 keyringRead / keyringWrite / keyringDelete）：
//   密钥真源 = 系统密钥库；Rust keyring → Command → DTO → Frontend 的数据流中，
//   原始密钥（password/secret/api_key/...）禁止：前端返回、日志输出、剪贴板。
//
// Rules:
//   S1 REJECTED_CREDENTIAL_INTENT  任何代码调用 exposePassword/copyPassword/exportCredential
//                                   （Phase 5 已否决意图；对应 keyringRead 禁止 front_return/log/clipboard）
//   S2 KEYRING_SECRET_IN_DTO        回传前端的凭据 DTO（struct *Credential* 派生 Serialize）
//                                   不得含原始密钥字段（password/secret/api_key/access_token）
//
// CLI:
//   --help      打印本帮助
//   --self-test 运行内嵌的正/负夹具，全过则 exit 0
//   --json      输出 JSON 报告
//   --strict    把 warn 级发现也当作失败（本脚本无 warn，语义同默认；保留开关）
//
// Exit: 0 = PASS, 1 = FAIL。

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const REJECTED = ["exposePassword", "copyPassword", "exportCredential"];
const SECRET_FIELD = /\b(password|secret|api_key|access_token|plain_token)\b/;

function walk(dir, exts, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, exts, out);
    else if (exts.some((x) => p.endsWith(x))) out.push(p);
  }
  return out;
}

function readContract() {
  const f = path.join(ROOT, "docs/architecture/semantic-registry/side-effects.yaml");
  if (!fs.existsSync(f)) return null;
  const t = fs.readFileSync(f, "utf8");
  const m = t.match(/keyringRead:[\s\S]*?(?=\n\S|\n# ---|\n*$)/);
  return m ? m[0].trim() : null;
}

function scanRepo() {
  const rust = walk(path.join(ROOT, "src-tauri/src"), [".rs"]).map((p) => ({ path: p, src: fs.readFileSync(p, "utf8") }));
  const fe = walk(path.join(ROOT, "src"), [".ts", ".vue"]).map((p) => ({ path: p, src: fs.readFileSync(p, "utf8") }));
  return { rust, fe, all: [...rust, ...fe] };
}

// S1：已否决凭据泄露意图（exposePassword/copyPassword/exportCredential）
function checkS1(files) {
  const out = [];
  const re = new RegExp("\\b(" + REJECTED.join("|") + ")\\s*\\(");
  for (const f of files) {
    for (const line of f.src.split(/\r?\n/)) {
      const mm = line.match(re);
      if (mm) {
        out.push({ rule: "S1", code: "REJECTED_CREDENTIAL_INTENT", file: path.relative(ROOT, f.path), detail: `调用已否决凭据泄露意图 ${mm[1]}()（keyringRead 禁止前端返回/日志/剪贴板）`, severity: "fail" });
        break;
      }
    }
  }
  return out;
}

// S2：回传前端的凭据 DTO（struct *Credential* 且派生 Serialize）不得含原始密钥字段
function checkS2(rustFiles) {
  const out = [];
  const re = /#\[derive\(([^)]*)\)\](?:(?!#\[derive)[\s\S])*?struct\s+(\w*[Cc]redential\w*)\s*\{([^}]*)\}/g;
  for (const f of rustFiles) {
    let m;
    while ((m = re.exec(f.src))) {
      const derive = m[1];
      const name = m[2];
      const body = m[3];
      if (!/Serialize/.test(derive)) continue; // 仅约束会回传前端的 DTO
      const fields = body.replace(/\/\/[^\n]*/g, ""); // 去掉行注释（含 /// 文档），避免注释里的 password 误报
      if (!SECRET_FIELD.test(fields)) continue;
      out.push({ rule: "S2", code: "KEYRING_SECRET_IN_DTO", file: path.relative(ROOT, f.path), detail: `凭据 DTO ${name} 派生 Serialize 却含原始密钥字段（password/secret/api_key/...），违反 keyringRead 禁止前端返回密钥`, severity: "fail" });
    }
  }
  return out;
}

// ---------------- self-test 夹具（内存）----------------
const SELF = [
  {
    name: "POS BrowserCredentialItem 无 password 字段（正确 DTO）",
    files: [{ path: "fixtures/pos_cred.rs", src: 'use serde::Serialize;\n#[derive(Clone, Serialize)]\nstruct BrowserCredentialItem { id: String, username: String, origin: String, has_password: bool }' }],
    expect: "pass",
  },
  {
    name: "POS 入站凭据结构含 password 但仅 Deserialize（不回传前端）",
    files: [{ path: "fixtures/pos_inbound.rs", src: '#[derive(Clone, serde::Deserialize)]\nstruct BrowserCredentialRow { url: String, username: String, password: String }' }],
    expect: "pass",
  },
  {
    name: "NEG 凭据 DTO 派生 Serialize 且含 password（泄露到前端）",
    files: [{ path: "fixtures/neg_cred.rs", src: '#[derive(Serialize)]\nstruct BrowserCredentialRow { url: String, username: String, password: String }' }],
    expect: "fail",
  },
  {
    name: "NEG 调用 exposePassword（前端返回密钥）",
    files: [{ path: "fixtures/neg_intent.ts", src: "function exposePassword(){ return readKeyring(); }" }],
    expect: "fail",
  },
  {
    name: "NEG 调用 exportCredential（导出密钥）",
    files: [{ path: "fixtures/neg_export.ts", src: "function exportCredential(x){ fs.writeFileSync('cred.json', x); }" }],
    expect: "fail",
  },
];

function runSelfTest() {
  let ok = true;
  for (const spec of SELF) {
    const rust = spec.files.filter((x) => x.path.endsWith(".rs"));
    const findings = [...checkS1(spec.files), ...checkS2(rust)];
    const got = findings.length > 0 ? "fail" : "pass";
    const pass = got === spec.expect;
    if (!pass) ok = false;
    console.log(`  ${pass ? "✓" : "✗"} ${spec.name} → ${got}（期望 ${spec.expect}）` + (findings.length ? ` [${findings.map((f) => f.code).join(",")}]` : ""));
  }
  console.log(ok ? "SELF_TEST_RESULT=ALL_PASS" : "SELF_TEST_RESULT=FAIL");
  return ok;
}

function printHelp() {
  console.log(`check-sensitive-side-effects.mjs — Phase 5.1-C keyring 副作用契约门禁

Rules:
  S1 REJECTED_CREDENTIAL_INTENT  调用 exposePassword/copyPassword/exportCredential（keyringRead 禁止前端返回/日志/剪贴板）
  S2 KEYRING_SECRET_IN_DTO        回传前端的凭据 DTO（*Credential* + Serialize）不得含原始密钥字段

Usage:
  node scripts/check-sensitive-side-effects.mjs            # 真实仓库扫描（PASS→exit 0）
  node scripts/check-sensitive-side-effects.mjs --self-test
  node scripts/check-sensitive-side-effects.mjs --json
  node scripts/check-sensitive-side-effects.mjs --strict   # warn 级也计为失败
  node scripts/check-sensitive-side-effects.mjs --help`);
}

// ---------------- main ----------------
const args = process.argv.slice(2);
const opt = (n) => args.includes(n);

if (opt("--help") || opt("-h")) { printHelp(); process.exit(0); }
if (opt("--self-test")) { process.exit(runSelfTest() ? 0 : 1); }

const strict = opt("--strict");
const json = opt("--json");
const findings = (() => { const { rust, all } = scanRepo(); return [...checkS1(all), ...checkS2(rust)]; })();
const fail = findings.filter((f) => f.severity === "fail").length;
const warn = findings.filter((f) => f.severity === "warn").length;
const blocked = strict ? fail + warn : fail;
const result = blocked > 0 ? "FAIL" : "PASS";

if (json) {
  console.log(JSON.stringify({ result, fail, warn, strict, findings }, null, 2));
} else {
  for (const f of findings) console.log(`${f.rule} ${f.code} ${f.file}: ${f.detail}`);
  const contract = readContract();
  if (contract) console.log("KEYRING_CONTRACT:\n" + contract.split("\n").map((l) => "  " + l).join("\n"));
  console.log(`SENSITIVE_SIDE_EFFECT_RESULT=${result} fail=${fail} warn=${warn}`);
}
process.exit(blocked > 0 ? 1 : 0);
