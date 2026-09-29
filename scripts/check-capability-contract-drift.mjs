#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-contract-drift.mjs — 能力契约漂移门禁（STAGE H-B，§7/§8）
//
// 能力元数据当前分布在两处：
//   - docs/architecture/capability-registry/capabilities.yaml（注册表：编排/组合/门禁消费）
//   - src/capabilities/<id>/manifest.ts（代码侧 Building Block Contract v1）
//
// 两者必须一致。本门禁保证 **DRIFT → FAIL**，避免多份手填事实长期互相同步而静默漂移。
//
//   DRIFT-01 fail  id 不一致
//   DRIFT-02 fail  dependsOn 不一致（registry ↔ manifest 顶层 / v1.dependencies）
//   DRIFT-03 fail  optionalDependencies 不一致
//   DRIFT-04 fail  entrypoint 不一致（归一化：去掉 /index.ts 与末尾斜杠后比较）
//   DRIFT-05 fail  manifest 存在但 registry 缺对应条目（或反之）
//
// 用法:
//   node scripts/check-capability-contract-drift.mjs               真实扫描
//   node scripts/check-capability-contract-drift.mjs --self-test    自检
//   node scripts/check-capability-contract-drift.mjs --json         JSON 输出
//   node scripts/check-capability-contract-drift.mjs --help
// ---------------------------------------------------------------------------

import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, posix } from 'node:path'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const REG_REL = 'docs/architecture/capability-registry/capabilities.yaml'

/** 解析 capabilities.yaml：id → { dependsOn, optionalDependencies, entrypoint, status } */
function parseRegistry(text) {
  const out = {}
  let cur = null
  let field = null
  for (const raw of text.split(/\r?\n/)) {
    const mId = raw.match(/^ {2}- id:\s*([a-zA-Z_0-9]+)\s*$/)
    if (mId) {
      cur = mId[1]
      out[cur] = { dependsOn: [], optionalDependencies: [], entrypoint: '', status: '' }
      field = null
      continue
    }
    if (!cur) continue
    const mInlineList = raw.match(/^ {4}(dependsOn|optionalDependencies):\s*\[(.*)\]\s*$/)
    if (mInlineList) {
      out[cur][mInlineList[1]] = mInlineList[2]
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean)
      field = null
      continue
    }
    const mBlockList = raw.match(/^ {4}(dependsOn|optionalDependencies):\s*$/)
    if (mBlockList) {
      field = mBlockList[1]
      continue
    }
    const mItem = raw.match(/^ {6}- ([a-zA-Z_0-9]+)\s*$/)
    if (mItem && field) {
      out[cur][field].push(mItem[1])
      continue
    }
    const mScalar = raw.match(/^ {4}(entrypoint|status):\s*(.+)\s*$/)
    if (mScalar) {
      let v = mScalar[2].trim()
      if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1)
      out[cur][mScalar[1]] = v
      field = null
      continue
    }
    // 任何新的 4 空格字段都结束列表收集
    if (/^ {4}[a-zA-Z_]+:/.test(raw)) field = null
  }
  return out
}

