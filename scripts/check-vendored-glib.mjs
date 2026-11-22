#!/usr/bin/env node
import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const VENDOR = "src-tauri/vendor/glib";
const PROVENANCE = "BROWSEROS-SOURCE.json";
const EXPECTED_PROVENANCE_BLOB = "de12c3401f0eeb59532ab828eed7da296a9c71db";
const UPSTREAM_COMMIT = "42b9caf98e03ded086362d9653ca58fe94dc8658";
const PATCH_COMMIT = "b5a4071e439bef2b5eea76c3aa25e5ae84839e34";
const REGISTRY = "registry+https://github.com/rust-lang/crates.io-index";
const REGISTRY_CHECKSUM = "233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5";
const EDITS = [
  {
    "path": "Cargo.toml",
    "before": "ffi = { package = \"glib-sys\", version = \"0.18\", path = \"sys\" }",
    "after": "ffi = { package = \"glib-sys\", version = \"0.18\" }"
  },
  {
    "path": "Cargo.toml",
    "before": "gobject_ffi = { package = \"gobject-sys\", version = \"0.18\", path = \"gobject-sys\" }",
    "after": "gobject_ffi = { package = \"gobject-sys\", version = \"0.18\" }"
  },
  {
    "path": "Cargo.toml",
    "before": "glib-macros = { path = \"../glib-macros\", version = \"0.18\" }",
    "after": "glib-macros = { version = \"0.18\" }"
  },
  {
    "path": "Cargo.toml",
    "before": "gio_ffi = { package = \"gio-sys\", path = \"../gio/sys\", version = \"0.18\", optional = true }",
    "after": "gio_ffi = { package = \"gio-sys\", version = \"0.18\", optional = true }"
  },
  {
    "path": "src/variant_iter.rs",
    "before": "            let p: *mut libc::c_char = std::ptr::null_mut();",
    "after": "            let mut p: *mut libc::c_char = std::ptr::null_mut();"
  },
  {
    "path": "src/variant_iter.rs",
    "before": "                &p,",
    "after": "                &mut p,"
  },
  {
    "path": "LICENSE",
    "before": "SOFTWARE.\n\n",
    "after": "SOFTWARE.\n"
  }
];
const REQUIRED_TESTS = [
  "glib_variant_str_iter_forward_preserves_utf8_and_empty_values",
  "glib_variant_str_iter_reverse_preserves_utf8_and_empty_values",
  "glib_variant_str_iter_nth_skips_and_exhausts",
  "glib_variant_str_iter_nth_back_skips_and_exhausts",
  "glib_variant_str_iter_last_handles_nonempty_and_empty"
];

function requireValue(condition, code) {
  if (!condition) throw new Error("GLIB_VENDOR_" + code);
}

function gitBlob(bytes) {
  return crypto.createHash("sha1").update(Buffer.from("blob " + bytes.length + "\0")).update(bytes).digest("hex");
}

function utf8(bytes, code) {
  const text = bytes.toString("utf8");
  requireValue(Buffer.from(text, "utf8").equals(bytes), code + "_UTF8");
  return text;
}

function replaceOnce(text, before, after, code) {
  requireValue(text.split(before).length === 2, code);
  return text.replace(before, () => after);
}

function readInventory(directory) {
  const files = new Map();
  function visit(current, prefix) {
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      const relative = prefix ? prefix + "/" + entry.name : entry.name;
      const stat = fs.lstatSync(full);
      requireValue(!stat.isSymbolicLink(), "SYMLINK");
      if (stat.isDirectory()) visit(full, relative);
      else {
        requireValue(stat.isFile(), "NON_FILE");
        files.set(relative, fs.readFileSync(full));
      }
    }
  }
  visit(directory, "");
  return files;
}

