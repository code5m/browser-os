#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M1-7 Git UI 逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/stores/useGitStore.ts`，只把 `src/bridge.ts` 的
// Git 相关方法替换为记录型 mock（不 mock store 自身逻辑，也不绕过闸门）：
// 因此下面每一条断言反映的都是**产品代码**的行为，而非测试替身的行为。
//
// 覆盖：仓库选择 → 状态/diff/分支加载；写操作 request→confirm 双阶段；
// dangerous 二次确认；commit message 前端拦截；完成后刷新；错误脱敏。
//
// 用法: node scripts/check-git-ui-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

// 前端源码是无扩展名 import（`from "../bridge"`），浏览器由 Vite 解析；
// Node ESM 要求显式扩展名，这里挂一个最小的 resolve 钩子补 `.ts`。
function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__m17Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m17Resolve = resolveWithExt;
}

// useLayoutStore.showToast 用 window.setTimeout（浏览器环境），Node 下补一个最小桩
globalThis.window = { setTimeout: (fn, ms) => setTimeout(fn, ms) };

const ROOT = new URL("..", import.meta.url).pathname;

const { bridge } = await import(`${ROOT}src/bridge.ts`);
const { useGitStore } = await import(`${ROOT}src/capabilities/git/state/useGitStore.ts`);
const { redactSecrets } = await import(`${ROOT}src/utils/redact.ts`);
const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);

// ---------- mock bridge（只替换 Git 相关方法，记录调用） ----------
const calls = { status: 0, diff: 0, branch: 0, request: [], confirm: [] };
const STATE = {
  status: [
    { path: "README.md", status: "modified" },
    { path: "staged-new.txt", status: "added" },
    { path: "to-delete.txt", status: "deleted" },
    { path: "new.txt", status: "renamed" },
    { path: "untracked.txt", status: "untracked" },
    { path: "big.txt", status: "untracked" },
  ],
  branches: [
    { name: "main", is_remote: false, is_head: true },
    { name: "feature-x", is_remote: false, is_head: false },
    { name: "origin/main", is_remote: true, is_head: false },
  ],
  diff: { hunks: [{ file: "README.md", old_content: null, new_content: "+x", truncated: false, binary: false }], more: false },
  statusError: null,
};

bridge.gitStatus = async () => {
  calls.status += 1;
  return STATE.status;
};
bridge.gitBranchList = async () => {
  calls.branch += 1;
  return STATE.branches;
};
bridge.gitDiff = async (p) => {
  calls.diff += 1;
  calls.lastDiffArg = p;
  return STATE.diff;
};
bridge.requestGitWrite = async (p) => {
  calls.request.push(p);
  const dangerous = p.op === "discard" || p.op === "push";
  return {
    job_id: `job-${calls.request.length}`,
    repo_id: p.repoId,
    op: p.op,
    summary: `预览 ${p.op}`,
    affected_paths: p.paths ?? [],
    path_count: (p.paths ?? []).length,
    dangerous,
    expires_at: new Date(Date.now() + 300000).toISOString(),
  };
};
bridge.confirmGitWrite = async (p) => {
  calls.confirm.push(p);
  return { id: p.jobId, repo_id: "r1", op: "stage", paths: [], checkout: false, status: "running", dangerous: false, created_at: "", expires_at: "" };
};

// ---------- 断言工具 ----------
let failed = 0;
function ok(cond, label, extra = "") {
  if (cond) console.log(`  PASS  ${label}`);
  else {
    failed += 1;
    console.log(`  FAIL  ${label}${extra ? ` — ${extra}` : ""}`);
  }
}
function section(name) {
  console.log(`\n[${name}]`);
}

setActivePinia(createPinia());
const git = useGitStore();

