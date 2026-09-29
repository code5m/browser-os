import type { CapabilityManifestV1 } from '../../../src/capability/platform/contract'

export const demoManifest: CapabilityManifestV1 = {
  id: 'demo', version: '1.0.0', runtimeApiVersion: '1.0.0', displayName: 'Demo Capability',
  description: 'Acceptance fixture for generated discovery and lifecycle wiring.', kind: 'optional',
  maturity: 'C1', maturityEvidence: ['check:capability-discovery'], dependencies: [], optionalDependencies: [], conflicts: [],
  provides: ['demo.lifecycle'], requires: [], contributions: [{ id: 'demo.panel', slot: 'panels', type: 'surface' }],
  permissions: [], resources: [], persistenceScope: 'runtime_only', persistenceSensitive: false,
  activationPolicy: 'manual', deactivationPolicy: 'graceful', installPolicy: 'static', uninstallPolicy: 'denied',
  hotPlug: { level: 'HP1', enable: true, disable: true, register: false, unregister: false, install: false, uninstall: false, limitationReason: 'fixture only' },
  publicContract: [], entrypoint: 'packages/capability-demo/src/index.ts', semanticOwner: null,
}
