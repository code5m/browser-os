import type { CapabilityRuntime } from '../runtime'
import type { CapabilityManifestV1 } from './contract'
import type { ContributionRegistry } from '../contribution/registry'

export interface CapabilitySnapshot {
  id: string
  displayName: string
  version: string
  kind: string
  enabled: boolean
  state: string
  dependencies: string[]
  optionalDependencies: string[]
  conflicts: string[]
  provides: string[]
  contributions: string[]
  config: Record<string, unknown>
  configErrors: string[]
  disableReason: string | null
}

export function inspectCapabilities(
  catalog: Record<string, CapabilityManifestV1>,
  runtime: CapabilityRuntime,
  contributions: ContributionRegistry | undefined,
  configOf: (id: string) => { values: Record<string, unknown>; errors: string[] },
): CapabilitySnapshot[] {
  const records = new Map(runtime.inspect().map((entry) => [entry.id, entry]))
  return Object.values(catalog).sort((a, b) => a.id.localeCompare(b.id)).map((manifest) => {
    const record = records.get(manifest.id)
    const dependents = Object.values(catalog).filter((candidate) => candidate.dependencies.includes(manifest.id)).map((candidate) => candidate.id)
    const config = configOf(manifest.id)
    return {
      id: manifest.id,
      displayName: manifest.displayName,
      version: manifest.version,
      kind: manifest.kind ?? 'feature',
      enabled: record?.enabled ?? false,
      state: record?.state ?? 'AVAILABLE',
      dependencies: [...manifest.dependencies],
      optionalDependencies: [...manifest.optionalDependencies],
      conflicts: [...manifest.conflicts],
      provides: [...manifest.provides],
      contributions: contributions?.getBySlot('workbench-main').filter((item) => item.capabilityId === manifest.id).map((item) => item.id) ?? [],
      config: config.values,
      configErrors: config.errors,
      disableReason: manifest.kind === 'core' ? 'CORE capability cannot be disabled' : dependents.length ? `required by: ${dependents.join(', ')}` : null,
    }
  })
}
