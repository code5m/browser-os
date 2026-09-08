#!/usr/bin/env node
// A8 · M5-W18-R · managed-ripgrep 路线真实基准（synthetic, non-secret corpus）
//
// 目的：在 W18-R 沙箱内（禁装产品依赖 / 禁下模型 / 禁 daemon / 禁远端）对 zvec-grep 的
// managed ripgrep 检索路线做可复现基准。zvec-grep 经 @vscode/ripgrep 包一层 rg 二进制，
// 其实际调用参数见 src/engine/service/lexical.ts:buildRipgrepArgs：
//   rg --json --line-number --column --with-filename --color never
//      + hidden/discovery/glob/type/path 过滤 + --glob '!**/.git/**'
//      + --regexp <PATTERN> <root>
// 本脚本用系统 rg（14.1.0）镜像该参数集，直接度量该路线的核心搜索成本；
// 包装层（spawn + JSON 流式解析 + 可选上下文回填）开销相对 rg 本身可忽略，且为恒定小量。
//
// 不涉及任何产品依赖安装、模型下载、daemon 或网络。语料为合成、非机密内容。

import { mkdirSync, writeFileSync, rmSync, existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { tmpdir } from "node:os";

const ROOT = join(tmpdir(), `zg-a8-bench-${process.pid}`);
rmSync(ROOT, { recursive: true, force: true });
mkdirSync(join(ROOT, "src"), { recursive: true });
mkdirSync(join(ROOT, "docs"), { recursive: true });

// ---- 生成合成语料：代码(.ts) + 多语言文档(.md) ----
const N_CODE = 300;
const N_DOC = 120;
for (let i = 0; i < N_CODE; i++) {
  const hasAuth = i % 7 === 0;
  const lines = [
    `// module ${i}`,
    `export class Service${i} {`,
    `  private id = ${i};`,
  ];
  if (hasAuth) {
    lines.push(`  // 处理用户凭证 credential for login`);
    lines.push(
      `  handleCredentials(token: string) { return AuthService.verify(token); }`,
    );
  }
  lines.push(`  function processRequest${i}() { /* do work */ }`);
  lines.push(`  query${i}() { return this.id; }`);
  lines.push(`}`);
  writeFileSync(join(ROOT, "src", `mod${i}.ts`), lines.join("\n"));
}
for (let i = 0; i < N_DOC; i++) {
  const lines = [
    `# Document ${i}`,
    `This document describes the retrieval pipeline and 凭证管理 (credential management).`,
    `认证流程 authentication flow uses AuthService to validate tokens.`,
    `关键词 搜索 search over 混合内容 mixed content works in UTF-8.`,
  ];
  writeFileSync(join(ROOT, "docs", `doc${i}.md`), lines.join("\n"));
}

// ---- 基准函数：镜像 zvec-grep 的 rg 参数 ----
function bench(label, pattern, { fixed = false, regexp = false } = {}) {
  const args = [
    "--json",
    "--line-number",
    "--column",
    "--with-filename",
    "--color",
    "never",
    "--glob",
    "!**/.git/**",
  ];
  if (fixed) args.push("--fixed-strings");
  if (regexp) args.push("--regexp", pattern);
  else args.push("--regexp", pattern);
  args.push("--", ROOT);

  const iters = 3;
  const times = [];
  let lastMatchCount = 0;
  for (let k = 0; k < iters; k++) {
    const t0 = performance.now();
    const r = spawnSync("rg", args, { encoding: "utf8", maxBuffer: 1 << 28 });
    const t1 = performance.now();
    times.push(t1 - t0);
    if (k === iters - 1 && r.stdout) {
      lastMatchCount = r.stdout
        .split("\n")
        .filter((l) => l.trim().startsWith('{"type":"match"')).length;
    }
  }
  times.sort((a, b) => a - b);
  const median = times[Math.floor(iters / 2)];

  // stats 运行：获取扫描文件数 / 字节数
  const statsArgs = [
    "--stats",
    "--glob",
    "!**/.git/**",
    "--regexp",
    pattern,
    "--",
    ROOT,
  ];
  const s = spawnSync("rg", statsArgs, { encoding: "utf8", maxBuffer: 1 << 28 });
  const statsText = s.stdout || "";
  const filesScanned = (statsText.match(/(\d+) files scanned/) || [])[1] ?? "?";
  const bytes = (statsText.match(/(\d+) bytes printed/) || [])[1] ?? "?";

  return { label, medianMs: median.toFixed(1), matchCount: lastMatchCount, filesScanned, bytes };
}

console.log(`corpus: ${N_CODE} .ts + ${N_DOC} .md under ${ROOT}`);
console.log("rg:", String(spawnSync("rg", ["--version"]).stdout).split("\n")[0]);

const rows = [
  bench("exact literal  AuthService", "AuthService"),
  bench("fixed-string  凭证", "凭证", { fixed: true }),
  bench("regex         handle\\w+", "handle\\w+", { regexp: true }),
  bench("regex/case-i  authservice", "authservice", { regexp: true }),
];

// 内存：尝试 /usr/bin/time -v（沙箱可能缺失）
let memNote = "未测量（/usr/bin/time 不可用）";
if (existsSync("/usr/bin/time")) {
  const mt = spawnSync(
    "/usr/bin/time",
    ["-v", "rg", "--glob", "!**/.git/**", "--regexp", "AuthService", "--", ROOT],
    { encoding: "utf8", maxBuffer: 1 << 28 },
  );
  const m = (mt.stderr || "").match(/Maximum resident set size \(KB\): (\d+)/);
  if (m) memNote = `${(Number(m[1]) / 1024).toFixed(1)} MB (rg RSS)`;
}

console.log("\n=== managed-ripgrep 路线基准（中位延迟 / 3 次预热，合成语料）===");
for (const r of rows) {
  console.log(
    `  ${r.label.padEnd(20)} median=${String(r.medianMs).padStart(6)}ms  matches=${String(
      r.matchCount,
    ).padStart(4)}  files_scanned=${r.filesScanned}  bytes=${r.bytes}`,
  );
}
console.log(`  memory: ${memNote}`);
console.log(
  "  说明：ripgrep 路线无索引（零索引体积），按需扫描；限制内由 zvec-grep 包装层在超限时 kill 截断。",
);

rmSync(ROOT, { recursive: true, force: true });
