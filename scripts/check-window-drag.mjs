#!/usr/bin/env node
// Execute the actual SFC handler with native APIs stubbed. This does not prove
// that a desktop compositor moves the window; physical-mouse QA is separate.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { runInNewContext } from "node:vm";
import { parse, compileScript } from "@vue/compiler-sfc";
import { baseParse } from "@vue/compiler-dom";
import { transformSync } from "esbuild";

const root = new URL("../", import.meta.url);
const read = (path) => readFileSync(new URL(path, root), "utf8");
const source = read("src/components/layout/UnifiedTabBar.vue");
const { descriptor } = parse(source);
const template = baseParse(descriptor.template.content);
let presses = 0;
function inspect(node) {
  for (const prop of node.props ?? []) {
    assert.notEqual(prop.name, "data-tauri-drag-region", "Do not mix automatic and explicit drag handlers");
    if (prop.type === 7 && prop.name === "on") {
      assert.notEqual(prop.arg?.content, "pointerdown", "Do not duplicate mousedown with pointerdown");
      if (prop.arg?.content === "mousedown") {
        assert.equal(prop.exp?.content, "startWindowDrag");
        presses++;
      }
    }
  }
  for (const child of node.children ?? []) inspect(child);
}
inspect(template);
assert.equal(presses, 1, "Exactly one titlebar press binding");
assert.ok(!source.includes("-webkit-app-region"), "Do not rely on Chromium drag CSS in WebKitGTK");

const require = createRequire(import.meta.url);
const messages = [];
let drags = 0;
let maximizes = 0;
let rejectDrag = false;
const compiled = compileScript(descriptor, { id: "window-drag-check" });
const { code } = transformSync(compiled.content, { loader: "ts", format: "cjs" });
const module = { exports: {} };
runInNewContext(code, {
  module,
  exports: module.exports,
  require(id) {
    if (id === "vue") return require("vue");
    if (id === "@tauri-apps/api/window") return {
      getCurrentWindow: () => ({
        toggleMaximize: async () => { maximizes++; },
        startDragging: async () => {
          drags++;
          if (rejectDrag) throw new Error("native permission denied");
        },
      }),
    };
    const store = id.match(/\/use(\w+)Store$/);
    if (store) return { [`use${store[1]}Store`]: () => ({ showToast: (text) => messages.push(text) }) };
    throw new Error(`Unexpected dependency: ${id}`);
  },
});
const component = module.exports.default.setup({}, { expose() {} });
const selector = "button, .tab, input, select, textarea, a";
function event(button = 0, control = false, detail = 1) {
  return {
    button,
    detail,
    target: { closest(value) { assert.equal(value, selector); return control ? {} : null; } },
    prevented: false,
    stopped: false,
    preventDefault() { this.prevented = true; },
    stopPropagation() { this.stopped = true; },
  };
}
const press = event();
await component.startWindowDrag(press);
assert.equal(drags, 1, "One native call per left press");
assert.ok(press.prevented && press.stopped, "Prevent selection and document drag handling");
for (const e of [event(1), event(2), event(0, true)]) {
  await component.startWindowDrag(e);
  assert.ok(!e.prevented && !e.stopped, "Other buttons and controls retain their events");
}
assert.equal(drags, 1, "Controls and non-left presses never start a drag");
await component.startWindowDrag(event(0, false, 2));
assert.equal(maximizes, 1, "Double press preserves maximize/restore");
assert.equal(drags, 1, "Second press does not start another native drag");
rejectDrag = true;
await component.startWindowDrag(event());
assert.equal(messages.length, 1, "Native failure must not be silently swallowed");
assert.ok(!messages[0].includes("permission denied"), "Do not expose raw native errors");

for (const path of ["src-tauri/capabilities/default.json", "src-tauri/dev-capabilities/main.json"]) {
  const capability = JSON.parse(read(path));
  assert.deepEqual(capability.windows, ["main"]);
  for (const action of ["start-dragging", "minimize", "toggle-maximize", "close"]) {
    assert.ok(capability.permissions.includes(`core:window:allow-${action}`), `${path}: ${action}`);
  }
}
console.log("WINDOW_DRAG_CHECK=PASS (actual handler + template + ACL; native movement requires desktop QA)");
