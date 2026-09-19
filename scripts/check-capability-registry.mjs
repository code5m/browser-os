#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-registry.mjs — Capability Registry 门禁（Phase 7B）
//
// 职责：校验 docs/architecture/capability-registry/*.yaml 的 manifest 契约。
//   C1  CAP_DUPLICATE_ID            重复能力 id                        fail
//   C2  CAP_MISSING_FIELD           缺失必填字段                        fail
//   C3  CAP_UNKNOWN_DEPENDENCY      依赖指向不存在的能力/基础设施        fail
//   C4  CAP_CIRCULAR_DEPENDENCY     强依赖成环                          fail
//   C5  CAP_MISSING_RESOURCE_POLICY 能力缺 resources.yaml policy        fail
//   C6  CAP_UNKNOWN_RESOURCE_CLASS  resource class 未定义               fail
//   C7  CAP_UNGOVERNED_ACTIVATABLE  未登记 Owner 却声明可激活            fail
//   C8  CAP_FORBIDDEN_EDGE          出现禁止的依赖方向                   fail
//   C9  CAP_DEPENDENCY_MISMATCH     capabilities.yaml 与 dependencies.yaml 不一致 warn(--strict fail)
//   C10 CAP_INVALID_LIFECYCLE       lifecycle 声明非法                   fail
//
// 用法：
//   node scripts/check-capability-registry.mjs              # 扫描真实 registry
//   node scripts/check-capability-registry.mjs --self-test  # 自检（positive/negative/false-positive）
//   node scripts/check-capability-registry.mjs --strict     # warn 也判失败
//   node scripts/check-capability-registry.mjs --json       # 机器可读
//   node scripts/check-capability-registry.mjs --help
//
// 依赖：仅 Node 标准库（自带 YAML 子集解析器，避免引入依赖）
// ---------------------------------------------------------------------------

import { readFileSync, existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, resolve } from 'node:path'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const REG_DIR = join(ROOT, 'docs/architecture/capability-registry')

const REQUIRED_FIELDS = [
  'id', 'name', 'category', 'provides', 'dependsOn', 'optionalDependencies',
  'lifecycle', 'resources', 'permissions', 'persistence', 'entrypoint',
]

const VALID_STATES = ['DEFINED', 'READY', 'ACTIVE', 'BACKGROUND', 'SUSPENDED', 'DESTROYED']
const VALID_CATEGORIES = [
  'CAPABILITY', 'SUB_CAPABILITY', 'UI_COMPONENT', 'SERVICE',
  'ADAPTER', 'INFRASTRUCTURE', 'IMPLEMENTATION_DETAIL',
]

// ── YAML 子集解析器 ──────────────────────────────────────────────────────

function tokenize(text) {
  return text
    .split('\n')
    .map((l) => l.replace(/[\s\r]+$/, ''))
    .filter((l) => l.trim() !== '' && !/^\s*#/.test(l))
    .map((l) => {
      const m = l.match(/^(\s*)(.*)$/)
      return { indent: m[1].length, text: m[2] }
    })
}

function parseScalar(raw) {
  const v = String(raw).trim()
  if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) {
    return v.slice(1, -1)
  }
  if (v === 'true') return true
  if (v === 'false') return false
  if (v === 'null' || v === '~' || v === '') return null
  return v
}

function parseInlineArray(raw) {
  const inner = raw.trim().replace(/^\[/, '').replace(/\]$/, '')
  if (inner.trim() === '') return []
  return inner.split(',').map((s) => parseScalar(s))
}

function parseInlineMap(raw) {
  const inner = raw.trim().replace(/^\{/, '').replace(/\}$/, '')
  const obj = {}
  if (inner.trim() === '') return obj
  for (const part of inner.split(',')) {
    const p = part.trim()
    const m = p.match(/^([A-Za-z_][\w-]*)\s*:\s*(.*)$/)
    if (m) obj[m[1]] = parseScalar(m[2])
  }
  return obj
}

function parseValue(raw) {
  const v = String(raw).trim()
  if (v.startsWith('[')) return parseInlineArray(v)
  if (v.startsWith('{')) return parseInlineMap(v)
  return parseScalar(v)
}

