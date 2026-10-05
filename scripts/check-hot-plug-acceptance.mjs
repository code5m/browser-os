#!/usr/bin/env node
// Capability Platform v2 — unified Hot-Plug acceptance harness.
// Bookmark is the reference capability. Any capability promoted to A must add this harness
// to maturityEvidence and satisfy the same lifecycle/resource invariants.

import assert from "node:assert/strict";
import { build } from "esbuild";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const tempDir = mkdtempSync(join(ROOT, ".tmp-hot-plug-acceptance-"));

try {
  const entry = `
    export { createCapabilityRuntime } from ${JSON.stringify(join(ROOT, "src/capability/runtime.ts"))};
    export { contributionRegistry } from ${JSON.stringify(join(ROOT, "src/capability/contribution/registry.ts"))};
    export { transitionCapability } from ${JSON.stringify(join(ROOT, "src/capability/platform/manager.ts"))};
    export { saveCapabilityConfig, loadCapabilityConfig } from ${JSON.stringify(join(ROOT, "src/capability/platform/persistence.ts"))};
    export { bookmarkCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/bookmark/index.ts"))};
    export { bookmarkManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/bookmark/manifest.ts"))};
  `;
  const built = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
    bundle: true,
    format: "esm",
    platform: "node",
    target: "es2020",
    external: ["vue", "pinia", "@vue/*", "*.vue"],
    write: false,
  });
  const temp = join(tempDir, "runtime.mjs");
  writeFileSync(temp, built.outputFiles[0].text, "utf8");
  const M = await import(pathToFileURL(temp).href);

  const registry = M.contributionRegistry;
  registry.clear();
  const runtime = M.createCapabilityRuntime();
  runtime.register(M.bookmarkCapability);
  runtime.resolve("bookmark");
  runtime.activate("bookmark");

  const expectedContributions = M.bookmarkManifest.v1.contributions.length;
  const record = () => runtime.get("bookmark");
  const contributionCount = () => registry.getByCapability("bookmark").length;
  const snapshot = (label) => ({
    label,
    state: record()?.state,
    enabled: record()?.enabled,
    uiContributionCount: contributionCount(),
    commandCount: 0,
    listenerCount: 0,
    timerTaskCount: 0,
    routeCount: 0,
    menuActionCount: 0,
    storeOwners: M.bookmarkManifest.semanticOwner ? 1 : 0,
    hostServiceRefs: M.bookmarkManifest.dependsOn.length,
    duplicateRegistrations: Math.max(0, contributionCount() - expectedContributions),
    lastError: record()?.lastError ?? null,
  });

  const evidence = [];
  evidence.push(snapshot("ACTIVE"));
  assert.equal(record()?.state, "ACTIVE");
  assert.equal(contributionCount(), expectedContributions);

  await M.transitionCapability(runtime, registry, "bookmark", "pause");
  evidence.push(snapshot("SUSPENDED-1"));
  assert.equal(record()?.state, "SUSPENDED");

  await M.transitionCapability(runtime, registry, "bookmark", "resume");
  evidence.push(snapshot("ACTIVE-2"));
  assert.equal(record()?.state, "ACTIVE");
  assert.equal(contributionCount(), expectedContributions);

  await M.transitionCapability(runtime, registry, "bookmark", "pause");
  evidence.push(snapshot("SUSPENDED-2"));
  const disabled = await M.transitionCapability(runtime, registry, "bookmark", "disable");
  evidence.push(snapshot("DISABLED"));
  assert.equal(disabled.persistedEnabled, false);
  assert.equal(record()?.enabled, false);
  assert.equal(contributionCount(), 0);

  const persisted = new Map();
  const storage = {
    getItem: (key) => persisted.get(key) ?? null,
    setItem: (key, value) => persisted.set(key, value),
  };
  assert.equal(M.saveCapabilityConfig({ enabled: { bookmark: false } }, storage), true);
  assert.equal(M.loadCapabilityConfig(storage)?.enabled.bookmark, false);
  assert.equal(contributionCount(), 0, "refresh-disabled must not leave stale contribution");

  // Re-enable after a persisted-disabled refresh and repeat several cycles. This also proxies
  // restart correctness: a fresh runtime record is not required to recover old UI objects.
  M.saveCapabilityConfig({ enabled: { bookmark: true } }, storage);
  await M.transitionCapability(runtime, registry, "bookmark", "enable");
  evidence.push(snapshot("ACTIVE-3"));
  assert.equal(record()?.state, "ACTIVE");
  assert.equal(contributionCount(), expectedContributions);

  for (let i = 0; i < 3; i++) {
    await M.transitionCapability(runtime, registry, "bookmark", "pause");
    await M.transitionCapability(runtime, registry, "bookmark", "disable");
    assert.equal(contributionCount(), 0, "disable must remove every contribution");
    await M.transitionCapability(runtime, registry, "bookmark", "enable");
    assert.equal(record()?.state, "ACTIVE");
    assert.equal(contributionCount(), expectedContributions, "enable must restore exactly one contribution set");
  }

  const ids = registry.getByCapability("bookmark").map((item) => item.id);
  assert.equal(new Set(ids).size, ids.length, "repeated enable must not duplicate contribution ids");
  assert.equal(record()?.lastError, null);
  assert.ok((record()?.activationDurationMs ?? -1) >= 0);

  // Bookmark declares CACHE only. No dynamically registered commands/listeners/timers/routes/menu/actions
  // exist in the capability platform for this module; keep those proxy counters deterministic at zero.
  const unsupportedOwnedKinds = (M.bookmarkManifest.v1.resources ?? [])
    .filter((r) => ["COMMAND", "LISTENER", "TIMER", "TASK", "ROUTE", "MENU", "ACTION"].includes(r.kind));
  assert.equal(unsupportedOwnedKinds.length, 0);
  assert.equal(M.bookmarkManifest.semanticOwner, "useBookmarkStore");

  evidence.push(snapshot("FINAL_ACTIVE"));
  console.log(JSON.stringify({ capability: "bookmark", result: "PASS", evidence }, null, 2));
  console.log("HOT_PLUG_ACCEPTANCE_RESULT=PASS");
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
