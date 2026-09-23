#!/usr/bin/env node
/**
 * R11 — Stale Module Reference（消费 Migration Tombstone）
 *
 * 读取 docs/architecture/module-resolution/module-migrations.yaml 的 tombstoned `from` 路径，
 * 扫描「可执行引用面」（src/、scripts/、src-tauri/ 与根配置），确认没有任何 executable /
 * runtime reference 再指向已 TOMBSTONED 的旧模块地址。
 *
 * 允许出现 `from` 的面（NON_EXECUTABLE_HISTORY，本脚本直接排除）：
 *   - docs/** 整体（历史 Markdown 文档）
 *   - node_modules / dist / target / logs / .git（产物与依赖）
 *
 * 与 R10 不重复：R10 管「实现唯一」，R11 管「引用唯一」；二者合起来强制
 *   ONE IMPLEMENTATION + ONE RUNTIME BINDING。
 *
 * 区分力（--self-test）：运行时生成一份引用 tombstoned 路径的 fixture .vue 并断言能被抓到，
 *   且当前真树命中数必须为 0。
 *
 * 仅用 Node 标准库 + 复用 check-capability-registry.mjs 的 YAML 子集解析器。
 */
import { readFileSync, existsSync, readdirSync, writeFileSync, unlinkSync } from "node:fs";
import { join, relative } from "node:path";
import { parseYaml } from "./check-capability-registry.mjs";

const ROOT = process.cwd();
const TOMB = join(ROOT, "docs/architecture/module-resolution/module-migrations.yaml");
const SKIP_DIRS = new Set(["node_modules", "dist", "target", "logs", ".git", "docs"]);
const EXEC_EXT = new Set([".ts", ".vue", ".js", ".mjs", ".json", ".rs", ".toml", ".html", ".cjs"]);

function walk(dir, out = []) {
  let entries;
  try { entries = readdirSync(dir, { withFileTypes: true }); } catch { return out; }
  for (const e of entries) {
    const p = join(dir, e.name);
    const rel = relative(ROOT, p).split("\\").join("/");
    if (e.isDirectory()) {
      if (!SKIP_DIRS.has(e.name)) walk(p, out);
    } else if (EXEC_EXT.has(e.name.slice(e.name.lastIndexOf(".")))) {
      out.push(rel);
    }
  }
  return out;
}

function scanReferences(tombstonedFrom) {
  const files = walk(ROOT);
  const hits = [];
  for (const f of files) {
    let content;
    try { content = readFileSync(join(ROOT, f), "utf8"); } catch { continue; }
    for (const from of tombstonedFrom) {
      if (content.includes(from)) hits.push({ file: f, from });
    }
  }
  return hits;
}

function loadTomb() {
  if (!existsSync(TOMB)) { console.error(`R11: missing ${TOMB}`); process.exit(2); }
  const tomb = parseYaml(readFileSync(TOMB, "utf8"));
  const froms = (tomb.migrations || [])
    .filter((m) => m.migration_status === "TOMBSTONED")
    .map((m) => m.from);
  return { tomb, froms };
}

function runSelfTest() {
  let pass = true;
  const { tomb, froms } = loadTomb();
  const sample = froms[0];
  if (!sample) { console.log("R11_SELF_TEST=FAIL (no tombstoned from)"); process.exit(1); }
  const tmp = join(ROOT, "__r11_fixture__.vue");
  try {
    writeFileSync(tmp, `import Stale from "${sample}"\n`);
    const hits = scanReferences(froms);
    if (!hits.some((h) => h.file === "__r11_fixture__.vue")) {
      console.log("R11_SELF_TEST=FAIL (did not detect fixture referencing tombstoned path)");
      pass = false;
    }
    if (hits.some((h) => h.file !== "__r11_fixture__.vue")) {
      console.log("R11_SELF_TEST=FAIL (unexpected real-tree hits):", JSON.stringify(hits.filter((h) => h.file !== "__r11_fixture__.vue")));
      pass = false;
    }
  } finally {
    try { unlinkSync(tmp); } catch { /* ignore */ }
  }
  console.log(pass ? "R11_SELF_TEST=PASS" : "R11_SELF_TEST=FAIL");
  process.exit(pass ? 0 : 1);
}

function main() {
  const { froms, tomb } = loadTomb();
  const hits = scanReferences(froms);
  for (const h of hits) console.log(`[FAIL] R11_STALE_TOMBSTONED_REFERENCE file=${h.file} from=${h.from}`);
  console.log(`R11_STALE_MODULE_REFERENCE=${hits.length === 0 ? "PASS" : "FAIL"} (tombstoned=${froms.length} hits=${hits.length})`);
  process.exit(hits.length === 0 ? 0 : 1);
}

if (process.argv.includes("--self-test")) runSelfTest();
else main();
