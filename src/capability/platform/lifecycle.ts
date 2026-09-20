// Runtime Lifecycle Model（§18）
//
// 目的：把「在不在产品里」和「跑没跑起来」彻底分开。
//   AVAILABLE    在 catalog 中，但未注册进 runtime（可以被装配）
//   REGISTERED   已注册，尚未启用（不暴露任何贡献）
//   ENABLED      已启用，但未激活（可以被 render，但还没有跑起来）
//   ACTIVE       正在运行（持有其占用的资源）
//   BACKGROUND   仍在运行，但被降优先级（前台不可见，后台策略仍生效）
//   SUSPENDED    暂挂（是否释放资源由该能力的 resource policy 决定）
//   DISABLED     已停用（贡献摘除、不得暴露能力动作）
//   UNREGISTERED 已移出 runtime（可以从产品中拿掉）
//
// 三条必须区分的语义：
//   AVAILABLE   != ENABLED        （在库里 ≠ 已启用）
//   ENABLED     != ACTIVE         （已启用 ≠ 正在跑）
//   DISABLED    != UNINSTALLED    （停用 ≠ 卸载）

export type PlatformState =
  | "AVAILABLE"
  | "REGISTERED"
  | "ENABLED"
  | "ACTIVE"
  | "BACKGROUND"
  | "SUSPENDED"
  | "DISABLED"
  | "UNREGISTERED";

export const PLATFORM_STATES: PlatformState[] = [
  "AVAILABLE",
  "REGISTERED",
  "ENABLED",
  "ACTIVE",
  "BACKGROUND",
  "SUSPENDED",
  "DISABLED",
  "UNREGISTERED",
];

/** 合法迁移表：未列出的迁移一律 INVALID_TRANSITION */
const TRANSITIONS: Record<PlatformState, PlatformState[]> = {
  AVAILABLE: ["REGISTERED"],
  REGISTERED: ["ENABLED", "DISABLED", "UNREGISTERED"],
  ENABLED: ["ACTIVE", "DISABLED", "UNREGISTERED"],
  ACTIVE: ["BACKGROUND", "SUSPENDED", "DISABLED"],
  BACKGROUND: ["ACTIVE", "SUSPENDED", "DISABLED"],
  SUSPENDED: ["ACTIVE", "DISABLED"],
  DISABLED: ["ENABLED", "UNREGISTERED"],
  UNREGISTERED: ["REGISTERED"],
};

export class LifecycleError extends Error {
  code: "INVALID_TRANSITION" | "UNKNOWN_STATE";
  constructor(code: "INVALID_TRANSITION" | "UNKNOWN_STATE", message: string) {
    super(message);
    this.name = "LifecycleError";
    this.code = code;
  }
}

export function nextStates(from: PlatformState): PlatformState[] {
  if (!PLATFORM_STATES.includes(from)) throw new LifecycleError("UNKNOWN_STATE", `未知状态: ${from}`);
  return [...(TRANSITIONS[from] ?? [])];
}

export function canTransition(from: PlatformState, to: PlatformState): boolean {
  return nextStates(from).includes(to);
}

export function assertTransition(from: PlatformState, to: PlatformState): void {
  if (!PLATFORM_STATES.includes(from)) throw new LifecycleError("UNKNOWN_STATE", `未知状态: ${String(from)}`);
  if (!PLATFORM_STATES.includes(to)) throw new LifecycleError("UNKNOWN_STATE", `未知状态: ${String(to)}`);
  if (!canTransition(from, to)) {
    throw new LifecycleError(
      "INVALID_TRANSITION",
      `非法生命周期迁移: ${from} → ${to}（允许: ${nextStates(from).join("/") || "无"}）`,
    );
  }
}

/** 该状态是否允许向用户暴露能力动作（§28 disabled means disabled） */
export function exposesActions(s: PlatformState): boolean {
  return s === "ACTIVE" || s === "BACKGROUND";
}

/** 该状态是否要求「重资源必须按需创建」（§27 absent means absent） */
export function mayHoldHeavyResources(s: PlatformState): boolean {
  return s === "ACTIVE" || s === "BACKGROUND" || s === "SUSPENDED";
}
