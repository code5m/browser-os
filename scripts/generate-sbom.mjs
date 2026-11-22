#!/usr/bin/env node
import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { verifyVendoredGlib } from "./check-vendored-glib.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const RA_ROOT = "vendor/windows-rust-analyzer";
const RA_COMMIT = "0ef1bc386059596dada8fb65b50425aab2ea637a";

function sha256(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function npmComponents(lock) {
  const out = [];
  for (const [p, meta] of Object.entries(lock.packages || {})) {
    if (!p || !meta?.version || !p.includes("node_modules/")) continue;
    const name = meta.name || p.split("node_modules/").at(-1);
    out.push({ type: "library", name, version: meta.version, purl: "pkg:npm/" + encodeURIComponent(name) + "@" + meta.version });
  }
  return out;
}

function cargoComponents(text) {
  const blocks = text.replace(/\r\n/g, "\n").split(/\n\[\[package\]\]\n/).slice(1);
  return blocks.map((block) => {
    const name = block.match(/^name = "([^"]+)"/m)?.[1];
    const version = block.match(/^version = "([^"]+)"/m)?.[1];
    const checksum = block.match(/^checksum = "([^"]+)"/m)?.[1];
    const source = block.match(/^source = "([^"]+)"/m)?.[1];
    if (!name || !version) return null;
    return {
      type: "library", name, version,
      purl: "pkg:cargo/" + encodeURIComponent(name) + "@" + version,
      properties: [{ name: "browseros:cargo-source", value: source || "path-or-workspace" }],
      ...(checksum ? { hashes: [{ alg: "SHA-256", content: checksum }] } : {}),
    };
  }).filter(Boolean);
}

function annotateGlib(components, verified) {
  const matches = components.filter(component => component.name === "glib");
  assert.equal(matches.length, 1, "SBOM_GLIB_COUNT");
  const component = matches[0];
  assert.equal(component.version, "0.18.5", "SBOM_GLIB_VERSION");
  assert.equal(component.hashes, undefined, "SBOM_GLIB_MUST_BE_LOCAL");
  const origin = verified.provenance;
  component.licenses = [{ license: { id: "MIT" } }];
  component.externalReferences = [
    { type: "vcs", url: origin.upstream.repository + "/commit/" + origin.upstream.commit },
    { type: "documentation", url: origin.patch.url },
  ];
  component.properties.push(
    { name: "browseros:modified", value: "true" },
    { name: "browseros:source-kind", value: "vendored-git-backport" },
    { name: "browseros:upstream-revision", value: origin.upstream.commit },
    { name: "browseros:backport-commit", value: origin.patch.commit },
    { name: "browseros:upstream-advisory", value: origin.patch.advisory },
    { name: "browseros:registry-origin-checksum", value: origin.registryOrigin.checksum },
    { name: "browseros:source-digest", value: verified.sourceDigest },
    { name: "browseros:source-digest-format", value: "SHA256 of sorted relative-path + NUL + SHA256(file bytes) + LF records" },
    { name: "browseros:provenance-git-blob", value: verified.provenanceGitBlob },
    { name: "browseros:advisory-policy", value: "Registry-origin projection scanned separately; no advisory suppression" },
  );
}

function verifySidecar(source, lock, patch) {
  assert.equal(source.schema_version, 1, "SBOM_RA_SCHEMA");
  assert.equal(source.source_repository, "https://github.com/rust-lang/rust", "SBOM_RA_REPOSITORY");
  assert.equal(source.source_commit, RA_COMMIT, "SBOM_RA_SOURCE");
  assert.equal(source.source_subdirectory, "src/tools/rust-analyzer", "SBOM_RA_SUBDIRECTORY");
  assert.equal(source.target, "x86_64-pc-windows-msvc", "SBOM_RA_TARGET");
  assert.equal(source.lock_file, "Cargo.lock", "SBOM_RA_LOCK_PATH");
  assert.equal(source.patch_file, "metadata-stdin.patch", "SBOM_RA_PATCH_PATH");
  assert.equal(source.lock_sha256, sha256(lock), "SBOM_RA_LOCK_DIGEST");
  assert.equal(source.patch_sha256, sha256(patch), "SBOM_RA_PATCH_DIGEST");
  assert.ok(typeof source.runtime_id === "string" && source.runtime_id.length > 0, "SBOM_RA_RUNTIME");
  assert.equal(source.licenses.length, 2, "SBOM_RA_LICENSES");
  for (const name of ["LICENSE-MIT", "LICENSE-APACHE"]) {
    const entry = source.licenses.filter(item => item.file === name);
    assert.equal(entry.length, 1, "SBOM_RA_LICENSE_NAME");
    assert.equal(entry[0].source, "src/tools/rust-analyzer/" + name, "SBOM_RA_LICENSE_SOURCE");
    assert.match(entry[0].sha256, /^[a-f0-9]{64}$/, "SBOM_RA_LICENSE_DIGEST");
  }
  return source;
}

function readSidecar(root) {
  const directory = path.join(root, RA_ROOT);
  const source = JSON.parse(fs.readFileSync(path.join(directory, "source.json"), "utf8"));
  // Fixed paths only. The runtime build/package gate verifies actual license
  // bytes; this source inventory records the same declared upstream hash pins.
  const lock = fs.readFileSync(path.join(directory, "Cargo.lock"));
  const patch = fs.readFileSync(path.join(directory, "metadata-stdin.patch"));
  verifySidecar(source, lock, patch);
  return { source, lock: lock.toString("utf8") };
}

