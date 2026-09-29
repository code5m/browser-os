import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const capabilityRoot = path.join(root, 'src/capabilities')
const violations = []
function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(file)
    else if (/\.(ts|vue)$/.test(entry.name)) {
      const text = fs.readFileSync(file, 'utf8')
      const rel = path.relative(root, file)
      if (/from\s+['"][^'"]*src\/capability\/(runtime|platform|index)/.test(text)) violations.push(`${rel}: direct runtime/platform import`)
      if (/from\s+['"]\.\.\/\.\.\/capabilities\/[^/]+\/(state|ui|index)/.test(text)) violations.push(`${rel}: cross-capability internal import`)
    }
  }
}
walk(capabilityRoot)
if (violations.length) { console.error(`CAPABILITY_BOUNDARY=FAIL\n${violations.join('\n')}`); process.exit(1) }
console.log('CAPABILITY_BOUNDARY=PASS')
