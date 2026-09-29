#!/usr/bin/env node
// Pluggable Capability Runtime 门禁：Manifest + enabled config + dependency + contribution + lifecycle。

import { build } from "esbuild";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { writeFileSync, unlinkSync } from "node:fs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const entry = `
  export * from '${join(ROOT, "src/capability/platform/pluggable.ts").replace(/\\/g, "/")}';
  export * from '${join(ROOT, "src/capability/platform/config.ts").replace(/\\/g, "/")}';
  export * from '${join(ROOT, "src/capability/contribution/registry.ts").replace(/\\/g, "/")}';
`;
const built = await build({
  stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
  bundle: true,
  format: "esm",
  platform: "node",
  target: "es2020",
  external: ["vue"],
  write: false,
});
const temp = join(ROOT, ".tmp-pluggable-runtime.mjs");
writeFileSync(temp, built.outputFiles[0].text, "utf8");

try {
  const { createPluggableRuntime, configFromEnv, createContributionRegistry } =
    await import(pathToFileURL(temp).href);
  const contribution = (id) => ({ id: `${id}.panel`, capabilityId: id, type: "surface", slot: "workbench-main" });
  const manifest = (id, dependencies = []) => ({
    id,
    version: "1.0.0",
    displayName: id,
    description: `${id} test capability`,
    maturity: "C2",
    dependencies,
    optionalDependencies: [],
    conflicts: [],
    provides: [`${id}.use`],
    requires: [],
    contributions: [{ id: `${id}.panel`, slot: "workbench-main", type: "surface" }],
    permissions: [],
    resources: [],
    persistenceScope: "runtime_only",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "denied",
    hotPlug: { level: "HP1", enable: true, disable: true, register: false, unregister: false, install: false, uninstall: false, limitationReason: "static test" },
    publicContract: [],
    entrypoint: `src/capabilities/${id}/index.ts`,
    semanticOwner: null,
  });
  const definition = (id, dep = []) => ({
    id,
    name: id,
    category: "CAPABILITY",
    provides: [`${id}.use`],
    dependsOn: dep,
    optionalDependencies: [],
    lifecycle: { supported: ["ACTIVE", "SUSPENDED"], default: "ACTIVE", activatable: true, resident: false,
      onActivate: () => registry.registerContribution(contribution(id)) },
    resources: { class: [], suspendable: true, destroyable: true },
    permissions: [],
    persistence: { scope: "runtime_only", sensitive: false },
    entrypoint: `src/capabilities/${id}/index.ts`,
    semanticOwner: "testOwner",
    governanceStatus: "GOVERNED",
    status: "COMPATIBILITY_WRAPPED",
  });

  const registry = createContributionRegistry();
  const catalog = { core: manifest("core"), feature: manifest("feature", ["core"]) };
  const definitions = { core: definition("core"), feature: definition("feature", ["core"]) };
  const app = createPluggableRuntime({ catalog, definitions, config: { enabled: { feature: true } }, contributions: registry });
  if (app.assembly.activationOrder.join(",") !== "core,feature") throw new Error("依赖激活顺序错误");
  app.activate();
  if (app.runtime.inspect().filter((x) => x.state === "ACTIVE").length !== 2) throw new Error("enabled 配置未激活完整依赖闭包");
  if (registry.getBySlot("workbench-main").length !== 2) throw new Error("activate 未注册全部 contribution");
  app.deactivate("feature");
  if (registry.getBySlot("workbench-main").some((x) => x.capabilityId === "feature")) throw new Error("deactivate 未摘除 contribution");

  const disabled = createPluggableRuntime({ catalog, definitions, config: { enabled: { core: true, feature: false } } });
  if (disabled.assembly.resolved.join(",") !== "core") throw new Error("enabled=false 未排除能力");

  const env = configFromEnv({ VITE_CAPABILITY_ENABLED: "feature,core", VITE_CAPABILITY_DISABLED: "feature" });
  if (!env || env.enabled.feature !== false || env.enabled.core !== true) throw new Error("环境配置解析错误");

  let rejected = false;
  try { createPluggableRuntime({ catalog: { feature: manifest("feature", ["missing"]) }, definitions, config: { enabled: { feature: true } } }); }
  catch { rejected = true; }
  if (!rejected) throw new Error("缺失强依赖未拒绝");
  console.log("PLUGGABLE_RUNTIME_RESULT=PASS");
} finally {
  try { unlinkSync(temp); } catch {}
}
