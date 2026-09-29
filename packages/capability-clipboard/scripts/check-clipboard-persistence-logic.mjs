#!/usr/bin/env node
// ---------------------------------------------------------------------------
// Clipboard B11-1 持久化/脱敏逻辑层自动化测试（headless，M2 包内权威实现）。
//
// 加载真实 packages/capability-clipboard/src/state/useClipboardStore.ts（pinia 真实实例）
// 与 src/utils/redact.ts 真实实现，并对 ClipboardPanel.vue 做源码级断言。
//
// store 经 Host 注入端口（PHASE 4 反转）：app.provide(CLIPBOARD_PORTS_KEY, ports)
// + app.runWithContext 提供 inject 作用域（store 在 setup 内 inject，故必须在
// app 提供作用域内实例化）。
//
// 覆盖（B11-1 · P0 明文落盘 / 明文展示）：
//   G1 写入历史不落 localStorage
//   G2 loadClipHistory 不读 localStorage
//   G3 内存有界（CLIP_CAP=30）
//   G4 clearClipHistory 不落 localStorage
//   G5 redactSecrets 运行时脱敏
//   G6 ClipboardPanel 不再明文渲染
//   G7 store 无 bridge 直连 / 无持久化
// 用法: node scripts/check-clipboard-persistence-logic.mjs
// ---------------------------------------------------------------------------
import * as nodeModule from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../../", import.meta.url)); // packages/capability-clipboard/scripts -> repo root

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
          /* try next */
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
    "data:text/javascript," +
      encodeURIComponent(`export async function resolve(s,n,c){return globalThis.__cbResolve(s,n,c);}`)
  );
  globalThis.__cbResolve = resolveWithExt;
}

globalThis.window = {
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  addEventListener: () => {},
  __TAURI_INTERNALS__: undefined,
};
const lsWrites = [];
globalThis.localStorage = {
  _s: {},
  getItem(k) {
    return Object.prototype.hasOwnProperty.call(this._s, k) ? this._s[k] : null;
  },
  setItem(k, v) {
    lsWrites.push([k, String(v)]);
    this._s[k] = String(v);
  },
  removeItem(k) {
    delete this._s[k];
  },
};

const { createApp } = await import("vue");
const { createPinia, setActivePinia } = await import(
  `${ROOT}node_modules/pinia/dist/pinia.mjs`
);
const { CLIPBOARD_PORTS_KEY } = await import(
  `${ROOT}packages/capability-clipboard/src/ports/index.ts`
);
const { useClipboardStore } = await import(
  `${ROOT}packages/capability-clipboard/src/state/useClipboardStore.ts`
);
const { redactSecrets } = await import(`${ROOT}src/utils/redact.ts`);

const pinia = createPinia();
setActivePinia(pinia);
const app = createApp({});
app.use(pinia);
const ports = {
  native: { clipboardRead: async () => "", clipboardWrite: async () => {} },
  ui: {
    showToast: () => {},
    requestClose: () => {},
    redactSecrets,
  },
};
app.provide(CLIPBOARD_PORTS_KEY, ports);

let pass = 0;
let fail = 0;
const failures = [];
function check(name, cond, detail) {
  if (cond) {
    pass++;
    console.log(`  ok   ${name}`);
  } else {
    fail++;
    failures.push(`${name}${detail ? " — " + detail : ""}`);
    console.log(`  FAIL ${name}${detail ? " — " + detail : ""}`);
  }
}

let clipboard;
app.runWithContext(() => {
  clipboard = useClipboardStore();
});
const EXPECTED_CAP = 30;

// G1
for (let i = 0; i < 5; i++) {
  clipboard.clipText = "clip-" + i;
  await clipboard.clipCopy();
}
check("G1 写入历史不落 localStorage", lsWrites.length === 0, `lsWrites=${JSON.stringify(lsWrites)}`);

// G2
const seed = JSON.stringify([{ text: "SECRET-TOKEN-XYZ", at: 999 }]);
localStorage.setItem("browser-os-clipboard", seed);
clipboard.loadClipHistory();
const loadedSeeded = clipboard.clipHistory.some((c) => c.text.includes("SECRET-TOKEN-XYZ"));
check("G2 loadClipHistory 不从 localStorage 载入明文", !loadedSeeded);
const clipWrites = lsWrites.filter(([k]) => k === "browser-os-clipboard");
check(
  "G2b 产品代码不再写 browser-os-clipboard",
  clipWrites.length === 1 && clipWrites[0][1] === seed,
  `clipWrites=${JSON.stringify(clipWrites)}`
);

// G3
for (let i = 0; i < 40; i++) {
  clipboard.clipText = "cap-" + i;
  await clipboard.clipCopy();
}
check(
  `G3 clipHistory 上限 = ${EXPECTED_CAP}`,
  clipboard.clipHistory.length <= EXPECTED_CAP,
  `len=${clipboard.clipHistory.length}`
);

// G4
const writesBeforeClear = lsWrites.length;
clipboard.clearClipHistory();
check("G4 clearClipHistory 不落 localStorage", lsWrites.length === writesBeforeClear);
check("G4b 清空后历史为空", clipboard.clipHistory.length === 0);

// G5
const r1 = redactSecrets("https://user:pwd@example.com/path?access_token=abc123XYZ");
check("G5 URL userinfo 被掩码", r1.includes("***:***@") && !r1.includes("pwd@example"));
check("G5 凭据查询参数被掩码", r1.includes("access_token=***") && !r1.includes("abc123XYZ"));
const r2 = redactSecrets("token ghp_aBcDeFgHiJkLmNoPqRsT");
check("G5b GitHub token 前缀被掩码", r2.includes("***") && !r2.includes("ghp_aBcD"));

// G6
const panelSrc = readFileSync(
  `${ROOT}packages/capability-clipboard/src/ui/ClipboardPanel.vue`,
  "utf8"
);
check("G6 模板不使用原始 :title=\"item.text\"", !/:title="item\.text"/.test(panelSrc));
check("G6b 模板不使用原始 {{ item.text }} 渲染", !/\{\{\s*item\.text\s*\}\}/.test(panelSrc));
check("G6c 模板对历史条目走 redactSecrets", /redactSecrets\(item\.text\)/.test(panelSrc));
check("G6d 明确「仅会话/不写磁盘」提示", /仅本次会话|不写入磁盘/.test(panelSrc));

// G7（剥离注释后再断言，避免注释中"bridge"字样误报）
const storeSrcRaw = readFileSync(
  `${ROOT}packages/capability-clipboard/src/state/useClipboardStore.ts`,
  "utf8"
);
const storeSrc = storeSrcRaw.replace(/\/\*[\s\S]*?\*\//g, "").replace(/(^|\s)\/\/[^\n]*/g, "$1");
check("G7 无 localStorage.setItem(CLIP_KEY …)", !/localStorage\.setItem\(\s*CLIP_KEY/.test(storeSrc));
check("G7b 无 localStorage.getItem(CLIP_KEY …)", !/localStorage\.getItem\(\s*CLIP_KEY/.test(storeSrc));
check("G7c 存在内存上限常量 CLIP_CAP = 30", /const CLIP_CAP = 30;/.test(storeSrc));
check("G7d store 不直连 bridge", !/bridge/.test(storeSrc));

console.log("");
if (fail === 0) {
  console.log(`CLIPBOARD_PERSIST_RESULT=PASS (${pass}/${pass})`);
  process.exit(0);
} else {
  console.log(`CLIPBOARD_PERSIST_RESULT=FAIL (${fail} failed, ${pass} passed)`);
  for (const f of failures) console.log("  - " + f);
  process.exit(1);
}
