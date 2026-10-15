import assert from "node:assert/strict";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

// A Vue SFC scope id is an implementation detail: the renderer only calls
// setAttribute(scopeId, ""), and compiled scoped CSS uses [scopeId] selectors.
// Keep the readable, conventional data-v-XXXXXXXX names in source and shorten
// them only in the production build. No external dependencies are needed.
//
// The mapping is a lossless, deterministic base-62 encoding of the full 32-bit
// scope hash. Unlike assigning IDs by chunk order, adding a new component
// never renames existing components, which is important for asset caching.
const CSS_SCOPE = /\[(data-v-[0-9a-f]{8})(?=(?:-s)?\])/g;
const JS_SCOPE = /\bdata-v-[0-9a-f]{8}\b/g;
const DIGITS = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

function shortScope(id) {
  let n = Number.parseInt(id.slice("data-v-".length), 16);
  let digits = "";
  do {
    digits = DIGITS[n % DIGITS.length] + digits;
    n = Math.floor(n / DIGITS.length);
  } while (n !== 0);
  return "v" + digits;
}

/**
 * Rewrite only scope IDs appearing in BOTH final CSS selectors and final JS.
 * A CSS-only or JS-only occurrence stays intact (fail-closed for unmatched
 * generated artifacts). The '-s' suffix for slotted selectors is preserved.
 *
 * This is deliberately an asset-only build optimization: no source files,
 * public component properties, features, or runtime CSS rules are deleted.
 */
export function compactVueScopes(files) {
  const cssIds = new Set();
  const jsIds = new Set();
  for (const f of files) {
    if (f.path.endsWith(".css")) {
      for (const m of f.content.matchAll(CSS_SCOPE)) cssIds.add(m[1]);
    } else if (f.path.endsWith(".js")) {
      for (const m of f.content.matchAll(JS_SCOPE)) jsIds.add(m[0]);
    }
  }
  const aliases = new Map();
  const taken = new Set();
  for (const id of cssIds) {
    if (!jsIds.has(id)) continue;
    const alias = shortScope(id);
    if (taken.has(alias)) throw new Error("VUE_SCOPE_COMPACT_ALIAS_COLLISION");
    taken.add(alias);
    aliases.set(id, alias);
  }
  // Do not silently alias a genuine selector that an application already uses.
  for (const f of files.filter(f => f.path.endsWith(".css"))) {
    for (const alias of taken) {
      if (f.content.includes("[" + alias + "]") ||
          f.content.includes("[" + alias + "-s]")) {
        throw new Error("VUE_SCOPE_COMPACT_SELECTOR_COLLISION: " + alias);
      }
    }
  }
  let bytesSaved = 0;
  const output = files.map(f => {
    let next = f.content;
    if (f.path.endsWith(".css")) {
      next = next.replace(CSS_SCOPE, (match, id) =>
        aliases.has(id) ? "[" + aliases.get(id) : match);
    } else if (f.path.endsWith(".js")) {
      next = next.replace(JS_SCOPE, id => aliases.get(id) ?? id);
    }
    bytesSaved += Buffer.byteLength(f.content) - Buffer.byteLength(next);
    return { ...f, content: next };
  });
  return { output, aliases, bytesSaved };
}

function productionAssets(dir) {
  const files = [];
  function walk(path) {
    for (const item of readdirSync(path, { withFileTypes: true })) {
      const filename = join(path, item.name);
      if (item.isDirectory()) walk(filename);
      else if (item.isFile() && /\.(?:css|js)$/.test(item.name)) {
        files.push({ path: filename, content: readFileSync(filename, "utf8") });
      }
    }
  }
  walk(dir);
  return files;
}

function selfTest() {
  const a = "data-v-1234abcd";
  const b = "data-v-00ab019f";
  assert.notEqual(shortScope(a), shortScope(b));
  assert.equal(shortScope(a), shortScope(a));
  const files = [
    { path: "entry.js", content: 'const component = { __scopeId: "' + a + '" };' },
    { path: "style.css", content: '.a[' + a + ']{color:red}.b[' + a + '-s]{color:blue}.c[' + b + ']{color:green}' },
  ];
  const { output, aliases, bytesSaved } = compactVueScopes(files);
  assert.equal(aliases.size, 1);
  assert.ok(bytesSaved > 0);
  assert.ok(output[0].content.includes(shortScope(a)));
  assert.ok(output[1].content.includes("[" + shortScope(a) + "]"));
  assert.ok(output[1].content.includes("[" + shortScope(a) + "-s]"));
  assert.ok(output[1].content.includes("[" + b + "]"));
  assert.ok(!output[0].content.includes(a));
  assert.deepEqual(compactVueScopes(output).output, output);
  assert.throws(() => compactVueScopes([
    ...files,
    { path: "extra.css", content: ".conflict[" + shortScope(a) + "]{display:block}" },
  ]), /SELECTOR_COLLISION/);
  process.stdout.write("VUE_SCOPE_COMPACTION_SELF_TEST=PASS\n");
}

export function compactVueScopeIds() {
  return {
    name: "browseros-compact-vue-scope-ids",
    apply: "build",
    // Run after Terser and after Vite has emitted CSS/JS. This supports
    // cssCodeSplit:false and true without depending on plugin hook ordering.
    writeBundle(outputOptions) {
      const dir = resolve(outputOptions.dir || "dist");
      const files = productionAssets(dir);
      const { output, aliases, bytesSaved } = compactVueScopes(files);
      for (let i = 0; i < files.length; i++) {
        if (files[i].content !== output[i].content) {
          writeFileSync(files[i].path, output[i].content, "utf8");
        }
      }
      console.log("[browseros] VUE_SCOPE_COMPACTION scopes=" + aliases.size +
        " bytes_saved=" + bytesSaved);
    },
  };
}

if (process.argv.includes("--self-test")) selfTest();
