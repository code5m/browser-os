// Dependency Resolver + Assembly Engine v1（§13 / §14）
//
// 硬约束：
//   1. Deterministic：相同输入必须产生相同输出（拓扑序 + id 字典序 tie-break）。
//   2. required 依赖缺失 → REJECT（禁止启动后才 undefined/crash）。
//   3. optional 依赖缺失 → 合法 degraded（记 warning，不拒绝）。
//   4. 环 / 冲突 / 未知 id → 确定性拒绝，并给出可读原因。
//   5. 纯函数：零副作用、零业务状态、零 UI/store import（Runtime 不做 God Object）。

import type {
  CapabilityManifestV1,
  ContributionDeclaration,
  ResourceOwnership,
} from "./contract";

export type RejectionCode =
  | "UNKNOWN_CAPABILITY"
  | "MISSING_REQUIRED_DEPENDENCY"
  | "DEPENDENCY_CYCLE"
  | "CAPABILITY_CONFLICT"
  | "DUPLICATE_REQUEST";

export interface Rejection {
  code: RejectionCode;
  message: string;
}

export interface OptionalGap {
  id: string;
  missing: string;
}

export interface ConflictHit {
  a: string;
  b: string;
  declaredBy: string;
}

export interface AssemblyInput {
  /** 请求的 capability id 列表（允许空集 = framework-only） */
  capabilities: string[];
}

export interface AssemblyResult {
  ok: boolean;
  requested: string[];
  /** 最终包含集（含自动补入的 required 依赖），字典序 */
  resolved: string[];
  /** 激活顺序：依赖优先的确定性拓扑序 */
  activationOrder: string[];
  /** 被自动补入的 required 依赖（requested 中未显式列出） */
  autoIncluded: string[];
  missingRequired: Rejection[];
  missingOptional: OptionalGap[];
  conflicts: ConflictHit[];
  cycles: string[][];
  unknown: string[];
  rejections: Rejection[];
  contributions: ContributionDeclaration[];
  permissions: string[];
  resources: ResourceOwnership[];
  warnings: string[];
}

const uniqSorted = (a: string[]): string[] => Array.from(new Set(a)).sort();

/** 允许同一 id 的输入形式：Record<string, CapabilityManifestV1> 或 [{id, manifest}] */
export function toCatalogMap(
  input: Record<string, CapabilityManifestV1> | CapabilityManifestV1[],
): Record<string, CapabilityManifestV1> {
  if (Array.isArray(input)) {
    const out: Record<string, CapabilityManifestV1> = {};
    for (const m of input) out[m.id] = m;
    return out;
  }
  return input;
}

function detectCycles(
  id: string,
  catalog: Record<string, CapabilityManifestV1>,
  sees: Set<string> = new Set(),
  path: string[] = [],
): string[][] {
  if (sees.has(id)) {
    const start = path.indexOf(id);
    return [path.slice(start >= 0 ? start : 0).concat(id)];
  }
  const m = catalog[id];
  if (!m) return [];
  const out: string[][] = [];
  const nextPath = path.concat(id);
  const nextSeen = new Set(sees).add(id);
  for (const d of uniqSorted(m.dependencies ?? [])) {
    out.push(...detectCycles(d, catalog, nextSeen, nextPath));
  }
  return out;
}

function topologicalOrder(
  ids: string[],
  catalog: Record<string, CapabilityManifestV1>,
): string[] {
  // Kahn 算法 + 字典序队列 → deterministic
  const inSet = new Set(ids);
  const indeg = new Map<string, number>();
  const children = new Map<string, string[]>();
  for (const id of uniqSorted(ids)) {
    indeg.set(id, 0);
    children.set(id, []);
  }
  for (const id of uniqSorted(ids)) {
    for (const d of uniqSorted(catalog[id]?.dependencies ?? [])) {
      if (!inSet.has(d) || d === id) continue;
      children.get(d)!.push(id);
      indeg.set(id, (indeg.get(id) ?? 0) + 1);
    }
  }
  const queue = uniqSorted([...indeg.entries()].filter(([, d]) => d === 0).map(([k]) => k));
  const order: string[] = [];
  while (queue.length > 0) {
    const n = queue.shift()!;
    order.push(n);
    for (const c of uniqSorted(children.get(n) ?? [])) {
      const left = (indeg.get(c) ?? 0) - 1;
      indeg.set(c, left);
      if (left === 0) queue.push(c);
    }
    queue.sort();
  }
  return order;
}

/**
 * 真实依赖解析：requested → resolved + activationOrder + rejections/warnings。
 */
