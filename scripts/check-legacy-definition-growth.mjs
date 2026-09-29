import fs from 'node:fs'
const text = fs.readFileSync('src/capability/platform/legacy-definitions.ts', 'utf8')
const count = (text.match(/Manifest\s*[,\]]/g) ?? []).length
const baseline = 17
if (count > baseline) { console.error(`LEGACY_DEFINITION_COUNT=FAIL before=${baseline} after=${count}`); process.exit(1) }
console.log(`LEGACY_DEFINITION_COUNT_BEFORE=${baseline}`)
console.log(`LEGACY_DEFINITION_COUNT_AFTER=${count}`)
