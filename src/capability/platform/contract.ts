// Building Block Contract v1（Universal Capability Platform）
//
// 铁律：
//   1. 本模块**只有类型与纯校验函数**，零副作用、零 import 任何能力/store/UI —— 保证
//      「引入契约」本身不会加载任何重资源（absent 语义成立的前提）。
//   2. 不得把 secret 放入 manifest。
//   3. maturity / hotPlug 必须与真实证据一致：禁止把目标态写成现状（§19/§40）。
//   4. 所有集合字段（数组）在解析前必须已排序无关地 determinism 处理 —— 由 resolver 负责排序。

export const BUILDING_BLOCK_CONTRACT_VERSION = "1.0.0";

/** §40 成熟度（与既有 capability maturity 模型一致） */
export type MaturityLevel = "C0" | "C1" | "C2" | "C3" | "C4" | "C5";

/** §19/§40 热插拔等级（不得与成熟度混淆：C4 ≠ HP2） */
export type HotPlugLevel = "HP0" | "HP1" | "HP2" | "HP3";

/** §26 资源归属：以真实代码为准 */
export type ResourceKind =
  | "WEBVIEW"
  | "PTY"
  | "CHILD_PROCESS"
  | "DB_CONNECTION"
  | "WATCHER"
  | "BACKGROUND_TASK"
  | "SOCKET"
  | "CACHE";

export interface HotPlugPolicy {
  level: HotPlugLevel;
  /** HP1：运行时启用/停用 */
  enable: boolean;
  disable: boolean;
  /** HP2：运行时注册/注销 */
  register: boolean;
  unregister: boolean;
  /** HP3：运行时安装/卸载外部能力包 */
  install: boolean;
  uninstall: boolean;
  /** 未能达到更高等级的诚实原因（HP0/HP1 必须给出） */
  limitationReason?: string;
}

export interface ResourceOwnership {
  kind: ResourceKind;
  ownership: "owned" | "shared" | "borrowed";
  /** 证据：真实代码位置或测量脚本，缺失则归类 DECLARED 而非 MEASURED（§44） */
  evidence?: string;
}

export interface ContributionDeclaration {
  id: string;
  slot: string;
  type: string;
  /** 该贡献对应的 mainView 认领键（仅 WORKBENCH_MAIN 类槽使用） */
  view?: string;
}

export interface PublicContractEntry {
  name: string;
  /** implementation locator：必须能指向真实代码路径（checker 校验存在，§34） */
  locator: string;
}

export interface CapabilityManifestV1 {
  id: string;
  version: string;
  displayName: string;
  description: string;

  maturity: MaturityLevel;
  /** 支撑该 maturity 判定的Checker/测试名（禁止空口宣称） */
  maturityEvidence?: string[];

  dependencies: string[];
  optionalDependencies: string[];
  conflicts: string[];

  provides: string[];
  requires: string[];

  contributions: ContributionDeclaration[];

  permissions: string[];
  resources: ResourceOwnership[];
  persistenceScope: "session" | "disk" | "os_keyring" | "runtime_only";
  persistenceSensitive: boolean;

  activationPolicy: "auto" | "lazy" | "manual";
  deactivationPolicy: "auto" | "graceful" | "reject" | "manual";
  installPolicy: "static" | "runtime" | "denied";
  uninstallPolicy: "static" | "runtime" | "denied";
  hotPlug: HotPlugPolicy;

  publicContract: PublicContractEntry[];

  entrypoint: string;
  semanticOwner: string | null;
}

export interface ManifestViolation {
  code: string;
  message: string;
}

const ID_RE = /^[a-z][a-z0-9.]*$/;
const VERSION_RE = /^\d+\.\d+\.\d+$/;
const MATURITY: MaturityLevel[] = ["C0", "C1", "C2", "C3", "C4", "C5"];
const HP: HotPlugLevel[] = ["HP0", "HP1", "HP2", "HP3"];
const UNIQ = (a: string[]): string[] => Array.from(new Set(a));

function isStringArray(v: unknown): v is string[] {
  return Array.isArray(v) && v.every((x) => typeof x === "string");
}