/** 解析 manifest.ts：顶层 dependsOn / optionalDependencies + v1.{id,dependencies,optionalDependencies,entrypoint} */
function parseManifest(text) {
  const out = {
    id: '',
    dependsOn: null,
    optionalDependencies: null,
    v1Id: '',
    v1Dependencies: null,
    v1OptionalDependencies: null,
    v1Entrypoint: '',
  }
  const listField = (re) => {
    const m = text.match(re)
    if (!m) return null
    return m[1]
      .split(',')
      .map((s) => s.trim().replace(/^["']|["']$/g, ''))
      .filter(Boolean)
  }
  out.dependsOn = listField(/^ {2}dependsOn:\s*\[(.*)\]/m)
  out.optionalDependencies = listField(/^ {2}optionalDependencies:\s*\[(.*)\]/m)
  out.v1Dependencies = listField(/^ {4}dependencies:\s*\[(.*)\]/m)
  out.v1OptionalDependencies = listField(/^ {4}optionalDependencies:\s*\[(.*)\]/m)
  const id = text.match(/^ {2}id:\s*"([a-zA-Z_0-9]+)"/m)
  if (id) out.id = id[1]
  const v1id = text.match(/^ {4}id:\s*"([a-zA-Z_0-9]+)"/m)
  if (v1id) out.v1Id = v1id[1]
  const ep = text.match(/^ {4}entrypoint:\s*"([^"]+)"/m)
  if (ep) out.v1Entrypoint = ep[1]
  return out
}

// §13：Capability Dependency ≠ Infrastructure Dependency。
// bridge（及同类）是 SHARED_INFRASTRUCTURE（见 dependencies.yaml shared_infrastructure），
// 不是 capability，不在 CAPABILITY_CATALOG 中。能力依赖比较时两侧都归一化掉 ——
// 这不是隐藏漂移：基础设施依赖的真源是 dependencies.yaml，由别的门禁消费。
const SHARED_INFRA = new Set(['bridge'])
const dropInfra = (l) => (l || []).filter((x) => !SHARED_INFRA.has(x))
const normEntry = (s) => String(s || '').trim().replace(/\/index\.ts$/, '').replace(/\/+$/, '')
const eqList = (a, b) => {
  const A = [...(a || [])].sort()
  const B = [...(b || [])].sort()
  return A.length === B.length && A.every((x, i) => x === B[i])
}
const show = (l) => `[${(l || []).join(', ')}]`

function run({ registryText, manifests }) {
  const findings = []
  const push = (code, detail) => findings.push({ code, severity: 'fail', detail })
  const reg = parseRegistry(registryText || '')

  for (const [id, m] of Object.entries(manifests)) {
    const r = reg[id]
    if (!r) {
      push('DRIFT-05', `manifest 存在但 registry 缺条目：${id}`)
      continue
    }
    if (m.v1Id && m.v1Id !== id) push('DRIFT-01', `${id}: v1.id=${m.v1Id} 与目录/registry id 不一致`)
    // dependsOn：顶层与 v1.dependencies 任一与 registry 不一致即漂移
    const rDeps = dropInfra(r.dependsOn)
    const rOpt = dropInfra(r.optionalDependencies)
    if (m.dependsOn && !eqList(dropInfra(m.dependsOn), rDeps)) {
      push('DRIFT-02', `${id}: manifest.dependsOn=${show(m.dependsOn)} ≠ registry.dependsOn=${show(r.dependsOn)}（已归一化 shared infra）`)
    }
    if (m.v1Dependencies && !eqList(dropInfra(m.v1Dependencies), rDeps)) {
      push('DRIFT-02', `${id}: v1.dependencies=${show(m.v1Dependencies)} ≠ registry.dependsOn=${show(r.dependsOn)}（已归一化 shared infra）`)
    }
    if (m.optionalDependencies && !eqList(dropInfra(m.optionalDependencies), rOpt)) {
      push('DRIFT-03', `${id}: manifest.optionalDependencies=${show(m.optionalDependencies)} ≠ registry=${show(r.optionalDependencies)}（已归一化 shared infra）`)
    }
    if (m.v1OptionalDependencies && !eqList(dropInfra(m.v1OptionalDependencies), rOpt)) {
      push('DRIFT-03', `${id}: v1.optionalDependencies=${show(m.v1OptionalDependencies)} ≠ registry=${show(r.optionalDependencies)}（已归一化 shared infra）`)
    }
    if (m.v1Entrypoint && normEntry(m.v1Entrypoint) !== normEntry(r.entrypoint)) {
      push('DRIFT-04', `${id}: v1.entrypoint=${m.v1Entrypoint} ≠ registry.entrypoint=${r.entrypoint}`)
    }
  }
  // DRIFT-05：registry 有条目但无 manifest.ts。
  // 显式豁免：`status: NOT_INTEGRATED` 的能力**本就无代码侧能力包**（无 capabilities/<id>/ 目录），
  // 因而没有、也不应凭空造一个 manifest.ts。这是结构性事实，不是被忽略的漂移 —— 见 §16 裁决矩阵。
  // 其余状态（COMPATIBILITY_WRAPPED / C1..C3 等）缺 manifest 一律 FAIL。
  for (const id of Object.keys(reg)) {
    if (manifests[id]) continue
    if (reg[id].status === 'NOT_INTEGRATED') {
      findings.push({ code: 'DRIFT-05-EXEMPT', severity: 'info', detail: `${id}: status=NOT_INTEGRATED，无代码能力包 ⇒ 无 manifest（结构性事实，非漂移）` })
      continue
    }
    push('DRIFT-05', `registry 有条目但无 manifest.ts：${id}（status=${reg[id].status || '(空)'}）`)
  }
  return findings
}

function loadReal() {
  const registryText = existsSync(join(ROOT, REG_REL)) ? readFileSync(join(ROOT, REG_REL), 'utf8') : ''
  const manifests = {}
  const dirs = [
    ...readdirSync(join(ROOT, 'src/capabilities')).map((d) => `src/capabilities/${d}`),
    'src/settings',
  ]
  // 包形态能力（@browser-os/capability-*）：manifest 位于 packages/capability-<id>/src/manifest.ts
  // （与 src/capabilities/<id>/manifest.ts 同构，只是物理位置不同；contribution 单点注册由 Host 完成）。
  for (const d of readdirSync(join(ROOT, 'packages'))) {
    if (/^capability-/.test(d)) dirs.push(`packages/${d}/src`)
  }
  for (const d of dirs) {
    const p = join(ROOT, d, 'manifest.ts')
    if (!existsSync(p)) continue
    let id
    if (d.startsWith('src/capabilities/')) id = d.split('/')[2]
    else if (d.startsWith('packages/capability-')) id = d.split('/')[1].replace(/^capability-/, '')
    else id = 'settings'
    manifests[id] = parseManifest(readFileSync(p, 'utf8'))
  }
  // M2 包：manifest 已迁入 packages/capability-<id>/src/manifest.ts（原 src/capabilities/<id>/manifest.ts
  // 在 M2(B) 删除）。DRIFT-05 必须识别包内 manifest，否则误报「registry 有条目但无 manifest.ts」。
  // id 由包名 capability-<id> 推导（与 capabilityId 一致）。
  for (const d of readdirSync(join(ROOT, 'packages'))) {
    if (!d.startsWith('capability-')) continue
    const p = join(ROOT, 'packages', d, 'src', 'manifest.ts')
    if (!existsSync(p)) continue
    const id = d.slice('capability-'.length)
    manifests[id] = parseManifest(readFileSync(p, 'utf8'))
  }
  return { registryText, manifests }
}

function runSelfTest() {
  let bad = 0
  const t = (name, expect, input) => {
    const got = [...new Set(run(input).map((f) => f.code))]
    const ok = expect.length === 0 ? got.length === 0 : expect.every((c) => got.includes(c))
    if (ok) console.log(`  ✓ ${name}`)
    else {
      bad++
      console.log(`  ✗ ${name} — expect [${expect}] got [${got}]`)
    }
  }
  const regOk = '  - id: demo\n    dependsOn:\n      - workspace\n    optionalDependencies: []\n    entrypoint: src/capabilities/demo\n'
  // DRIFT-02 使用真实 capability 依赖，避免 shared infrastructure 被归一化后造成假绿。
  const regDepGit = '  - id: demo\n    dependsOn:\n      - git\n    optionalDependencies: []\n    entrypoint: src/capabilities/demo\n'
  t('POSITIVE: 完全一致', [], {
    registryText: regOk,
    manifests: {
      demo: {
        id: 'demo', v1Id: 'demo',
        dependsOn: ['workspace'], optionalDependencies: [],
        v1Dependencies: ['workspace'], v1OptionalDependencies: [],
        v1Entrypoint: 'src/capabilities/demo/index.ts',
      },
    },
  })
  t('DRIFT-02: dependsOn 漂移', ['DRIFT-02'], {
    registryText: regDepGit,
    manifests: {
      demo: {
        id: 'demo', v1Id: 'demo',
        dependsOn: [], optionalDependencies: [],
        v1Dependencies: [], v1OptionalDependencies: [],
        v1Entrypoint: 'src/capabilities/demo/index.ts',
      },
    },
  })
  t('DRIFT-04: entrypoint 漂移', ['DRIFT-04'], {
    registryText: regOk,
    manifests: {
      demo: {
        id: 'demo', v1Id: 'demo',
        dependsOn: ['workspace'], optionalDependencies: [],
        v1Dependencies: ['workspace'], v1OptionalDependencies: [],
        v1Entrypoint: 'src/capabilities/other/index.ts',
      },
    },
  })
  t('DRIFT-05: registry 缺条目', ['DRIFT-05'], { registryText: '', manifests: { demo: { id: 'demo' } } })
  console.log(bad === 0 ? '\nCAPABILITY_DRIFT_SELF_TEST=ALL_PASS' : `\nCAPABILITY_DRIFT_SELF_TEST=FAIL (${bad})`)
  return bad
}

const args = process.argv.slice(2)
if (args.includes('--help')) {
  console.log('check-capability-contract-drift.mjs — STAGE H-B 能力契约漂移门禁（DRIFT → FAIL）')
  process.exit(0)
}
if (args.includes('--self-test')) process.exit(runSelfTest() === 0 ? 0 : 1)

const input = loadReal()
const findings = run(input)
const fail = findings.filter((f) => f.severity === 'fail')
const info = findings.filter((f) => f.severity !== 'fail')
if (args.includes('--json')) console.log(JSON.stringify({ fail, info }, null, 1))
else {
  console.log(`\n== CAPABILITY CONTRACT DRIFT ==\nmanifests=${Object.keys(input.manifests).length}`)
  for (const f of fail) console.log(`  [FAIL] ${f.code} ${f.detail}`)
  for (const f of info) console.log(`  [INFO] ${f.code} ${f.detail}`)
  console.log(`\nfail=${fail.length} info=${info.length}`)
  console.log(`CAPABILITY_CONTRACT_DRIFT_RESULT=${fail.length ? 'FAIL' : 'PASS'}`)
}
process.exit(fail.length ? 1 : 0)
