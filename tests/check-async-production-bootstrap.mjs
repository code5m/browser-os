#!/usr/bin/env node
// Imports the *real* production bootstrap bundle; no replacement implementation.
import assert from 'node:assert/strict'
import { build } from 'esbuild'
import { mkdtempSync, rmSync, writeFileSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
const root=join(dirname(fileURLToPath(import.meta.url)),'..')
const work=mkdtempSync(join(root,'.tmp-async-bootstrap-'))
try {
  const entry=`export { bootstrapCapabilityRuntimeAsync } from ${JSON.stringify(join(root,'src/capability/index.ts'))};`
  const built=await build({stdin:{contents:entry,resolveDir:root,loader:'ts'},bundle:true,format:'esm',platform:'neutral',target:'es2020',external:['pinia','vue','@vue/*','*.vue','turndown','dompurify','marked'],write:false,logLevel:'silent'})
  const out=join(work,'main.mjs');writeFileSync(out,built.outputFiles[0].text)
  const source=readFileSync(join(root,'src/main.ts'),'utf8')
  assert.match(source,/bootstrapCapabilityRuntimeAsync\(\)/)
  assert.doesNotMatch(source,/\bbootstrapCapabilityRuntime\(\)/)
  assert.match(source, /const capabilityBootstrap = bootstrapCapabilityRuntimeAsync\(\);/)
  assert.ok(source.indexOf('const capabilityBootstrap') < source.indexOf('mountApp();'), 'runtime gate must be published before Vue reads capability state')
  assert.ok(source.indexOf('mountApp();') < source.indexOf('void capabilityBootstrap'), 'first paint must not await Owner hooks')
  assert.doesNotMatch(source, /bootstrapCapabilityRuntimeAsync\(\)[\s\S]*?\.finally\(\(\) => \{ mountApp\(/)
  console.log('ASYNC_BOOTSTRAP_PRODUCTION_ENTRY=PASS')
  const {bootstrapCapabilityRuntimeAsync}=await import(pathToFileURL(out).href)
  const first=bootstrapCapabilityRuntimeAsync('framework')
  const second=bootstrapCapabilityRuntimeAsync('framework')
  assert.equal(first,second,'concurrent production bootstrap calls must share one promise')
  const result=await first
  assert.equal(result.error,null)
  assert.equal(result.runtime.inspect().length,0)
  assert.equal((await bootstrapCapabilityRuntimeAsync('framework')).runtime,result.runtime)
  console.log('ASYNC_BOOTSTRAP_IDEMPOTENCE=PASS')
  const isolated=await import(`${pathToFileURL(out).href}?profile=minimal`)
  const minimal=await isolated.bootstrapCapabilityRuntimeAsync('minimal')
  assert.equal(minimal.error,null, `minimal bootstrap: ${minimal.error}`)
  assert.equal(minimal.activated,true)
  assert.deepEqual(minimal.runtime.inspect().map(e=>e.id).sort(),['bookmark','workspace'])
  assert.ok(minimal.runtime.inspect().every(e=>e.state === 'ACTIVE'))
  console.log('ASYNC_BOOTSTRAP_MINIMAL=PASS')
  const fullModule=await import(`${pathToFileURL(out).href}?profile=full`)
  const full=await fullModule.bootstrapCapabilityRuntimeAsync('full')
  assert.equal(full.error,null, `full bootstrap: ${full.error}`)
  assert.equal(full.activated,true)
  assert.ok(full.runtime.inspect().length >= 20, `full registration incomplete: ${full.runtime.inspect().length}`)
  console.log(`ASYNC_BOOTSTRAP_FULL=PASS (${full.runtime.inspect().length} modules)`)
} finally {rmSync(work,{force:true,recursive:true})}