/** 机器可读的结构校验（§9：必须有 schema 且有 checker 验证）。 */
export function validateManifestV1(m: CapabilityManifestV1): ManifestViolation[] {
  const v: ManifestViolation[] = [];
  const push = (code: string, message: string) => v.push({ code, message });

  if (!ID_RE.test(m.id)) push("CAP-ID", `id 必须匹配 ${ID_RE}：${String(m.id)}`);
  if (!VERSION_RE.test(m.version ?? "")) push("CAP-VERSION", `version 必须为 semver：${String(m.version)}`);
  if (typeof m.displayName !== "string" || m.displayName.length === 0) push("CAP-NAME", "displayName 缺失");
  if (typeof m.description !== "string" || m.description.length === 0) push("CAP-DESC", "description 缺失");
  if (!MATURITY.includes(m.maturity)) push("CAP-MATURITY", `maturity 非法：${String(m.maturity)}`);
  if (!HP.includes(m.hotPlug?.level)) push("CAP-HP", `hotPlug.level 非法：${String(m.hotPlug?.level)}`);

  for (const [field, arr] of [
    ["dependencies", m.dependencies],
    ["optionalDependencies", m.optionalDependencies],
    ["conflicts", m.conflicts],
    ["provides", m.provides],
    ["requires", m.requires],
    ["permissions", m.permissions],
  ] as const) {
    if (!isStringArray(arr)) push("CAP-TYPE", `${field} 必须为字符串数组`);
  }

  // 依赖与冲突不得自相矛盾
  const overlap = (m.dependencies ?? []).filter((d) => (m.conflicts ?? []).includes(d));
  if (overlap.length > 0) push("CAP-DEP-CONFLICT", `依赖与冲突重叠：${overlap.join(",")}`);
  if ((m.dependencies ?? []).includes(m.id)) push("CAP-SELF-DEP", "不允许自依赖");
  if (!UNIQ(m.dependencies ?? []).length && (m.dependencies ?? []).length > 0) push("CAP-DUP-DEP", "依赖重复");

  // 等级与能力必须自洽（§19：禁止伪造 HP）
  const hp = m.hotPlug;
  if (hp) {
    if (hp.level === "HP0" && (hp.enable || hp.register || hp.install)) {
      push("CAP-HP-INCONSISTENT", "HP0 不得声明任何运行时操作");
    }
    if (hp.level === "HP1" && (hp.register || hp.install || hp.unregister || hp.uninstall)) {
      push("CAP-HP-INCONSISTENT", "HP1 只允许 enable/disable，不得声明 register/install");
    }
    if (hp.level === "HP2" && (hp.install || hp.uninstall)) {
      push("CAP-HP-INCONSISTENT", "HP2 不得声明 install/uninstall");
    }
    if (hp.level === "HP1" && !hp.enable) push("CAP-HP-INCONSISTENT", "HP1 必须 allow enable");
    if (hp.level === "HP2" && !(hp.enable && hp.register)) push("CAP-HP-INCONSISTENT", "HP2 必须允许 enable + register");
    if (hp.level === "HP3" && !hp.install) push("CAP-HP-INCONSISTENT", "HP3 必须 allow install");
    if (hp.level !== "HP3" && !hp.limitationReason) {
      push("CAP-HP-NO-REASON", `${hp.level} 未给出未达更高等级的诚实原因`);
    }
  } else {
    push("CAP-HP", "hotPlug policy 缺失");
  }

  // C5 只能由真实 release 证据支撑（§29）；由外部 checker 交叉验证 MaturityEvidence
  if (m.maturity === "C5" && !(m.maturityEvidence ?? []).some((e) => /release|destroy|evidence/i.test(e))) {
    push("CAP-MATURITY-EVIDENCE", "声称 C5 但缺少 release 证据项");
  }
  if (m.maturity === "C4" && !(m.maturityEvidence ?? []).length) {
    push("CAP-MATURITY-EVIDENCE", "声称 C4 但未给 evidence");
  }

  if (!Array.isArray(m.contributions)) push("CAP-CONTRIB", "contributions 必须为数组");
  else if (m.contributions.length === 0) push("CAP-CONTRIB-EMPTY", "能力未声明任何 contribution");

  if (!Array.isArray(m.resources)) push("CAP-RES", "resources 必须为数组");
  if (!["session", "disk", "os_keyring", "runtime_only"].includes(m.persistenceScope)) {
    push("CAP-PERSIST", `persistenceScope 非法：${String(m.persistenceScope)}`);
  }
  if (typeof m.entrypoint !== "string" || m.entrypoint.length === 0) push("CAP-ENTRY", "entrypoint 缺失");
  if (m.semanticOwner !== null && typeof m.semanticOwner !== "string") push("CAP-OWNER", "semanticOwner 类型非法");

  return v;
}

/** 由 giving capability 提供的、可被他人合法引用的契约名集合（§11 公共契约） */
export function providedContractNames(m: CapabilityManifestV1): Set<string> {
  return new Set([
    ...m.provides,
    ...m.publicContract.map((p) => `${m.id}:${p.name}`),
  ]);
}
