#!/usr/bin/env node
/**
 * R10 — Canonical Module Location（单一语义：ONE IMPLEMENTATION）
 *
 * 一个 governed logical module 只能有一个 canonical implementation：
 *   - canonicalImplementation 必须存在；
 *   - oldLocations（已迁走的旧地址）不得再以真实文件存在（无第二实现）；
 *   - src/ 内该 basename 只能有 canonical 一个落点（单实现，零副本）。
 *
 * 与 capability-registry / semantic-registry 不重复：本规则只管「模块物理落点唯一」，
 * 不重新声明能力语义。真源：docs/architecture/module-resolution/module-identity.yaml。
 *
 * 区分力（--self-test）：
 *   - 注入「canonical 缺失」必须 FAIL；
 *   - 注入「旧地址仍有第二实现」/「同 basename 两份」必须 FAIL；
 *   - 合法单实现必须 PASS。
 *
 * 仅用 Node 标准库 + 复用 check-capability-registry.mjs 的 YAML 子集解析器。
 */
import { readFileSync, existsSync, readdirSync } from "node:fs";
import { join, relative, basename } from "node:path";
import { parseYaml } from "./check-capability-registry.mjs";

const ROOT = process.cwd();
const IDENTITY = join(ROOT, "docs/architecture/module-resolution/module-identity.yaml");
const SRC = join(ROOT, "src");
const SKIP_DIRS = new Set(["node_modules", "dist", "target", "logs", ".git"]);

function walk(dir, out = []) {
  let entries;
  try { entries = readdirSync(dir, { withFileTypes: true }); } catch { return out; }
  for (const e of entries) {
    const p = join(dir, e.name);
    if (e.isDirectory()) {
      if (!SKIP_DIRS.has(e.name)) walk(p, out);
    } else {
      out.push(relative(ROOT, p).split("\\").join("/"));
    }
  }
  return out;
}

/** 注入式校验：fileExists / lsSrc 可在 self-test 中替换为合成数据 */
function validate(identity, { fileExists, lsSrc }) {
  const issues = [];
  for (const m of identity.modules || []) {
    const canon = m.canonicalImplementation;
    if (!canon) { issues.push({ code: "R10_CANONICAL_UNSPECIFIED", moduleId: m.moduleId }); continue; }
    if (!fileExists(canon)) {
      issues.push({ code: "R10_CANONICAL_MISSING", moduleId: m.moduleId, detail: canon });
    }
    for (const old of m.oldLocations || []) {
      if (fileExists(old)) {
        issues.push({ code: "R10_SECOND_IMPLEMENTATION_AT_OLD_LOCATION", moduleId: m.moduleId, detail: old });
      }
    }
    // 单实现：src 内该 basename 只能有 canonical 一个落点
    const bn = basename(canon);
    const matches = lsSrc().filter((p) => basename(p) === bn);
    if (matches.length > 1) {
      issues.push({ code: "R10_SECOND_IMPLEMENTATION", moduleId: m.moduleId, detail: `basename ${bn} -> ${matches.length}: ${matches.join(", ")}` });
    } else if (matches.length === 1 && matches[0].split("\\").join("/") !== canon.split("\\").join("/")) {
      issues.push({ code: "R10_CANONICAL_MISMATCH", moduleId: m.moduleId, detail: `canonical=${canon} but only match=${matches[0]}` });
    }
  }
  return issues;
}

function realFileExists(p) { return existsSync(join(ROOT, p)); }
function realLsSrc() { return walk(SRC); }

function runSelfTest() {
  let pass = true;
  const assert = (cond, label) => { if (!cond) { console.log(`SELFTEST FAIL: ${label}`); pass = false; } };

  // 合法单实现必须 PASS
  const good = { modules: [{ moduleId: "x.a", canonicalImplementation: "src/capabilities/x/ui/A.vue", oldLocations: ["src/components/x/A.vue"] }] };
  assert(validate(good, { fileExists: (p) => p === "src/capabilities/x/ui/A.vue", lsSrc: () => ["src/capabilities/x/ui/A.vue", "src/other/B.vue"] }).length === 0, "good case should pass");

  // canonical 缺失必须 FAIL
  const badMissing = { modules: [{ moduleId: "x.a", canonicalImplementation: "src/capabilities/x/ui/A.vue", oldLocations: [] }] };
  assert(validate(badMissing, { fileExists: () => false, lsSrc: () => [] }).some((i) => i.code === "R10_CANONICAL_MISSING"), "missing canonical must fail");

  // 旧地址仍有第二实现必须 FAIL
  const badSecond = { modules: [{ moduleId: "x.a", canonicalImplementation: "src/capabilities/x/ui/A.vue", oldLocations: ["src/components/x/A.vue"] }] };
  assert(validate(badSecond, { fileExists: (p) => p === "src/capabilities/x/ui/A.vue" || p === "src/components/x/A.vue", lsSrc: () => ["src/capabilities/x/ui/A.vue", "src/components/x/A.vue"] }).some((i) => i.code === "R10_SECOND_IMPLEMENTATION_AT_OLD_LOCATION"), "second impl at old location must fail");

  // 同 basename 两份必须 FAIL
  const badDup = { modules: [{ moduleId: "x.a", canonicalImplementation: "src/capabilities/x/ui/A.vue", oldLocations: [] }] };
  assert(validate(badDup, { fileExists: () => true, lsSrc: () => ["src/capabilities/x/ui/A.vue", "src/other/A.vue"] }).some((i) => i.code === "R10_SECOND_IMPLEMENTATION"), "two basename matches must fail");

  console.log(pass ? "R10_SELF_TEST=PASS" : "R10_SELF_TEST=FAIL");
  process.exit(pass ? 0 : 1);
}

function main() {
  if (!existsSync(IDENTITY)) { console.error(`R10: missing ${IDENTITY}`); process.exit(2); }
  const identity = parseYaml(readFileSync(IDENTITY, "utf8"));
  const issues = validate(identity, { fileExists: realFileExists, lsSrc: realLsSrc });
  for (const i of issues) console.log(`[FAIL] ${i.code} ${i.moduleId || ""} ${i.detail || ""}`);
  console.log(`R10_CANONICAL_MODULE_LOCATION=${issues.length === 0 ? "PASS" : "FAIL"} (modules=${(identity.modules || []).length} issues=${issues.length})`);
  process.exit(issues.length === 0 ? 0 : 1);
}

if (process.argv.includes("--self-test")) runSelfTest();
else main();
