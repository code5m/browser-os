// Capability Catalog（§51-B：catalog 必须存在且机器可读）
//
// 关键：本模块**只 import 各能力的 manifest.ts（纯数据、零副作用）**，
//      绝不 import 其 index.ts / state / ui —— 保证 framework-only 装配时
//      不会加载任何能力内部代码，也不会有任何重资源被创建（absent 语义前提）。

import { bookmarkManifest } from "../../capabilities/bookmark/manifest";
import { workspaceManifest } from "../../capabilities/workspace/manifest";
import { browserManifest } from "../../capabilities/browser/manifest";
import { terminalManifest } from "../../capabilities/terminal/manifest";
import { gitManifest } from "../../capabilities/git/manifest";
import { databaseManifest } from "../../capabilities/database/manifest";
import { agentManifest } from "../../capabilities/agent/manifest";
import { skillManifest } from "../../capabilities/skill/manifest";
import { pluginManifest } from "../../capabilities/plugin/manifest";
import type { CapabilityManifestV1 } from "./contract";

/** 已具备 Building Block Contract v1 的能力清单（未声明 v1 的能力不进 catalog） */
export const CATALOG_SOURCES = [
  bookmarkManifest,
  workspaceManifest,
  browserManifest,
  terminalManifest,
  gitManifest,
  databaseManifest,
  agentManifest,
  skillManifest,
  pluginManifest,
] as const;

export function buildCatalog(): Record<string, CapabilityManifestV1> {
  const out: Record<string, CapabilityManifestV1> = {};
  for (const def of CATALOG_SOURCES) {
    if (!def?.v1) continue;
    out[def.v1.id] = def.v1;
  }
  return out;
}

export const CAPABILITY_CATALOG: Record<string, CapabilityManifestV1> = buildCatalog();

/** 真实能力定义（同一对象即含 v1，零复制 → 无漂移；供 runtime.register / hotplug 使用） */
export const CAPABILITY_DEFINITIONS: Record<string, (typeof CATALOG_SOURCES)[number]> = Object.fromEntries(
  CATALOG_SOURCES.map((d) => [d.id, d]),
);

/** 尚未具备 Building Block Contract v1 的能力（诚实暴露，不得静默忽略） */
export function nonCompliantCapabilities(): string[] {
  return CATALOG_SOURCES.filter((d) => !d?.v1).map((d) => d?.id ?? "unknown");
}

export function catalogIds(): string[] {
  return Object.keys(CAPABILITY_CATALOG).sort();
}

/** §27/§29：回答「某资源类型由谁拥有」——缺席/释放证据的归属依据 */
export function ownersOfResource(kind: string): string[] {
  return Object.values(CAPABILITY_CATALOG)
    .filter((m) => (m.resources ?? []).some((r) => r.kind === kind && r.ownership === "owned"))
    .map((m) => m.id)
    .sort();
}
