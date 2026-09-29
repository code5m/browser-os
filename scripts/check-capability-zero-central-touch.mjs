import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const demo = path.join(root, 'packages/capability-demo')
const files = []
function walk(dir) { for (const entry of fs.readdirSync(dir, { withFileTypes: true })) { const file = path.join(dir, entry.name); if (entry.isDirectory()) walk(file); else files.push(file) } }
walk(demo)
const forbidden = files.filter((file) => /\.(ts|vue)$/.test(file) && /src\/App|src\/capability\/index|MainArea|SettingsPanel/.test(fs.readFileSync(file, 'utf8')))
if (forbidden.length) { console.error(`NEW_CAPABILITY_ZERO_CENTRAL_TOUCH=FAIL ${forbidden.join(',')}`); process.exit(1) }
const manifest = fs.readFileSync(path.join(demo, 'src/manifest.ts'), 'utf8')
for (const token of ['runtimeApiVersion', 'contributions', 'activationPolicy']) if (!manifest.includes(token)) { console.error(`NEW_CAPABILITY_ZERO_CENTRAL_TOUCH=FAIL missing=${token}`); process.exit(1) }
console.log('NEW_CAPABILITY_ZERO_CENTRAL_TOUCH=PASS')
