import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const central = [
  'src/capability/index.ts', 'src/capability/platform/catalog.ts', 'src/main.ts', 'src/App.vue',
  'src/components/layout/MainArea.vue', 'src/components/system/SettingsPanel.vue',
]
const text = central.map((file) => fs.readFileSync(path.join(root, file), 'utf8')).join('\n')
const counts = {
  centralImports: (text.match(/from\s+['"][^'"]*(?:capabilities|capability-vault|capability-clipboard)[^'"]*['"]/g) ?? []).length,
  capabilityBranches: (text.match(/if\s*\([^\n]*(?:capability|mainView)[^\n]*\)|switch\s*\([^)]*(?:capability|mainView)/g) ?? []).length,
  manualContributions: (text.match(/register(?:Settings|[A-Z]\w*)Contributions/g) ?? []).length,
  centralManifestReferences: (text.match(/Manifest/g) ?? []).length,
  capabilitySpecificHostWiring: (text.match(/(?:vaultInstancePorts|clipboardInstancePorts|createVaultCapability|createClipboardCapability)/g) ?? []).length,
  hostInfrastructurePortCount: (fs.readFileSync(path.join(root, 'src/capability/platform/host-services.ts'), 'utf8').match(/'[^']+'/g) ?? []).length,
}
console.log(`LEGACY_CAPABILITY_MANUAL_WIRING_COUNT=${Object.values(counts).reduce((a, b) => a + b, 0)}`)
console.log(JSON.stringify(counts, null, 2))
