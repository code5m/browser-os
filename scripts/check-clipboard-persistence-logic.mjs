#!/usr/bin/env node
// ---------------------------------------------------------------------------
// 委托入口：B11-1 持久化门禁的单一真源在
//   packages/capability-clipboard/scripts/check-clipboard-persistence-logic.mjs
// 本文件仅转发，避免两份逻辑（SECOND_TRUTH=0）。
// 用法: node scripts/check-clipboard-persistence-logic.mjs
// ---------------------------------------------------------------------------
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { join, dirname } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");
const target = join(ROOT, "packages/capability-clipboard/scripts/check-clipboard-persistence-logic.mjs");
try {
  execFileSync(process.execPath, [target], { stdio: "inherit", cwd: ROOT });
} catch {
  process.exit(1);
}
