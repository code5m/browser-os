import fs from 'node:fs'

const registry = fs.readFileSync('src/capability/platform/host-services.ts', 'utf8')
const bootstrap = fs.readFileSync('src/capability/index.ts', 'utf8')
const required = ['bridge', 'layout', 'workbench', 'graph-layout', 'redact-secrets']
const missing = required.filter((id) => !registry.includes(`'${id}'`) || !bootstrap.includes(`register('${id}'`))
if (missing.length) { console.error(`HOST_SERVICE_REGISTRY=FAIL ${missing.join(',')}`); process.exit(1) }
console.log(`HOST_SERVICE_REGISTRY=PASS services=${required.length}`)
