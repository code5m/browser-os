import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const source = fs.readFileSync(path.join(root, 'src/capability/platform/contract.ts'), 'utf8')
const runtime = fs.readFileSync(path.join(root, 'src/capability/runtime.ts'), 'utf8')
const config = fs.readFileSync(path.join(root, 'src/capability/platform/config.ts'), 'utf8')
const required = [
  ['manifest', /interface CapabilityManifestV1/],
  ['kind', /CapabilityKind = "core" \| "feature" \| "optional"/],
  ['config schema', /interface CapabilityConfigSchema/],
  ['reverse dependency guard', /DEPENDENT_PRESENT/],
  ['persistent configuration', /config\?: Record<string, Record<string, CapabilityConfigValue>>/],
]
const failures = required.filter(([, pattern]) => !pattern.test(`${source}\n${runtime}\n${config}`))
if (failures.length) {
  console.error(`CAPABILITY_REMOVAL_AUDIT=FAIL missing=${failures.map(([name]) => name).join(',')}`)
  process.exit(1)
}
console.log('CAPABILITY_REMOVAL_AUDIT=PASS')