function parseList(tokens, i, indent) {
  const arr = []
  while (i < tokens.length && tokens[i].indent === indent && tokens[i].text.startsWith('- ')) {
    const rest = tokens[i].text.slice(2)
    if (/^[A-Za-z_][\w-]*\s*:/.test(rest)) {
      // 列表项是 map：把首行 indent 抬到 indent+2，收集后续更深的行
      const sub = [{ indent: indent + 2, text: rest }]
      let j = i + 1
      while (j < tokens.length && tokens[j].indent > indent) {
        sub.push(tokens[j])
        j++
      }
      const [val] = parseMap(sub, 0, indent + 2)
      arr.push(val)
      i = j
    } else {
      arr.push(parseValue(rest))
      i++
    }
  }
  return [arr, i]
}

function parseMap(tokens, i, indent) {
  const obj = {}
  while (i < tokens.length && tokens[i].indent === indent && !tokens[i].text.startsWith('- ')) {
    const m = tokens[i].text.match(/^([A-Za-z_][\w-]*)\s*:\s*(.*)$/)
    if (!m) break
    const key = m[1]
    const rest = m[2].trim()
    if (rest === '' || rest === '>' || rest === '|') {
      if (rest === '>' || rest === '|') {
        // 折叠/字面块：收集更深行拼成字符串
        const parts = []
        let j = i + 1
        while (j < tokens.length && tokens[j].indent > indent) {
          parts.push(tokens[j].text.trim())
          j++
        }
        obj[key] = parts.join(' ')
        i = j
        continue
      }
      if (i + 1 < tokens.length && tokens[i + 1].indent > indent) {
        const [val, ni] = parse(tokens, i + 1, tokens[i + 1].indent)
        obj[key] = val
        i = ni
      } else {
        obj[key] = null
        i++
      }
    } else {
      obj[key] = parseValue(rest)
      i++
    }
  }
  return [obj, i]
}

function parse(tokens, i, indent) {
  if (tokens[i] && tokens[i].text.startsWith('- ')) return parseList(tokens, i, indent)
  return parseMap(tokens, i, indent)
}

function parseYaml(text) {
  const tokens = tokenize(text)
  const [value] = parse(tokens, 0, tokens.length ? tokens[0].indent : 0)
  return value || {}
}

// ── 校验 ─────────────────────────────────────────────────────────────────