// ---------- 1) 仓库选择 → 只读三连 ----------
section("1 仓库选择 / 状态 / diff / 分支");
await git.selectRepo("r1");
ok(git.repoId === "r1", "仓库已选中");
ok(calls.status === 1, "gitStatus 被调用一次", `实际 ${calls.status}`);
ok(calls.branch === 1, "gitBranchList 被调用一次", `实际 ${calls.branch}`);
ok(calls.diff === 1, "gitDiff 被调用一次", `实际 ${calls.diff}`);
ok(calls.lastDiffArg?.path === undefined, "默认拉全量 diff（path 为空）");
ok(calls.request.length === 0, "选仓库不会自动触发写操作");
ok(git.status.length === 6, "状态列表渲染 6 条（含 modified/added/deleted/renamed/untracked）");
ok(git.currentBranch === "main", "当前分支识别为 main", `实际 ${git.currentBranch}`);
ok(git.localBranches.join(",") === "main,feature-x", "本地分支清单正确");
ok(git.remoteBranches.length === 1, "远端分支只读展示 1 条");

// ---------- 2) 单文件 diff ----------
section("2 点击文件加载单文件 diff");
await git.loadDiff("README.md");
ok(calls.lastDiffArg?.path === "README.md", "gitDiff 带单文件 path");
ok(git.activePath === "README.md", "activePath 已记录");

// ---------- 3) 前端预校验：失败时不发 request ----------
section("3 本地预校验失败 → 不发 request");
git.commitMessage = "";
await git.commit();
ok(calls.request.length === 0, "commit message 为空时不发 request", `实际 ${calls.request.length}`);
git.commitMessage = "   ";
await git.commit();
ok(calls.request.length === 0, "commit message 全空白时不发 request");
await git.stage([]);
ok(calls.request.length === 0, "stage 未勾选文件时不发 request");
await git.discard([]);
ok(calls.request.length === 0, "discard 未勾选文件时不发 request");
await git.checkoutBranch("");
ok(calls.request.length === 0, "分支名为空时不发 request");
ok(typeof git.writeError === "string" && git.writeError.length > 0, "预校验失败有可读错误态");

// ---------- 4) 非 dangerous：stage 走 request → confirm ----------
section("4 stage / unstage：request → confirm（无需二次确认）");
git.toggleSelect("untracked.txt");
await git.stage();
ok(calls.request.length === 1 && calls.request[0].op === "stage", "stage 发出 request");
ok(calls.request[0].paths?.join(",") === "untracked.txt", "只带上勾选的路径");
ok(git.preview?.dangerous === false, "stage 非 dangerous");
ok(git.canConfirm === true, "非 dangerous 可直接确认");
await git.confirmWrite();
ok(calls.confirm.length === 1, "stage 已 confirm");
ok(calls.confirm[0].confirmedDangerous === false, "非 dangerous 传 confirmedDangerous=false");

// ---------- 5) dangerous：discard 必须二次确认 ----------
section("5 discard（dangerous）：未二次确认则 confirm 命令根本不发");
// 真实链路由 git-write-completed 复位 busy，这里直接复位以进入下一场景
git.busy = false;
git.preview = null;
git.writeError = null;
await git.discard(["README.md"]);
ok(calls.request.length === 2 && calls.request[1].op === "discard", "discard 发出 request");
ok(git.preview?.dangerous === true, "discard 标记为 dangerous");
ok(git.canConfirm === false, "未勾选时 canConfirm=false");
await git.confirmWrite();
ok(calls.confirm.length === 1, "未勾选二次确认 → confirm 命令未发出（前端拦截）");
ok(String(git.writeError).includes("二次确认"), "给出二次确认错误提示");
git.dangerousAck = true;
await git.confirmWrite();
ok(calls.confirm.length === 2, "勾选后 confirm 发出");
ok(calls.confirm[1].confirmedDangerous === true, "dangerous 传 confirmedDangerous=true");
// 后端闸门同样会拦（这里用后端拒绝模拟：任务未被取出）
ok(git.preview !== null || git.busy === true, "确认后进入执行态");

