#!/usr/bin/env node
import assert from "node:assert/strict";
import { build } from "esbuild";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const tmp = mkdtempSync(join(ROOT, ".tmp-v3-a-promotions-"));
const expectedA = ["bookmark", "vault", "home", "graph", "workspace", "clipboard", "database"];

try {
  const entry = [
    ["bookmark", "src/capabilities/bookmark/manifest.ts", "bookmarkManifest"],
    ["vault", "packages/capability-vault/src/manifest.ts", "vaultManifest"],
    ["home", "src/capabilities/home/manifest.ts", "homeManifest"],
    ["graph", "src/capabilities/graph/manifest.ts", "graphManifest"],
    ["workspace", "src/capabilities/workspace/manifest.ts", "workspaceManifest"],
    ["clipboard", "packages/capability-clipboard/src/manifest.ts", "clipboardManifest"],
    ["database", "src/capabilities/database/manifest.ts", "databaseManifest"],
  ].map(([id, path, symbol]) =>
    `export { ${symbol} as ${id} } from ${JSON.stringify(join(ROOT, path))};`
  ).join("\n");

  const built = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: "ts" },
    bundle: true,
    format: "esm",
    platform: "node",
    target: "es2020",
    write: false,
  });
  const out = join(tmp, "promotions.mjs");
  writeFileSync(out, built.outputFiles[0].text, "utf8");
  const M = await import(pathToFileURL(out).href);

  for (const id of expectedA) {
    const manifest = M[id];
    assert.ok(manifest, `${id}: manifest present`);
    assert.equal(manifest.v1?.maturity, "C3", `${id}: maturity must remain C3`);
    assert.equal(manifest.v1?.hotPlug?.level, "HP2", `${id}: hotPlug must remain HP2`);
    assert.equal(manifest.v1?.hotPlug?.enable, true, `${id}: enable`);
    assert.equal(manifest.v1?.hotPlug?.disable, true, `${id}: disable`);
    assert.equal(manifest.v1?.hotPlug?.register, true, `${id}: register`);
    assert.equal(manifest.v1?.hotPlug?.unregister, true, `${id}: unregister`);
    assert.equal(manifest.lifecycle?.resident, false, `${id}: A capability must be non-resident`);
    assert.ok(manifest.lifecycle?.supported?.includes("SUSPENDED"), `${id}: suspend lifecycle`);
    assert.equal(manifest.resources?.suspendable, true, `${id}: resources suspendable`);
    assert.ok(
      manifest.v1?.maturityEvidence?.includes("scripts/check-hot-plug-acceptance.mjs"),
      `${id}: unified hot-plug evidence required`,
    );
  }

  console.log(`V3_A_PROMOTION_RESULT=PASS A=${expectedA.length} ids=${expectedA.join(",")}`);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
