#!/usr/bin/env node
// Tests the REAL pure helper, not a mock; snapshots the frozen scheduler
// text contract and verifies both consumers use the same semantic owner.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { transform } from "esbuild";

const read = (path) => readFileSync(new URL("../" + path, import.meta.url), "utf8");
const purePath = "src/shared/pure/time/relative.ts";
const pure = read(purePath);
const task = read("src/utils/taskUi.ts");
const agent = read("src/utils/agentSkillUi.ts");

// Explicit dependency rule: a shared pure formatter must not depend on UI,
// native bridge, stores, browser globals, or the scheduler's heavyweight entry.
assert.doesNotMatch(pure, /^\s*import\s/m);
assert.match(task, /import\s*\{\s*formatRelative\s*\}\s*from\s*["']\.\.\/shared\/pure\/time\/relative["']/);
assert.match(task, /export\s*\{\s*formatRelative\s*\}/);
assert.match(agent, /import\s*\{\s*formatRelative\s*\}\s*from\s*["']\.\.\/shared\/pure\/time\/relative["']/);
assert.doesNotMatch(agent, /from\s*["']\.\/taskUi["']/);
assert.equal((task.match(/function\s+formatRelative\s*\(/g) ?? []).length, 0);

const transformed = await transform(pure, { loader: "ts", format: "esm", target: "es2022" });
const { formatRelative } = await import(
  "data:text/javascript;base64," + Buffer.from(transformed.code).toString("base64")
);
const now = Date.parse("2026-01-01T00:00:00.000Z");
const at = (offsetMs) => new Date(now + offsetMs).toISOString();
const cases = [
  [null, "—"], [undefined, "—"], ["", "—"], ["invalid-date", "—"],
  [at(0), "0 秒后"],
  [at(-59_000), "59 秒前"],
  [at(59_000), "59 秒后"],
  [at(-60_000), "1 分00 秒前"],
  [at(61_000), "1 分01 秒后"],
  [at(3_600_000), "1 小时00 分后"],
  [at(3_720_000), "1 小时02 分后"],
  [at(-86_400_000), "1 天00 小时前"],
  [at(97_800_000), "1 天03 小时后"],
  [at(-500), "0 秒后"], // preserve Math.trunc toward zero
];
for (const [input, expected] of cases) {
  assert.equal(formatRelative(input, now), expected, JSON.stringify({ input, expected }));
}
console.log("SHARED_PURE_TIME_CONTRACT=PASS (" + cases.length + " cases, owners aligned)");