export function assemble(
  catalogInput: Record<string, CapabilityManifestV1> | CapabilityManifestV1[],
  input: AssemblyInput,
): AssemblyResult {
  const catalog = toCatalogMap(catalogInput);
  const requestedRaw = input.capabilities ?? [];
  const requested = uniqSorted(requestedRaw);

  const result: AssemblyResult = {
    ok: true,
    requested,
    resolved: [],
    activationOrder: [],
    autoIncluded: [],
    missingRequired: [],
    missingOptional: [],
    conflicts: [],
    cycles: [],
    unknown: [],
    rejections: [],
    contributions: [],
    permissions: [],
    resources: [],
    warnings: [],
  };

  // 1) 未知 id（显式请求的能力必须存在）
  for (const id of requested) {
    if (!catalog[id]) {
      result.unknown.push(id);
      result.rejections.push({
        code: "UNKNOWN_CAPABILITY",
        message: `请求的能力不存在于 catalog: ${id}`,
      });
    }
  }
  // 重复请求（uniqSorted 已去重，此处仅记录）
  if (requestedRaw.length !== requested.length) {
    result.rejections.push({
      code: "DUPLICATE_REQUEST",
      message: `请求列表存在重复项，已按唯一化处理: ${requestedRaw.join(",")}`,
    });
  }

  // 2) 展开 required 依赖（递归）
  const included = new Set<string>();
  const visited = new Set<string>();
  const visit = (id: string) => {
    const m = catalog[id];
    if (!m) return;
    // 环的处理由 detectCycles 负责；此处只保证遍历终止（否则环会栈溢出）
    if (visited.has(id)) return;
    visited.add(id);
    included.add(id);
    for (const d of uniqSorted(m.dependencies ?? [])) {
      if (!catalog[d]) {
        result.missingRequired.push({
          code: "MISSING_REQUIRED_DEPENDENCY",
          message: `${id} 的强依赖不存在于 catalog: ${d}`,
        });
        continue;
      }
      visit(d);
    }
  };
  for (const id of requested) visit(id);

  // 依赖补入过程中可能引入新节点，重复直到稳定；并检测环
  let grew = true;
  while (grew) {
    grew = false;
    for (const id of Array.from(included)) {
      for (const d of uniqSorted(catalog[id]?.dependencies ?? [])) {
        if (catalog[d] && !included.has(d)) {
          included.add(d);
          grew = true;
        }
      }
    }
  }

  for (const id of uniqSorted(Array.from(included))) {
    const cyc = detectCycles(id, catalog);
    if (cyc.length > 0) result.cycles.push(...cyc);
  }
  if (result.cycles.length > 0) {
    for (const c of result.cycles) {
      result.rejections.push({
        code: "DEPENDENCY_CYCLE",
        message: `依赖环: ${c.join(" → ")}`,
      });
    }
  }

  // 3) 冲突（双向）
  const list = uniqSorted(Array.from(included));
  for (const a of list) {
    for (const b of uniqSorted(catalog[a]?.conflicts ?? [])) {
      if (included.has(b)) {
        result.conflicts.push({ a, b, declaredBy: a });
        result.rejections.push({
          code: "CAPABILITY_CONFLICT",
          message: `${a} 与 ${b} 冲突（由 ${a} 声明）`,
        });
      }
    }
  }
  // 反向：b 声明了与 a 冲突
  for (const b of list) {
    for (const a of uniqSorted(catalog[b]?.conflicts ?? [])) {
      if (included.has(a) && !result.conflicts.some((c) => c.a === a && c.b === b)) {
        result.conflicts.push({ a, b, declaredBy: b });
        result.rejections.push({
          code: "CAPABILITY_CONFLICT",
          message: `${b} 与 ${a} 冲突（由 ${b} 声明）`,
        });
      }
    }
  }

  // 4) optional 依赖缺失 → degraded warning（不拒绝）
  for (const id of list) {
    for (const o of uniqSorted(catalog[id]?.optionalDependencies ?? [])) {
      if (!included.has(o)) {
        result.missingOptional.push({ id, missing: o });
        result.warnings.push(`${id}: optional 依赖 ${o} 未装配 → 以降级方式运行（相关展示面不可用）`);
      }
    }
  }

  // 5) 结果
  result.resolved = list;
  result.autoIncluded = list.filter((id) => !requested.includes(id));
  result.ok =
    result.unknown.length === 0 &&
    result.missingRequired.length === 0 &&
    result.cycles.length === 0 &&
    result.conflicts.length === 0;
  result.activationOrder = result.ok
    ? topologicalOrder(list, catalog)
    : uniqSorted(list);

  for (const id of result.activationOrder) {
    const m = catalog[id];
    if (!m) continue;
    result.contributions.push(...(m.contributions ?? []));
    result.permissions.push(...(m.permissions ?? []));
    result.resources.push(...(m.resources ?? []));
  }
  result.permissions = uniqSorted(result.permissions);
  // 资源去重（kind + ownership）
  const seenRes = new Set<string>();
  result.resources = result.resources.filter((r) => {
    const k = `${r.kind}|${r.ownership}`;
    if (seenRes.has(k)) return false;
    seenRes.add(k);
    return true;
  });

  return result;
}
