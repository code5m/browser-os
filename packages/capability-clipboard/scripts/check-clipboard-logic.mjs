#!/usr/bin/env node
// ---------------------------------------------------------------------------
// Clipboard 领域逻辑层门禁（headless）：store 行为契约。
// 加载真实 useClipboardStore.ts，经 app.runWithContext 注入端口后断言领域行为。
// 覆盖：D1 内存有界 / D2 clipCopy 去重 / D3 clipReadSilent 读 native mock / D4 clearClipHistory
// 用法: node scripts/check-clipboard-logic.mjs
// ---------------------------------------------------------------------------
import * as nodeModule from "node:module";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../../", import.meta.url));

function resolveWithExt(specifier, context, next) {
  if (specifier.endsWith("/bridge")) {
    return {
      url: "data:text/javascript," + encodeURIComponent('export const bridge = { clipboardRead: async () => "", clipboardWrite: async () => {} };'),
      shortCircuit: true,
    };
  }
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {
          /* next */
        }
      }
    }
    throw err;
  }
}
if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," + encodeURIComponent(`export async function resolve(s,n,c){return globalThis.__cbResolve(s,n,c);}`)
  );
  globalThis.__cbResolve = resolveWithExt;
}

globalThis.window = { setTimeout: (fn, ms) => setTimeout(fn, ms), addEventListener: () => {}, __TAURI_INTERNALS__: undefined };
globalThis.localStorage = { _s: {}, getItem() { return null; }, setItem() {}, removeItem() {} };

const { createApp } = await import("vue");
const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);
const { CLIPBOARD_PORTS_KEY } = await import(`${ROOT}packages/capability-clipboard/src/ports/index.ts`);
const { useClipboardStore } = await import(`${ROOT}packages/capability-clipboard/src/state/useClipboardStore.ts`);

let readVal = "";
const pinia = createPinia();
setActivePinia(pinia);
const app = createApp({});
app.use(pinia);
const ports = {
  native: { clipboardRead: async () => readVal, clipboardWrite: async () => {} },
  ui: { showToast: () => {}, requestClose: () => {}, redactSecrets: (s) => s, EmptyState: { render: () => null } },
};
app.provide(CLIPBOARD_PORTS_KEY, ports);

let pass = 0, fail = 0;
const failures = [];
function check(name, cond, detail) {
  if (cond) { pass++; console.log(`  ok   ${name}`); }
  else { fail++; failures.push(`${name}${detail ? " — " + detail : ""}`); console.log(`  FAIL ${name}${detail ? " — " + detail : ""}`); }
}

let clipboard;
app.runWithContext(() => { clipboard = useClipboardStore(); });
const EXPECTED_CAP = 30;

// D1 内存有界
for (let i = 0; i < 40; i++) { clipboard.clipText = "d1-" + i; await clipboard.clipCopy(); }
check(`D1 clipHistory 上限 = ${EXPECTED_CAP}`, clipboard.clipHistory.length <= EXPECTED_CAP, `len=${clipboard.clipHistory.length}`);

// D2 clipCopy 去重（相同文本不重复入历史）
clipboard.clearClipHistory();
clipboard.clipText = "dup";
await clipboard.clipCopy();
clipboard.clipText = "dup";
await clipboard.clipCopy();
check("D2 clipCopy 相同文本去重", clipboard.clipHistory.filter((c) => c.text === "dup").length === 1, `count=${clipboard.clipHistory.filter((c) => c.text === "dup").length}`);

// D3 clipReadSilent 读 native mock 并入历史（去重）
clipboard.clearClipHistory();
readVal = "from-native";
await clipboard.clipReadSilent();
check("D3 clipReadSilent 从 native 读取并入历史", clipboard.clipHistory.some((c) => c.text === "from-native"));
await clipboard.clipReadSilent();
check("D3b clipReadSilent 去重", clipboard.clipHistory.filter((c) => c.text === "from-native").length === 1);

// D4 clearClipHistory 清空
clipboard.clipText = "x"; await clipboard.clipCopy();
clipboard.clearClipHistory();
check("D4 clearClipHistory 清空历史", clipboard.clipHistory.length === 0);

console.log("");
if (fail === 0) { console.log(`CLIPBOARD_LOGIC_RESULT=PASS (${pass}/${pass})`); process.exit(0); }
else { console.log(`CLIPBOARD_LOGIC_RESULT=FAIL (${fail} failed, ${pass} passed)`); for (const f of failures) console.log("  - " + f); process.exit(1); }
