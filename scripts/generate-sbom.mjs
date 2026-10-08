#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function npmComponents(lock) {
  const out = [];
  for (const [p, meta] of Object.entries(lock.packages || {})) {
    if (!p || !meta?.version || !p.includes("node_modules/")) continue;
    const name = meta.name || p.split("node_modules/").at(-1);
    out.push({ type: "library", name, version: meta.version, purl: `pkg:npm/${encodeURIComponent(name)}@${meta.version}` });
  }
  return out;
}

function cargoComponents(text) {
  const blocks = text.split(/\n\[\[package\]\]\n/).slice(1);
  return blocks.map((block) => {
    const name = block.match(/^name = "([^"]+)"/m)?.[1];
    const version = block.match(/^version = "([^"]+)"/m)?.[1];
    const checksum = block.match(/^checksum = "([^"]+)"/m)?.[1];
    if (!name || !version) return null;
    return {
      type: "library",
      name,
      version,
      purl: `pkg:cargo/${encodeURIComponent(name)}@${version}`,
      ...(checksum ? { hashes: [{ alg: "SHA-256", content: checksum }] } : {}),
    };
  }).filter(Boolean);
}

function bom(name, components) {
  const unique = new Map();
  for (const c of components) unique.set(c.purl, c);
  return {
    bomFormat: "CycloneDX",
    specVersion: "1.6",
    version: 1,
    metadata: { component: { type: "application", name: "BrowserOS", version: "0.1.0" }, tools: [{ vendor: "BrowserOS", name }] },
    components: [...unique.values()].sort((a, b) => a.purl.localeCompare(b.purl)),
  };
}

if (process.argv.includes("--self-test")) {
  const n = npmComponents({ packages: { "": {}, "node_modules/a": { version: "1.0.0" } } });
  const c = cargoComponents('\n[[package]]\nname = "serde"\nversion = "1.0.0"\nchecksum = "abc"\n');
  const ok = n.length === 1 && c.length === 1 && bom("test", [...n, ...c]).components.length === 2;
  console.log(`SBOM_SELF_TEST=${ok ? "PASS" : "FAIL"}`);
  process.exit(ok ? 0 : 1);
}

const outIdx = process.argv.indexOf("--output-dir");
const outDir = path.resolve(ROOT, outIdx >= 0 ? process.argv[outIdx + 1] : "artifacts/sbom");
fs.mkdirSync(outDir, { recursive: true });
const npmLock = JSON.parse(fs.readFileSync(path.join(ROOT, "package-lock.json"), "utf8"));
const cargoLock = fs.readFileSync(path.join(ROOT, "src-tauri/Cargo.lock"), "utf8");
const nodeBom = bom("BrowserOS lockfile SBOM generator", npmComponents(npmLock));
const rustBom = bom("BrowserOS lockfile SBOM generator", cargoComponents(cargoLock));
fs.writeFileSync(path.join(outDir, "node.cdx.json"), JSON.stringify(nodeBom, null, 2) + "\n");
fs.writeFileSync(path.join(outDir, "rust.cdx.json"), JSON.stringify(rustBom, null, 2) + "\n");
console.log(`SBOM_RESULT=PASS node=${nodeBom.components.length} rust=${rustBom.components.length}`);