function validate(input) {
  const findings = []
  const push = (code, level, msg, where) =>
    findings.push({ code, level, message: msg, where })

  const caps = Array.isArray(input.capabilities) ? input.capabilities : []
  const infra = Array.isArray(input.sharedInfrastructure) ? input.sharedInfrastructure : []
  const edges = Array.isArray(input.edges) ? input.edges : []
  const forbidden = Array.isArray(input.forbiddenEdges) ? input.forbiddenEdges : []
  const policies = Array.isArray(input.policies) ? input.policies : []
  const resourceClasses = input.resourceClasses || {}
  const lifecycleStates = Array.isArray(input.lifecycleStates) ? input.lifecycleStates : VALID_STATES

  const capIds = new Set(caps.map((c) => c && c.id).filter(Boolean))
  const infraIds = new Set(infra.map((n) => n && n.id).filter(Boolean))
  const knownIds = new Set([...capIds, ...infraIds])
  const policyIds = new Set(policies.map((p) => p && p.id).filter(Boolean))

  // C1 重复 id
  const seen = new Set()
  for (const c of caps) {
    if (!c || !c.id) continue
    if (seen.has(c.id)) push('CAP_DUPLICATE_ID', 'fail', `重复能力 id: ${c.id}`, c.id)
    seen.add(c.id)
  }

  for (const c of caps) {
    if (!c || !c.id) {
      push('CAP_MISSING_FIELD', 'fail', '存在无 id 的能力条目', '<unknown>')
      continue
    }
    // C2 必填字段
    for (const f of REQUIRED_FIELDS) {
      if (c[f] === undefined || c[f] === null) {
        push('CAP_MISSING_FIELD', 'fail', `缺失必填字段: ${f}`, c.id)
      }
    }
    if (c.category && !VALID_CATEGORIES.includes(c.category)) {
      push('CAP_MISSING_FIELD', 'fail', `非法 category: ${c.category}`, c.id)
    }

    // C3 依赖存在性
    for (const dep of [...(c.dependsOn || []), ...(c.optionalDependencies || [])]) {
      if (!knownIds.has(dep)) {
        push('CAP_UNKNOWN_DEPENDENCY', 'fail', `依赖不存在: ${dep}`, c.id)
      }
    }

    // C5 资源策略存在
    if (!policyIds.has(c.id)) {
      push('CAP_MISSING_RESOURCE_POLICY', 'fail', 'resources.yaml 缺少对应 policy', c.id)
    }

    // C6 资源类别合法
    for (const rc of c.resources?.class || []) {
      if (!(rc in resourceClasses)) {
        push('CAP_UNKNOWN_RESOURCE_CLASS', 'fail', `未定义的 resource class: ${rc}`, c.id)
      }
    }

    // C7 未登记 Owner 不得可激活
    const governed = c.governanceStatus === 'GOVERNED' && !!c.semanticOwner
    if (c.lifecycle?.activatable === true && !governed) {
      push(
        'CAP_UNGOVERNED_ACTIVATABLE',
        'fail',
        'semanticOwner 未登记或 governanceStatus != GOVERNED，activatable 必须为 false',
        c.id,
      )
    }

    // C10 生命周期合法性
    const supported = c.lifecycle?.supported || []
    for (const s of supported) {
      if (!lifecycleStates.includes(s)) {
        push('CAP_INVALID_LIFECYCLE', 'fail', `lifecycle.supported 含非法状态: ${s}`, c.id)
      }
    }
    if (c.lifecycle?.default && supported.length && !supported.includes(c.lifecycle.default)) {
      push('CAP_INVALID_LIFECYCLE', 'fail', `lifecycle.default 不在 supported 内: ${c.lifecycle.default}`, c.id)
    }
    if (c.lifecycle?.suspendable === true && c.resources?.suspendable === false) {
      push('CAP_INVALID_LIFECYCLE', 'warn', '声明 suspendable 但 resources.suspendable=false', c.id)
    }
  }

  // C4 强依赖成环
  const graph = new Map()
  for (const e of edges) {
    if (!e || !e.from || !e.to) continue
    if (String(e.kind) === 'optional') continue
    if (!graph.has(e.from)) graph.set(e.from, [])
    graph.get(e.from).push(e.to)
  }
  const WHITE = 0, GRAY = 1, BLACK = 2
  const color = new Map()
  let cycleFound = null
  const dfs = (n, path) => {
    color.set(n, GRAY)
    for (const m of graph.get(n) || []) {
      if (color.get(m) === GRAY) {
        cycleFound = [...path, n, m].join(' -> ')
        return true
      }
      if ((color.get(m) || WHITE) === WHITE) {
        if (dfs(m, [...path, n])) return true
      }
    }
    color.set(n, BLACK)
    return false
  }
  for (const n of graph.keys()) {
    if ((color.get(n) || WHITE) === WHITE) {
      if (dfs(n, [])) break
    }
  }
  if (cycleFound) {
    push('CAP_CIRCULAR_DEPENDENCY', 'fail', `强依赖成环: ${cycleFound}`, '<graph>')
  }

  // C8 禁止边
  for (const f of forbidden) {
    if (!f || !f.from || !f.to) continue
    const hit = edges.some((e) => e && e.from === f.from && e.to === f.to)
    if (hit) {
      push('CAP_FORBIDDEN_EDGE', 'fail', `出现禁止依赖方向: ${f.from} -> ${f.to}`, '<edges>')
    }
  }

  // C9 两文件依赖一致性（warn）
  const declared = new Map()
  for (const c of caps) {
    if (!c || !c.id) continue
    declared.set(c.id, new Set([...(c.dependsOn || []), ...(c.optionalDependencies || [])]))
  }
  for (const e of edges) {
    if (!e || !e.from || !e.to) continue
    if (!capIds.has(e.from)) continue
    const set = declared.get(e.from)
    if (set && !set.has(e.to)) {
      push('CAP_DEPENDENCY_MISMATCH', 'warn', `dependencies.yaml 有边 ${e.from} -> ${e.to}，但 capabilities.yaml 未声明`, e.from)
    }
  }
  for (const [id, set] of declared) {
    for (const dep of set) {
      const hit = edges.some((e) => e && e.from === id && e.to === dep)
      if (!hit) {
        push('CAP_DEPENDENCY_MISMATCH', 'warn', `capabilities.yaml 声明依赖 ${id} -> ${dep}，但 dependencies.yaml 无边`, id)
      }
    }
  }

  return findings
}

// ── 加载真实 registry ────────────────────────────────────────────────────

function loadRegistry() {
  const rd = (f) => {
    const p = join(REG_DIR, f)
    if (!existsSync(p)) throw new Error(`缺少 registry 文件: ${f}`)
    return parseYaml(readFileSync(p, 'utf8'))
  }
  const caps = rd('capabilities.yaml')
  const deps = rd('dependencies.yaml')
  const res = rd('resources.yaml')
  return {
    capabilities: caps.capabilities || [],
    sharedInfrastructure: deps.shared_infrastructure || [],
    edges: deps.edges || [],
    forbiddenEdges: deps.forbidden_edges || [],
    policies: res.policies || [],
    resourceClasses: res.resource_classes || {},
    lifecycleStates: res.resource_lifecycle || VALID_STATES,
  }
}

