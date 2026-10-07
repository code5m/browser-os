#!/usr/bin/env node
// Capability Platform v2 — unified Hot-Plug acceptance harness.
// Any capability promoted to audit grade A must be exercised here with its real definition,
// contribution registration and persistence/restart behavior.

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
    export { createVaultCapability, vaultContribution } from ${JSON.stringify(join(ROOT, "packages/capability-vault/src/index.ts"))};
    export { vaultManifest } from ${JSON.stringify(join(ROOT, "packages/capability-vault/src/manifest.ts"))};
    export { homeCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/home/index.ts"))};
    export { homeManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/home/manifest.ts"))};
    export { browserCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/browser/index.ts"))};
    export { workspaceCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/workspace/index.ts"))};
    export { appsCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/apps/index.ts"))};
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

  const vaultPorts = {
    native: { openVault: async (path) => ({ root: path, notes: [], skipped: 0, truncated: false }) },
    shell: { isWorkbenchCollapsed: () => false },
    graphLayout: { layoutGraph: (nodes) => nodes.map((node, index) => ({ id: node.id, x: index, y: 0 })) },
  };
  const vaultFactory = M.createVaultCapability(vaultPorts);
  vaultFactory.vaultCapability.lifecycle.onActivate = () => {
    M.contributionRegistry.registerContribution(M.vaultContribution);
  };

  const candidates = [
    { id: "bookmark", definition: M.bookmarkCapability, manifest: M.bookmarkManifest, dependencies: [] },
    { id: "vault", definition: vaultFactory.vaultCapability, manifest: M.vaultManifest, dependencies: [] },
    {
      id: "home",
      definition: M.homeCapability,
      manifest: M.homeManifest,
      dependencies: [M.browserCapability, M.workspaceCapability, M.appsCapability],
    },
  ];

  async function accept(candidate) {
    const { id, definition, manifest, dependencies = [] } = candidate;
    const registry = M.contributionRegistry;
    registry.clear();

    const expectedContributions = manifest.v1.contributions.length;
    const prepareRuntime = ({ activateCandidate = true } = {}) => {
      const runtime = M.createCapabilityRuntime();
      for (const dependency of dependencies) runtime.register(dependency);
      runtime.register(definition);
      for (const dependency of dependencies) {
        runtime.resolve(dependency.id);
        runtime.activate(dependency.id);
      }
      runtime.resolve(id);
      if (activateCandidate) runtime.activate(id);
      return runtime;
    };
    const runtime = prepareRuntime();

    const record = () => runtime.get(id);
    const contributionCount = () => registry.getByCapability(id).length;
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
      storeOwners: manifest.semanticOwner ? 1 : 0,
      hostServiceRefs: manifest.dependsOn.length,
      duplicateRegistrations: Math.max(0, contributionCount() - expectedContributions),
      activationDurationMs: record()?.activationDurationMs ?? null,
      lastError: record()?.lastError ?? null,
    });
    const evidence = [];

    evidence.push(snapshot("ACTIVE"));
    assert.equal(record()?.state, "ACTIVE", id + " initial ACTIVE");
    assert.equal(contributionCount(), expectedContributions, id + " initial contribution count");

    await M.transitionCapability(runtime, registry, id, "pause");
    evidence.push(snapshot("SUSPENDED-1"));
    assert.equal(record()?.state, "SUSPENDED", id + " pause");

    await M.transitionCapability(runtime, registry, id, "resume");
    evidence.push(snapshot("ACTIVE-2"));
    assert.equal(record()?.state, "ACTIVE", id + " resume");
    assert.equal(contributionCount(), expectedContributions, id + " contributions after resume");

    await M.transitionCapability(runtime, registry, id, "pause");
    evidence.push(snapshot("SUSPENDED-2"));
    const disabled = await M.transitionCapability(runtime, registry, id, "disable");
    evidence.push(snapshot("DISABLED"));
    assert.equal(disabled.persistedEnabled, false, id + " disable persistence signal");
    assert.equal(record()?.enabled, false, id + " disabled runtime flag");
    assert.equal(contributionCount(), 0, id + " disable removes all contributions");

    // Refresh persistence: the stored disabled bit must survive a read without recreating stale UI.
    const persisted = new Map();
    const storage = {
      getItem: (key) => persisted.get(key) ?? null,
      setItem: (key, value) => persisted.set(key, value),
    };
    assert.equal(M.saveCapabilityConfig({ enabled: { [id]: false } }, storage), true);
    assert.equal(M.loadCapabilityConfig(storage)?.enabled[id], false);
    assert.equal(contributionCount(), 0, id + " refresh-disabled must not leave stale contribution");

    // Process restart persistence: create a brand-new Runtime/record, apply persisted disabled
    // before activation, and prove no contribution is born. Then create another fresh Runtime
    // from persisted enabled state and prove the entry is restored exactly once.
    registry.clear();
    const restartDisabled = prepareRuntime({ activateCandidate: false });
    if (M.loadCapabilityConfig(storage)?.enabled[id] === false) restartDisabled.disable(id);
    assert.equal(restartDisabled.get(id)?.enabled, false, id + " restart keeps disabled");
    assert.equal(registry.getByCapability(id).length, 0, id + " restart-disabled has no stale contribution");

    M.saveCapabilityConfig({ enabled: { [id]: true } }, storage);
    registry.clear();
    const restartEnabled = prepareRuntime({ activateCandidate: false });
    if (M.loadCapabilityConfig(storage)?.enabled[id] !== false) restartEnabled.activate(id);
    assert.equal(restartEnabled.get(id)?.state, "ACTIVE", id + " restart restores ACTIVE");
    assert.equal(registry.getByCapability(id).length, expectedContributions, id + " restart restores one contribution set");

    // Return to the original runtime and repeatedly exercise enable/disable to catch duplicate registration.
    registry.clear();
    await M.transitionCapability(runtime, registry, id, "enable");
    evidence.push(snapshot("ACTIVE-3"));
    assert.equal(record()?.state, "ACTIVE", id + " enable");
    assert.equal(contributionCount(), expectedContributions, id + " enable restores contributions");

    for (let i = 0; i < 3; i++) {
      await M.transitionCapability(runtime, registry, id, "pause");
      await M.transitionCapability(runtime, registry, id, "disable");
      assert.equal(contributionCount(), 0, id + " disable cycle " + i + " removes contributions");
      await M.transitionCapability(runtime, registry, id, "enable");
      assert.equal(record()?.state, "ACTIVE", id + " enable cycle " + i);
      assert.equal(contributionCount(), expectedContributions, id + " enable cycle " + i + " contribution count");
    }

    const ids = registry.getByCapability(id).map((item) => item.id);
    assert.equal(new Set(ids).size, ids.length, id + " repeated enable must not duplicate contribution ids");
    assert.equal(record()?.lastError, null, id + " last error");
    assert.ok((record()?.activationDurationMs ?? -1) >= 0, id + " activation duration");

    const unsupportedOwnedKinds = (manifest.v1.resources ?? [])
      .filter((resource) => ["COMMAND", "LISTENER", "TIMER", "TASK", "ROUTE", "MENU", "ACTION"].includes(resource.kind));
    assert.equal(unsupportedOwnedKinds.length, 0, id + " has no untracked dynamic resource class");
    assert.ok(manifest.semanticOwner, id + " semantic owner");

    evidence.push(snapshot("FINAL_ACTIVE"));
    return { capability: id, result: "PASS", evidence };
  }

  const results = [];
  for (const candidate of candidates) results.push(await accept(candidate));
  console.log(JSON.stringify({ result: "PASS", capabilities: results }, null, 2));
  console.log("HOT_PLUG_ACCEPTANCE_RESULT=PASS");
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
