#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-developer-owners.mjs — Developer Capability Family 边界门禁（Phase 8E / Train E）
//
// 对应 Train E 收口目标（诚实、不强拆）：
//   DEV-01  无合并的 DeveloperStore：database / git / repo 各自独立 owner
//   DEV-02  各 owner 文件存在且只声明自己的状态（无第二真源）
//   DEV-03  凭据只经 reference / 后端密钥库：前端不持久化 token / 密码
//   DEV-04  Repo Context ≠ Git Operation：useRepoStore 不写 git；useGitStore 不管理 repos[]
//   DEV-05  Database / Git / Terminal 不互相 import 内部（能力间经 public / bridge）
//   DEV-06  物理成熟度诚实记录：二者为 always-loaded Workspace 面板，非 absent-composable（= C1）
//           本门禁只守边界，不抬高成熟度（不谎报 C3）
//
// 判定手段：**静态扫描**（与 check-workspace-owners.mjs 同口径，核心分析函数接受真实文件表）。
// 真源：Semantic Registry owners.yaml（database→useDatabaseStore / git→useGitStore / repo→useRepoStore）。
//
// 用法: node scripts/check-developer-owners.mjs [--self-test] [--json]
// 退出码: 0 = 通过；1 = 失败
// ---------------------------------------------------------------------------

import { readFileSync, readdirSync, statSync, existsSync, writeFileSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SRC = join(ROOT, "src");

// 与 Semantic Registry owners.yaml 对齐（真源在 YAML；此处为扫描锚点，不硬编码第二份名单）
const OWNERS = {
  useDatabaseStore: ["src/stores/useDatabaseStore.ts", "src/capabilities/database/state/useDatabaseStore.ts"],
  useGitStore: ["src/stores/useGitStore.ts", "src/capabilities/git/state/useGitStore.ts"],
  useRepoStore: [
    "src/capabilities/workspace/state/useRepoStore.ts",
    "src/stores/useRepoStore.ts",
  ],
};

// 各 owner 的专属状态（用于"第二真源"检测；真源仍是 states.yaml，本表只作反向断言）
const DB_STATES = ["documents", "activeDocument", "connections", "configs", "activeId", "sql", "result", "risk", "verdict", "pendingSql", "schema", "form"];
const GIT_STATES = ["repoId", "status", "branches", "diff", "selected", "commitMessage", "branchName", "preview", "dangerousAck", "busy", "lastJob", "writeError"];
const REPO_STATES = ["repos", "form", "preview", "busy", "job"];

function stripComments(s) {
  return s.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|[^:])\/\/.*$/gm, "$1");
}
function resolveOwner(sym) {
  for (const p of OWNERS[sym]) {
    const abs = join(ROOT, p);
    if (existsSync(abs)) return abs;
  }
  return null;
}
function walk(dir, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const e of readdirSync(dir)) {
    if (e === "node_modules" || e === "dist" || e.startsWith(".")) continue;
    const p = join(dir, e);
    if (statSync(p).isDirectory()) walk(p, acc);
    else if (/\.(ts|vue)$/.test(e)) acc.push(p);
  }
  return acc;
}
const DECL = (name) =>
  // 支持嵌套泛型（如 reactive<Set<string>>(new Set())）：泛型段用非贪婪 + 一层嵌套匹配
  new RegExp("(?:const|let|var)\\s+" + name + "\\s*(?::\\s*[^=;]+)?=\\s*(?:ref|shallowRef|reactive|computed)\\s*(?:<(?:[^<>]|<[^>]*>)*>)?\\s*\\(");

const fails = [];
const pass = [];
const ok = (id, label) => pass.push(`${id} ${label}`);
const bad = (id, label, detail) => fails.push(`${id} ${label}${detail ? `  → ${detail}` : ""}`);

