import { minify } from "terser";

/**
 * Bundle-size optimisation for the already-isolated @xterm vendor chunk.
 *
 * BrowserOS consumes the documented Terminal/FitAddon public API and does
 * not access any _private fields outside this chunk. Mangle only private
 * fields inside that one final chunk; NEVER mangle other capabilities or
 * global application properties. Keep quoted fields and _core stable.
 *
 * Rollup renderChunk executes before content-hash file naming. A failure
 * aborts the build instead of shipping a half-renamed terminal vendor.
 */
export function compactXtermPrivateChunk() {
  return {
    name: "browseros-xterm-private-chunk-compact",
    apply: "build",
    enforce: "post",
    async renderChunk(code, chunk) {
      // SSR/diagnostic probes may create a stub named xterm without shipping
      // the real vendor. Do not require a saving from an empty probe chunk.
      if (chunk.name !== "xterm" || code.length < 100_000) return null;
      const optimized = await minify(code, {
        module: true,
        compress: false,
        mangle: {
          properties: {
            regex: /^_[A-Za-z]/,
            keep_quoted: true,
            reserved: ["_core"],
          },
        },
        format: { comments: false },
      });
      if (!optimized.code || optimized.code.length >= code.length) {
        throw new Error("XTERM_PRIVATE_MANGLE_NO_SAVINGS");
      }
      return { code: optimized.code, map: null };
    },
  };
}