// ── 自检（positive / negative / false-positive 夹具） ────────────────────

const BASE_CLASSES = { LIGHT: 'x', MEDIUM: 'x', HEAVY: 'x' }

function baseCap(over = {}) {
  return {
    id: 'bookmark',
    name: 'Bookmark',
    category: 'CAPABILITY',
    provides: ['bookmark.list'],
    dependsOn: [],
    optionalDependencies: [],
    lifecycle: { supported: ['ACTIVE', 'SUSPENDED'], default: 'ACTIVE', activatable: true, resident: false },
    resources: { class: ['LIGHT'], suspendable: true, destroyable: true },
    permissions: [],
    persistence: { scope: 'disk', sensitive: false },
    entrypoint: 'src/x',
    semanticOwner: 'useBookmarkStore',
    governanceStatus: 'GOVERNED',
    status: 'COMPATIBILITY_WRAPPED',
    ...over,
  }
}

function baseInput(caps, over = {}) {
  return {
    capabilities: caps,
    sharedInfrastructure: [{ id: 'bridge' }],
    edges: [],
    forbiddenEdges: [],
    policies: caps.map((c) => ({ id: c.id, class: ['LIGHT'], suspendable: true, destroyable: true })),
    resourceClasses: BASE_CLASSES,
    lifecycleStates: VALID_STATES,
    ...over,
  }
}

function runSelfTest() {
  const cases = []

  // POSITIVE：合法 registry 必须零 fail
  cases.push({
    name: 'POSITIVE: valid single capability',
    expectCodes: [],
    input: baseInput([
      { ...baseCap(), dependsOn: ['bridge'] },
    ], { edges: [{ from: 'bookmark', to: 'bridge', kind: 'required' }] }),
  })

  // NEGATIVE：每条规则都必须被检出
  cases.push({
    name: 'NEGATIVE C1: duplicate id',
    expectCodes: ['CAP_DUPLICATE_ID'],
    input: baseInput([baseCap(), baseCap()]),
  })
  cases.push({
    name: 'NEGATIVE C2: missing field',
    expectCodes: ['CAP_MISSING_FIELD'],
    input: baseInput([baseCap({ permissions: undefined })]),
  })
  cases.push({
    name: 'NEGATIVE C3: unknown dependency',
    expectCodes: ['CAP_UNKNOWN_DEPENDENCY'],
    input: baseInput([baseCap({ dependsOn: ['nope'] })]),
  })
  cases.push({
    name: 'NEGATIVE C4: circular dependency',
    expectCodes: ['CAP_CIRCULAR_DEPENDENCY'],
    input: baseInput([
      baseCap({ id: 'a', dependsOn: ['b'] }),
      baseCap({ id: 'b', dependsOn: ['a'] }),
    ], { edges: [{ from: 'a', to: 'b', kind: 'required' }, { from: 'b', to: 'a', kind: 'required' }] }),
  })
  cases.push({
    name: 'NEGATIVE C5: missing resource policy',
    expectCodes: ['CAP_MISSING_RESOURCE_POLICY'],
    input: baseInput([baseCap()], { policies: [] }),
  })
  cases.push({
    name: 'NEGATIVE C6: unknown resource class',
    expectCodes: ['CAP_UNKNOWN_RESOURCE_CLASS'],
    input: baseInput([baseCap({ resources: { class: ['NOPE'], suspendable: false, destroyable: false } })]),
  })
  cases.push({
    name: 'NEGATIVE C7: ungoverned but activatable',
    expectCodes: ['CAP_UNGOVERNED_ACTIVATABLE'],
    input: baseInput([baseCap({ semanticOwner: null, governanceStatus: 'OWNER_PENDING_SCR' })]),
  })
  cases.push({
    name: 'NEGATIVE C8: forbidden edge',
    expectCodes: ['CAP_FORBIDDEN_EDGE'],
    input: baseInput([
      baseCap({ id: 'credential', dependsOn: ['browser'] }),
      baseCap({ id: 'browser', dependsOn: [] }),
    ], {
      edges: [{ from: 'credential', to: 'browser', kind: 'required' }],
      forbiddenEdges: [{ from: 'credential', to: 'browser', reason: 'x' }],
    }),
  })
  cases.push({
    name: 'NEGATIVE C10: invalid lifecycle state',
    expectCodes: ['CAP_INVALID_LIFECYCLE'],
    input: baseInput([baseCap({ lifecycle: { supported: ['NOPE'], default: 'ACTIVE', activatable: true } })]),
  })

  // FALSE-POSITIVE：合法但"看起来可疑"的模式必须**不**报错
  cases.push({
    name: 'FALSE-POSITIVE: optional dependency 不参与成环判定',
    expectCodes: [],
    input: baseInput([
      baseCap({ id: 'a', optionalDependencies: ['b'] }),
      baseCap({ id: 'b', optionalDependencies: ['a'] }),
    ], { edges: [{ from: 'a', to: 'b', kind: 'optional' }, { from: 'b', to: 'a', kind: 'optional' }] }),
  })
  cases.push({
    name: 'FALSE-POSITIVE: 未治理但 activatable=false 合法',
    expectCodes: [],
    input: baseInput([baseCap({
      semanticOwner: null,
      governanceStatus: 'OWNER_PENDING_SCR',
      lifecycle: { supported: ['ACTIVE'], default: 'ACTIVE', activatable: false, resident: false },
    })]),
  })
  cases.push({
    name: 'FALSE-POSITIVE: 空 dependsOn 合法',
    expectCodes: [],
    input: baseInput([baseCap({ dependsOn: [], optionalDependencies: [] })]),
  })

  let pass = 0
  let fail = 0
  for (const c of cases) {
    const got = new Set(validate(c.input).map((f) => f.code))
    const expected = new Set(c.expectCodes)
    const ok =
      [...expected].every((e) => got.has(e)) &&
      (expected.size > 0 ? true : got.size === 0)
    if (ok) pass++
    else fail++
    console.log(
      `${ok ? 'PASS' : 'FAIL'}  ${c.name}` +
        (ok ? '' : `\n      expected=[${[...expected]}] got=[${[...got]}]`),
    )
  }
  console.log(`\nSELF_TEST: ${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`)
  return fail === 0 ? 0 : 1
}

