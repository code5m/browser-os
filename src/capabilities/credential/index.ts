import { withLazyContributions } from "../../capability/platform/contributed"
import { credentialManifest } from "./manifest"

export const CREDENTIAL_CAPABILITY_ID = "credential"
export const credentialCapability = withLazyContributions(credentialManifest, [
  { id: "credential.list", type: "surface", slot: "bookmark-credentials", load: () => import("./ui/CredentialList.vue") },
])
