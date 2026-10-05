import { createClipboardCapability, clipboardContribution, type ClipboardPorts } from '@browser-os/capability-clipboard'
import { createVaultCapability, vaultContribution, type VaultPorts } from '@browser-os/capability-vault'
import { bridge } from '../../bridge'
import { useLayoutStore } from '../../stores/useLayoutStore'
import { useWorkbenchStore } from '../../capabilities/workbench/public'
import { layoutPositions } from '../../utils/graphUi'
import { redactSecrets } from '../../utils/redact'
import { contributionRegistry, type ContributionRegistry } from '../contribution/registry'
import { hostServices, type HostServiceRegistry } from './host-services'

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
