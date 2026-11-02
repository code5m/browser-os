#!/usr/bin/env node
// Independent module-level manifest/Owner/Public API contract smoke test.
// This intentionally does not claim to run each module's native resource tests.
import assert from 'node:assert/strict'
import { readFileSync, existsSync, readdirSync } from 'node:fs'
import { resolve } from 'node:path'
import { build } from 'esbuild'
const root = resolve(import.meta.dirname, '..')
const moduleRoot = resolve(root, 'src/capabilities')
const all = readdirSync(moduleRoot, { withFileTypes: true }).filter(e => e.isDirectory() && existsSync(resolve(moduleRoot, e.name, 'manifest.ts'))).map(e => e.name).sort()
const requested = process.argv[2] === '--all' ? all : [process.argv[2]]
if (!requested[0] || requested.some(id => !all.includes(id))) {
  console.error(`Usage: node tests/check-capability-module-contract.mjs <${all.join('|')}|--all>`)
  process.exit(2)
}
for (const id of requested) {
  const entry = resolve(moduleRoot, id, 'manifest.ts')
  const built = await build({ entryPoints: [entry], bundle: true, write: false, format: 'esm', platform: 'neutral', target: 'es2020', logLevel: 'silent' })
  const code = built.outputFiles[0].text
  const exports = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`)
  const manifest = Object.values(exports).find(item => item?.id === id && item?.v1)
  assert.ok(manifest, `No standalone manifest for ${id}`)
  assert.equal(manifest.id, id)
  assert.equal(manifest.v1.id, id)
  assert.ok(manifest.semanticOwner && typeof manifest.semanticOwner === 'string', `${id}: missing owner`)
  assert.equal(manifest.v1.semanticOwner, manifest.semanticOwner, `${id}: duplicate owner drift`)
  assert.ok(Array.isArray(manifest.v1.publicContract) && manifest.v1.publicContract.length > 0, `${id}: no formal public contract`)
  for (const contract of manifest.v1.publicContract) {
    assert.ok(contract.name && contract.locator, `${id}: invalid contract entry`)
    assert.ok(existsSync(resolve(root, contract.locator)), `${id}: stale public locator ${contract.locator}`)
  }
  assert.ok(existsSync(resolve(root, manifest.v1.entrypoint)), `${id}: stale entrypoint`)
  console.log(`MODULE_CONTRACT_PASS=${id}`)
}
console.log(`MODULE_CONTRACT_RESULT=PASS (${requested.length}/${all.length} checked)`)
