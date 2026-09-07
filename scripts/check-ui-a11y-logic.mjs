#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5-W15（Lane A6）UI 无障碍逻辑层自动化测试（headless，无 GUI 依赖）。
//
// 加载**真实** src/utils/modalA11y.ts（纯函数层），断言焦点决策。
// 不重实现逻辑，每条断言反映的都是**产品代码**的行为。
//
// 用法: node scripts/check-ui-a11y-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

function resolveWithExt(specifier, context, next) {
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
        `export async function resolve(specifier, context, next) { return globalThis.__uiA11yResolve(specifier, context, next); }`,
      ),
  );
  globalThis.__uiA11yResolve = resolveWithExt;
}

const a11y = await import("../src/utils/modalA11y.ts");

let failures = 0;
function check(name, cond) {
  if (cond) {
    console.log("  ok   " + name);
  } else {
    console.log("  FAIL " + name);
    failures++;
  }
}

// ---- modalA11y.ts：role / escape / focusable selector ----

check("ariaRole dialog -> dialog", a11y.ariaRoleForVariant("dialog") === "dialog");
check("ariaRole confirm -> alertdialog", a11y.ariaRoleForVariant("confirm") === "alertdialog");
check("ariaRole danger -> alertdialog", a11y.ariaRoleForVariant("danger") === "alertdialog");
check("ariaRole alert -> alertdialog", a11y.ariaRoleForVariant("alert") === "alertdialog");

check("escape dialog -> true", a11y.shouldCloseOnEscape("dialog") === true);
check("escape confirm -> true", a11y.shouldCloseOnEscape("confirm") === true);
check("escape alert -> false", a11y.shouldCloseOnEscape("alert") === false);

check("focusable selector 包含按钮", a11y.FOCUSABLE_SELECTOR.includes("button:not([disabled])"));
check("focusable selector 排除 hidden input", a11y.FOCUSABLE_SELECTOR.includes('input:not([disabled]):not([type="hidden"])'));

if (failures) {
  console.log(`UI_A11Y_RESULT=FAIL (${failures})`);
  process.exit(1);
}
console.log("UI_A11Y_RESULT=PASS");
process.exit(0);
