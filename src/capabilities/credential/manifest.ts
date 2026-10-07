import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"

export const credentialManifest: CapabilityDefinition = defineIntegratedCapability({
  id: "credential",
  name: "凭据",
  semanticOwner: "KeyringStore",
  maturity: "C2",
  provides: ["credential.save", "credential.get", "credential.delete"],
  optionalDependencies: ["browser"],
  resident: true,
  resourceClass: ["SECURITY_SENSITIVE"],
  suspendable: false,
  destroyable: false,
  permissions: ["keyring.access"],
  persistence: { scope: "os_keyring", sensitive: true },
  contributions: [{ id: "credential.list", slot: "bookmark-credentials", type: "surface" }],
  publicContract: [{ name: "publicApi", locator: "src/capabilities/credential/public.ts" }],
  limitationReason: "Resident keyring security service is not disableable.",
})
