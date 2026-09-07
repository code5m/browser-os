#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5 BUG-HUNT（Lane A6）剪贴板持久化/脱敏逻辑层自动化测试（headless）。
//
// 加载**真实**的 src/stores/useSystemStore.ts（pinia 真实实例）与 src/utils/redact.ts
// 真实实现，并对 src/components/system/ClipboardPanel.vue 做源码级断言。
// 每条运行时断言都反映产品代码行为；bridge 以最小桩注入（避免引入 Tauri 运行时）。
//
// 覆盖（对应 B11-1 · P0 明文落盘 / 明文展示）：
//   1) 默认不持久化：通过公共动作写入/清空历史均不写 localStorage；
//   2) 内存有界：clipHistory 上限 = CLIP_CAP（源码常量 30）；
//   3) 不展示凭据：redactSecrets 对 token/URL userinfo 生效，且 ClipboardPanel
//      不再以原始 item.text 渲染（title / 文本均走 redactSecrets）。
//
// 用法: node scripts/check-clipboard-persistence-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const BRIDGE_STUB =
  "export const bridge = { clipboardRead: async () => \"\", clipboardWrite: async () => {} };" +
  "export const M0Config = {};";

function resolveWithExt(specifier, context, next) {
  // 注入 bridge 桩：避免测试期加载 Tauri 运行时（且保证不触发真实剪贴板写）
  if (specifier === "../bridge" || specifier.endsWith("/bridge")) {
    return {
      url: "data:text/javascript," + encodeURIComponent(BRIDGE_STUB),
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
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__clipResolve(specifier, context, next);
}`
      )
  );
  globalThis.__clipResolve = resolveWithExt;
}

// 最小浏览器环境桩
globalThis.window = {
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  addEventListener: () => {},
  __TAURI_INTERNALS__: undefined,
};

// localStorage 监控桩：记录所有写入，便于断言"剪贴板正文从不落盘"
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

const ROOT = fileURLToPath(new URL("..", import.meta.url));

const { createPinia, setActivePinia } = await import(
  `${ROOT}node_modules/pinia/dist/pinia.mjs`
);
setActivePinia(createPinia());

const { useSystemStore } = await import(`${ROOT}src/stores/useSystemStore.ts`);
const { redactSecrets } = await import(`${ROOT}src/utils/redact.ts`);

// ---------------------------------------------------------------------------
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

const system = useSystemStore();
const EXPECTED_CAP = 30;

// G1：通过公共动作写入历史，断言不写 localStorage
for (let i = 0; i < 5; i++) {
  system.clipText = "clip-" + i;
  await system.clipCopy();
}
check(
  "G1 写入历史不落 localStorage",
  lsWrites.length === 0,
  `lsWrites=${JSON.stringify(lsWrites)}`
);

// G2：loadClipHistory 为无副作用（预置脏数据也不被读入）
const seed = JSON.stringify([{ text: "SECRET-TOKEN-XYZ", at: 999 }]);
localStorage.setItem("browser-os-clipboard", seed);
system.loadClipHistory();
const loadedSeeded = system.clipHistory.some((c) => c.text.includes("SECRET-TOKEN-XYZ"));
check("G2 loadClipHistory 不从 localStorage 载入明文", !loadedSeeded);
// 该 key 此后只可能存在测试自身写入的 seed，产品代码不得再写它
const clipWrites = lsWrites.filter(([k]) => k === "browser-os-clipboard");
check(
  "G2b 产品代码不再写 browser-os-clipboard",
  clipWrites.length === 1 && clipWrites[0][1] === seed,
  `clipWrites=${JSON.stringify(clipWrites)}`
);

// G3：内存有界（通过公共动作持续写入后由 saveClipHistory 裁剪）
for (let i = 0; i < 40; i++) {
  system.clipText = "cap-" + i;
  await system.clipCopy();
}
check(
  `G3 clipHistory 上限 = ${EXPECTED_CAP}`,
  system.clipHistory.length <= EXPECTED_CAP,
  `len=${system.clipHistory.length}`
);

// G4：clearClipHistory 不写 localStorage
const writesBeforeClear = lsWrites.length;
system.clearClipHistory();
check("G4 clearClipHistory 不写 localStorage", lsWrites.length === writesBeforeClear);
check("G4b 清空后历史为空", system.clipHistory.length === 0);

// G5：redactSecrets 运行时脱敏（凭据不展示）
const r1 = redactSecrets("https://user:pwd@example.com/path?access_token=abc123XYZ");
check("G5 URL userinfo 被掩码", r1.includes("***:***@") && !r1.includes("pwd@example"));
check("G5 凭据查询参数被掩码", r1.includes("access_token=***") && !r1.includes("abc123XYZ"));
const r2 = redactSecrets("token ghp_aBcDeFgHiJkLmNoPqRsT");
check("G5b GitHub token 前缀被掩码", r2.includes("***") && !r2.includes("ghp_aBcD"));

// G6：ClipboardPanel.vue 源码——不再以原始 item.text 明文渲染
const panelSrc = readFileSync(`${ROOT}src/components/system/ClipboardPanel.vue`, "utf8");
check("G6 模板不使用原始 :title=\"item.text\"", !/:title="item\.text"/.test(panelSrc));
check("G6b 模板不使用原始 {{ item.text }} 渲染", !/\{\{\s*item\.text\s*\}\}/.test(panelSrc));
check("G6c 模板对历史条目走 redactSecrets", /redactSecrets\(item\.text\)/.test(panelSrc));
check(
  "G6d 明确「仅会话/不写磁盘」提示",
  /仅本次会话|不写入磁盘/.test(panelSrc)
);

// G7：useSystemStore.ts 源码——已无剪贴板持久化
const storeSrc = readFileSync(`${ROOT}src/stores/useSystemStore.ts`, "utf8");
check(
  "G7 无 localStorage.setItem(CLIP_KEY …)",
  !/localStorage\.setItem\(\s*CLIP_KEY/.test(storeSrc)
);
check(
  "G7b 无 localStorage.getItem(CLIP_KEY …)",
  !/localStorage\.getItem\(\s*CLIP_KEY/.test(storeSrc)
);
check("G7c 存在内存上限常量 CLIP_CAP = 30", /const CLIP_CAP = 30;/.test(storeSrc));

console.log("");
if (fail === 0) {
  console.log(`CLIPBOARD_PERSIST_RESULT=PASS (${pass}/${pass})`);
  process.exit(0);
} else {
  console.log(`CLIPBOARD_PERSIST_RESULT=FAIL (${fail} failed, ${pass} passed)`);
  for (const f of failures) console.log("  - " + f);
  process.exit(1);
}
