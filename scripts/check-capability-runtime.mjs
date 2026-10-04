#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-runtime.mjs — Capability Runtime 行为门禁（Phase 7C）
//
// 用 esbuild（vite 自带）转译真实 src/capability/runtime.ts 后加载测试，
// 不是复制一份逻辑来测 —— 保证测的是产品代码本身。
//
// 断言：
//   RT-01 register 成功 → state=DEFINED
//   RT-02 重复 register 抛 DUPLICATE_ID
//   RT-03 resolve 依赖齐全 → READY
//   RT-04 resolve 强依赖缺失 → MISSING_DEPENDENCY
//   RT-05 activate 成功 → ACTIVE（需 COMPATIBILITY_WRAPPED + activatable）
//   RT-06 activate 未声明可激活 → NOT_ACTIVATABLE
//   RT-07 activate 未 resolve → INVALID_TRANSITION
//   RT-08 suspend 成功 → SUSPENDED
//   RT-09 suspend 未声明支持 → SUSPEND_NOT_SUPPORTED
//   RT-10 disable 常驻/未治理能力 → UNSAFE_OPERATION
//   RT-11 disable ACTIVE 能力 → INVALID_TRANSITION（须先 suspend）
//   RT-12 inspect 只返回编排元数据，不含业务状态
//   RT-13 Runtime 不持有业务状态（无 store/业务字段）
//   RT-14 unregistered 操作 → NOT_FOUND
//   RT-16 suspend → disable → enable → activate 闭环保持编排真源一致
//
// 用法：
//   node scripts/check-capability-runtime.mjs [--self-test] [--json] [--help]
// ---------------------------------------------------------------------------

import { readFileSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, join } from 'node:path'
import { transformSync } from 'esbuild'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const RUNTIME_TS = join(ROOT, 'src/capability/runtime.ts')

async function loadRuntime() {
  const src = readFileSync(RUNTIME_TS, 'utf8')
  const { code } = transformSync(src, { loader: 'ts', format: 'esm', target: 'es2020' })
  const tmp = join(ROOT, '.tmp-capability-runtime.mjs')
  const { writeFileSync, unlinkSync } = await import('node:fs')
  writeFileSync(tmp, code, 'utf8')
  try {
    return await import(pathToFileURL(tmp).href)
  } finally {
    try {
      unlinkSync(tmp)
    } catch {
      /* ignore */
    }
  }
}

// ── 测试夹具 ──────────────────────────────────────────────────────────────

function cap(over = {}) {
  return {
    id: 'bookmark',
    name: 'Bookmark',
    category: 'CAPABILITY',
    provides: ['bookmark.list'],
    dependsOn: [],
    optionalDependencies: [],
    lifecycle: {
      supported: ['ACTIVE', 'SUSPENDED'],
      default: 'ACTIVE',
      activatable: true,
      resident: false,
    },
    resources: { class: ['LIGHT'], suspendable: true, destroyable: true },
    permissions: [],
    persistence: { scope: 'disk', sensitive: false },
    entrypoint: 'src/stores/useBookmarkStore.ts',
    semanticOwner: 'useBookmarkStore',
    governanceStatus: 'GOVERNED',
    status: 'COMPATIBILITY_WRAPPED',
    ...over,
  }
}

function expectThrow(fn, code) {
  try {
    fn()
  } catch (e) {
    return e && e.code === code
  }
  return false
}

