import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const roots = [path.join(root, 'src/capabilities'), path.join(root, 'packages')]
const manifests = []
function walk(dir) {
  if (!fs.existsSync(dir)) return
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const file = path.join(dir, entry.name)
    if (entry.isDirectory()) walk(file)
    else if (entry.name === 'manifest.ts') manifests.push(path.relative(root, file))
  }
}
roots.forEach(walk)
const ids = []
for (const file of manifests) {
  const text = fs.readFileSync(path.join(root, file), 'utf8')
  const match = text.match(/\bid:\s*["']([^"']+)["']/)
  if (match) ids.push({ id: match[1], file })
}
const duplicates = ids.filter((entry, index) => ids.findIndex((candidate) => candidate.id === entry.id) !== index)
if (duplicates.length) { console.error(`CAPABILITY_DISCOVERY=FAIL duplicate=${duplicates.map((x) => x.id).join(',')}`); process.exit(1) }
if (!ids.length) { console.error('CAPABILITY_DISCOVERY=FAIL no package-local manifests'); process.exit(1) }
console.log(`CAPABILITY_DISCOVERY=PASS manifests=${ids.length}`)
