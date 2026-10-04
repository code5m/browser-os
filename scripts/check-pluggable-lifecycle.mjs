#!/usr/bin/env node
// Execute the real pluggable runtime with component-bearing lifecycle contributions.

import assert from "node:assert/strict";
import { build } from "esbuild";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const tempDir = mkdtempSync(join(ROOT, ".tmp-pluggable-lifecycle-"));
const entry = `
  export { createPluggableRuntime } from ${JSON.stringify(join(ROOT, "src/capability/platform/pluggable.ts"))};
  export { createContributionRegistry } from ${JSON.stringify(join(ROOT, "src/capability/contribution/registry.ts"))};
`;
let failed = 0;

try {
  const built = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
    bundle: true,
    format: "esm",
    platform: "node",
    target: "es2020",
    external: ["vue"],
    write: false,
  });
  const temp = join(tempDir, "runtime.mjs");
  writeFileSync(temp, built.outputFiles[0].text, "utf8");
  const { createPluggableRuntime, createContributionRegistry } = await import(pathToFileURL(temp).href);

  function fixture() {
    const registry = createContributionRegistry();
    const component = { name: "FeaturePanel", render: () => null };
    const counts = { activate: 0, suspend: 0, deactivate: 0 };
    const manifest = {
      id: "feature", version: "1.0.0", displayName: "Feature", description: "Lifecycle test capability",
      maturity: "C3", dependencies: [], optionalDependencies: [], conflicts: [], provides: ["feature.use"], requires: [],
      contributions: [
        { id: "feature.panel", slot: "workbench-main", type: "surface", view: "feature" },
        { id: "feature.navigation", slot: "activity-bar", type: "navigation" },
      ],
      permissions: [], resources: [], persistenceScope: "runtime_only", persistenceSensitive: false,
      activationPolicy: "auto", deactivationPolicy: "graceful", installPolicy: "static", uninstallPolicy: "denied",
      hotPlug: { level: "HP1", enable: true, disable: true, register: false, unregister: false, install: false, uninstall: false, limitationReason: "Static lifecycle fixture" },
      publicContract: [], entrypoint: "src/capabilities/feature/index.ts", semanticOwner: "featureOwner", kind: "feature",
    };
    const definition = {
      id: "feature", name: "Feature", category: "CAPABILITY", provides: ["feature.use"], dependsOn: [], optionalDependencies: [],
      lifecycle: {
        supported: ["ACTIVE", "SUSPENDED"], default: "ACTIVE", activatable: true, resident: false,
        onActivate() {
          counts.activate++;
          registry.registerContribution({ id: "feature.panel", capabilityId: "feature", slot: "workbench-main", type: "surface", view: "feature", label: "Real feature panel", component });
        },
        onSuspend() { counts.suspend++; },
        onDeactivate() { counts.deactivate++; },
      },
      resources: { class: [], suspendable: true, destroyable: true }, permissions: [],
      persistence: { scope: "runtime_only", sensitive: false }, entrypoint: manifest.entrypoint,
      semanticOwner: "featureOwner", governanceStatus: "GOVERNED", status: "COMPATIBILITY_WRAPPED",
    };
    const app = createPluggableRuntime({ catalog: { feature: manifest }, definitions: { feature: definition }, config: { enabled: { feature: true } }, contributions: registry });
    return { app, registry, component, counts };
  }

  function check(name, run) {
    try {
      run();
      console.log(`[PASS] ${name}`);
    } catch (error) {
      failed++;
      console.error(`[FAIL] ${name}: ${error.message}`);
    }
  }

  check("manifest metadata preserves lifecycle component and label", () => {
    const { app, registry, component } = fixture();
    app.activate();
    const [panel] = registry.getBySlot("workbench-main");
    assert.equal(panel.component, component);
    assert.equal(panel.label, "Real feature panel");
    assert.equal(panel.view, "feature");
    assert.equal(registry.getBySlot("activity-bar")[0].id, "feature.navigation");
  });

  check("repeated activate does not duplicate hooks or contributions", () => {
    const { app, registry, counts } = fixture();
    app.activate();
    app.activate();
    assert.equal(counts.activate, 1);
    assert.equal(registry.getBySlot("workbench-main").length, 1);
    assert.equal(registry.getBySlot("activity-bar").length, 1);
  });

  check("repeated deactivate invokes cleanup once and removes every slot", () => {
    const { app, registry, counts } = fixture();
    app.activate();
    app.deactivate("feature");
    app.deactivate("feature");
    assert.deepEqual(counts, { activate: 1, suspend: 1, deactivate: 1 });
    assert.equal(app.runtime.get("feature").enabled, false);
    assert.equal(app.runtime.get("feature").state, "SUSPENDED");
    assert.equal(registry.getBySlot("workbench-main").length, 0);
    assert.equal(registry.getBySlot("activity-bar").length, 0);
    assert.equal(app.events.history().filter((event) => event.type === "capability.deactivated").length, 1);
  });

  check("enable and reactivate restores one component across repeated cycles", () => {
    const { app, registry, component, counts } = fixture();
    app.activate();
    for (let cycle = 0; cycle < 3; cycle++) {
      app.deactivate("feature");
      app.deactivate("feature");
      app.runtime.enable("feature");
      app.runtime.activate("feature");
      const panels = registry.getBySlot("workbench-main");
      assert.equal(panels.length, 1);
      assert.equal(panels[0].component, component);
      assert.equal(app.runtime.get("feature").state, "ACTIVE");
    }
    assert.deepEqual(counts, { activate: 4, suspend: 3, deactivate: 3 });
  });

  console.log(`PLUGGABLE_LIFECYCLE_RESULT=${failed ? "FAIL" : "PASS"}`);
  if (failed) process.exitCode = 1;
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}