function bom(name, components, application = { type: "application", name: "BrowserOS", version: "0.1.0" }) {
  const unique = new Map();
  for (const c of components) unique.set(c.purl, c);
  return {
    bomFormat: "CycloneDX",
    specVersion: "1.6",
    version: 1,
    metadata: {
      component: application,
      tools: [{ vendor: "BrowserOS", name }],
      properties: [{ name: "browseros:inventory-scope", value: "Lockfile source inventory across targets; not compiled reachability or binary attestation" }],
    },
    components: [...unique.values()].sort((a, b) => a.purl.localeCompare(b.purl)),
  };
}

if (process.argv.includes("--self-test")) {
  const n = npmComponents({ packages: { "": {}, "node_modules/a": { version: "1.0.0" } } });
  const c = cargoComponents('\n[[package]]\nname = "serde"\nversion = "1.0.0"\nchecksum = "abc"\n');
  assert.equal(n.length, 1);
  assert.equal(c.length, 1);
  assert.equal(bom("test", [...n, ...c]).components.length, 2);
  const lock = Buffer.from("checked-in-lock\n");
  const patch = Buffer.from("checked-in-patch\n");
  const source = {
    schema_version: 1, source_repository: "https://github.com/rust-lang/rust",
    source_commit: RA_COMMIT, source_subdirectory: "src/tools/rust-analyzer",
    runtime_id: "fixture-runtime", target: "x86_64-pc-windows-msvc",
    lock_file: "Cargo.lock", patch_file: "metadata-stdin.patch",
    lock_sha256: sha256(lock), patch_sha256: sha256(patch),
    licenses: ["LICENSE-MIT", "LICENSE-APACHE"].map(file => ({
      file, source: "src/tools/rust-analyzer/" + file, sha256: "a".repeat(64),
    })),
  };
  verifySidecar(source, lock, patch);
  assert.throws(() => verifySidecar(source, Buffer.from("tampered\n"), patch), /SBOM_RA_LOCK_DIGEST/);
  assert.throws(() => verifySidecar(source, lock, Buffer.from("tampered\n")), /SBOM_RA_PATCH_DIGEST/);
  assert.throws(() => verifySidecar({ ...source, lock_file: "../Cargo.lock" }, lock, patch), /SBOM_RA_LOCK_PATH/);
  assert.throws(() => verifySidecar({ ...source, source_commit: "0".repeat(40) }, lock, patch), /SBOM_RA_SOURCE/);
  assert.throws(() => verifySidecar({ ...source, licenses: source.licenses.slice(0, 1) }, lock, patch), /SBOM_RA_LICENSES/);
  const local = cargoComponents('\n[[package]]\nname = "glib"\nversion = "0.18.5"\n');
  annotateGlib(local, {
    provenance: { upstream: { repository: "https://example.invalid", commit: "source" },
      patch: { url: "https://example.invalid/patch", commit: "patch", advisory: "RUSTSEC-2024-0429" },
      registryOrigin: { checksum: "origin" } },
    sourceDigest: "local-source", provenanceGitBlob: "provenance",
  });
  assert.equal(local[0].hashes, undefined);
  assert.ok(local[0].properties.some(property => property.name === "browseros:modified" && property.value === "true"));
  assert.ok(local[0].properties.some(property => property.name === "browseros:upstream-advisory"));
  console.log("SBOM_SELF_TEST=PASS sidecar_negative=5 glib_origin=visible");
  process.exit(0);
}

const outIdx = process.argv.indexOf("--output-dir");
if (outIdx >= 0 && !process.argv[outIdx + 1]) throw new Error("SBOM_OUTPUT_DIR_REQUIRED");
const outDir = path.resolve(ROOT, outIdx >= 0 ? process.argv[outIdx + 1] : "artifacts/sbom");
fs.mkdirSync(outDir, { recursive: true });
const npmLock = JSON.parse(fs.readFileSync(path.join(ROOT, "package-lock.json"), "utf8"));
const cargoLock = fs.readFileSync(path.join(ROOT, "src-tauri/Cargo.lock"), "utf8");
const verifiedGlib = verifyVendoredGlib(ROOT);
const sidecar = readSidecar(ROOT);
const nodeBom = bom("BrowserOS lockfile SBOM generator", npmComponents(npmLock));
const hostComponents = cargoComponents(cargoLock);
annotateGlib(hostComponents, verifiedGlib);
const rustBom = bom("BrowserOS lockfile SBOM generator", hostComponents);
const analyzerBom = bom("BrowserOS rust-analyzer lockfile SBOM generator", cargoComponents(sidecar.lock), {
  type: "application", name: "browseros-rust-analyzer", version: sidecar.source.runtime_id,
  properties: [
    { name: "browseros:source-repository", value: sidecar.source.source_repository },
    { name: "browseros:source-revision", value: sidecar.source.source_commit },
    { name: "browseros:source-subdirectory", value: sidecar.source.source_subdirectory },
    { name: "browseros:build-target", value: sidecar.source.target },
    { name: "browseros:lock-sha256", value: sidecar.source.lock_sha256 },
    { name: "browseros:patch-sha256", value: sidecar.source.patch_sha256 },
    { name: "browseros:licenses", value: JSON.stringify(sidecar.source.licenses) },
    { name: "browseros:license-byte-validation", value: "Pinned source metadata; actual license bytes verified by runtime build/package gate" },
  ],
});
fs.writeFileSync(path.join(outDir, "node.cdx.json"), JSON.stringify(nodeBom, null, 2) + "\n");
fs.writeFileSync(path.join(outDir, "rust.cdx.json"), JSON.stringify(rustBom, null, 2) + "\n");
fs.writeFileSync(path.join(outDir, "rust-analyzer.cdx.json"), JSON.stringify(analyzerBom, null, 2) + "\n");
console.log("SBOM_RESULT=PASS node=" + nodeBom.components.length + " rust=" + rustBom.components.length + " rust_analyzer=" + analyzerBom.components.length);
