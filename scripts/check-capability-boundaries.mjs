#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-boundaries.mjs — Capability 物理边界门禁（Phase 8A）
//
//   CB-01 fail  跨 Capability import 内部实现
//   CB-02 fail  Shell import Capability 内部实现
//   CB-03 fail  未声明 dependency 却依赖另一 Capability
//   CB-04 fail  Capability 循环依赖
//   CB-05 fail  直接跨 Capability 取 state / store
//   CB-06 fail  绕过 public entrypoint（深路径 import）
//   CB-07 warn  Capability 非 adapter 文件直接触碰 native side effect
//
// 架构：核心分析函数接受「虚拟文件表」，因此自检可用夹具、
//       真实扫描读磁盘 —— 测的是同一套逻辑。
//
// 用法:
//   node scripts/check-capability-boundaries.mjs [--self-test] [--strict] [--json] [--help]
// ---------------------------------------------------------------------------

import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, posix, resolve as resolvePath } from 'node:path'
import { loadRegistry } from './check-capability-registry.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const CAP_ROOT = 'src/capabilities'
const PUBLIC_FILES = new Set(['index.ts', 'manifest.ts', 'public.ts'])
const PUBLIC_DIRS = new Set(['contracts'])
const INTERNAL_DIRS = new Set(['state', 'services', 'ui', 'lifecycle', 'resource', 'internal'])

/** shell 判定：不属于任何 capability 的应用骨架代码 */
function isShell(rel) {
  return (
    rel === 'src/App.vue' ||
    rel === 'src/main.ts' ||
    rel.startsWith('src/shell/') ||
    rel.startsWith('src/components/layout/')
  )
}

function isShared(rel) {
  return rel.startsWith('src/shared/') || rel.startsWith('src/utils/') || rel.startsWith('src/composables/')
}

/** 归属：返回 capability 名，或 null */
function ownerOf(rel) {
  if (!rel.startsWith(CAP_ROOT + '/')) return null
  const rest = rel.slice(CAP_ROOT.length + 1)
  const parts = rest.split('/')
  return parts.length >= 2 ? parts[0] : null
}

/** 目标相对 helper：判断 import 目标落在哪个区域 */
function classifyTarget(rel) {
  const owner = ownerOf(rel)
  if (!owner) return { owner: null, zone: 'outside' }
  const rest = rel.slice(CAP_ROOT.length + 1 + owner.length + 1)
  const top = rest.split('/')[0]
  if (PUBLIC_FILES.has(rest)) return { owner, zone: 'public' }
  if (PUBLIC_DIRS.has(top)) return { owner, zone: 'public' }
  if (INTERNAL_DIRS.has(top)) return { owner, zone: 'internal', internalTop: top }
  if (rest === 'adapters' || top === 'adapters') return { owner, zone: 'adapter' }
  return { owner, zone: 'internal', internalTop: top || '(root)' }
}

const IMPORT_RE = /(?:^|[\s;{}])(?:import|export)\s+(?:[\s\S]*?\sfrom\s+)?['"]([^'"]+)['"]|import\s*\(\s*['"]([^'"]+)['"]\s*\)/g

function extractImports(content) {
  const out = []
  const stripped = content
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/.*$/gm, '$1')
  IMPORT_RE.lastIndex = 0
  let m
  while ((m = IMPORT_RE.exec(stripped)) !== null) {
    const spec = m[1] || m[2]
    if (spec) out.push(spec)
  }
  return out
}

function normalize(fromFileRel, spec) {
  if (!spec.startsWith('.')) return null // 非相对 = 外部包/别名，不归边界管
  const dir = posix.dirname(fromFileRel)
  return posix.normalize(posix.join(dir, spec))
}

function toRel(candidate) {
  // candidate 形如 src/...，去掉扩展名歧义由调用方处理
  return candidate
}

/**
 * @param files  [{ path: string(相对仓库根，规范化), content: string }]
 * @param deps   Record<capId, string[]>  声明的依赖（来自 registry）
 */
