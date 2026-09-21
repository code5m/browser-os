#!/usr/bin/env node
// ---------------------------------------------------------------------------
// runtime-resource-absence.mjs — RUNTIME RESOURCE ABSENCE 验收（H-G blocker）
//
// 与既有门禁的根本区别：
//   既有门禁（check-composition-profiles 等）证明的是 **STRUCTURAL_ABSENCE**
//   （「没有 contribution / 槽为空」）。
//   本脚本证明的是 **RUNTIME_RESOURCE_ABSENCE**：
//   真的把产品代码（capability runtime + 真实 store）跑起来，替换 native 边界
//   （bridge），然后统计 **重资源创建调用次数**。
//
//   结构性 PASS ≠ 运行期 PASS。这条界线不得再被混淆。
//
// 断言（真实执行，非字符串匹配）：
//   RRA-00 framework profile 装配成功（零能力注册）
//   RRA-01 framework：gridToolbarOpen=true + activateGrid + openGrid → createGrid 0 次
//         （createGrid 是 grid-child 进程的唯一出生点 ⇒ grid-child ≡ 0）
//   RRA-02 framework：gridOpen 恒 false（资源不存在，不是「看不见」）
//   RRA-03 framework：ensureTerm → PTY 创建 0 次
//   RRA-10 full profile 装配成功
//   RRA-04 full：偏好驱动 → createGrid 恰好 1 次 + gridOpen=true（原行为保留）
//   RRA-05 full：mainView==="grid" ⇒ gridOpen===true（冻结语义不回归）
//   RRA-06 full：Grid → Browser 视图切换 = HIDE ONLY（不销毁资源）
//   RRA-07 full：显式 closeGrid = DESTROY（close_grid 1 次 + gridOpen=false）
//   RRA-08 full：ensureTerm → PTY 恰好 1 次（不回归）
//   RRA-09 闸的输入不是 profile 名 / 环境变量 / 第二真源（源码级）
//   RRA-11 useLayoutStore 不再直接调用 Browser internal store（源码级）
//
// 用法：
//   node scripts/runtime-resource-absence.mjs [--self-test] [--json] [--help]
// ---------------------------------------------------------------------------

import { readFileSync } from 'node:fs'
import { writeFileSync, unlinkSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, join } from 'node:path'
import { build } from 'esbuild'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')

let pass = 0
let fail = 0
const failures = []

function ok(id, msg) {
  pass += 1
  console.log(`PASS  ${id}  ${msg}`)
}
function bad(id, msg, detail) {
  fail += 1
  failures.push(`${id} ${msg}${detail ? ` — ${detail}` : ''}`)
  console.log(`FAIL  ${id}  ${msg}${detail ? ` — ${detail}` : ''}`)
}

const tick = () => new Promise((r) => setTimeout(r, 10))

/**
 * 负例夹具：把能力资源闸模块替换成「恒放行」，其余全部是真实产品代码。
 * 用途：证明本脚本的断言**具备区分力** —— 闸一失效，framework 场景立刻 FAIL。
 * 不修改 src 任何文件（只在 esbuild 层替换模块解析）。
 */
const ALWAYS_ALLOW_GUARD = {
  name: 'always-allow-guard',
  setup(b) {
    b.onResolve({ filter: /resource[\\/]guard$/ }, (args) => ({
      path: args.path,
      namespace: 'mutant-guard',
    }))
    b.onLoad({ filter: /.*/, namespace: 'mutant-guard' }, () => ({
      contents:
        'export function isBrowserResourceAllowed(){return true}\nexport function isTerminalResourceAllowed(){return true}',
      loader: 'ts',
    }))
  },
}

