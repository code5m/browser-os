#!/usr/bin/env node
// BrowserOS Adaptive Chrome regression guard: source contracts plus channel ownership.
// Real GTK behavior is validated by the Full Validation GUI workflow.
import { readFileSync } from "node:fs";
import assert from "node:assert/strict";
const read = (p) => readFileSync(new URL("../" + p, import.meta.url), "utf8");
const app = read("src/App.vue");
const layout = read("src/stores/useLayoutStore.ts");
const main = read("src/components/layout/MainArea.vue");
const bar = read("src/components/layout/ActivityBar.vue");
const plugin = read("tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/commands.rs");
const bridge = read("src/bridge.ts");
const adapter = read("src/composables/shellShortcuts.ts");
const workflow = read(".github/workflows/capability-v2-validation.yml");
const gates = read("package.json");

const check = (description, expression) => {
  assert.ok(expression, description);
  console.log("PASS " + description);
};
check("Three adaptive modes have a single state owner", layout.includes('ref<"standard" | "compact" | "immersive">("standard")') && layout.includes("function setShellMode("));
check("Normal and compact retain tab and address bar", app.includes('layout.shellMode !== \'immersive\''));
check("Immersive mode hides browser dock without destroying resources", main.includes("layout.shellMode !== 'immersive'") && main.includes("layout.browserDockOpen"));
check("Compact mode has mouse-accessible action", bar.includes('aria-label="紧凑模式"') && bar.includes("changeShellMode('compact')"));
check("Immersive mode has universal escape button", app.includes("focus-return") && app.includes("退出全屏"));
check("Only one Ctrl+L handler", !app.includes("matchKey(e, km.focusAddr)") && app.includes('e.key.toLowerCase() === "l"'));
check("Remote pages have no privileged keyboard shortcut IPC", plugin.includes("connect_key_press_event") && plugin.includes('browser-tabs://shell-shortcut') && !plugin.includes('window.__browser_os_shell'));
check("Native shortcut input is bound to active browser tab", app.includes('event.id !== browser.activeTabId') && app.includes('layout.mainView !== "browser"'));
check("Native shell shortcut listeners are cleaned", app.includes("unlistenChildShortcut?.()"));
check("Native event routed through adapter", bridge.includes("onChildShellShortcut") && adapter.includes("bridge.onChildShellShortcut") && app.includes("await onChildShellShortcut("));
check("Real GUI workflow remains required", workflow.includes("Full Tauri GUI regression"));
check("Shell contract included in npm check", gates.includes("check-adaptive-shell.mjs"));
console.log("ADAPTIVE_SHELL_RESULT=PASS");
