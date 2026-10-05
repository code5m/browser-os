import { defineAsyncComponent } from "vue"
import { contributionRegistry } from "../../capability/contribution/registry"
import { CONTRIBUTION_SLOTS } from "../../capability/contribution/types"
import { credentialManifest } from "./manifest"

export const CREDENTIAL_CAPABILITY_ID = "credential"
const CredentialList = defineAsyncComponent(() => import("./ui/CredentialList.vue"))

export function registerCredentialContributions(): void {
  contributionRegistry.registerContribution({
    id: "credential.list",
    capabilityId: CREDENTIAL_CAPABILITY_ID,
    type: "surface",
    slot: CONTRIBUTION_SLOTS.BOOKMARK_CREDENTIALS,
    component: CredentialList,
  })
}

export const credentialCapability = {
  ...credentialManifest,
  lifecycle: {
    ...credentialManifest.lifecycle,
    onActivate: registerCredentialContributions,
  },
}
