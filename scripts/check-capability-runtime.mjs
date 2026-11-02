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
      'activationDurationMs,category,enabled,governanceStatus,id,lastError,name,resident,resourceClass,state,status'
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

  t('RT-17', '同步 activate 遇异步 hook 不得提前标记 ACTIVE', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onActivate: () => Promise.resolve() } }))
    rt.resolve('bookmark')
    const blocked = expectThrow(() => rt.activate('bookmark'), 'INVALID_TRANSITION')
    return blocked && rt.get('bookmark')?.state !== 'ACTIVE'
  })

  t('RT-18', '同步 suspend 遇异步 hook 不得提前标记 SUSPENDED', () => {
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onSuspend: () => Promise.resolve() } }))
    rt.resolve('bookmark')
    rt.activate('bookmark')
    const blocked = expectThrow(() => rt.suspend('bookmark'), 'INVALID_TRANSITION')
    return blocked && rt.get('bookmark')?.state === 'ACTIVE'
  })

  const ta = async (id, name, fn) => {
    try {
      const ok = await fn()
      results.push({ id, name, ok: ok === true, detail: ok === true ? '' : String(ok) })
    } catch (error) { results.push({ id, name, ok: false, detail: String(error) }) }
  }
  await ta('RT-19', '并发 activateAsync 幂等、取消后先回滚再保留 READY', async () => {
    let finish, started = 0, cleanups = 0
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle,
      onActivate: () => { started++; return new Promise(resolve => { finish = resolve }) },
      onDeactivate: async () => { cleanups++ },
    } }))
    rt.resolve('bookmark')
    const first = rt.activateAsync('bookmark')
    const duplicate = rt.activateAsync('bookmark')
    if (first !== duplicate || started !== 1 || rt.get('bookmark').state !== 'READY') return false
    if (!expectThrow(() => rt.disable('bookmark'), 'TRANSITION_IN_PROGRESS')) return false
    if (!rt.cancelTransition('bookmark')) return false
    finish()
    const settled = await Promise.allSettled([first, duplicate])
    return settled.every(value => value.status === 'rejected' && value.reason?.code === 'TRANSITION_CANCELLED')
      && cleanups === 1 && rt.get('bookmark').state === 'READY'
  })
  await ta('RT-20', '激活失败执行 Owner 回滚、原状态不变', async () => {
    let released = 0
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle,
      onActivate: async () => { throw new Error('startup failed') },
      onDeactivate: async () => { released++ },
    } }))
    rt.resolve('bookmark')
    const result = await Promise.allSettled([rt.activateAsync('bookmark')])
    return result[0].status === 'rejected' && released === 1 && rt.get('bookmark').state === 'READY'
      && rt.get('bookmark').lastError.includes('startup failed')
  })
  await ta('RT-21', '并发 suspendAsync 幂等，执行期间不能 disable', async () => {
    let finish, suspensions = 0
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onSuspend: () => {
      suspensions++; return new Promise(resolve => { finish = resolve })
    } } }))
    rt.resolve('bookmark'); rt.activate('bookmark')
    const one = rt.suspendAsync('bookmark'), two = rt.suspendAsync('bookmark')
    if (one !== two || suspensions !== 1 || !expectThrow(() => rt.disable('bookmark'), 'TRANSITION_IN_PROGRESS')) return false
    finish(); await one
    return rt.get('bookmark').state === 'SUSPENDED' && suspensions === 1
  })
  await ta('RT-22', '销毁等待 suspend 与 Owner release 完成后才能 disable', async () => {
    let finish, announceStart, destroyed = 0
    const started = new Promise(resolve => { announceStart = resolve })
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onDestroy: async () => {
      destroyed++; announceStart(); await new Promise(resolve => { finish = resolve })
    } } }))
    rt.resolve('bookmark'); rt.activate('bookmark')
    const ending = rt.releaseAsync('bookmark')
    await started
    if (destroyed !== 1 || !rt.get('bookmark').enabled) { finish(); await ending; return false }
    finish(); const receipt = await ending
    return receipt.ownerCleanup === true && rt.get('bookmark').state === 'SUSPENDED' && !rt.get('bookmark').enabled
  })
  await ta('RT-23', 'Owner release 报错保持 enabled 且记录错误', async () => {
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onDestroy: async () => { throw Error('owner failed') } } }))
    rt.resolve('bookmark'); rt.activate('bookmark')
    const result = await Promise.allSettled([rt.releaseAsync('bookmark')])
    return result[0].status === 'rejected' && rt.get('bookmark').enabled
      && rt.get('bookmark').lastError.includes('owner failed')
  })
  await ta('RT-24', '已知 async hook 的同步入口在执行前拒绝', async () => {
    let invoked = false
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onActivate: async () => { invoked = true } } }))
    rt.resolve('bookmark')
    return expectThrow(() => rt.activate('bookmark'), 'INVALID_TRANSITION') && !invoked
  })
  await ta('RT-25', '激活未决时 reset 应拒绝并保留注册', async () => {
    let finish
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onActivate: () => new Promise(resolve => { finish = resolve }) } }))
    rt.resolve('bookmark'); const activation = rt.activateAsync('bookmark')
    const rejected = expectThrow(() => rt.reset(), 'TRANSITION_IN_PROGRESS')
    rt.cancelTransition('bookmark'); finish(); await Promise.allSettled([activation])
    return rejected && !!rt.get('bookmark')
  })

  await ta('RT-26', '并发 releaseAsync 应共享同一 Owner 释放事务', async () => {
    let finish, announceStart, calls = 0
    const started = new Promise(resolve => { announceStart = resolve })
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle, onDestroy: async () => {
      calls++; announceStart(); await new Promise(resolve => { finish = resolve })
    } } }))
    rt.resolve('bookmark'); rt.activate('bookmark')
    const first = rt.releaseAsync('bookmark'), duplicate = rt.releaseAsync('bookmark')
    await started
    const blocked = expectThrow(() => rt.activate('bookmark'), 'TRANSITION_IN_PROGRESS')
    const disabled = expectThrow(() => rt.disable('bookmark'), 'TRANSITION_IN_PROGRESS')
    finish(); const [one,two] = await Promise.all([first,duplicate])
    return first === duplicate && calls === 1 && blocked && disabled && one.ownerCleanup && two.ownerCleanup
      && !rt.get('bookmark').enabled
  })
  await ta('RT-27', '正在激活的资源遭遇关闭应取消并等待 Owner 清理', async () => {
    let finish, deactivated = 0, destroyed = 0
    const rt = createCapabilityRuntime()
    rt.register(cap({ lifecycle: { ...cap().lifecycle,
      onActivate: () => new Promise(resolve => { finish = resolve }),
      onDeactivate: async () => { deactivated++ },
      onDestroy: async () => { destroyed++ },
    } }))
    rt.resolve('bookmark')
    const activate = rt.activateAsync('bookmark')
    const close = rt.releaseAsync('bookmark')
    finish()
    const result = await Promise.allSettled([activate])
    const receipt = await close
    return result[0].status === 'rejected' && deactivated === 1 && destroyed === 1
      && receipt.ownerCleanup && rt.get('bookmark').state === 'READY' && !rt.get('bookmark').enabled
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
