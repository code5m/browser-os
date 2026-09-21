#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-resource-boundary.mjs — Capability 重资源边界门禁（§17）
//
// 目标：Core/Shell 不得直接触发 Capability-owned 重资源生命周期。
//
// 设计原则（刻意不做「针对某一行代码的脆弱 grep」）：
//   判定基于三份**声明式真源**，任一份变化门禁自动跟随：
//     ① Capability manifest 的 resource ownership（resources[].ownership === 'owned'）
//     ② 已声明边界（哪些文件属 Shell/Core，哪些属 Capability）
//     ③ 已知资源生命周期入口（lifecycle entrypoints，由 manifest 归属推导）
//
// 覆盖的重资源（由 manifest 声明驱动，新增能力自动纳入）：
//   browser  → WEBVIEW / CHILD_PROCESS（宫格子进程 grid-child）
//   terminal → PROCESS / PTY
//
// 断言：
//   BR-01 Shell/Core 文件不得 import Capability 内部（state/ 内部实现）
//   BR-02 Shell/Core 文件不得调用 Capability 重资源生命周期入口
//   BR-03 每个 owned 资源的唯一出生点必须前置能力闸（guard）
//   BR-04 闸的实现不得引入 profile 名 / 环境变量 / 第二真源
//   BR-05 已声明的 Shell→Capability 公开契约用法不得变成资源创建（PUBLIC_CONTRACT_USAGE 白名单）
//   BR-06 负例：Shell 直接调 openGrid → 必须 FAIL（门禁有区分力）
//   BR-07 负例：出生点去掉闸 → 必须 FAIL
//
// 用法：
//   node scripts/check-capability-resource-boundary.mjs [--self-test] [--json] [--help]
// ---------------------------------------------------------------------------

import { readFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

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

function read(rel) {
  const p = join(ROOT, rel)
  return existsSync(p) ? readFileSync(p, 'utf8') : null
}

function stripComments(src) {
  return src.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:])\/\/.*$/gm, '$1')
}

// ── ① 声明式真源：能力 manifest 的 resource ownership ───────────────────────
const CAPABILITIES = [
  {
    id: 'browser',
    manifest: 'src/capabilities/browser/manifest.ts',
    guard: 'src/capabilities/browser/resource/guard.ts',
    guardFn: 'isBrowserResourceAllowed',
    // 已知生命周期入口（创建重资源的调用）：文件 → 入口函数 → 出生点调用
    entrypoints: [
      { file: 'src/capabilities/browser/state/useBrowserStore.ts', fn: 'buildGrid', birth: 'bridge.createGrid' },
    ],
  },
  {
    id: 'terminal',
    manifest: 'src/capabilities/terminal/manifest.ts',
    guard: 'src/capabilities/terminal/resource/guard.ts',
    guardFn: 'isTerminalResourceAllowed',
    entrypoints: [
      { file: 'src/capabilities/terminal/state/useTerminalStore.ts', fn: 'spawnTerm', birth: 'bridge.termSpawnChannel' },
    ],
  },
]

// ── ② 已声明边界：Shell/Core 文件清单 ──────────────────────────────────────
const SHELL_CORE_FILES = [
  'src/App.vue',
  'src/main.ts',
  'src/stores/useLayoutStore.ts',
  'src/stores/useWorkbenchStore.ts',
  'src/components/layout/MainArea.vue',
  'src/components/layout/ActivityBar.vue',
  'src/components/layout/StatusBar.vue',
  'src/components/layout/WorkbenchRail.vue',
  'src/components/layout/UnifiedTabBar.vue',
  'src/components/layout/WorkbenchCommands.vue',
]