function verifyContents(files) {
  const bytes = files.get(PROVENANCE);
  requireValue(bytes && gitBlob(bytes) === EXPECTED_PROVENANCE_BLOB, "PROVENANCE_CHANGED");
  const provenance = JSON.parse(utf8(bytes, "PROVENANCE"));
  requireValue(provenance.schemaVersion === 1, "SCHEMA");
  requireValue(provenance.package.name === "glib" && provenance.package.version === "0.18.5", "VERSION");
  requireValue(provenance.upstream.commit === UPSTREAM_COMMIT, "UPSTREAM");
  requireValue(provenance.patch.commit === PATCH_COMMIT && provenance.patch.advisory === "RUSTSEC-2024-0429", "PATCH");
  requireValue(provenance.registryOrigin.source === REGISTRY && provenance.registryOrigin.checksum === REGISTRY_CHECKSUM, "REGISTRY");
  requireValue(provenance.registryOrigin.archiveVerified === false, "ARCHIVE_CLAIM");
  requireValue(provenance.files.length === 119, "INVENTORY_COUNT");
  const seen = new Set([PROVENANCE, "BROWSEROS-PATCHES.md"]);
  for (const entry of provenance.files) {
    requireValue(typeof entry.path === "string" && !entry.path.includes("\\") &&
      !entry.path.split("/").some(part => !part || part === "." || part === "..") &&
      !entry.path.includes("\0"), "PATH");
    requireValue(!seen.has(entry.path), "DUPLICATE_PATH");
    seen.add(entry.path);
    const content = files.get(entry.path);
    requireValue(content, "MISSING_FILE:" + entry.path);
    let upstream = content;
    const edits = EDITS.filter(edit => edit.path === entry.path);
    if (edits.length) {
      let text = utf8(content, entry.path);
      for (const edit of edits) {
        text = replaceOnce(text, edit.after, edit.before, "PATCH_DRIFT:" + entry.path);
      }
      upstream = Buffer.from(text, "utf8");
    }
    requireValue(upstream.length === entry.size && gitBlob(upstream) === entry.gitBlob, "SOURCE_DRIFT:" + entry.path);
  }
  requireValue(files.size === seen.size && [...files.keys()].every(file => seen.has(file)), "EXTRA_OR_MISSING_FILE");
  requireValue(files.get("BROWSEROS-PATCHES.md")?.length > 0, "PATCH_NOTE");
  const hash = crypto.createHash("sha256");
  for (const name of [...files.keys()].sort()) {
    hash.update(name + "\0" + crypto.createHash("sha256").update(files.get(name)).digest("hex") + "\n");
  }
  return { provenance, sourceDigest: hash.digest("hex"), provenanceGitBlob: EXPECTED_PROVENANCE_BLOB };
}