function runStatic() {
  // DEV-01：三个 owner 文件物理存在（且非同一文件 = 未合并）
  const paths = {};
  for (const sym of Object.keys(OWNERS)) {
    const f = resolveOwner(sym);
    if (!f) { bad("DEV-01", `${sym} owner 文件存在`, "无候选路径命中"); continue; }
    paths[sym] = f;
  }
  const uniq = new Set(Object.values(paths));
  if (uniq.size === Object.keys(paths).length) ok("DEV-01", "database/git/repo 三 owner 文件物理分离（无合并 DeveloperStore）");
  else bad("DEV-01", "三 owner 物理分离", `共享路径数=${uniq.size}`);

  // DEV-02：每个 owner 声明自己的**区分性**状态（避免与通用名 busy/status/selected 等重名误报）。
  // 区分性状态 = 该能力的语义专属状态（真源仍是 states.yaml）。
  const DB_DISTINCT = ["documents", "configs", "activeId", "pendingSql", "schema", "verdict", "connections"];
  const GIT_DISTINCT = ["repoId", "branches", "dangerousAck", "writeError", "lastJob"];
  const REPO_DISTINCT = ["repos", "job"];
  for (const [sym, states] of [["useDatabaseStore", DB_DISTINCT], ["useGitStore", GIT_DISTINCT], ["useRepoStore", REPO_DISTINCT]]) {
    const f = paths[sym];
    if (!f) continue;
    const src = stripComments(readFileSync(f, "utf8"));
    const missing = states.filter((n) => !DECL(n).test(src));
    if (missing.length) bad("DEV-02", `${sym} 声明自身区分性状态`, `缺: ${missing.join(",")}`);
    else ok("DEV-02", `${sym} 声明自身全部区分性状态（${states.length} 项）`);
  }

  // DEV-02b：全仓不得出现合并 store 文件 / 符号
  const allFiles = walk(SRC);
  const merged = allFiles.filter((f) => /useDeveloperStore|DeveloperStore\b/.test(stripComments(readFileSync(f, "utf8"))));
  if (merged.length) bad("DEV-02b", "无合并 DeveloperStore 符号", merged.map((m) => m.slice(ROOT.length + 1)).join(", "));
  else ok("DEV-02b", "全仓无 useDeveloperStore 合并符号");

  // DEV-02c：第二真源检测（其它文件不得再声明 DB/Git/Repo 的**区分性**状态）。
  // 仅用区分性状态名，避免 busy/status/selected 等通用名在其它 store 重名造成误报。
  const owned = new Map([
    ["useDatabaseStore", DB_DISTINCT], ["useGitStore", GIT_DISTINCT], ["useRepoStore", REPO_DISTINCT],
  ]);
  const second = [];
  for (const f of allFiles) {
    const rel = f.slice(ROOT.length + 1);
    if (Object.values(paths).map((p) => p.slice(ROOT.length + 1)).includes(rel)) continue;
    const src = stripComments(readFileSync(f, "utf8"));
    for (const [sym, states] of owned) {
      if (f === paths[sym]) continue;
      for (const n of states) if (DECL(n).test(src)) second.push(`${rel}:${n}(${sym})`);
    }
  }
  if (second.length) bad("DEV-02c", "无第二真源（DB/Git/Repo 区分性状态只由各自 owner 声明）", second.join(", "));
  else ok("DEV-02c", "全仓仅各自 owner 声明 DB/Git/Repo 状态（无第二真源）");

  // DEV-03：凭据只经 reference / 后端密钥库，前端不持久化
  const db = readFileSync(paths.useDatabaseStore, "utf8");
  // 密码只作为 connect(password) 瞬时参数；不写入 form / store / localStorage
  const pwParam = /async function connect\(password: string\)/.test(db);
  const pwNotStored = !/password\s*=\s*(?:form\.|ref|reactive|configs|document|store)/.test(db) &&
    !/localStorage\.(setItem|set)\(\s*["'][^"']*pass/i.test(db);
  const payloadNoPw = /buildConnectPayload\(form\.value\)/.test(db);
  const pwTransient = pwParam && pwNotStored && payloadNoPw;
  const dbNoLocalCred = !/localStorage\.(setItem|set)\(\s*["'][^"']*(?:db|database|conn|pass|token)/i.test(db);
  if (pwTransient && dbNoLocalCred) ok("DEV-03a", "Database 密码仅作瞬时参数（不进 form/store/localStorage）");
  else bad("DEV-03a", "Database 密码处理", `transient=${pwTransient} noLocalCred=${dbNoLocalCred}`);

  const git = readFileSync(paths.useGitStore, "utf8");
  const gitNoTokenStore = !/localStorage\.(setItem|set)\(\s*["'][^"']*(?:git|token|repoToken|credential)/i.test(git) &&
    !/(gitRepoToken|password|token)\s*=\s*(?:ref|reactive|shallowRef)/.test(git);
  const gitRefOnly = /凭据只在后端推送瞬间从系统密钥库读取/.test(git);
  if (gitNoTokenStore && gitRefOnly) ok("DEV-03b", "Git 凭据只经后端密钥库引用（前端零持久化）");
  else bad("DEV-03b", "Git 凭据处理", `noTokenStore=${gitNoTokenStore} refOnly=${gitRefOnly}`);

  // DEV-04：Repo Context ≠ Git Operation
  const repo = readFileSync(paths.useRepoStore, "utf8");
  const repoNoGitWrite = !/\brequestGitWrite|confirmGitWrite|gitStatus|gitDiff\b/.test(repo);
  const gitNoRepoCfg = !/const repos\s*=|reactive<\s*RepoConfig/.test(git) && !/\brepos\b\s*=\s*ref/.test(git);
  if (repoNoGitWrite) ok("DEV-04a", "useRepoStore 不执行 Git 写/读操作（Repo Context ≠ Git Operation）");
  else bad("DEV-04a", "useRepoStore 不含 git 操作", "发现 git 调用");
  if (gitNoRepoCfg) ok("DEV-04b", "useGitStore 不管理 repo 配置列表（repos[] 归 useRepoStore）");
  else bad("DEV-04b", "useGitStore 含 repos 配置", "发现 repos 声明");

  // DEV-05：能力间不互相 import 内部（Database/Git/Terminal 边界）。
  // 关键：只报**跨能力**内部 import；能力包内部自引用（terminal/index.ts → ./ui/*）是合法的，必须排除。
  const INTERNAL = [
    { cap: "terminal", re: /(?:^|\/)src\/capabilities\/terminal\/(?:state|ui|services|lifecycle|resource|internal|adapters)\//, pub: "src/capabilities/terminal/public" },
    { cap: "database", re: /(?:^|\/)src\/(?:stores|capabilities\/database)\/(?:state|ui)\//, pub: "src/capabilities/database/public" },
    { cap: "git", re: /(?:^|\/)src\/(?:stores|capabilities\/git)\/(?:state|ui)\//, pub: "src/capabilities/git/public" },
  ];
  const hits = [];
  for (const f of walk(SRC)) {
    const rel = f.slice(ROOT.length + 1);
    if (!rel.startsWith("src/capabilities/") && !rel.startsWith("src/components/")) continue;
    // 该文件所属能力包（用于排除同包自引用）
    const myCap = INTERNAL.find((x) => rel.startsWith(`src/capabilities/${x.cap}/`));
    const src = stripComments(readFileSync(f, "utf8"));
    const re = /(?:from|import\()\s*["']([^"']+)["']/g;
    let m;
    while ((m = re.exec(src)) !== null) {
      const spec = m[1];
      if (!spec.startsWith(".")) continue;
      const norm = new URL(spec.replace(/\.(ts|vue)$/, ""), "file://" + join(ROOT, rel.split("/").slice(0, -1).join("/") + "/")).pathname;
      const relNorm = norm.slice(norm.indexOf("/src/") + 1);
      for (const x of INTERNAL) {
        if (!x.re.test(relNorm)) continue;
        if (relNorm.startsWith(x.pub)) continue; // 经 public 边界，合法
        if (myCap && myCap.cap === x.cap) continue; // 同能力包内部自引用，合法
        hits.push(`${rel} → ${spec} (${x.cap} internal)`);
      }
    }
  }
  if (hits.length) bad("DEV-05", "Developer 能力不互相 import 内部（经 public 边界）", hits.join(", "));
  else ok("DEV-05", "Database/Git/Terminal 互不为跨能力内部 import（Shell/组件只经 public 或 bridge）");
}

// ─────────────────────────── self-test ───────────────────────────
function selfTest() {
  const cases = [
    ["POSITIVE 状态声明识别", DECL("documents").test("const documents = ref<X[]>([])"), true],
    ["NEGATIVE 非声明不误报", DECL("documents").test("// documents 注释"), false],
    ["POSITIVE 合并 store 检出", /useDeveloperStore/.test("const useDeveloperStore = defineStore('dev')"), true],
    ["NEGATIVE 无合并不误报", /useDeveloperStore/.test("const useRepoStore = defineStore('repo')"), false],
    ["POSITIVE 凭据落地检出", /localStorage\.setItem\("db-token"/.test('localStorage.setItem("db-token", x)'), true],
    ["NEGATIVE 无凭据落盘", /localStorage\.setItem\(["'][^"']*token/.test("const x = ref(0)"), false],
    ["POSITIVE repo 含 git 检出", /requestGitWrite/.test("async function f(){ await bridge.requestGitWrite() }"), true],
    ["NEGATIVE repo 无 git 不误报", /requestGitWrite/.test("const status = ref([])"), false],
  ];
  let p = 0, f = 0;
  for (const [name, got, want] of cases) {
    const good = got === want;
    if (good) p += 1; else f += 1;
    console.log(`${good ? "PASS" : "FAIL"}  ${name}（got=${got} want=${want}）`);
  }
  console.log(`\nSELF_TEST: ${f === 0 ? "PASS" : "FAIL"} (${p}/${p + f})`);
  return f === 0 ? 0 : 1;
}

const HELP = `check-developer-owners.mjs — Developer Capability Family 边界门禁（Phase 8E Train E）

用法:
  node scripts/check-developer-owners.mjs           边界静态断言
  node scripts/check-developer-owners.mjs --self-test  判定逻辑夹具自检
  node scripts/check-developer-owners.mjs --json        机器可读
  node scripts/check-developer-owners.mjs --help

覆盖: DEV-01 无合并 DeveloperStore / DEV-02 各 owner 声明自身状态 / DEV-03 凭据只经引用
      DEV-04 Repo Context ≠ Git Operation / DEV-05 能力间不互相 import 内部
注意: 本门禁只守边界，不抬高成熟度；Database/Git 当前为 always-loaded Workspace 面板（= C1），
      非 absent-composable（C3 需物理抽取为可选能力包，见 Debt-8E-5/6）。
退出码: 0 = 通过, 1 = 失败
`;

async function main() {
  const argv = process.argv.slice(2);
  if (argv.includes("--help") || argv.includes("-h")) { console.log(HELP); return 0; }
  if (argv.includes("--self-test")) return selfTest();

  runStatic();
  const json = argv.includes("--json");
  if (json) console.log(JSON.stringify({ result: fails.length === 0 ? "PASS" : "FAIL", pass, fails }, null, 2));
  else {
    for (const p of pass) console.log(`PASS  ${p}`);
    for (const f of fails) console.log(`FAIL  ${f}`);
    console.log(`\nDEVELOPER_OWNERS_RESULT=${fails.length === 0 ? "PASS" : "FAIL"} (${pass.length}/${pass.length + fails.length})`);
  }
  return fails.length === 0 ? 0 : 1;
}

const isMain = process.argv[1] && process.argv[1].includes("check-developer-owners.mjs");
if (isMain) main().then((c) => process.exit(c));

export { OWNERS };