export function analyze(files, deps = {}, optionalDeps = {}) {
  const findings = []
  const push = (code, level, message, where) => findings.push({ code, level, message, where })

  const byPathWithoutExt = new Map()
  for (const f of files) {
    const noExt = f.path.replace(/\.(ts|tsx|js|jsx|vue|mjs)$/, '')
    byPathWithoutExt.set(noExt, f.path)
    byPathWithoutExt.set(posix.join(noExt, 'index'), f.path)
  }

  // 注意：真实代码里 .vue import 会带扩展名，而文件表存的是去扩展名 key，因此要两边都试
  const resolveSpec = (rel) => {
    const candidates = [rel, rel.replace(/\.(ts|tsx|js|jsx|vue|mjs)$/, '')]
    for (const c of candidates) {
      if (byPathWithoutExt.has(c)) return byPathWithoutExt.get(c)
      if (byPathWithoutExt.has(posix.join(c, 'index'))) return byPathWithoutExt.get(posix.join(c, 'index'))
    }
    // 目录级命中（如 import './ui' 指向 ui/index.ts）
    for (const key of byPathWithoutExt.keys()) {
      if (key.startsWith(candidates[1] + '/')) return byPathWithoutExt.get(key)
    }
    return null
  }

  for (const f of files) {
    const fromOwner = ownerOf(f.path)
    const fromIsShell = isShell(f.path)
    for (const spec of extractImports(f.content)) {
      const raw = normalize(f.path, spec)
      if (!raw) continue
      const targetRel =
        raw.startsWith('src/') || raw.startsWith(CAP_ROOT) ? raw : posix.normalize(posix.join('src', raw.replace(/^(\.\.\/)+/, '')))
      const targetPath = resolveSpec(targetRel)
      if (!targetPath) continue
      // CB-07 必须先判定：bridge 不属于任何 capability，
      // 放在 `if (!t.owner) continue` 之后会永远不可达。
      const normTarget = targetPath.replace(/\.(ts|tsx|js|jsx|vue|mjs)$/, '')
      if (fromOwner && normTarget === 'src/bridge') {
        const hasAdapters = files.some((x) => ownerOf(x.path) === fromOwner && x.path.includes('/adapters/'))
        if (hasAdapters && !f.path.includes('/adapters/')) {
          push('CB-07', 'warn', `非 adapter 文件直接触碰 native bridge：${spec}`, f.path)
        }
        continue
      }

      const t = classifyTarget(targetPath)
      if (!t.owner) continue

      const sameCap = fromOwner === t.owner

      if (sameCap) continue // 同 capability 内部自由
      if (isShared(f.path)) continue // shared 不算越界（另有 CB 监控）

      if (t.zone === 'internal') {
        if (fromIsShell) {
          push('CB-02/06', 'fail', `Shell 直接 import Capability 内部实现：${targetPath}`, f.path)
        } else if (fromOwner) {
          // STAGE H-B：optionalDependencies 同样属于「已声明依赖」。
          // CB-03 的意图是禁止**未声明**就依赖另一能力；声明为 optional 已满足该意图。
          // 这是准确性修正（未声明仍 FAIL），不是放宽门禁。
          const declared = new Set([...(deps[fromOwner] || []), ...(optionalDeps[fromOwner] || [])])
          if (!declared.has(t.owner)) {
            push('CB-01', 'fail', `跨 Capability import 内部实现（且未声明依赖）：${t.owner} ← ${targetPath}`, f.path)
            push('CB-03', 'fail', `未声明 dependency 却依赖 ${t.owner}`, f.path)
          } else {
            push('CB-01', 'fail', `跨 Capability import 内部实现：${t.owner} ← ${targetPath}`, f.path)
          }
        }
        if (/state|store/i.test(t.target || t.internalTop || '')) {
          push('CB-05', 'fail', `直接跨 Capability 取 state/store：${targetPath}`, f.path)
        }
        continue
      }

      if (t.zone === 'public') {
        if (fromOwner && !fromIsShell) {
          // STAGE H-B：optionalDependencies 同样属于「已声明依赖」。
          // CB-03 的意图是禁止**未声明**就依赖另一能力；声明为 optional 已满足该意图。
          // 这是准确性修正（未声明仍 FAIL），不是放宽门禁。
          const declared = new Set([...(deps[fromOwner] || []), ...(optionalDeps[fromOwner] || [])])
          if (!declared.has(t.owner)) {
            push('CB-03', 'fail', `未声明 dependency 却依赖 ${t.owner} 的 public entry`, f.path)
          }
        }
      }
    }
  }

  // CB-04 循环依赖
  //   与 Registry C4 契约保持一致：可选依赖不阻断装配，
  //   因此「必须依赖成环」= fail，「仅可选依赖构成的环」= warn（设计异味，登记为债务）。
  const findCycle = (graph) => {
    const color = new Map()
    let cyc = null
    const dfs = (n, path) => {
      color.set(n, 1)
      for (const m of graph(n)) {
        if (color.get(m) === 1) {
          cyc = [...path, n, m].join(' -> ')
          return true
        }
        if (!color.get(m)) {
          if (dfs(m, [...path, n])) return true
        }
      }
      color.set(n, 2)
      return false
    }
    const roots = new Set([...Object.keys(deps), ...Object.keys(optionalDeps)])
    for (const n of roots) {
      if (!color.get(n)) if (dfs(n, [])) break
    }
    return cyc
  }
  const reqCycle = findCycle((n) => deps[n] || [])
  if (reqCycle) push('CB-04', 'fail', `Capability 必须依赖成环（阻断装配）：${reqCycle}`, '<deps>')
  const allCycle = findCycle((n) => [...(deps[n] || []), ...(optionalDeps[n] || [])])
  if (allCycle && !reqCycle) {
    push('CB-04', 'warn', `Capability 存在仅由「可选依赖」构成的环（设计异味，不阻断装配）：${allCycle}`, '<deps>')
  }

  return findings
}

