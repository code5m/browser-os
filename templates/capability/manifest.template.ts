import type { CapabilityDefinition } from "../../../src/capability/types";

// Capability Template — 复制本文件到 src/capabilities/<id>/manifest.ts 后改名使用
// 约束：
//   1. id 是语义身份，物理路径变了 id 不能变。
//   2. hotPlug 必须与你真实做到的程度一致（HP0 不能写 HP2）。
//   3. maturityEvidence 必须填真实 checker/测试名；声称 C4/C5 必须有释放/资源证据。
//   4. 不得把 secret 写进 manifest。
export const TEMPLATE_ID = "<changeme>";

export const templateDefinition: CapabilityDefinition = {
  id: TEMPLATE_ID,
  name: "<显示名>",
  category: "CAPABILITY",
  provides: [`${TEMPLATE_ID}.something`],
  dependsOn: [],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: { class: [], suspendable: false, destroyable: false },
  permissions: [],
  persistence: { scope: "runtime_only", sensitive: false },
  entrypoint: "index.ts",
  semanticOwner: null,
  governanceStatus: "OWNER_PENDING_SCR",
  status: "NOT_INTEGRATED",
  v1: {
    id: TEMPLATE_ID,
    version: "0.1.0",
    displayName: "<显示名>",
    description: "<一句话说明它提供什么、占什么资源>",
    maturity: "C0",
    maturityEvidence: [],
    dependencies: [],
    optionalDependencies: [],
    conflicts: [],
    provides: [`${TEMPLATE_ID}.something`],
    requires: [],
    contributions: [
      // { id: `${TEMPLATE_ID}.main.view`, slot: "workbench-main", type: "surface", view: "<mainViewKey>" }
    ],
    permissions: [],
    resources: [],
    persistenceScope: "runtime_only",
    persistenceSensitive: false,
    activationPolicy: "auto",
    deactivationPolicy: "manual",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP0",
      enable: false,
      disable: false,
      register: false,
      unregister: false,
      install: false,
      uninstall: false,
      limitationReason: "HP0：尚未验证运行时启停/释放，先诚实声明限制。",
    },
    publicContract: [{ name: "publicApi", locator: `src/capabilities/${TEMPLATE_ID}/public.ts` }],
    entrypoint: `src/capabilities/${TEMPLATE_ID}/index.ts`,
    semanticOwner: null,
  },
};
