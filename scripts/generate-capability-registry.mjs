import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const output = path.join(root, 'src/capability/platform/generated-registry.ts')
function moduleSpecifier(file) {
  const relative = path.relative(root, file).replaceAll(path.sep, '/')
  if (!relative.startsWith('packages/')) return './' + path.relative(path.dirname(output), file).replaceAll(path.sep, '/').replace(/\.ts$/, '')
  const [, packageDir, ...rest] = relative.split('/')
  const packageJsonPath = path.join(root, 'packages', packageDir, 'package.json')
  if (!fs.existsSync(packageJsonPath)) return './' + path.relative(path.dirname(output), file).replaceAll(path.sep, '/').replace(/\.ts$/, '')
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))
  const subpath = rest.join('/').replace(/\.ts$/, '')
  if (subpath === 'src/index') return packageJson.name
  if (subpath === 'src/manifest') return `${packageJson.name}/manifest`
  return `${packageJson.name}/${subpath.replace(/^src\//, '')}`
}
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
  return { file: moduleSpecifier(file), name: match[1], index: capability ? moduleSpecifier(index) : null, capability }
}).filter(Boolean).sort((a, b) => a.file.localeCompare(b.file))
const imports = entries.map((entry, index) => `import { ${entry.name} as manifest${index} } from '${entry.file}'`).join('\n')
let definitionIndex = 0
const definitionImports = entries.filter((entry) => entry.capability).map((entry) => `import { ${entry.capability} as definition${definitionIndex++} } from '${entry.index}'`).join('\n')
const values = entries.map((_, index) => `manifest${index}.v1 ?? manifest${index}`).join(', ')
let definitionValueIndex = 0
const definitions = entries.map((entry) => entry.capability ? `'${entry.name.replace(/Manifest$/, '')}': definition${definitionValueIndex++}` : '').filter(Boolean).join(', ')
const body = `// GENERATED FILE. Run npm run generate:capability-registry.\nimport type { CapabilityManifestV1 } from './contract'\n${imports}\n${definitionImports}\n\nexport const GENERATED_MANIFESTS: CapabilityManifestV1[] = [${values}]\nexport const GENERATED_MANIFEST_SOURCES = ${JSON.stringify(entries.map((entry) => entry.file))} as const\nexport const GENERATED_DEFINITIONS = { ${definitions} } as const\nexport const GENERATED_CANONICAL_METADATA = Object.fromEntries(\n  GENERATED_MANIFESTS.map((manifest) => [manifest.id, {\n    id: manifest.id,\n    displayName: manifest.displayName,\n    maturity: manifest.maturity,\n    dependencies: [...manifest.dependencies],\n    optionalDependencies: [...manifest.optionalDependencies],\n    semanticOwner: manifest.semanticOwner,\n    entrypoint: manifest.entrypoint,\n  }]),\n) as Record<string, Pick<CapabilityManifestV1, 'id' | 'displayName' | 'maturity' | 'dependencies' | 'optionalDependencies' | 'semanticOwner' | 'entrypoint'>>\n`
if (process.argv.includes('--check')) {
  if (!fs.existsSync(output) || fs.readFileSync(output, 'utf8') !== body) { console.error('CAPABILITY_GENERATED_REGISTRY=FAIL stale generated registry'); process.exit(1) }
} else fs.writeFileSync(output, body)
console.log(`CAPABILITY_GENERATED_REGISTRY=PASS manifests=${entries.length}`)
