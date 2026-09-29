import type { CapabilityDefinition } from '../../../src/capability/types'
import { demoManifest } from './manifest'

export const demoCapability: CapabilityDefinition = {
  id: demoManifest.id, name: demoManifest.displayName, category: 'CAPABILITY', provides: demoManifest.provides,
  dependsOn: [], optionalDependencies: [], lifecycle: { supported: ['ACTIVE', 'SUSPENDED'], default: 'ACTIVE', activatable: true, resident: false },
  resources: { class: [], suspendable: true, destroyable: true }, permissions: [], persistence: { scope: 'runtime_only', sensitive: false },
  entrypoint: demoManifest.entrypoint, semanticOwner: demoManifest.semanticOwner, governanceStatus: 'GOVERNED', status: 'COMPATIBILITY_WRAPPED', v1: demoManifest,
}