// ── CLI ──────────────────────────────────────────────────────────────────

const HELP = `check-capability-registry.mjs — Capability Registry 门禁

用法:
  node scripts/check-capability-registry.mjs              扫描真实 registry
  node scripts/check-capability-registry.mjs --self-test  自检（positive/negative/false-positive 夹具）
  node scripts/check-capability-registry.mjs --strict     warn 也判失败
  node scripts/check-capability-registry.mjs --json       机器可读输出
  node scripts/check-capability-registry.mjs --help       显示本帮助

规则:
  C1  CAP_DUPLICATE_ID            C6  CAP_UNKNOWN_RESOURCE_CLASS
  C2  CAP_MISSING_FIELD           C7  CAP_UNGOVERNED_ACTIVATABLE
  C3  CAP_UNKNOWN_DEPENDENCY      C8  CAP_FORBIDDEN_EDGE
  C4  CAP_CIRCULAR_DEPENDENCY     C9  CAP_DEPENDENCY_MISMATCH (warn)
  C5  CAP_MISSING_RESOURCE_POLICY C10 CAP_INVALID_LIFECYCLE

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

  let findings
  try {
    findings = validate(loadRegistry())
  } catch (e) {
    console.error(`ERROR: ${e.message}`)
    return 1
  }

  const fails = findings.filter((f) => f.level === 'fail')
  const warns = findings.filter((f) => f.level === 'warn')

  if (json) {
    console.log(
      JSON.stringify(
        {
          result: fails.length === 0 && !(strict && warns.length) ? 'PASS' : 'FAIL',
          fail: fails.length,
          warn: warns.length,
          findings,
        },
        null,
        2,
      ),
    )
  } else {
    for (const f of findings) {
      console.log(`[${f.level.toUpperCase()}] ${f.code}  ${f.where}: ${f.message}`)
    }
    console.log(
      `\nCAPABILITY_REGISTRY_RESULT=${fails.length === 0 && !(strict && warns.length) ? 'PASS' : 'FAIL'} ` +
        `(fail=${fails.length} warn=${warns.length})`,
    )
  }

  if (fails.length > 0) return 1
  if (strict && warns.length > 0) return 1
  return 0
}

// 仅当作为主模块运行时才执行 CLI；被 import 时只导出能力（供其它门禁复用解析器）
const isMain =
  process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1])
if (isMain) process.exit(main())

export { parseYaml, validate, loadRegistry, REG_DIR }
