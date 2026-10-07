#!/usr/bin/env node
import assert from "node:assert/strict";
import { build } from "esbuild";
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const tmp = mkdtempSync(join(ROOT, ".tmp-view-fallback-"));
try {
  const built = await build({
    entryPoints: [join(ROOT, "src/capability/platform/view-availability.ts")],
    bundle: true, format: "esm", platform: "node", target: "es2020", write: false,
  });
  const out = join(tmp, "fallback.mjs");
  writeFileSync(out, built.outputFiles[0].text, "utf8");
  const M = await import(pathToFileURL(out).href);
  const resolve = M.resolveAvailableWorkbenchView;

  assert.equal(resolve("files", ["home", "files"]), "files");
  assert.equal(resolve("files", ["home", "graph"]), "home");
  assert.equal(resolve("files", ["graph", "apps"]), "graph");
  assert.equal(resolve("browser", ["home"]), "browser");
  assert.equal(resolve("grid", ["home"]), "grid");
  assert.equal(resolve("term", ["home"]), "term");
  assert.equal(resolve("unknown", []), "unknown");

  const mainArea = readFileSync(join(ROOT, "src/components/layout/MainArea.vue"), "utf8");
  assert.ok(mainArea.includes("resolveAvailableWorkbenchView(current, available)"),
    "MainArea must use canonical availability fallback");
  console.log("HOT_PLUG_VIEW_FALLBACK_RESULT=PASS");
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