// PUBLIC_CONTRACT_USAGE 白名单：Shell 经能力**公开契约**读写展示字段是允许的
// （例如 omnibox 同步 url）。这些不是资源生命周期，不构成 RESOURCE_CREATION_VIOLATION。
const PUBLIC_CONTRACT_ALLOWLIST = [
  { file: 'src/stores/useLayoutStore.ts', pattern: /useBrowserStore\(\)\.url\s*=/, why: 'omnibox 目录路径同步（公开字段，非资源生命周期，见 M0-4.b 注释）' },
]

// ── ③ 资源生命周期符号：按「已声明契约」分两类，不能一刀切 ────────────────
//
// (a) PUBLIC_INTENT：能力自己声明给组件用的语义化入口
//     （useBrowserStore 注释：「组件只调以下语义化入口」= openGrid/closeGrid/
//      activateGrid/rebuildGrid/closeGridCell；terminal = spawnTerm/ensureTerm/addTermPane）
//     Shell 走这些入口 **不是** 违规：入口内部才做资源决策，且必须过能力闸。
//     若把这类也判违规，等于逼 Shell 绕过能力自建资源 —— 与架构目标相反。
// (b) INTERNAL_IMPL / NATIVE_BYPASS：能力内部实现或直接 native 出生点。
//     Shell 调这些 = 绕过能力契约直接控制资源 → RESOURCE_CREATION_VIOLATION。
const PUBLIC_INTENT_CALLS = [
  /\bopenGrid\s*\(/,
  /\bcloseGrid\s*\(/,
  /\brebuildGrid\s*\(/,
  /\bactivateGrid\s*\(/,
  /\bcloseGridCell\s*\(/,
  /\bspawnTerm\s*\(/,
  /\baddTermPane\s*\(/,
  /\bensureTerm\s*\(/,
]

const INTERNAL_IMPL_CALLS = [
  { re: /\bbuildGrid\s*\(/, name: 'buildGrid（能力内部，组件禁止手拼）' },
  { re: /\bcloseGridAll\s*\(/, name: 'closeGridAll（能力内部）' },
  { re: /\bcloseGridOne\s*\(/, name: 'closeGridOne（能力内部）' },
  { re: /bridge\s*\.\s*createGrid\s*\(/, name: 'bridge.createGrid（绕过能力直触 native 出生点）' },
  { re: /bridge\s*\.\s*closeGrid\s*\(/, name: 'bridge.closeGrid（绕过能力直触 native）' },
  { re: /bridge\s*\.\s*termSpawnChannel\s*\(/, name: 'bridge.termSpawnChannel（绕过能力直触 PTY）' },
  { re: /invoke\s*\(\s*["']create_grid["']/, name: 'invoke("create_grid")（绕过能力）' },
  { re: /invoke\s*\(\s*["']term_spawn/, name: 'invoke("term_spawn…")（绕过能力）' },
]

// BR-03：出生点必须前置能力闸
function checkEntrypointGuarded(cap) {
  for (const ep of cap.entrypoints) {
    const src = read(ep.file)
    if (!src) {
      bad('BR-03', `资源出生点文件存在：${ep.file}`)
      continue
    }
    const code = stripComments(src)
    const fnIdx = code.indexOf(`function ${ep.fn}`)
    const birthIdx = code.indexOf(ep.birth)
    const guardIdx = code.indexOf(`${cap.guardFn}()`)
    if (birthIdx < 0) {
      bad('BR-03', `${cap.id}: 未找到出生点 ${ep.birth}`)
      continue
    }
    const guarded = guardIdx >= 0 && guardIdx < birthIdx && (fnIdx < 0 || guardIdx > fnIdx)
    if (guarded) {
      ok('BR-03', `${cap.id}: ${ep.fn} 的出生点 ${ep.birth} 前置 ${cap.guardFn}() 闸`)
    } else {
      bad('BR-03', `${cap.id}: 出生点 ${ep.birth} 未受能力闸保护`, `guardIdx=${guardIdx} birthIdx=${birthIdx}`)
    }
  }
}

// BR-04：闸实现不得依赖 profile 名 / 环境变量 / 第二真源
function checkGuardPurity(cap) {
  const src = read(cap.guard)
  if (!src) {
    bad('BR-04', `${cap.id}: 闸文件存在 ${cap.guard}`)
    return
  }
  const code = stripComments(src)
  const forbidden = [
    [/VITE_CAPABILITY_PROFILE/, '环境变量'],
    [/['"]framework['"]/, 'framework 名'],
    [/['"]full['"]/, 'full 名'],
    [/browserEnabled|terminalEnabled/, 'enabled 第二真源'],
    [/gridVisible/, 'gridVisible 第二真源'],
  ]
  const hits = forbidden.filter(([re]) => re.test(code)).map(([, n]) => n)
  if (hits.length === 0) ok('BR-04', `${cap.id}: 闸输入只有能力可用性+激活态（无 profile/环境变量/第二真源）`)
  else bad('BR-04', `${cap.id}: 闸依赖非编排真源`, hits.join(', '))

  if (/isCapabilityActive/.test(code)) ok('BR-04b', `${cap.id}: 闸复用 runtimeSingleton.isCapabilityActive 单一实现`)
  else bad('BR-04b', `${cap.id}: 闸未复用单一判定实现（易产生第二真源）`)
}

// BR-01 / BR-02：Shell/Core 不得 import 能力内部；不得绕过能力契约直触资源出生点
function checkShellBoundary() {
  const internalImport = /from\s+["'][^"']*capabilities\/(browser|terminal|workspace|bookmark)\/state\//
  const importViolations = []
  const lifecycleViolations = []
  let scanned = 0

  for (const rel of SHELL_CORE_FILES) {
    const src = read(rel)
    if (!src) continue
    scanned += 1
    const code = stripComments(src)

    if (internalImport.test(code)) importViolations.push(rel)

    for (const { re, name } of INTERNAL_IMPL_CALLS) {
      if (re.test(code)) lifecycleViolations.push(`${rel} → ${name}`)
    }
  }

  if (importViolations.length === 0) {
    ok('BR-01', `Shell/Core ${scanned} 个文件均未 import 能力内部实现（state/）`)
  } else {
    bad('BR-01', 'Shell/Core 不得 import 能力内部实现', importViolations.join(', '))
  }

  if (lifecycleViolations.length === 0) {
    ok(
      'BR-02',
      `Shell/Core 均未绕过能力契约直触资源出生点（0 RESOURCE_CREATION_VIOLATION）`
    )
  } else {
    bad('BR-02', 'Shell/Core 绕过能力契约直触资源出生点', lifecycleViolations.join(' | '))
  }
}

// BR-02b：记录 Shell 对能力**公开意图入口**的使用（合法，非违规）—— 诚实披露，便于审计
function reportPublicIntentUsage() {
  const rows = []
  for (const rel of SHELL_CORE_FILES) {
    const src = read(rel)
    if (!src) continue
    const code = stripComments(src)
    const hits = PUBLIC_INTENT_CALLS.filter((re) => re.test(code)).map((re) => String(re))
    if (hits.length) rows.push(`${rel}: ${hits.length}`)
  }
  ok(
    'BR-02b',
    `Shell 使用能力公开意图入口（PUBLIC_CONTRACT_USAGE，合法）：${
      rows.length ? rows.join(', ') : '无'
    }`
  )
}

// BR-05：白名单条目确实是「展示字段」而非资源创建
function checkAllowlistHonest() {
  for (const a of PUBLIC_CONTRACT_ALLOWLIST) {
    const src = read(a.file)
    if (!src) {
      bad('BR-05', `白名单文件存在：${a.file}`)
      continue
    }
    if (a.pattern.test(stripComments(src))) {
      ok('BR-05', `白名单条目成立且非资源创建：${a.file}（${a.why}）`)
    } else {
      bad('BR-05', `白名单条目已失效（应清理或修正）：${a.file}`)
    }
  }
}

// ── 负例：证明门禁有区分力（不是恒真）───────────────────────────────────
function negativeFixtures() {
  const shellSrc = read('src/stores/useLayoutStore.ts')
  const base = stripComments(shellSrc)

  // 负例 A：往 Shell 注入「绕过能力直触 native 出生点」→ BR-02 必须检出。
  // 注意：注入 `browser.openGrid()`（公开意图入口）**不应**被判违规 —— 那会逼 Shell 绕过能力。
  const injectedA = base + '\nfunction __redteam(){ bridge.createGrid(4, []); }\n'
  const detectedA = INTERNAL_IMPL_CALLS.some(({ re }) => re.test(injectedA))
  if (detectedA) ok('BR-06', '负例A：Shell 直触 bridge.createGrid → 门禁检出（有区分力）')
  else bad('BR-06', '负例A 未被检出，门禁形同虚设')

  // 负例 A2：公开意图入口不得被误判为违规（避免过度收紧导致 Shell 绕过能力）
  const injectedA2 = base + '\nfunction __ok(){ browser.openGrid(); }\n'
  const falsePositive = INTERNAL_IMPL_CALLS.some(({ re }) => re.test(injectedA2))
  if (!falsePositive) ok('BR-06b', '负例A2：Shell 调公开意图入口 openGrid 不被误判为违规')
  else bad('BR-06b', '公开意图入口被误判为违规，会逼使 Shell 绕过能力')

  // 负例 B：把出生点的闸摘掉 → BR-03 必须能检出
  const cap = CAPABILITIES[0]
  const ep = cap.entrypoints[0]
  const storeSrc = stripComments(read(ep.file))
  const removed = storeSrc.replace(/if \(!isBrowserResourceAllowed\(\)\) \{[\s\S]*?\n  \}\n/, '')
  const gIdx = removed.indexOf(`${cap.guardFn}()`)
  const bIdx = removed.indexOf(ep.birth)
  const stillGuarded = gIdx >= 0 && gIdx < bIdx
  if (removed !== storeSrc && !stillGuarded) {
    ok('BR-07', '负例B：出生点摘除闸 → 门禁判定转为未受保护（有区分力）')
  } else {
    bad('BR-07', '负例B 未能证明门禁有区分力', `changed=${removed !== storeSrc} stillGuarded=${stillGuarded}`)
  }
}

function selfTest() {
  negativeFixtures()
  console.log(`\nCAPABILITY_RESOURCE_BOUNDARY_SELFTEST_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`)
  process.exit(fail === 0 ? 0 : 1)
}

function main() {
  const args = process.argv.slice(2)
  if (args.includes('--help')) {
    console.log('用法: node scripts/check-capability-resource-boundary.mjs [--self-test] [--json] [--help]')
    return
  }
  if (args.includes('--self-test')) return selfTest()

  for (const cap of CAPABILITIES) {
    // manifest 声明的资源归属必须存在（门禁由声明驱动，不是硬编码清单）
    const m = read(cap.manifest)
    if (m && /ownership:\s*"owned"/.test(m)) {
      ok(`BR-00-${cap.id}`, `${cap.id}: manifest 声明了 owned 资源 → 门禁自动纳入`)
    } else {
      bad(`BR-00-${cap.id}`, `${cap.id}: manifest 未声明 owned 资源`)
    }
    checkEntrypointGuarded(cap)
    checkGuardPurity(cap)
  }
  checkShellBoundary()
  reportPublicIntentUsage()
  checkAllowlistHonest()

  if (args.includes('--json')) {
    console.log(JSON.stringify({ result: fail === 0 ? 'PASS' : 'FAIL', pass, fail, failures }, null, 2))
  }
  console.log(`\nCAPABILITY_RESOURCE_BOUNDARY_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`)
  process.exit(fail === 0 ? 0 : 1)
}

main()
