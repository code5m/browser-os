import type {
  CapabilityCategory,
  CapabilityDefinition,
  CapabilityPersistence,
  CapabilityState,
  ResourceClass,
} from "../types"
import type {
  CapabilityManifestV1,
  ContributionDeclaration,
  PublicContractEntry,
  ResourceOwnership,
} from "./contract"

interface IntegratedCapabilitySpec {
  id: string
  name: string
  semanticOwner: string
  maturity: "C2" | "C3"
  version?: string
  maturityEvidence?: string[]
  supported?: CapabilityState[]
  provides: string[]
  dependencies?: string[]
  /** Assembly/portable contract dependencies may exclude host infrastructure such as bridge. */
  v1Dependencies?: string[]
  optionalDependencies?: string[]
  resident: boolean
  resourceClass: ResourceClass[]
  suspendable: boolean
  destroyable: boolean
  permissions?: string[]
  persistence: CapabilityPersistence
  contributions: ContributionDeclaration[]
  manifestResources?: ResourceOwnership[]
  publicContract: PublicContractEntry[]
  deactivationPolicy?: CapabilityManifestV1["deactivationPolicy"]
  limitationReason: string
  hotPlugLevel?: "HP0" | "HP1" | "HP2"
  description?: string
  category?: CapabilityCategory
}

export function defineIntegratedCapability(spec: IntegratedCapabilitySpec): CapabilityDefinition {
  const hotPlugLevel = spec.hotPlugLevel ?? "HP0"
  const toggleable = hotPlugLevel !== "HP0"
  const dependencies = spec.dependencies ?? ["bridge"]
  const optionalDependencies = spec.optionalDependencies ?? []
  const permissions = spec.permissions ?? []
  const v1: CapabilityManifestV1 = {
    id: spec.id,
    version: spec.version ?? "2.0.0",
    displayName: spec.name,
    description: spec.description ?? spec.name,
    maturity: spec.maturity,
    maturityEvidence: spec.maturityEvidence ?? ["scripts/check-v2-maturity-boundaries.mjs"],
    dependencies: spec.v1Dependencies ?? dependencies,
    optionalDependencies,
    conflicts: [],
    provides: spec.provides,
    requires: [],
    contributions: spec.contributions,
    permissions,
    resources: spec.manifestResources ?? [],
    persistenceScope: spec.persistence.scope,
    persistenceSensitive: spec.persistence.sensitive,
    activationPolicy: "auto",
    deactivationPolicy: spec.deactivationPolicy ?? "reject",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: hotPlugLevel,
      enable: toggleable,
      disable: toggleable,
      register: hotPlugLevel === "HP2",
      unregister: hotPlugLevel === "HP2",
      install: false,
      uninstall: false,
      limitationReason: spec.limitationReason,
    },
    publicContract: spec.publicContract,
    entrypoint: `src/capabilities/${spec.id}/index.ts`,
    semanticOwner: spec.semanticOwner,
  }
  return {
    id: spec.id,
    name: spec.name,
    category: spec.category ?? "CAPABILITY",
    provides: spec.provides,
    dependsOn: dependencies,
    optionalDependencies,
    lifecycle: {
      supported: spec.supported ?? ["ACTIVE"],
      default: "ACTIVE",
      activatable: true,
      resident: spec.resident,
    },
    resources: {
      class: spec.resourceClass,
      suspendable: spec.suspendable,
      destroyable: spec.destroyable,
    },
    permissions,
    persistence: spec.persistence,
    entrypoint: "index.ts",
    semanticOwner: spec.semanticOwner,
    governanceStatus: "GOVERNED",
    status: "COMPATIBILITY_WRAPPED",
    v1,
  }
}