async function runTests() {
  const { createCapabilityRuntime } = await loadRuntime()
  const results = []
  const t = (id, name, fn) => {
    try {
      const ok = fn()
      results.push({ id, name, ok: ok === true, detail: ok === true ? '' : String(ok) })
    } catch (e) {
      results.push({ id, name, ok: false, detail: e && e.message ? e.message : String(e) })
    }
  }

  t('RT-01', 'register 成功 → DEFINED', () => {
    const rt = createCapabilityRuntime()
    const r = rt.register(cap())
    return r.state === 'DEFINED' && r.enabled === true
  })

  t('RT-02', '重复 register → DUPLICATE_ID', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    return expectThrow(() => rt.register(cap()), 'DUPLICATE_ID')
  })

  t('RT-03', 'resolve 依赖齐全 → READY', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap({ id: 'bridge_like' }))
    rt.register(cap({ id: 'bookmark', dependsOn: ['bridge_like'] }))
    const r = rt.resolve('bookmark')
    return r.state === 'READY'
  })

  t('RT-04', 'resolve 强依赖缺失 → MISSING_DEPENDENCY', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap({ dependsOn: ['nope'] }))
    return expectThrow(() => rt.resolve('bookmark'), 'MISSING_DEPENDENCY')
  })

  t('RT-05', 'activate 成功 → ACTIVE', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    rt.resolve('bookmark')
    const r = rt.activate('bookmark')
    return r.state === 'ACTIVE'
  })

  t('RT-06', 'activate 未声明可激活 → NOT_ACTIVATABLE', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap({ status: 'NOT_INTEGRATED' }))
    rt.resolve('bookmark')
    return expectThrow(() => rt.activate('bookmark'), 'NOT_ACTIVATABLE')
  })

  t('RT-07', 'activate 未 resolve → INVALID_TRANSITION', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    return expectThrow(() => rt.activate('bookmark'), 'INVALID_TRANSITION')
  })

  t('RT-08', 'suspend 成功 → SUSPENDED', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    rt.resolve('bookmark')
    rt.activate('bookmark')
    return rt.suspend('bookmark').state === 'SUSPENDED'
  })

  t('RT-09', 'suspend 未声明支持 → SUSPEND_NOT_SUPPORTED', () => {
    const rt = createCapabilityRuntime()
    rt.register(
      cap({
        lifecycle: { supported: ['ACTIVE'], default: 'ACTIVE', activatable: true, resident: false },
        resources: { class: ['HEAVY'], suspendable: false, destroyable: false },
      }),
    )
    rt.resolve('bookmark')
    rt.activate('bookmark')
    return expectThrow(() => rt.suspend('bookmark'), 'SUSPEND_NOT_SUPPORTED')
  })

  t('RT-10', 'disable 常驻能力 → UNSAFE_OPERATION', () => {
    const rt = createCapabilityRuntime()
    rt.register(
      cap({
        lifecycle: { supported: ['ACTIVE'], default: 'ACTIVE', activatable: true, resident: true },
      }),
    )
    return expectThrow(() => rt.disable('bookmark'), 'UNSAFE_OPERATION')
  })

  t('RT-11', 'disable ACTIVE 能力 → INVALID_TRANSITION（须先 suspend）', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    rt.resolve('bookmark')
    rt.activate('bookmark')
    return expectThrow(() => rt.disable('bookmark'), 'INVALID_TRANSITION')
  })

  t('RT-12', 'inspect 只返回编排元数据', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    const list = rt.inspect()
    const keys = Object.keys(list[0]).sort().join(',')
    return (
      keys ===
      'category,enabled,governanceStatus,id,name,resident,resourceClass,state,status'
    )
  })

  t('RT-13', 'Runtime 源码不持有业务状态', () => {
    const src = readFileSync(RUNTIME_TS, 'utf8')
    const banned = ['useBrowserStore', 'useSystemStore', 'termPanes', 'gridSession', 'items.value']
    const hit = banned.filter((b) => src.includes(b))
    return hit.length === 0 || `发现业务引用: ${hit.join(',')}`
  })

  t('RT-14', '未注册能力操作 → NOT_FOUND', () => {
    const rt = createCapabilityRuntime()
    return expectThrow(() => rt.resolve('ghost'), 'NOT_FOUND')
  })

  t('RT-15', 'SUSPENDED 后可再次 activate', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    rt.resolve('bookmark')
    rt.activate('bookmark')
    rt.suspend('bookmark')
    return rt.activate('bookmark').state === 'ACTIVE'
  })

  t('RT-16', 'suspend → disable → enable → activate 生命周期闭环', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap())
    rt.resolve('bookmark')
    rt.activate('bookmark')
    rt.suspend('bookmark')
    rt.disable('bookmark')
    if (rt.get('bookmark')?.enabled !== false) return false
    rt.enable('bookmark')
    return rt.activate('bookmark').state === 'ACTIVE' && rt.get('bookmark')?.enabled === true
  })

  return results
}

const HELP = `check-capability-runtime.mjs — Capability Runtime 行为门禁

用法:
  node scripts/check-capability-runtime.mjs              运行行为断言（RT-01..RT-15）
  node scripts/check-capability-runtime.mjs --self-test  同上（自检模式）
  node scripts/check-capability-runtime.mjs --json       机器可读
  node scripts/check-capability-runtime.mjs --help

退出码: 0 = 全部通过, 1 = 有失败
`

async function main() {
  const argv = process.argv.slice(2)
  if (argv.includes('--help') || argv.includes('-h')) {
    console.log(HELP)
    return 0
  }
  const results = await runTests()
  const json = argv.includes('--json')
  const pass = results.filter((r) => r.ok).length
  const fail = results.length - pass

  if (json) {
    console.log(JSON.stringify({ result: fail === 0 ? 'PASS' : 'FAIL', pass, fail, results }, null, 2))
  } else {
    for (const r of results) {
      console.log(`${r.ok ? 'PASS' : 'FAIL'}  ${r.id}  ${r.name}${r.ok ? '' : `  → ${r.detail}`}`)
    }
    console.log(`\nCAPABILITY_RUNTIME_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${results.length})`)
  }
  return fail === 0 ? 0 : 1
}

main().then((c) => process.exit(c))
