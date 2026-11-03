// Hot-Plug Flows v1（§20 / §21 / §22）
//
// 两条流程都必须是「要么完全成功，要么完全回到原状态」：
//   ADD    validate → resolve → permission → register → contributions → lifecycle → verify
//   REMOVE dependent check → active work policy → deactivate → resource cleanup → unregister → verify
// 任何一步失败都必须 rollback（禁止半注册）。
//
// 本模块不 import 任何具体能力——真实加载器由产品装配层注入（避免再长一个 God Runtime）。

import type { CapabilityManifestV1 } from "./contract";
import { validateManifestV1 } from "./contract";
import { assemble } from "./assembly";
import {
  assertTransition,
  canTransition,
  type PlatformState,
} from "./lifecycle";

export type HotPlugErrorCode =
  | "INVALID_MANIFEST"
  | "MISSING_DEPENDENCY"
  | "DEPENDENT_PRESENT"
  | "PERMISSION_DENIED"
  | "LOADER_FAILED"
  | "INVALID_TRANSITION"
  | "ACTIVE_WORK_POLICY"
  | "NOT_INSTALLED";

export class HotPlugError extends Error {
  code: HotPlugErrorCode;
  constructor(code: HotPlugErrorCode, message: string) {
    super(message);
    this.name = "HotPlugError";
    this.code = code;
  }
}

export interface HotPlugTarget {
  /** 已注册能力 id → 其 manifest v1（由 runtime 侧提供真实数据，非复制品） */
  installed: Record<string, CapabilityManifestV1>;
  /** 某能力的当前平台状态 */
  stateOf(id: string): PlatformState;
  setState(id: string, to: PlatformState): void;
  /** 注册/注销 runtime 记录（由 runtime 提供，带自身安全检查） */
  addRecord(manifest: CapabilityManifestV1): void;
  removeRecord(id: string): void;
  /** 注册贡献 */
  addContributions(manifest: CapabilityManifestV1): void;
  removeContributions(id: string): void;
  /** 能力的停用后钩子：用于释放/收尾（可选；未提供且 policy 为 graceful 时按 policy 裁决） */
  onDeactivate?(id: string): void | Promise<void>;
  /** The authoritative Runtime Owner receipt; if provided it replaces the legacy hook. */
  releaseOwner?(id: string): Promise<void>;
  /** Restore the Runtime only after its Owner has activated successfully. */
  activateOwner?(id: string): Promise<void>;
  /** 是否存在未结束的工作（用于 REJECT / GRACEFUL 裁决） */
  hasActiveWork?(id: string): boolean;
}

// Serialise overlapping remove/disable/enable on the same Target + capability.
// It is a pure platform guard, not a second resource or task state owner.
const inFlight = new WeakMap<HotPlugTarget, Set<string>>();
function lockTransition(target: HotPlugTarget, id: string): () => void {
  let ids = inFlight.get(target);
  if (!ids) { ids = new Set(); inFlight.set(target, ids); }
  if (ids.has(id)) throw new HotPlugError("INVALID_TRANSITION", `${id} 正在释放或启用，禁止重入`);
  ids.add(id);
  return () => ids!.delete(id);
}

export interface AddRequest {
  manifest: CapabilityManifestV1;
  /** 依赖加载器：真正把该能力的代码拉起来（例如调用 registerXxxContributions） */
  load: (id: string) => void | Promise<void>;
  /** 权限裁决：未通过 → PERMISSION_DENIED */
  permit?: (manifest: CapabilityManifestV1) => boolean;
}

export interface AddResult {
  id: string;
  state: PlatformState;
  /** 装配过程报告（含 warning，便于 leader demo 展示可选依赖降级） */
  notes: string[];
}

/**
 * 运行时加入能力（HP2 主流程）。
 * 失败必须回滚：已注册的贡献 + 已注册的 runtime 记录都会被撤销。
 */
