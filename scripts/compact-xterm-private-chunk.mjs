import { minify } from "terser";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";

const require = createRequire(import.meta.url);
const apiFiles = [
  ["@xterm/xterm", "xterm.d.ts"],
  ["@xterm/addon-fit", "addon-fit.d.ts"],
];

/** Preserve the complete documented API, not merely methods BrowserOS uses.
 * Keep words from declaration comments too: an over-inclusive reservation is
 * preferable to shortening any external option, event payload or buffer field.
 * Declaration files belong to the exact installed, lockfile-pinned packages.
 */
export function xtermPublicNames() {
  const reserved = new Set(["_core"]);
  for (const [pkg, file] of apiFiles) {
    const declaration = readFileSync(
      resolve(dirname(require.resolve(pkg)), "../typings", file), "utf8",
    );
    for (const word of declaration.match(/[A-Za-z_$][\w$]*/g) || []) reserved.add(word);
  }
  return reserved;
}

/** Reserve every identifier-like word carried by a string/template literal.
 * This also protects reflection, string-key registries and options passed by
 * vendor code. Rollup supplies a real ESTree parser; no regex parses JavaScript.
 */
export function preserveLiteralNames(ast, reserved) {
  const stack = [ast];
  while (stack.length) {
    const node = stack.pop();
    if (!node || typeof node !== "object") continue;
    const text = node.type === "Literal" && typeof node.value === "string"
      ? node.value : node.type === "TemplateElement" ? node.value.cooked : null;
    if (text) for (const word of text.match(/[A-Za-z_$][\w$]*/g) || []) reserved.add(word);
    for (const [key, value] of Object.entries(node)) {
      if (key === "parent" || key === "loc") continue;
      if (Array.isArray(value)) stack.push(...value);
      else if (value && typeof value === "object") stack.push(value);
    }
  }
  return reserved;
}

/** Only the isolated terminal-vendor chunk is compacted. The public contract,
 * quoted/reflected names, DOM/JS builtins and all application chunks stay intact.
 * No vendor source, functionality, asset accounting or size baseline is removed.
 * This hook precedes Rollup's content hashing. Terminal differential tests and
 * packaged GUI checks are required when changing this implementation.
 */
export function compactXtermPrivateChunk() {
  return {
    name: "browseros-xterm-private-chunk-compact",
    apply: "build",
    enforce: "post",
    async renderChunk(code, chunk) {
      if (chunk.name !== "xterm" || code.length < 100_000) return null;
      const reserved = preserveLiteralNames(this.parse(code), xtermPublicNames());
      const optimized = await minify(code, {
        module: true,
        compress: false,
        mangle: {
          properties: {
            regex: /^(?:_[A-Za-z][\w$]*|[A-Za-z$][\w$]{3,})$/,
            keep_quoted: true,
            builtins: false,
            reserved: [...reserved],
          },
        },
        format: { comments: false },
      });
      if (!optimized.code || optimized.code.length >= code.length) {
        throw new Error("XTERM_INTERNAL_COMPACTION_NO_SAVINGS");
      }
      console.log(`[browseros] XTERM_PUBLIC_API_NAMES_RESERVED=${reserved.size}`);
      return { code: optimized.code, map: null };
    },
  };
}