// ── 真实扫描 ──────────────────────────────────────────────────────────────

function walk(dir, root, acc = []) {
  if (!existsSync(dir)) return acc
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) walk(p, root, acc)
    else if (/\.(ts|tsx|js|jsx|vue|mjs)$/.test(name)) acc.push(p)
  }
  return acc
}

function loadReal() {
  const targetDirs = ['src']
  const files = []
  for (const d of targetDirs) {
    for (const p of walk(join(ROOT, d), ROOT)) {
      const rel = posix.normalize(p.slice(ROOT.length + 1).replace(/\\/g, '/'))
      if (rel.startsWith('src/capability/')) continue // 旧 PID 目录，非本轮对象
      try {
        files.push({ path: rel, content: readFileSync(p, 'utf8') })
      } catch {
        /* ignore */
      }
    }
  }
  return files
}

/** 依赖真源 = capability-registry（与其它 checker 共用同一个解析器，避免二次漂移） */
function loadDepsFromRegistry() {
  let reg
  try {
    reg = loadRegistry()
  } catch {
    return { required: {}, optional: {} }
  }
  const required = {}
  const optional = {}
  for (const c of reg.capabilities || []) {
    if (!c || !c.id) continue
    required[c.id] = c.dependsOn || []
    optional[c.id] = c.optionalDependencies || []
  }
  return { required, optional }
}

// ── 自检（positive / negative / false-positive 夹具） ──────────────────────

function fx(path, content) {
  return { path, content }
}

function runSelfTest() {
  const cases = []

  // POSITIVE：同 capability 内部互相 import 合法；跨 cap 走 public 且已声明合法
  cases.push({
    name: 'POSITIVE: 同 capability 内部 import 合法',
    expectCodes: [],
    files: [
      fx('src/capabilities/bookmark/index.ts', "export * from './state/store'"),
      fx('src/capabilities/bookmark/state/store.ts', "export const x = 1"),
    ],
    deps: {},
  })
  cases.push({
    name: 'POSITIVE: 跨 capability 走 public entry 且已声明依赖',
    expectCodes: [],
    files: [
      fx('src/capabilities/git/index.ts', "import { b } from '../notes'"),
      fx('src/capabilities/notes/index.ts', 'export const b = 1'),
    ],
    deps: { git: ['notes'] },
  })

  // NEGATIVE
  cases.push({
    name: 'NEGATIVE CB-01/03: 跨 capability import 内部实现且未声明',
    expectCodes: ['CB-01', 'CB-03'],
    files: [
      fx('src/capabilities/git/services/x.ts', "import { s } from '../../notes/state/store'"),
      fx('src/capabilities/notes/state/store.ts', 'export const s = 1'),
    ],
    deps: {},
  })
  cases.push({
    name: 'NEGATIVE CB-02: Shell import capability 内部实现',
    expectCodes: ['CB-02/06'],
    files: [
      fx('src/components/layout/MainArea.vue', "import { s } from '../../capabilities/notes/state/store'"),
      fx('src/capabilities/notes/state/store.ts', 'export const s = 1'),
    ],
    deps: {},
  })
  cases.push({
    name: 'NEGATIVE CB-02: App.vue import capability 内部 ui',
    expectCodes: ['CB-02/06'],
    files: [
      fx('src/App.vue', "import P from './capabilities/notes/ui/Panel.vue'"),
      fx('src/capabilities/notes/ui/Panel.vue', '<template/>'),
    ],
    deps: {},
  })
  cases.push({
    name: 'NEGATIVE CB-03: 未声明依赖却 import 另一 capability public entry',
    expectCodes: ['CB-03'],
    files: [
      fx('src/capabilities/git/index.ts', "import { b } from '../notes'"),
      fx('src/capabilities/notes/index.ts', 'export const b = 1'),
    ],
    deps: { git: [] },
  })
  cases.push({
    name: 'NEGATIVE CB-04: 循环依赖',
    expectCodes: ['CB-04'],
    files: [],
    deps: { a: ['b'], b: ['a'] },
  })
  cases.push({
    name: 'NEGATIVE CB-05: 跨 capability 直取 state',
    expectCodes: ['CB-01', 'CB-05'],
    files: [
      fx('src/capabilities/git/services/y.ts', "import { st } from '../../notes/state/store'"),
      fx('src/capabilities/notes/state/store.ts', 'export const st = 1'),
    ],
    deps: { git: ['notes'] },
  })
  cases.push({
    name: 'NEGATIVE CB-07: 非 adapter 文件触碰 native bridge',
    expectCodes: ['CB-07'],
    files: [
      fx('src/capabilities/notes/services/io.ts', "import { b } from '../../../bridge'"),
      fx('src/capabilities/notes/adapters/native.ts', "import { b } from '../../../bridge'"),
      fx('src/bridge.ts', 'export const b = 1'),
    ],
    deps: {},
  })

  // FALSE-POSITIVE：合法但"看起来可疑"
  cases.push({
    name: 'FALSE-POSITIVE: external 包名不参与边界判定',
    expectCodes: [],
    files: [fx('src/capabilities/git/services/z.ts', "import { ref } from 'vue'")],
    deps: {},
  })
  cases.push({
    name: 'FALSE-POSITIVE: 引用 contracts 目录合法',
    expectCodes: [],
    files: [
      fx('src/capabilities/git/services/w.ts', "import type { T } from '../../notes/contracts/types'"),
      fx('src/capabilities/notes/contracts/types.ts', 'export type T = {}'),
    ],
    deps: { git: ['notes'] },
  })
  cases.push({
    name: 'FALSE-POSITIVE: shell 引用 public entry 合法',
    expectCodes: [],
    files: [
      fx('src/App.vue', "import { cap } from './capabilities/notes'"),
      fx('src/capabilities/notes/index.ts', 'export const cap = 1'),
    ],
    deps: {},
  })

  let pass = 0
  let fail = 0
  for (const c of cases) {
    const got = new Set(analyze(c.files, c.deps).map((f) => f.code))
    const expected = new Set(c.expectCodes)
    const ok = [...expected].every((e) => got.has(e)) && (expected.size > 0 ? true : got.size === 0)
    if (ok) pass++
    else fail++
    console.log(
      `${ok ? 'PASS' : 'FAIL'}  ${c.name}` + (ok ? '' : `\n      expected=[${[...expected]}] got=[${[...got]}]`),
    )
  }
  console.log(`\nSELF_TEST: ${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`)
  return fail === 0 ? 0 : 1
}

