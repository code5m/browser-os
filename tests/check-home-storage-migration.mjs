// Execute the real Home Store with a readable but unwritable browser storage.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

function load(path, modules, localStorage) {
  const source = readFileSync(new URL("../" + path, import.meta.url), "utf8");
  const code = ts.transpileModule(source, { compilerOptions: {
    module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022,
  } }).outputText;
  const exports = {};
  vm.runInNewContext(code, { exports, URL, URLSearchParams, localStorage,
    require(name) {
      if (!(name in modules)) throw Error("Unexpected import: " + name);
      return modules[name];
    },
  });
  return exports;
}

for (const unwritable of [true, false]) {
  const safe = { id: "safe", type: "url", name: "Safe", icon: "", target: "https://fixture.test", at: 1 };
  const unsafe = { ...safe, id: "private", target: "https://fixture.test/?token=synthetic" };
  const values = new Map([
    ["browser-os-home-shortcuts", JSON.stringify([safe, unsafe])],
    ["browser-os-home-recents-v1", JSON.stringify([safe, unsafe])],
  ]);
  const storage = {
    getItem(key) { return values.get(key) ?? null; },
    setItem(key, value) {
      if (unwritable) throw Error("QuotaExceededError");
      values.set(key, value);
    },
  };
  const homeUi = load("src/utils/homeUi.ts", {}, storage);
  const store = load("src/capabilities/home/state/useHomeStore.ts", {
    pinia: { defineStore: (_id, setup) => setup },
    vue: { reactive: value => value, computed: get => ({ get value() { return get(); } }), ref: value => ({ value }) },
    "../../../bridge": { bridge: {} },
    "../../browser/public": { useBrowserStore: () => ({}) },
    "../../../stores/useLayoutStore": { useLayoutStore: () => ({}) },
    "../../workspace/public": { useFileStore: () => ({}) },
    "../../../utils/homeUi": homeUi,
  }, storage).useHomeStore();
  assert.equal(store.shortcuts.length, 1);
  assert.equal(store.recents.length, 1, "migration write failure must preserve readable safe recents");
  assert.equal(store.recents[0].id, "safe");
  if (!unwritable) {
    for (const value of values.values()) assert.equal(value.includes("synthetic"), false);
  }
}
console.log("HOME_STORAGE_MIGRATION=PASS");