function checkCargoPatch(text, expectedPath) {
  text = text.replace(/\r\n/g, "\n");
  const tables = [...text.matchAll(/^\[patch\.crates-io\][ \t]*$/gm)];
  requireValue(tables.length === 1, "CARGO_PATCH_TABLE");
  const tail = text.slice(tables[0].index + tables[0][0].length);
  const next = tail.search(/^\[/m);
  const table = next < 0 ? tail : tail.slice(0, next);
  const lines = table.split("\n").map(line => line.trim()).filter(line => /^glib\s*=/.test(line));
  requireValue(lines.length === 1 && lines[0] === 'glib = { path = "' + expectedPath + '" }', "CARGO_PATCH_PATH");
}

function glibLockBlock(lock) {
  const blocks = lock.replace(/\r\n/g, "\n").split(/(?=^\[\[package\]\]$)/m);
  const matches = blocks.filter(block => /^name = "glib"$/m.test(block));
  requireValue(matches.length === 1 && /^version = "0\.18\.5"$/m.test(matches[0]), "LOCK_VERSION");
  requireValue(!/^(?:source|checksum) = /m.test(matches[0]), "LOCK_NOT_PATCHED");
  return matches[0];
}

export function registryOriginProjection(lock) {
  lock = lock.replace(/\r\n/g, "\n");
  const block = glibLockBlock(lock);
  const origin = replaceOnce(block, 'version = "0.18.5"\n',
    'version = "0.18.5"\nsource = "' + REGISTRY + '"\nchecksum = "' + REGISTRY_CHECKSUM + '"\n',
    "LOCK_VERSION_LINE");
  return "# ADVISORY-ONLY REGISTRY ORIGIN PROJECTION; NEVER USE AS THE BUILD LOCK.\n" +
    "# glib is locally patched; this projection preserves upstream advisory visibility.\n" +
    lock.replace(block, () => origin);
}

export function verifyVendoredGlib(root = ROOT) {
  const result = verifyContents(readInventory(path.join(root, VENDOR)));
  checkCargoPatch(fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8"), "vendor/glib");
  checkCargoPatch(fs.readFileSync(path.join(root, "tauri-browser-tabs/Cargo.toml"), "utf8"), "../src-tauri/vendor/glib");
  glibLockBlock(fs.readFileSync(path.join(root, "src-tauri/Cargo.lock"), "utf8"));
  return result;
}

export function verifyCargoMetadata(metadata) {
  requireValue(Array.isArray(metadata.packages) && Array.isArray(metadata.workspace_members), "CARGO_METADATA");
  const matches = metadata.packages.filter(item => item.name === "glib");
  requireValue(matches.length === 1, "CARGO_GLIB_COUNT");
  const glib = matches[0];
  requireValue(glib.version === "0.18.5" && glib.source === null, "CARGO_GLIB_SOURCE");
  requireValue(path.resolve(glib.manifest_path) === path.join(ROOT, VENDOR, "Cargo.toml"), "CARGO_GLIB_PATH");
  requireValue(!metadata.workspace_members.includes(glib.id), "CARGO_GLIB_WORKSPACE_MEMBER");
  requireValue(Array.isArray(metadata.resolve?.nodes) &&
    metadata.resolve.nodes.some(node => node.id === glib.id), "CARGO_GLIB_NOT_RESOLVED");
  return { version: glib.version, source: "vendored-path", workspaceMember: false };
}

export function verifyRegressionLog(text) {
  const lines = text.replace(/\u001b\[[0-?]*[ -/]*[@-~]/g, "").replace(/\r/g, "").split("\n");
  const results = lines.map(line => /^test (\S+) \.\.\. (ok|FAILED|ignored)\s*$/.exec(line)).filter(Boolean);
  requireValue(results.length === REQUIRED_TESTS.length, "REGRESSION_COUNT");
  for (const name of REQUIRED_TESTS) {
    const found = results.filter(result => result[1] === name);
    requireValue(found.length === 1 && found[0][2] === "ok", "REGRESSION_TEST:" + name);
  }
  const summaries = lines.map(line => /^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;/.exec(line)).filter(Boolean);
  requireValue(summaries.length === 1 && summaries[0][1] === "ok" &&
    Number(summaries[0][2]) === REQUIRED_TESTS.length &&
    summaries[0].slice(3, 7).every(count => Number(count) === 0), "REGRESSION_SUMMARY");
  requireValue(lines.filter(line => line === "GLIB_VARIANT_ITER_PROFILE=release").length === REQUIRED_TESTS.length &&
    !lines.some(line => line === "GLIB_VARIANT_ITER_PROFILE=debug"), "REGRESSION_NOT_RELEASE");
  return { passed: REQUIRED_TESTS.length, failed: 0, ignored: 0, profile: "release" };
}

function selfTest() {
  const files = readInventory(path.join(ROOT, VENDOR));
  verifyContents(files);
  const rejectFile = (file, mutate) => {
    const bad = new Map(files);
    bad.set(file, Buffer.from(mutate(utf8(bad.get(file), "FIXTURE")), "utf8"));
    assert.throws(() => verifyContents(bad), /GLIB_VENDOR_/);
  };
  rejectFile("src/variant_iter.rs", text => text.replace("                &mut p,", "                &p,"));
  rejectFile("src/variant_iter.rs", text => text.replace("            let mut p:", "            let p:"));
  rejectFile("Cargo.toml", text => text.replace('version = "0.18.5"', 'version = "0.20.0"'));
  rejectFile("Cargo.toml", text => text + '\n[patch.crates-io]\nlibc = { path = "/unreviewed" }\n');
  rejectFile("LICENSE", text => text + "changed\n");
  const extra = new Map(files);
  extra.set("extra.rs", Buffer.from("fn unreviewed() {}\n"));
  assert.throws(() => verifyContents(extra), /GLIB_VENDOR_/);
  const missing = new Map(files);
  missing.delete("src/variant_iter.rs");
  assert.throws(() => verifyContents(missing), /GLIB_VENDOR_/);
  const hostLock = fs.readFileSync(path.join(ROOT, "src-tauri/Cargo.lock"), "utf8").replace(/\r\n/g, "\n");
  const projected = registryOriginProjection(hostLock);
  assert.ok(projected.includes('name = "glib"\nversion = "0.18.5"\nsource = "' + REGISTRY + '"\nchecksum = "' + REGISTRY_CHECKSUM + '"'));
  const withoutComment = projected.split("\n").slice(2).join("\n");
  const projectedBlock = withoutComment.split(/(?=^\[\[package\]\]$)/m).find(block => /^name = "glib"$/m.test(block));
  const restored = withoutComment.replace(projectedBlock, () => projectedBlock.replace('source = "' + REGISTRY + '"\nchecksum = "' + REGISTRY_CHECKSUM + '"\n', ""));
  assert.equal(restored, hostLock);
  assert.throws(() => registryOriginProjection(projected), /GLIB_VENDOR_LOCK_NOT_PATCHED/);
  checkCargoPatch('[patch.crates-io]\nglib = { path = "vendor/glib" }\n', "vendor/glib");
  assert.throws(() => checkCargoPatch('[patch.crates-io]\nglib = { git = "https://unreviewed.invalid" }\n', "vendor/glib"), /GLIB_VENDOR_/);
  const goodLog = REQUIRED_TESTS.map(name => "test " + name + " ... ok").join("\n") + "\n" +
    REQUIRED_TESTS.map(() => "GLIB_VARIANT_ITER_PROFILE=release").join("\n") +
    "\ntest result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n";
  assert.equal(verifyRegressionLog(goodLog).passed, 5);
  assert.throws(() => verifyRegressionLog(goodLog.replace(" ... ok", " ... ignored")), /GLIB_VENDOR_/);
  assert.throws(() => verifyRegressionLog(goodLog.replace(" ... ok", " ... FAILED")), /GLIB_VENDOR_/);
  assert.throws(() => verifyRegressionLog(goodLog.replace(/^test .+\n/, "")), /GLIB_VENDOR_/);
  assert.throws(() => verifyRegressionLog(goodLog.replace("PROFILE=release", "PROFILE=debug")), /GLIB_VENDOR_/);
  assert.throws(() => verifyRegressionLog(goodLog.replace("0 ignored", "1 ignored")), /GLIB_VENDOR_/);
  assert.throws(() => verifyRegressionLog(goodLog + "test " + REQUIRED_TESTS[0] + " ... ok\n"), /GLIB_VENDOR_/);
  const graph = {
    packages: [{ name: "glib", id: "local-glib", version: "0.18.5", source: null,
      manifest_path: path.join(ROOT, VENDOR, "Cargo.toml") }],
    workspace_members: ["host"],
    resolve: { nodes: [{ id: "local-glib" }] },
  };
  assert.equal(verifyCargoMetadata(graph).workspaceMember, false);
  assert.throws(() => verifyCargoMetadata({ ...graph, workspace_members: ["host", "local-glib"] }), /GLIB_VENDOR_CARGO_GLIB_WORKSPACE_MEMBER/);
  assert.throws(() => verifyCargoMetadata({ ...graph, packages: [{ ...graph.packages[0], source: REGISTRY }] }), /GLIB_VENDOR_CARGO_GLIB_SOURCE/);
  assert.throws(() => verifyCargoMetadata({ ...graph, resolve: { nodes: [] } }), /GLIB_VENDOR_CARGO_GLIB_NOT_RESOLVED/);
  console.log("GLIB_VENDOR_SELF_TEST=PASS source_tamper=7 lock_projection=2 patch_target=1 runtime_negative=6 graph_negative=3");
}

function main() {
  const args = process.argv.slice(2);
  if (args.length === 1 && args[0] === "--self-test") {
    selfTest();
    return;
  }
  if (args.length === 2 && args[0] === "--check-cargo-metadata") {
    verifyVendoredGlib();
    verifyCargoMetadata(JSON.parse(fs.readFileSync(args[1], "utf8")));
    console.log("GLIB_CARGO_SOURCE=PASS version=0.18.5 workspace_member=false");
    return;
  }
  if (args.length === 2 && args[0] === "--check-regression-log") {
    verifyVendoredGlib();
    const result = verifyRegressionLog(fs.readFileSync(args[1], "utf8"));
    console.log("GLIB_VARIANT_ITER_REGRESSION=PASS tests=" + result.passed + " ignored=0 profile=release");
    return;
  }
  requireValue(args.length === 0 || (args.length === 2 && args[0] === "--audit-origin-dir"), "ARGUMENTS");
  const result = verifyVendoredGlib();
  if (args[0] === "--audit-origin-dir") {
    const output = path.resolve(args[1]);
    fs.mkdirSync(output, { recursive: true });
    const lock = fs.readFileSync(path.join(ROOT, "src-tauri/Cargo.lock"), "utf8");
    fs.writeFileSync(path.join(output, "registry-origin.Cargo.lock"), registryOriginProjection(lock));
    fs.writeFileSync(path.join(output, "glib-backport-provenance.json"), JSON.stringify({
      ...result,
      status: "SOURCE_BACKPORT_VERIFIED",
      optimizedRuntime: "NOT_RUN_BY_SOURCE_CHECK",
      auditProjection: "Upstream registry identity for advisory discovery only; never the application build lock",
      sourceDigestFormat: "SHA256 of sorted relative-path + NUL + SHA256(file bytes) + LF records",
    }, null, 2) + "\n");
  }
  console.log("GLIB_VENDOR_SOURCE=PASS files=119 version=0.18.5 upstream=" + UPSTREAM_COMMIT + " patch=" + PATCH_COMMIT);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
