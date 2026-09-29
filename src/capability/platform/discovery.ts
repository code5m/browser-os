import type { CapabilityManifestV1 } from './contract'
import { validateManifestV1 } from './contract'

export interface DiscoveredCapability {
  manifest: CapabilityManifestV1
  source: string
}

export interface DiscoveryResult {
  capabilities: DiscoveredCapability[]
  catalog: Record<string, CapabilityManifestV1>
  errors: string[]
}

/** Build-time generated/imported sources are passed here; runtime never imports arbitrary strings. */
export function discoverCapabilities(sources: DiscoveredCapability[]): DiscoveryResult {
  const catalog: Record<string, CapabilityManifestV1> = {}
  const errors: string[] = []
  for (const source of sources) {
    const violations = validateManifestV1(source.manifest)
    if (violations.length) errors.push(`${source.source}: ${violations.map((v) => v.message).join('; ')}`)
    if (catalog[source.manifest.id]) errors.push(`重复 capability id: ${source.manifest.id} (${source.source})`)
    else catalog[source.manifest.id] = source.manifest
  }
  return { capabilities: sources, catalog, errors }
}
