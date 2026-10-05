#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];
const fail = (message) => failures.push(message);
const read = (path) => readFileSync(join(ROOT, path), "utf8");

const promoted = {
  script: {
    owner: "useScriptStore",
    required: [
      "src/capabilities/script/manifest.ts",
      "src/capabilities/script/index.ts",
      "src/capabilities/script/public.ts",
      "src/capabilities/script/state/useScriptStore.ts",
      "src/capabilities/script/ui/ScriptPanel.vue",
    ],
    forbidden: [
      "src/capabilities/workspace/state/useScriptStore.ts",
      "src/capabilities/workspace/ui/ScriptPanel.vue",
    ],
  },
  credential: {
    owner: "KeyringStore",
    required: [
      "src/capabilities/credential/manifest.ts",
      "src/capabilities/credential/index.ts",
      "src/capabilities/credential/public.ts",
      "src/capabilities/credential/ui/CredentialList.vue",
    ],
    forbidden: ["src/capabilities/browser/ui/CredentialList.vue"],
  },
  session: {
    owner: "useSessionStore",
    required: [
      "src/capabilities/session/manifest.ts",
      "src/capabilities/session/index.ts",
      "src/capabilities/session/public.ts",
      "src/capabilities/session/state/useSessionStore.ts",
      "src/capabilities/session/ui/SessionPanel.vue",
    ],
    forbidden: [
      "src/capabilities/browser/state/useSessionStore.ts",
      "src/capabilities/browser/ui/SessionPanel.vue",
    ],
  },
  workbench: {
    owner: "useWorkbenchStore",
    required: [
      "src/capabilities/workbench/manifest.ts",
      "src/capabilities/workbench/index.ts",
      "src/capabilities/workbench/public.ts",
      "src/capabilities/workbench/state/useWorkbenchStore.ts",
      "src/capabilities/workbench/ui/WorkbenchRail.vue",
      "src/capabilities/workbench/ui/WorkbenchCommands.vue",
    ],
    forbidden: [
      "src/components/layout/WorkbenchRail.vue",
      "src/components/layout/WorkbenchCommands.vue",
    ],
  },
};

const generated = read("src/capability/platform/generated-registry.ts");
const capabilities = read("docs/architecture/capability-registry/capabilities.yaml");
const dependencies = read("docs/architecture/capability-registry/dependencies.yaml");

for (const [id, spec] of Object.entries(promoted)) {
  for (const path of spec.required) {
    if (!existsSync(join(ROOT, path))) fail(id + " missing required module file: " + path);
  }
  for (const path of spec.forbidden) {
    if (existsSync(join(ROOT, path))) fail(id + " legacy physical owner still exists: " + path);
  }
  const manifest = read("src/capabilities/" + id + "/manifest.ts");
  if (!manifest.includes('id: "' + id + '"')) fail(id + " manifest id mismatch");
  if (!manifest.includes('semanticOwner: "' + spec.owner + '"')) fail(id + " semantic owner mismatch");
  if (!manifest.includes('maturity: "C2"')) fail(id + " must remain at truthful C2/B-level contract");
  if (!manifest.includes("publicContract: [")) fail(id + " missing public contract");
  if (!generated.includes("/capabilities/" + id + "/manifest")) fail(id + " missing generated manifest registration");
  if (!generated.includes("'" + id + "': definition")) fail(id + " missing generated runtime definition");
  const marker = "\n  - id: " + id + "\n";
  const start = capabilities.indexOf(marker);
  if (start < 0) fail(id + " missing capability registry block");
  else {
    const next = capabilities.indexOf("\n  - id: ", start + marker.length);
    const block = capabilities.slice(start, next < 0 ? capabilities.length : next);
    if (!block.includes("status: COMPATIBILITY_WRAPPED")) fail(id + " registry status not integrated");
  }
}

for (const [name, path] of [
  ["credential UI", "src/capabilities/credential/ui/CredentialList.vue"],
  ["session store", "src/capabilities/session/state/useSessionStore.ts"],
  ["session UI", "src/capabilities/session/ui/SessionPanel.vue"],
]) {
  const source = read(path);
  if (source.includes("useBrowserStore") || source.includes("capabilities/browser") || source.includes("../../browser/public")) {
    fail(name + " must not depend on Browser capability");
  }
}

if (/from:\s*credential[^\n}]*to:\s*browser/.test(dependencies)) fail("credential->browser dependency edge is forbidden");
if (/from:\s*session[^\n}]*to:\s*browser/.test(dependencies)) fail("session->browser dependency edge is forbidden");
if (!/from:\s*workbench[^\n}]*to:\s*browser/.test(dependencies)) fail("workbench->browser dependency edge missing");

const compat = read("src/stores/useWorkbenchStore.ts");
if (!compat.includes('export { useWorkbenchStore } from "../capabilities/workbench/public"')) {
  fail("legacy workbench store path must be a pure public re-export");
}
if (compat.includes("defineStore(") || compat.includes("ref(") || compat.includes("reactive(")) {
  fail("legacy workbench store path must not retain business state");
}

if (failures.length) {
  for (const failure of failures) console.error("[FAIL] " + failure);
  console.error("V2_P0_MATURITY_BOUNDARY_RESULT=FAIL");
  process.exit(1);
}
console.log("V2_P0_MATURITY_BOUNDARY_RESULT=PASS promoted=script,credential,session,workbench");