async function loadBundle(tag, { mutant = false } = {}) {
  // 关键：capability runtime + 各 store + layout store 必须在**同一个 bundle 实例**内导出。
  // 否则 esbuild 会把 runtimeSingleton.ts / useLayoutStore.ts 复制成多份：
  //   ① 能力闸永远读到 null（假 FAIL）；② 断言的 layout 不是 store 实际使用的那个（假 PASS）。
  const p = (rel) => join(ROOT, rel).replace(/\\/g, '/')
  const entry = `
    import { bootstrapCapabilityRuntime } from '${p('src/capability/index.ts')}';
    import { useBrowserStore } from '${p('src/capabilities/browser/state/useBrowserStore.ts')}';
    import { useTerminalStore } from '${p('src/capabilities/terminal/state/useTerminalStore.ts')}';
    import { useLayoutStore } from '${p('src/stores/useLayoutStore.ts')}';
    import { bridge } from '${p('src/bridge.ts')}';
    export { bootstrapCapabilityRuntime, useBrowserStore, useTerminalStore, useLayoutStore, bridge };
  `
  const res = await build({
    stdin: { contents: entry, resolveDir: ROOT, loader: 'ts' },
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    target: 'es2020',
    external: ['pinia', 'vue', '@vue/*', '*.vue'],
    plugins: mutant ? [ALWAYS_ALLOW_GUARD] : [],
    write: false,
  })
  const tmp = join(ROOT, `.tmp-rra-${tag}.mjs`)
  writeFileSync(tmp, res.outputFiles[0].text, 'utf8')
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

function installGlobals() {
  globalThis.window = {
    setTimeout: (fn, ms) => setTimeout(fn, ms),
    clearTimeout: (h) => clearTimeout(h),
    addEventListener: () => {},
    removeEventListener: () => {},
  }
  globalThis.localStorage = {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
  }
}

/**
 * 装配一次真实应用：bootstrap 指定 profile → 安装记录型 bridge → 创建真实 store。
 * 返回的 counter 就是 RUNTIME 证据本身（重资源出生点的调用计数）。
 */
async function boot(tag, profile, opts = {}) {
  installGlobals()
  const mod = await loadBundle(tag, opts)
  const { bootstrapCapabilityRuntime, useBrowserStore, useTerminalStore, useLayoutStore, bridge } =
    mod

  const counter = { createGrid: 0, closeGrid: 0, termSpawn: 0, evalInTab: 0 }

  bridge.debugLog = () => {}
  bridge.createGrid = async (n) => {
    counter.createGrid += 1
    return n
  }
  bridge.closeGrid = async () => {
    counter.closeGrid += 1
  }
  bridge.gridOpen = async () => {}
  bridge.gridPosition = async () => {}
  bridge.gridSetZoom = async () => {}
  bridge.evalInTab = async () => {
    counter.evalInTab += 1
    return ''
  }
  bridge.tabNew = async () => ({ id: 'tab-1' })
  bridge.tabClose = async () => {}
  bridge.createTermChannel = () => ({ __channel: true })
  bridge.termSpawnChannel = async () => {
    counter.termSpawn += 1
    return { id: `pty-${counter.termSpawn}` }
  }
  bridge.termKill = async () => {}
  bridge.termWrite = async () => {}
  bridge.termResize = async () => {}
  bridge.m0Config = async () => null

  const bootRes = bootstrapCapabilityRuntime(profile)

  const piniaMod = await import(
    pathToFileURL(join(ROOT, 'node_modules/pinia/dist/pinia.mjs')).href
  )
  piniaMod.setActivePinia(piniaMod.createPinia())

  return {
    counter,
    bootRes,
    browser: useBrowserStore(),
    terminal: useTerminalStore(),
    layout: useLayoutStore(),
  }
}

async function runFramework() {
  console.log('--- framework profile（Browser/Terminal absent）---')
  const { counter, browser, terminal, layout, bootRes } = await boot('framework', 'framework')

  if (bootRes.profile === 'framework') ok('RRA-00', 'framework profile 装配成功（零能力注册）')
  else bad('RRA-00', 'framework profile 装配', `profile=${bootRes.profile}`)

  // 最严苛组合（CASE B）：Shell 持久化偏好 true + 显式激活宫格 + 显式 openGrid。
  // 这正是人工验收暴露的场景：偏好存在 + 能力缺席 → 绝不能产出资源。
  // 注意：偏好必须写在 **Shell（layout）** 上 —— gridToolbarOpen 的 owner 是 useLayoutStore。
  layout.gridToolbarOpen = true
  await tick()
  await browser.activateGrid()
  await tick()
  await browser.openGrid()
  await tick()

  if (counter.createGrid === 0) {
    ok(
      'RRA-01',
      'Browser absent：偏好true + activateGrid + openGrid → createGrid 0 次（grid-child ≡ 0）'
    )
  } else {
    bad('RRA-01', 'Browser absent 不得创建 Grid 资源', `createGrid=${counter.createGrid}`)
  }

  if (browser.gridOpen === false) {
    ok('RRA-02', 'Browser absent：gridOpen 恒 false（资源不存在，非「看不见」）')
  } else {
    bad('RRA-02', 'Browser absent 时 gridOpen 应为 false', `gridOpen=${browser.gridOpen}`)
  }

  await terminal.ensureTerm()
  await tick()
  if (counter.termSpawn === 0) {
    ok('RRA-03', 'Terminal absent：ensureTerm → PTY 创建 0 次')
  } else {
    bad('RRA-03', 'Terminal absent 不得创建 PTY', `termSpawn=${counter.termSpawn}`)
  }
}

async function runFull() {
  console.log('--- full profile（Browser/Terminal present）---')
  const { counter, browser, terminal, layout, bootRes } = await boot('full', 'full')

  if (bootRes.profile === 'full') ok('RRA-10', 'full profile 装配成功（4 能力 ACTIVE）')
  else bad('RRA-10', 'full profile 装配', `profile=${bootRes.profile}`)

  // CASE D：偏好 true（写在 Shell 的 layout 上，owner 正确）+ 能力在 → 沿用原语义
  layout.gridToolbarOpen = true
  await tick()

  if (counter.createGrid === 1 && browser.gridOpen === true) {
    ok('RRA-04', 'Browser present：偏好驱动 → createGrid 恰好 1 次 + gridOpen=true（原行为保留）')
  } else {
    bad(
      'RRA-04',
      'Browser present 应正常创建 Grid',
      `createGrid=${counter.createGrid} gridOpen=${browser.gridOpen}`
    )
  }

  // 冻结语义：mainView==="grid" ⇒ gridOpen===true
  browser.activateGrid()
  await tick()
  if (browser.gridOpen === true) {
    ok('RRA-05', 'mainView="grid" ⇒ gridOpen=true（冻结语义不回归）')
  } else {
    bad('RRA-05', 'grid 视图下 gridOpen 应为 true', `gridOpen=${browser.gridOpen}`)
  }

  // HIDE 语义：切走视图（Grid → Browser）只隐藏，不销毁
  const closeBeforeHide = counter.closeGrid
  layout.activateBrowser()
  await tick()
  if (counter.closeGrid === closeBeforeHide && browser.gridOpen === true) {
    ok('RRA-06', 'Grid → Browser 视图切换 = HIDE ONLY（close_grid 0 次，gridOpen 仍 true）')
  } else {
    bad(
      'RRA-06',
      '视图切换不得销毁 Grid',
      `closeGrid=${counter.closeGrid}（前 ${closeBeforeHide}） gridOpen=${browser.gridOpen}`
    )
  }

  // DESTROY 语义：只有显式 closeGrid 才销毁
  const closeBeforeDestroy = counter.closeGrid
  await browser.closeGrid()
  await tick()
  if (counter.closeGrid === closeBeforeDestroy + 1 && browser.gridOpen === false) {
    ok('RRA-07', '显式 closeGrid = DESTROY（close_grid 1 次 + gridOpen=false）')
  } else {
    bad(
      'RRA-07',
      '显式 closeGrid 必须销毁资源',
      `closeGrid=${counter.closeGrid} gridOpen=${browser.gridOpen}`
    )
  }

  await terminal.ensureTerm()
  await tick()
  if (counter.termSpawn === 1) {
    ok('RRA-08', 'Terminal present：ensureTerm → PTY 恰好 1 次（不回归）')
  } else {
    bad('RRA-08', 'Terminal present 应正常创建 PTY', `termSpawn=${counter.termSpawn}`)
  }
}

/** 去掉注释后再做源码断言：guard 里「禁止做什么」的说明文字本身含这些词 */
function stripComments(src) {
  return src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1')
}

/** 源码级断言：闸的输入不得是 profile 名 / 环境变量 / 第二真源 */
function runSourceAssertions() {
  const guard = stripComments(
    readFileSync(join(ROOT, 'src/capabilities/browser/resource/guard.ts'), 'utf8')
  )
  const forbidden = [
    [/VITE_CAPABILITY_PROFILE/, '环境变量 profile 判断'],
    [/['"]framework['"]/, '硬编码 framework profile 名'],
    [/['"]full['"]/, '硬编码 full profile 名'],
    [/browserEnabled/, 'browser 布尔第二真源'],
    [/gridVisible/, 'gridVisible 第二真源'],
  ]
  const hits = forbidden.filter(([re]) => re.test(guard)).map(([, name]) => name)
  if (hits.length === 0) {
    ok('RRA-09', 'Browser 资源闸输入只有「能力可用性+激活态」，无 profile 名/环境变量/第二真源')
  } else {
    bad('RRA-09', '资源闸不得依赖非编排真源', hits.join(', '))
  }

  // RRA-11：Shell 不得触发 Browser **资源生命周期**（这才是 RESOURCE_CREATION_VIOLATION）。
  // 注意口径：Shell 仍可能经能力公开契约读取/写入能力侧的展示字段（如地址栏 url），
  // 那是 PUBLIC_CONTRACT_USAGE，不是本条要抓的「直接创建/销毁重资源」。
  const shell = stripComments(readFileSync(join(ROOT, 'src/stores/useLayoutStore.ts'), 'utf8'))
  const lifecycle = [
    [/browser\s*\.\s*openGrid\s*\(/, 'openGrid'],
    [/browser\s*\.\s*closeGrid\s*\(/, 'closeGrid'],
    [/browser\s*\.\s*buildGrid\s*\(/, 'buildGrid'],
    [/browser\s*\.\s*rebuildGrid\s*\(/, 'rebuildGrid'],
    [/browser\s*\.\s*activateGrid\s*\(/, 'activateGrid'],
    [/\bopenGrid\s*\(/, '裸 openGrid'],
    [/\bcloseGrid\s*\(/, '裸 closeGrid'],
  ]
  const lifecycleHits = lifecycle.filter(([re]) => re.test(shell)).map(([, name]) => name)
  if (lifecycleHits.length === 0) {
    ok('RRA-11', 'useLayoutStore 不再触发 Browser 资源生命周期（PROBLEM A/B 已解除）')
  } else {
    bad('RRA-11', 'Shell 仍直接触发 Browser 资源生命周期', lifecycleHits.join(', '))
  }
}

async function selfTest() {
  const src = readFileSync(
    join(ROOT, 'src/capabilities/browser/state/useBrowserStore.ts'),
    'utf8'
  )
  // 正例：闸存在，且位于唯一 createGrid 调用点之前
  const guarded = /isBrowserResourceAllowed\(\)[\s\S]{0,2000}?bridge\.createGrid/.test(src)
  if (guarded) {
    ok('SELF-01', '正例：唯一 createGrid 调用点前存在 isBrowserResourceAllowed() 闸')
  } else {
    bad('SELF-01', 'createGrid 调用点未受能力闸保护')
  }
  const termSrc = readFileSync(
    join(ROOT, 'src/capabilities/terminal/state/useTerminalStore.ts'),
    'utf8'
  )
  const termGuarded =
    /isTerminalResourceAllowed\(\)[\s\S]{0,1200}?bridge\.termSpawnChannel/.test(termSrc)
  if (termGuarded) ok('SELF-02', '正例：PTY 出生点前存在 isTerminalResourceAllowed() 闸')
  else bad('SELF-02', 'PTY 出生点未受能力闸保护')

  // 负例（真运行，非字符串变异）：把闸模块替换为「恒放行」，其余全是真实产品代码。
  // framework profile（Browser absent）下若断言有区分力，必须观测到 createGrid ≥ 1
  // —— 即「泄漏被复现」。若仍是 0，说明断言恒真、门禁形同虚设。
  const mutant = await boot('mutant', 'framework', { mutant: true })
  await mutant.browser.activateGrid()
  await tick()
  const leaked = mutant.counter.createGrid
  if (leaked >= 1) {
    ok(
      'SELF-03',
      `负例：闸恒放行后 framework 场景复现泄漏（createGrid=${leaked}）→ RRA-01 断言具备区分力`
    )
  } else {
    bad('SELF-03', '负例未复现泄漏，RRA-01 可能恒真（门禁无区分力）', `createGrid=${leaked}`)
  }
}

async function main() {
  const args = process.argv.slice(2)
  if (args.includes('--help')) {
    console.log('用法: node scripts/runtime-resource-absence.mjs [--self-test] [--json] [--help]')
    return
  }
  if (args.includes('--self-test')) {
    await selfTest()
    console.log(
      `\nRUNTIME_RESOURCE_SELFTEST_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`
    )
    process.exit(fail === 0 ? 0 : 1)
  }

  await runFramework()
  await runFull()
  runSourceAssertions()

  if (args.includes('--json')) {
    console.log(JSON.stringify({ result: fail === 0 ? 'PASS' : 'FAIL', pass, fail, failures }, null, 2))
  }
  console.log(
    `\nRUNTIME_RESOURCE_ABSENCE_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`
  )
  process.exit(fail === 0 ? 0 : 1)
}

main().catch((e) => {
  console.error('runtime-resource-absence 执行失败:', e)
  process.exit(1)
})