const HELP = `check-capability-boundaries.mjs — Capability 物理边界门禁

用法:
  node scripts/check-capability-boundaries.mjs              真实扫描
  node scripts/check-capability-boundaries.mjs --self-test   自检
  node scripts/check-capability-boundaries.mjs --strict      warn 也判失败
  node scripts/check-capability-boundaries.mjs --json        机器可读
  node scripts/check-capability-boundaries.mjs --help

规则:
  CB-01 跨 Capability import 内部实现      CB-05 跨 Capability 取 state/store
  CB-02 Shell import Capability 内部       CB-06 绕过 public entrypoint
  CB-03 未声明 dependency                  CB-07 非 adapter 触碰 native(warn)
  CB-04 Capability 循环依赖

退出码: 0 = 通过, 1 = 失败
`

function main() {
  const argv = process.argv.slice(2)
  if (argv.includes('--help') || argv.includes('-h')) {
    console.log(HELP)
    return 0
  }
  if (argv.includes('--self-test')) return runSelfTest()

  const strict = argv.includes('--strict')
  const json = argv.includes('--json')
  const { required, optional } = loadDepsFromRegistry()
  const findings = analyze(loadReal(), required, optional)
  const fails = findings.filter((f) => f.level === 'fail')
  const warns = findings.filter((f) => f.level === 'warn')
  const bad = fails.length > 0 || (strict && warns.length > 0)

  if (json) {
    console.log(JSON.stringify({ result: bad ? 'FAIL' : 'PASS', fail: fails.length, warn: warns.length, findings }, null, 2))
  } else {
    for (const f of findings.slice(0, 60)) {
      console.log(`[${f.level.toUpperCase()}] ${f.code}  ${f.where}: ${f.message}`)
    }
    if (findings.length > 60) console.log(`...（共 ${findings.length} 条，已截断显示）`)
    console.log(`\nCAPABILITY_BOUNDARIES_RESULT=${bad ? 'FAIL' : 'PASS'} (fail=${fails.length} warn=${warns.length})`)
  }
  return bad ? 1 : 0
}

const isMain = process.argv[1] && resolvePath(process.argv[1]) === fileURLToPath(import.meta.url)
if (isMain) process.exit(main())

export { analyze as analyzeBoundaries }