export async function requestAdd(target: HotPlugTarget, req: AddRequest): Promise<AddResult> {
  const m = req.manifest;
  const violations = validateManifestV1(m);
  if (violations.length > 0) {
    throw new HotPlugError(
      "INVALID_MANIFEST",
      `manifest 校验失败: ${violations.map((v) => `${v.code}(${v.message})`).join("; ")}`,
    );
  }
  const unlock = lockTransition(target, m.id);
  try {
  if (target.installed[m.id]) throw new HotPlugError("INVALID_MANIFEST", `能力已注册: ${m.id}`);
  if (!m.hotPlug?.register) {
    throw new HotPlugError("INVALID_TRANSITION", `${m.id} 未声明支持运行时 register（HP level=${m.hotPlug?.level}）`);
  }

  // 1) 解析：用「已安装 + 待加入」重算装配，强依赖缺失必须 deterministic reject
  const merged = { ...target.installed, [m.id]: m };
  const report = assemble(merged, { capabilities: Object.keys(merged) });
  if (!report.ok) {
    const first = report.rejections[0];
    throw new HotPlugError(
      first?.code === "UNKNOWN_CAPABILITY" ? "MISSING_DEPENDENCY" : (first?.code as HotPlugErrorCode) ?? "MISSING_DEPENDENCY",
      `装配校验失败: ${report.rejections.map((r) => r.message).join("; ")}`,
    );
  }

  // 2) 权限
  if (!req.permit?.(m)) throw new HotPlugError("PERMISSION_DENIED", `缺少授权或权限裁决未通过: ${m.id}`);

  // 3) register（AVAILABLE → REGISTERED → ENABLED）
  assertTransition("AVAILABLE", "REGISTERED");
  const rolledBack: string[] = [];
  try {
    target.addRecord(m);
    rolledBack.push("record");
    target.setState(m.id, "REGISTERED");
    assertTransition("REGISTERED", "ENABLED");
    target.setState(m.id, "ENABLED");

    // 4) contributions + loader
    rolledBack.push("contributions"); // loader may partially register before rejecting
    await req.load(m.id);
    target.addContributions(m);

    // 5) 若 policy=auto 则直接激活
    const notes = [
      `dependency resolution OK (resolved: ${report.resolved.join(",") || "∅"})`,
      ...report.warnings,
    ];
    if (m.activationPolicy === "auto") {
      // A visible ACTIVE bit is valid only after the Runtime Owner activation
      // promise has settled. Otherwise the UI can expose a dead capability.
      await target.activateOwner?.(m.id);
      assertTransition("ENABLED", "ACTIVE");
      target.setState(m.id, "ACTIVE");
    }
    // 6) verify：贡献必须真实存在
    const seen = target.stateOf(m.id);
    if (!["ENABLED", "ACTIVE"].includes(seen)) {
      throw new HotPlugError("INVALID_TRANSITION", `加入后状态异常: ${seen}`);
    }
    return { id: m.id, state: target.stateOf(m.id), notes };
  } catch (e) {
    // Failed Owner cleanup must retain a record for controlled recovery:
    // removing the record would orphan resources. UI actions fail closed.
    if (rolledBack.includes("contributions")) target.removeContributions(m.id);
    if (String(e).includes("OWNER_CLEANUP_FAILED")) {
      target.setState(m.id, "SUSPENDED");
      throw new HotPlugError("LOADER_FAILED",
        "加入失败且 Owner 清理未确认，保留 Runtime 记录: " + String(e));
    }
    if (rolledBack.includes("record")) {
      try { target.removeRecord(m.id); }
      catch (cleanupError) {
        target.setState(m.id, "SUSPENDED");
        throw new HotPlugError("LOADER_FAILED",
          "加入回滚未完成，保留 Runtime 记录: " + String(cleanupError));
      }
    }
    throw e instanceof HotPlugError ? e :
      new HotPlugError("LOADER_FAILED", "加入失败已回滚: " + String(e));
  }
  } finally { unlock(); }
}

export interface RemoveRequest {
  id: string;
  /** 显式绕过「存在强依赖」拒绝（对应 dependency policy，默认禁止） */
  force?: boolean;
}

export interface RemoveResult {
  id: string;
  state: PlatformState;
  notes: string[];
}

/**
 * 运行时移除能力（HP2 主流程）。
 * 默认策略：存在强依赖方 → REJECT；存在未结束工作 → 按 deactivationPolicy 裁决（graceful 先收尾，reject 则拒绝）。
 */
