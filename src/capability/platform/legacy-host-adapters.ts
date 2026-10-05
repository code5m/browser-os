import { createClipboardCapability, clipboardContribution, type ClipboardPorts } from '@browser-os/capability-clipboard'
import { createVaultCapability, vaultContribution, type VaultPorts } from '@browser-os/capability-vault'
import { bridge } from '../../bridge'
import { useLayoutStore } from '../../stores/useLayoutStore'
import { useWorkbenchStore } from '../../capabilities/workbench/public'
import { useBrowserStore } from '../../capabilities/browser/public'
import { layoutPositions } from '../../utils/graphUi'
import { redactSecrets } from '../../utils/redact'
import { contributionRegistry, type ContributionRegistry } from '../contribution/registry'
import { hostServices, type BrowserContextPort, type HostServiceRegistry } from './host-services'

export interface LegacyHostAdapters {
  vault: ReturnType<typeof createVaultCapability>
  clipboard: ReturnType<typeof createClipboardCapability>
  vaultPorts: VaultPorts
  clipboardPorts: ClipboardPorts
}

export function createLegacyHostAdapters(
  services: HostServiceRegistry = hostServices,
  contributions: ContributionRegistry = contributionRegistry,
): LegacyHostAdapters {
  services.register('bridge', bridge)
  services.register('layout', useLayoutStore)
  services.register('workbench', useWorkbenchStore)
  services.register('graph-layout', (nodes: unknown[], edges: unknown[]) => layoutPositions(nodes as any, edges as any))
  services.register('redact-secrets', redactSecrets)
  const browserContext: BrowserContextPort = {
    get activeTabId() { return useBrowserStore().activeTabId },
    get activeUrl() { return useBrowserStore().activeTab?.url || useBrowserStore().url || '' },
    get recentlyClosed() { return useBrowserStore().recentlyClosed },
    get aiNavOpen() { return useBrowserStore().aiNavOpen },
    openTab(url: string) { return useBrowserStore().tabNew(url) },
    evalInTab(tabId: string, script: string) { return bridge.evalInTab(tabId, script) },
    async captureTextPreview(tabId: string) {
      const text = await bridge.evalInTab(
        tabId,
        "(document.body && document.body.innerText ? document.body.innerText.slice(0, 2000) : '')",
      )
      return typeof text === 'string' ? text : ''
    },
    activateGrid() { return useBrowserStore().activateGrid() },
    setAiNavOpen(value: boolean) { useBrowserStore().aiNavOpen = value },
    adoptRestoredTab(tab: unknown) {
      const browser = useBrowserStore()
      const restored = tab as { id: string; url?: string }
      browser.tabs.push(restored as any)
      browser.activeTabId = restored.id
      useLayoutStore().setView('browser')
    },
  }
  services.register('browser-context', browserContext)

  const vaultPorts: VaultPorts = {
    native: { openVault: (path: string) => services.require<typeof bridge>('bridge').vaultOpen(path) },
    shell: { isWorkbenchCollapsed: () => services.require<typeof useWorkbenchStore>('workbench')().collapsed.value },
    graphLayout: { layoutGraph: (nodes, edges) => services.require<(nodes: unknown[], edges: unknown[]) => unknown>('graph-layout')(nodes as any, edges as any) },
  }
  const vault = createVaultCapability(vaultPorts)
  vault.vaultCapability.lifecycle.onActivate = () => contributions.registerContribution(vaultContribution as any)

  const clipboardPorts: ClipboardPorts = {
    native: {
      clipboardRead: () => services.require<typeof bridge>('bridge').clipboardRead(),
      clipboardWrite: (text: string) => services.require<typeof bridge>('bridge').clipboardWrite(text),
    },
    ui: {
      showToast: (message: string) => services.require<ReturnType<typeof useLayoutStore>>('layout')().showToast(message),
      requestClose: () => { services.require<ReturnType<typeof useLayoutStore>>('layout')().clipOpen = false },
      redactSecrets: (text: string) => services.require<(text: string) => string>('redact-secrets')(text),
    },
  }
  const clipboard = createClipboardCapability(clipboardPorts)
  clipboard.clipboardCapability.lifecycle.onActivate = () => contributions.registerContribution(clipboardContribution as any)

  return { vault, clipboard, vaultPorts, clipboardPorts }
}