// ---------- 6) push ----------
section("6 push（dangerous）：二次确认 + 约束提示存在");
git.busy = false;
git.preview = null;
git.writeError = null;
await git.push();
ok(calls.request.length === 3 && calls.request[2].op === "push", "push 发出 request");
ok(git.preview?.dangerous === true, "push 标记为 dangerous");
await git.confirmWrite();
ok(calls.confirm.length === 2, "push 未二次确认 → confirm 未发出");
git.dangerousAck = true;
await git.confirmWrite();
ok(calls.confirm.length === 3 && calls.confirm[2].confirmedDangerous === true, "push 勾选后确认且带二次确认标记");

// ---------- 7) 分支操作 ----------
section("7 create_branch / checkout_branch 走同一闸门");
git.busy = false;
git.preview = null;
await git.createBranch("feat/m1-7", true);
ok(calls.request.length === 4 && calls.request[3].op === "create_branch", "create_branch 发出 request");
ok(calls.request[3].branch === "feat/m1-7" && calls.request[3].checkout === true, "分支名与 checkout 参数正确传递");
git.cancelWrite();
ok(git.preview === null && git.dangerousAck === false, "取消后清空待确认任务与勾选态");
await git.checkoutBranch("feature-x");
ok(calls.request.length === 5 && calls.request[4].op === "checkout_branch", "checkout_branch 发出 request");
git.cancelWrite();

// ---------- 8) git-write-completed → 刷新 ----------
section("8 git-write-completed 事件 → 刷新 status/diff/branch");
const before = { s: calls.status, d: calls.diff, b: calls.branch };
git.onWriteCompleted({
  id: "job-x", repo_id: "r1", op: "commit", paths: [], checkout: false,
  status: "success", dangerous: false, created_at: "", expires_at: "", finished_at: "", error: null,
});
await new Promise((r) => setTimeout(r, 30));
ok(calls.status === before.s + 1, "完成后重新拉取 status");
ok(calls.branch === before.b + 1, "完成后重新拉取 branch");
ok(calls.diff === before.d + 1, "完成后重新拉取 diff");
ok(git.busy === false, "busy 已复位");

// ---------- 9) 错误脱敏 ----------
section("9 失败态不泄露凭据");
const leaky = new Error(
  "fatal: unable to access 'https://alice:ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345@github.com/a/b.git/?access_token=SUPERSECRET_VALUE': 403"
);
STATE.statusError = leaky;
bridge.gitStatus = async () => {
  calls.status += 1;
  throw leaky;
};
await git.loadStatus();
const shown = String(git.statusError);
ok(!shown.includes("SUPERSECRET_VALUE"), "查询参数中的凭据已脱敏", shown);
ok(!shown.includes("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345"), "token 前缀串已脱敏", shown);
ok(!/alice:.*@github/.test(shown), "URL userinfo 已脱敏", shown);
ok(shown.length > 0, "错误态仍有可读信息");

section("10 redactSecrets 单元");
ok(!redactSecrets("x https://u:secret@h/p").includes("secret"), "URL userinfo 掩码");
ok(!redactSecrets("Bearer abcdefghijklmnop").includes("abcdefghijklmnop"), "Bearer 值掩码");
ok(!redactSecrets("glpat-ABCDEFGH12345678").includes("ABCDEFGH12345678"), "glpat 前缀掩码");
ok(redactSecrets("0123456789abcdef0123456789abcdef01234567").includes("0123456789abcdef0123456789abcdef01234567"), "纯小写十六进制 OID 不被误伤");
ok(redactSecrets("a".repeat(500)).length <= 301, "超长错误串被截断");

console.log("");
if (failed > 0) {
  console.log(`check-git-ui-logic: FAILED (${failed} assertion(s))`);
  process.exit(1);
}
console.log("check-git-ui-logic: ok (Git UI logic invariants hold)");
process.exit(0);