export async function requestRemove(target: HotPlugTarget, req: RemoveRequest): Promise<RemoveResult> {
  const m = target.installed[req.id];
  if (!m) throw new HotPlugError("NOT_INSTALLED", `能力未注册: ${req.id}`);
  if (!m.hotPlug?.unregister) {
    throw new HotPlugError("INVALID_TRANSITION", `${req.id} 未声明支持运行时 unregister（HP level=${m.hotPlug?.level}）`);
  }

  // 1) dependent check
  const dependents = Object.values(target.installed)
    .filter((x) => x.id !== req.id && (x.dependencies ?? []).includes(req.id))
    .map((x) => x.id)
    .sort();
  if (dependents.length > 0 && !req.force) {
    throw new HotPlugError(
      "DEPENDENT_PRESENT",
      `存在强依赖方，拒绝移除: ${dependents.join(",")}（可用 force 显式确认依赖策略）`,
    );
  }

  const notes: string[] = [];
  const from = target.stateOf(req.id);

  // 2) active work check
  const busy = from === "ACTIVE" || from === "BACKGROUND" || from === "SUSPENDED";
  if (busy && target.hasActiveWork?.(req.id) && m.deactivationPolicy === "reject") {
    throw new HotPlugError("ACTIVE_WORK_POLICY", `${req.id} 存在未结束工作且策略为 reject`);
  }

  // 3) Owner release must complete before DISABLED is externally observable.
  // A real Orchestrator supplies releaseOwner() backed by runtime.releaseAsync.
  // A generic Hot-Plug Target may supply its legacy onDeactivate hook instead.
  const unlock = lockTransition(target, req.id);
  try {
    if (from !== "DISABLED") {
      if (!canTransition(from, "DISABLED")) assertTransition(from, "DISABLED");
      if (target.releaseOwner) await target.releaseOwner(req.id);
      else if (target.onDeactivate) await target.onDeactivate(req.id);
      target.setState(req.id, "DISABLED");
    }
    // Existing DISABLED state already carries its release receipt.
    // Never call the Owner again for a disable -> remove transition.
    notes.push(`deactivated ${from} → DISABLED after Owner cleanup`);

    // The Runtime record may reject unregister for another still-live owner.
    // In that case fail closed (DISABLED) and remove UI actions rather than
    // claiming the capability is ACTIVE while its resources were released.
    try {
      target.removeRecord(req.id);
    } catch (error) {
      target.removeContributions(req.id);
      throw new HotPlugError("INVALID_TRANSITION", `Owner 已释放但注销未完成: ${String(error)}`);
    }
    target.removeContributions(req.id);
    target.setState(req.id, "UNREGISTERED");
    notes.push("runtime record removed; contributions removed");

    if (Object.keys(target.installed).includes(req.id)) {
      throw new HotPlugError("INVALID_TRANSITION", `移除后仍存在记录: ${req.id}`);
    }
  } finally { unlock(); }

  return { id: req.id, state: target.stateOf(req.id), notes };
}

/** HP1: every disable is an awaited Owner release, never a fire-and-forget hook. */
export async function disableCapability(target: HotPlugTarget, id: string): Promise<PlatformState> {
  const unlock = lockTransition(target, id);
  try {
    const m = target.installed[id];
    if (!m) throw new HotPlugError("NOT_INSTALLED", `能力未注册: ${id}`);
    if (!m.hotPlug?.disable) {
      throw new HotPlugError("INVALID_TRANSITION", `${id} 未声明支持 runtime disable（HP level=${m.hotPlug?.level}）`);
    }
    const from = target.stateOf(id);
    if (from === "DISABLED") return from;
    if (target.hasActiveWork?.(id) && m.deactivationPolicy === "reject") {
      throw new HotPlugError("ACTIVE_WORK_POLICY", `${id} 存在未结束工作且策略为 reject`);
    }
    assertTransition(from, "DISABLED");
    if (target.releaseOwner) await target.releaseOwner(id);
    else await target.onDeactivate?.(id);
    // State/UI removal are the commit phase, after the Owner receipt.
    target.setState(id, "DISABLED");
    target.removeContributions(id);
    return target.stateOf(id);
  } finally { unlock(); }
}

/** HP1 enable: preserve the same async owner/metadata ordering in reverse. */
export async function enableCapability(target: HotPlugTarget, id: string): Promise<PlatformState> {
  const unlock = lockTransition(target, id);
  try {
    const m = target.installed[id];
    if (!m) throw new HotPlugError("NOT_INSTALLED", `能力未注册: ${id}`);
    if (!m.hotPlug?.enable) {
      throw new HotPlugError("INVALID_TRANSITION", `${id} 未声明支持 runtime enable（HP level=${m.hotPlug?.level}）`);
    }
    const from = target.stateOf(id);
    assertTransition(from === "UNREGISTERED" ? "REGISTERED" : from, "ENABLED");
    try { await target.activateOwner?.(id); }
    catch (error) {
      // A partly cleaned Owner cannot be represented as fully DISABLED.
      // Keep the capability out of visible UI and mark a non-executable state
      // so it can be diagnosed/released before another activation attempt.
      if (String(error).includes("OWNER_CLEANUP_FAILED")) {
        target.removeContributions(id);
        target.setState(id, "SUSPENDED");
      }
      throw error;
    }
    target.setState(id, "ENABLED");
    target.addContributions(m);
    if (m.activationPolicy === "auto" && canTransition("ENABLED", "ACTIVE")) {
      target.setState(id, "ACTIVE");
    }
    return target.stateOf(id);
  } finally { unlock(); }
}
