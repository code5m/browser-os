#!/usr/bin/env node
// check-workspace-files-owner.mjs
// Phase 8C-0A — Files owner 边界守护（WS-OWNER-*）
//
// 规则：
//   WS-OWNER-01: src/stores/useWorkspaceStore.ts 不得再声明 Files-owned state
//                （文件域状态唯一真源已迁 useFileStore；真实抽取，无 facade）
//   WS-OWNER-02: 除 src/stores/useFileStore.ts 外，任何文件不得直写 Files-owned state：
//                  - <alias>.<state> = ...              （如 ws.filePath = x）
//                  - v-model="<alias>.<state>"           （直绑 store 状态）
//                  - <state>.value = ...                 （写 ref/computed）
//                必须经 useFileStore canonical writer（enterDir / openFile /
//                setFileContent / setInlineText / setEditorView / clearDragSource …）
//
// 退出码：0 通过；1 发现越界；2 用法错误。
import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, "..");
const SRC = join(ROOT, "src");
const WS_STORE = join(SRC, "stores", "useWorkspaceStore.ts");
const FILE_STORE = join(SRC, "stores", "useFileStore.ts");

// Files 域状态真源（与 owners.yaml files.owns + states.yaml 治理态一致）
const FILE_STATES = [
  "filePath", "inlineFile", "previewDir", "pathInput", "currentLocalPath",
  "fileEntries", "fileContent", "editingFile", "mdPreview", "mdHtml", "startDirs",
  "previewEntries", "previewImages", "previewImageErrors", "previewLoading", "previewError",
  "previewTileSize", "compareImages", "treeRoots", "treeChildren", "treeExpanded",
  "treeLoading", "treeErrors", "locateTarget", "dragSource", "dropTarget", "moveConfirm",
  "fileCtx", "inlineText", "inlineIsMd", "inlineEdit", "inlineHtml",
];

// WS-OWNER-02 直写防护：仅覆盖"单一真源"文档/内容/视图态（指令枚举 + 交互源）。
// 不含 fileCtx（右键菜单交互态，其输入必然经 v-model 绑 store 对象字段，属合法 UI 状态）
// 亦不含 tree*/preview*/compareImages 等内部派生/交互态（由 store action 维护）。
const WRITE_STATES = [
  "filePath", "inlineFile", "previewDir", "pathInput", "fileContent", "inlineText",
  "mdPreview", "editingFile", "dragSource", "dropTarget", "moveConfirm",
];

// 常见 store 实例别名（组件里 const ws/fs = useXxxStore()）
const STORE_ALIASES = ["ws", "fs", "workspace", "wstore", "fstore", "fileStore"];
const STATE_WORD = FILE_STATES.join("|");
const WRITE_WORD = WRITE_STATES.join("|");

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

// ---- WS-OWNER-01：useWorkspaceStore.ts 不得声明 Files-owned state ----
if (existsSync(WS_STORE)) {
  const src = readFileSync(WS_STORE, "utf8");
  const re = new RegExp(
    "const\\s+(" + STATE_WORD + ")\\s*(?::[^=;]+)?=\\s*(?:ref|shallowRef|reactive|computed)\\b",
    "g"
  );
  for (const m of src.matchAll(re)) {
    problems.push({
      rule: "WS-OWNER-01",
      file: "src/stores/useWorkspaceStore.ts",
      detail: `useWorkspaceStore 重新声明了 Files-owned state "${m[1]}"（应仅由 useFileStore 持有）`,
    });
  }
}

// ---- WS-OWNER-02：禁止在 useFileStore.ts 之外直写 Files-owned state ----
const assignRe = new RegExp(
  "\\b(" + STORE_ALIASES.join("|") + ")\\.(" + WRITE_WORD + ")\\s*=(?!=)",
  "g"
);
const valueRe = new RegExp("\\.(" + WRITE_WORD + ")\\.value\\s*=(?!=)", "g");
const vmodelRe = new RegExp("v-model\\s*=\\s*[\"'][^\"']*\\.(" + WRITE_WORD + ")\\b", "g");

if (existsSync(SRC)) {
  for (const f of walk(SRC)) {
    if (f === FILE_STORE) continue;
    const rel = f.replace(ROOT + "/", "");
    let src;
    try { src = readFileSync(f, "utf8"); } catch { continue; }
    for (const m of src.matchAll(assignRe)) {
      problems.push({
        rule: "WS-OWNER-02",
        file: rel,
        detail: `直写 Files-owned state "${m[2]}"（${m[1]}.${m[2]} = …）；须经 useFileStore canonical writer`,
      });
    }
    for (const m of src.matchAll(valueRe)) {
      problems.push({
        rule: "WS-OWNER-02",
        file: rel,
        detail: `直写 Files-owned ref "${m[1]}.value = …"；须经 useFileStore canonical writer`,
      });
    }
    for (const m of src.matchAll(vmodelRe)) {
      problems.push({
        rule: "WS-OWNER-02",
        file: rel,
        detail: `v-model 直绑 Files-owned state "${m[1]}"；须经 :value + @input 走 canonical writer`,
      });
    }
  }
}

if (problems.length) {
  console.error("❌ WS-OWNER 检查未通过（Files owner 边界越界）：");
  for (const p of problems) console.error(`  [${p.rule}] ${p.file}: ${p.detail}`);
  process.exit(1);
}
console.log("✅ WS-OWNER 检查通过：useWorkspaceStore 不含文件态；无组件/store 直写 Files 状态");
process.exit(0);
