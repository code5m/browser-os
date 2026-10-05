import { bridge } from "../../bridge"
import type { AutofillResult, BrowserCredentialItem } from "../../types"

export const credentialPublicApi = {
  importBrowserCredentials(rows: { url: string; username: string; password: string }[]): Promise<number> {
    return bridge.importBrowserCredentials(rows)
  },
  listBrowserCredentials(): Promise<BrowserCredentialItem[]> {
    return bridge.listBrowserCredentials()
  },
  fillBrowserCredential(credentialId: string, tabId: string): Promise<AutofillResult> {
    return bridge.fillBrowserCredential(credentialId, tabId)
  },
}
