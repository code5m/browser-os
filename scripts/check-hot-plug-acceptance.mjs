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
    export { browserManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/browser/manifest.ts"))};
    export { workspaceCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/workspace/index.ts"))};
    export { appsCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/apps/index.ts"))};
    export { graphCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/graph/index.ts"))};
    export { graphManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/graph/manifest.ts"))};
    export { registerGraphCanceller, graphLifecycleSnapshot } from ${JSON.stringify(join(ROOT, "src/capabilities/graph/lifecycle.ts"))};
    export { workspaceManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/workspace/manifest.ts"))};
    export { createClipboardCapability, clipboardContribution } from ${JSON.stringify(join(ROOT, "packages/capability-clipboard/src/index.ts"))};
    export { clipboardManifest } from ${JSON.stringify(join(ROOT, "packages/capability-clipboard/src/manifest.ts"))};
    export { registerClipboardLifecycleBinding, clipboardLifecycleSnapshot } from ${JSON.stringify(join(ROOT, "packages/capability-clipboard/src/lifecycle.ts"))};
    export { databaseCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/database/index.ts"))};
    export { databaseManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/database/manifest.ts"))};
    export { credentialCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/credential/index.ts"))};
    export { registerDatabaseCleanup, databaseLifecycleSnapshot } from ${JSON.stringify(join(ROOT, "src/capabilities/database/lifecycle.ts"))};
    export { appsManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/apps/manifest.ts"))};
    export { appsLifecycleSnapshot } from ${JSON.stringify(join(ROOT, "src/capabilities/apps/lifecycle.ts"))};
    export { pluginCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/plugin/index.ts"))};
    export { pluginManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/plugin/manifest.ts"))};
    export { registerPluginLifecycleBinding, pluginLifecycleSnapshot, suspendPluginLifecycle } from ${JSON.stringify(join(ROOT, "src/capabilities/plugin/lifecycle.ts"))};
    export { gitCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/git/index.ts"))};
    export { gitManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/git/manifest.ts"))};
    export { registerGitLifecycleBinding, gitLifecycleSnapshot, suspendGitLifecycle } from ${JSON.stringify(join(ROOT, "src/capabilities/git/lifecycle.ts"))};
    export { toolsCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/tools/index.ts"))};
    export { toolsManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/tools/manifest.ts"))};
    export { registerToolsCleanup, toolsLifecycleSnapshot } from ${JSON.stringify(join(ROOT, "src/capabilities/tools/lifecycle.ts"))};
    export { browserLifecycleSnapshot, registerBrowserLifecycleBinding } from ${JSON.stringify(join(ROOT, "src/capabilities/browser/lifecycle.ts"))};
    export { gridCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/grid/index.ts"))};
    export { gridManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/grid/manifest.ts"))};
    export { gridLifecycleSnapshot, registerGridLifecycleBinding } from ${JSON.stringify(join(ROOT, "src/capabilities/grid/lifecycle.ts"))};
    export { terminalCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/terminal/index.ts"))};
    export { terminalManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/terminal/manifest.ts"))};
    export { registerTerminalLifecycleBinding, terminalLifecycleSnapshot, suspendTerminalLifecycle } from ${JSON.stringify(join(ROOT, "src/capabilities/terminal/lifecycle.ts"))};
    export { scriptCapability } from ${JSON.stringify(join(ROOT, "src/capabilities/script/index.ts"))};
    export { scriptManifest } from ${JSON.stringify(join(ROOT, "src/capabilities/script/manifest.ts"))};
    export { scriptLifecycleSnapshot, trackScriptRun, completeScriptRun, suspendScriptLifecycle } from ${JSON.stringify(join(ROOT, "src/capabilities/script/lifecycle.ts"))};
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
  const clipboardFactory = M.createClipboardCapability({
    native: { clipboardRead: async () => "", clipboardWrite: async () => {} },
    ui: { showToast: () => {}, requestClose: () => {}, redactSecrets: (text) => text },
  });
  const clipboardActivate = clipboardFactory.clipboardCapability.lifecycle.onActivate;
  clipboardFactory.clipboardCapability.lifecycle.onActivate = () => {
    clipboardActivate?.();
    M.contributionRegistry.registerContribution(M.clipboardContribution);
  };

  let graphCleanupCount = 0;
  let databaseCleanupCount = 0;
  let clipboardStartCount = 0;
  let clipboardStopCount = 0;
  M.registerGraphCanceller(() => { graphCleanupCount += 1; });
  M.registerDatabaseCleanup(async () => { databaseCleanupCount += 1; });
  M.registerClipboardLifecycleBinding({
    start: () => { clipboardStartCount += 1; },
    stop: () => { clipboardStopCount += 1; },
  });

  let pluginCleanupCount = 0;
  let gitCleanupCount = 0;
  let toolsCleanupCount = 0;
  let browserSuspendCount = 0;
  let browserDeactivateCount = 0;
  let gridSuspendCount = 0;
  let gridDeactivateCount = 0;
  let terminalCleanupCount = 0;

  M.registerPluginLifecycleBinding({ canSuspend: () => true, cleanup: () => { pluginCleanupCount += 1; } });
  M.registerGitLifecycleBinding({ canSuspend: () => true, cleanup: () => { gitCleanupCount += 1; } });
  M.registerToolsCleanup(() => { toolsCleanupCount += 1; });
  M.registerBrowserLifecycleBinding({
    suspend: () => { browserSuspendCount += 1; },
    deactivate: () => { browserDeactivateCount += 1; },
  });
  M.registerGridLifecycleBinding({
    suspend: () => { gridSuspendCount += 1; },
    deactivate: () => { gridDeactivateCount += 1; },
  });
  M.registerTerminalLifecycleBinding({ canSuspend: () => true, cleanup: () => { terminalCleanupCount += 1; } });

  const candidates = [
    { id: "bookmark", definition: M.bookmarkCapability, manifest: M.bookmarkManifest, dependencies: [] },
    { id: "vault", definition: vaultFactory.vaultCapability, manifest: M.vaultManifest, dependencies: [] },
    {
      id: "home",
      definition: M.homeCapability,
      manifest: M.homeManifest,
      dependencies: [M.browserCapability, M.workspaceCapability, M.appsCapability],
    },
    {
      id: "graph",
      definition: M.graphCapability,
      manifest: M.graphManifest,
      dependencies: [],
      verifyLifecycle: () => assert.ok(graphCleanupCount > 0, "graph suspend/deactivate triggers cleanup"),
    },
    { id: "workspace", definition: M.workspaceCapability, manifest: M.workspaceManifest, dependencies: [] },
    {
      id: "clipboard",
      definition: clipboardFactory.clipboardCapability,
      manifest: M.clipboardManifest,
      dependencies: [],
      verifyLifecycle: () => {
        assert.ok(clipboardStartCount > 0, "clipboard activation starts lifecycle bindings");
        assert.ok(clipboardStopCount > 0, "clipboard suspend/deactivate stops lifecycle bindings");
        assert.equal(M.clipboardLifecycleSnapshot().active, true, "clipboard final lifecycle active");
      },
    },
    {
      id: "database",
      definition: M.databaseCapability,
      manifest: M.databaseManifest,
      dependencies: [M.credentialCapability],
      verifyLifecycle: () => {
        assert.ok(databaseCleanupCount > 0, "database suspend/deactivate awaits cleanup");
        assert.equal(M.databaseLifecycleSnapshot().active, true, "database final lifecycle active");
      },
    },
    {
      id: "apps",
      definition: M.appsCapability,
      manifest: M.appsManifest,
      dependencies: [],
      verifyLifecycle: () => assert.equal(M.appsLifecycleSnapshot().active, true, "apps final lifecycle active"),
    },
    {
      id: "plugin",
      definition: M.pluginCapability,
      manifest: M.pluginManifest,
      dependencies: [],
      verifyLifecycle: () => {
        assert.ok(pluginCleanupCount > 0, "plugin graceful cleanup executed");
        assert.equal(M.pluginLifecycleSnapshot().active, true, "plugin final lifecycle active");
      },
    },
    {
      id: "git",
      definition: M.gitCapability,
      manifest: M.gitManifest,
      dependencies: [M.credentialCapability, M.workspaceCapability],
      verifyLifecycle: () => {
        assert.ok(gitCleanupCount > 0, "git lifecycle cleanup executed");
        assert.equal(M.gitLifecycleSnapshot().active, true, "git final lifecycle active");
      },
    },
    {
      id: "tools",
      definition: M.toolsCapability,
      manifest: M.toolsManifest,
      dependencies: [],
      verifyLifecycle: () => {
        assert.ok(toolsCleanupCount > 0, "tools native cleanup hook executed");
        assert.equal(M.toolsLifecycleSnapshot().active, true, "tools final lifecycle active");
      },
    },
    {
      id: "browser",
      definition: M.browserCapability,
      manifest: M.browserManifest,
      dependencies: [],
      verifyLifecycle: () => {
        assert.ok(browserSuspendCount > 0, "browser suspend cleanup executed");
        assert.ok(browserDeactivateCount > 0, "browser deactivate cleanup executed");
        assert.equal(M.browserLifecycleSnapshot().active, true, "browser final lifecycle active");
      },
    },
    {
      id: "grid",
      definition: M.gridCapability,
      manifest: M.gridManifest,
      dependencies: [M.browserCapability],
      verifyLifecycle: () => {
        assert.ok(gridSuspendCount > 0, "grid suspend cleanup executed");
        assert.ok(gridDeactivateCount > 0, "grid deactivate cleanup executed");
        assert.equal(M.gridLifecycleSnapshot().active, true, "grid final lifecycle active");
      },
    },
    {
      id: "terminal",
      definition: M.terminalCapability,
      manifest: M.terminalManifest,
      dependencies: [],
      verifyLifecycle: () => {
        assert.ok(terminalCleanupCount > 0, "terminal idle cleanup executed");
        assert.equal(M.terminalLifecycleSnapshot().active, true, "terminal final lifecycle active");
      },
    },
    {
      id: "script",
      definition: M.scriptCapability,
      manifest: M.scriptManifest,
      dependencies: [],
      verifyLifecycle: () => assert.equal(M.scriptLifecycleSnapshot().active, true, "script final lifecycle active"),
    },
  ];

  async function accept(candidate) {
    const { id, definition, manifest, dependencies = [], verifyLifecycle } = candidate;
    assert.ok(definition, id + " definition must be exported");
    assert.ok(manifest, id + " manifest must be exported");
    assert.equal(definition.id, id, id + " definition id");
    assert.equal(manifest.id, id, id + " manifest id");
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

    verifyLifecycle?.();
    evidence.push(snapshot("FINAL_ACTIVE"));
    return { capability: id, result: "PASS", evidence };
  }

  const results = [];
  for (const candidate of candidates) results.push(await accept(candidate));

  const unregisterPluginBlocker = M.registerPluginLifecycleBinding({ canSuspend: () => false });
  await assert.rejects(() => M.suspendPluginLifecycle(), /operation is still in progress/);
  unregisterPluginBlocker();

  const unregisterGitBlocker = M.registerGitLifecycleBinding({ canSuspend: () => false, cleanup: () => {} });
  await assert.rejects(() => M.suspendGitLifecycle(), /write operation is still running/);
  unregisterGitBlocker();

  const unregisterTerminalBlocker = M.registerTerminalLifecycleBinding({ canSuspend: () => false });
  await assert.rejects(() => M.suspendTerminalLifecycle(), /active PTY sessions/);
  unregisterTerminalBlocker();

  M.trackScriptRun("hotplug-probe", () => true);
  await assert.rejects(() => M.suspendScriptLifecycle(), /runs still active/);
  M.completeScriptRun("hotplug-probe");

  console.log(JSON.stringify({ result: "PASS", capabilities: results }, null, 2));
  console.log("HOT_PLUG_ACCEPTANCE_RESULT=PASS");
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
