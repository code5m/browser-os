#!/usr/bin/env node
// check-workspace-owners.mjs
// Phase 8C-0 — Workspace God-Store 收敛守护（WS-OWNER-*）
//
// 每个已抽出的子域都必须：Workspace Core 不再声明其状态；其状态不得被 owner store 之外的文件直写。
//
// 规则：
//   WS-OWNER-01: src/capabilities/workspace/state/useWorkspaceStore.ts 不得再声明任何已抽出子域的 state
//   WS-OWNER-02: Files 核心态不得被 useFileStore.ts 之外的文件直写
//   WS-OWNER-03: Artifact 核心态不得被 useArtifactStore.ts 之外的文件直写
//
// 直写判定： <alias>.<state> = / v-model="…<state>"（经 store 别名 ws/fs/art…）
// 退出码：0 通过；1 越界。
import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const SRC = join(ROOT, "src");
const WS_STORE = join(SRC, "capabilities", "workspace", "state", "useWorkspaceStore.ts");
const WS_STORE_REL = "src/capabilities/workspace/state/useWorkspaceStore.ts";

// ---- Files 域（8C-0A）----
const FILES_ALL = [
  "filePath", "inlineFile", "previewDir", "pathInput", "currentLocalPath",
  "fileEntries", "fileContent", "editingFile", "mdPreview", "mdHtml", "startDirs",
  "previewEntries", "previewImages", "previewImageErrors", "previewLoading", "previewError",
  "previewTileSize", "compareImages", "treeRoots", "treeChildren", "treeExpanded",
  "treeLoading", "treeErrors", "locateTarget", "dragSource", "dropTarget", "moveConfirm",
  "fileCtx", "inlineText", "inlineIsMd", "inlineEdit", "inlineHtml",
];
const FILES_WRITE = [
  "filePath", "inlineFile", "previewDir", "pathInput", "fileContent", "inlineText",
  "mdPreview", "editingFile", "dragSource", "dropTarget", "moveConfirm",
];

// ---- Artifact 域（8C-0B）----
// all: WS-OWNER-01 守护（Workspace Core 不得声明）；write: 只守护可写编辑缓冲。
// tree/current/selected/flatArtifacts/ctxMenu 仅被读取或经 action 访问（无组件直写），
// 且名称过于通用（selected/tree/current），纳入 v-model 检查会产生误报，故不列入 write。
const ARTIFACT_ALL = ["tree", "current", "editTitle", "editTags", "editText", "selected", "flatArtifacts", "ctxMenu"];
const ARTIFACT_WRITE = ["editTitle", "editTags", "editText"];

// ---- Repo 域（8C-0C）----
// form 为 reactive 表单对象（v-model 于字段，类比 fileCtx/ctxMenu，属可接受 UI 态）；
// write 仅守护 preview（曾由 ConfirmModal 直写 ws.preview = null，已改 clearPreview）。
const REPO_ALL = ["repos", "form", "preview", "busy", "job"];
const REPO_WRITE = ["preview"];

// ---- Script / Snippet 域（8C-0D）----
// 状态 = 列表 ref + reactive 表单对象；表单经 v-model 于字段（可接受 UI 态）→ write 为空（仅 WS-OWNER-01 守护声明）。
const SCRIPT_ALL = ["scripts", "scriptForm"];
const SNIPPET_ALL = ["snippets", "snippetForm"];

const DOMAINS = [
  { name: "files", storeName: "useFileStore", storeFile: join(SRC, "capabilities", "workspace", "state", "useFileStore.ts"), all: FILES_ALL, write: FILES_WRITE, rule: "WS_OWNER_02" },
  { name: "artifact", storeName: "useArtifactStore", storeFile: join(SRC, "capabilities", "workspace", "state", "useArtifactStore.ts"), all: ARTIFACT_ALL, write: ARTIFACT_WRITE, rule: "WS_OWNER_06" },
  { name: "repo", storeName: "useRepoStore", storeFile: join(SRC, "capabilities", "workspace", "state", "useRepoStore.ts"), all: REPO_ALL, write: REPO_WRITE, rule: "WS_OWNER_08" },
  { name: "script", storeName: "useScriptStore", storeFile: join(SRC, "capabilities", "workspace", "state", "useScriptStore.ts"), all: SCRIPT_ALL, write: [], rule: "WS_OWNER_10" },
  { name: "snippet", storeName: "useSnippetStore", storeFile: join(SRC, "capabilities", "workspace", "state", "useSnippetStore.ts"), all: SNIPPET_ALL, write: [], rule: "WS_OWNER_12" },
];

const MOVED_OUT = [...new Set(DOMAINS.flatMap((d) => d.all))];
const STORE_ALIASES = ["ws", "fs", "workspace", "wstore", "fstore", "fileStore", "art", "artifact", "astore"];

function walk(dir, acc = []) {
  for (const e of readdirSync(dir)) {
    if (e === "node_modules" || e === ".git" || e === "dist" || e.startsWith(".")) continue;
    const p = join(dir, e);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, acc);
    else if (/\.(ts|vue)$/.test(e)) acc.push(p);
  }
  return acc;
}

const problems = [];

// ---- WS-OWNER-01：useWorkspaceStore 不得声明任何已抽出子域的 state ----
if (existsSync(WS_STORE)) {
  const src = readFileSync(WS_STORE, "utf8");
  const re = new RegExp(
    "const\\s+(" + MOVED_OUT.join("|") + ")\\s*(?::[^=;]+)?=\\s*(?:ref|shallowRef|reactive|computed)\\b",
    "g"
  );
  for (const m of src.matchAll(re)) {
    problems.push({
      rule: "WS-OWNER-01",
      file: WS_STORE_REL,
      detail: `useWorkspaceStore 重新声明了已抽出子域 state "${m[1]}"（应仅由对应 owner store 持有）`,
    });
  }
}

// ---- WS-OWNER-02/03：禁止在 owner store 之外直写该子域状态 ----
const files = existsSync(SRC) ? walk(SRC) : [];
for (const d of DOMAINS) {
  if (!existsSync(d.storeFile)) continue;
  if (d.write.length === 0) continue; // 无直写防护（表单态 v-model 于字段，属可接受 UI 态）
  const WORD = d.write.join("|");
  const assignRe = new RegExp("\\b(" + STORE_ALIASES.join("|") + ")\\.(" + WORD + ")\\s*=(?!=)", "g");
  const vmodelRe = new RegExp("v-model\\s*=\\s*[\"'][^\"']*\\.(" + WORD + ")\\b", "g");
  for (const f of files) {
    if (f === d.storeFile) continue;
    let src;
    try { src = readFileSync(f, "utf8"); } catch { continue; }
    const rel = f.replace(ROOT + "/", "");
    for (const m of src.matchAll(assignRe)) {
      problems.push({ rule: d.rule, file: rel, detail: `直写 ${d.name} 状态 "${m[2]}"（${m[1]}.${m[2]} = …）；须经 canonical writer` });
    }
    for (const m of src.matchAll(vmodelRe)) {
      problems.push({ rule: d.rule, file: rel, detail: `v-model 直绑 ${d.name} 状态 "${m[1]}"；须经 canonical writer` });
    }
  }
}

if (problems.length) {
  console.error("❌ WS-OWNER 检查未通过（Workspace 子域边界越界）：");
  for (const p of problems) console.error(`  [${p.rule}] ${p.file}: ${p.detail}`);
  process.exit(1);
}
console.log("✅ WS-OWNER 检查通过：Workspace Core 不含已抽出子域状态；无组件/store 直写 Files/Artifact 状态");
process.exit(0);
