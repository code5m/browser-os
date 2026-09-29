import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const output = path.join(root, 'src/capability/platform/generated-registry.ts')
const files = []
function walk(dir) {
  if (!fs.existsSync(dir)) return
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(full)
    else if (entry.name === 'manifest.ts') files.push(full)
  }
}
walk(path.join(root, 'src/capabilities'))
walk(path.join(root, 'packages'))
walk(path.join(root, 'src/settings'))
const entries = files.map((file) => {
  const match = fs.readFileSync(file, 'utf8').match(/export const (\w+Manifest)\s*:/)
  if (!match) return null
  const index = path.join(path.dirname(file), 'index.ts')
  const capability = fs.existsSync(index) ? fs.readFileSync(index, 'utf8').match(/export const (\w+Capability)\s*(?::|=)/)?.[1] : null
  return { file: './' + path.relative(path.dirname(output), file).replaceAll(path.sep, '/').replace(/\.ts$/, ''), name: match[1], index: capability ? './' + path.relative(path.dirname(output), index).replaceAll(path.sep, '/').replace(/\.ts$/, '') : null, capability }
}).filter(Boolean).sort((a, b) => a.file.localeCompare(b.file))
const imports = entries.map((entry, index) => `import { ${entry.name} as manifest${index} } from '${entry.file}'`).join('\n')
let definitionIndex = 0
const definitionImports = entries.filter((entry) => entry.capability).map((entry) => `import { ${entry.capability} as definition${definitionIndex++} } from '${entry.index}'`).join('\n')
const values = entries.map((_, index) => `manifest${index}.v1 ?? manifest${index}`).join(', ')
let definitionValueIndex = 0
const definitions = entries.map((entry) => entry.capability ? `'${entry.name.replace(/Manifest$/, '')}': definition${definitionValueIndex++}` : '').filter(Boolean).join(', ')
const body = `// GENERATED FILE. Run npm run generate:capability-registry.\nimport type { CapabilityManifestV1 } from './contract'\n${imports}\n${definitionImports}\n\nexport const GENERATED_MANIFESTS: CapabilityManifestV1[] = [${values}]\nexport const GENERATED_MANIFEST_SOURCES = ${JSON.stringify(entries.map((entry) => entry.file))} as const\nexport const GENERATED_DEFINITIONS = { ${definitions} } as const\n`
if (process.argv.includes('--check')) {
  if (!fs.existsSync(output) || fs.readFileSync(output, 'utf8') !== body) { console.error('CAPABILITY_GENERATED_REGISTRY=FAIL stale generated registry'); process.exit(1) }
} else fs.writeFileSync(output, body)
console.log(`CAPABILITY_GENERATED_REGISTRY=PASS manifests=${entries.length}`)
