#!/usr/bin/env node
// ---------------------------------------------------------------------------
// capability-resource-report.mjs — Capability Resource Report 生成器（Phase 7E）
//
// 职责：从 capability-registry 真源生成资源报告，回答"关闭某个功能后能省什么"。
//
// 诚实约束（硬）：
//   - 只输出 DECLARED RESOURCE CLASS，**不生成任何实测数字**
//   - 报告必须显式标注 measurement_status = NOT_AVAILABLE
//   - 不得把 TARGET_COMPOSABLE 写成已实现
//
// 用法:
//   node scripts/capability-resource-report.mjs              打印报告到 stdout
//   node scripts/capability-resource-report.mjs --out <file> 写入文件
//   node scripts/capability-resource-report.mjs --json       机器可读
//   node scripts/capability-resource-report.mjs --self-test  自检（护栏断言）
//   node scripts/capability-resource-report.mjs --help
// ---------------------------------------------------------------------------

import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import { parseYaml, validate, REG_DIR } from './check-capability-registry.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')

function load() {
  const caps = parseYaml(readFileSync(join(REG_DIR, 'capabilities.yaml'), 'utf8'))
  const res = parseYaml(readFileSync(join(REG_DIR, 'resources.yaml'), 'utf8'))
  const prof = parseYaml(readFileSync(join(REG_DIR, 'profiles.yaml'), 'utf8'))
  return { caps, res, prof }
}

function buildReport() {
  const { caps, res, prof } = load()
  const policies = new Map((res.policies || []).map((p) => [p.id, p]))
  const composability = prof.composability || {}
  const measurement = res.measurement_status || {}

  const rows = (caps.capabilities || []).map((c) => {
    const p = policies.get(c.id) || {}
    const cls = (c.resources?.class || p.class || []).join(' + ')
    return {
      id: c.id,
      name: c.name,
      resourceClass: cls || '(未声明)',
      suspendable: p.suspendable === true,
      destroyable: p.destroyable === true,
      resident: c.lifecycle?.resident === true,
      composability: composability[c.id] || 'UNSPECIFIED',
      releaseNote: p.release_note || '(无)',
    }
  })

  const profiles = (prof.profiles || []).map((p) => ({
    id: p.id,
    name: p.name,
    description: p.description,
    capabilities: p.capabilities || [],
    note: p.note,
  }))

  return { rows, profiles, measurement, disclaimer: prof.demo_disclaimer }
}

function renderMarkdown(rep) {
  const L = []
  L.push('# Capability Resource Report')
  L.push('')
  L.push('> 由 `scripts/capability-resource-report.mjs` 从 `capability-registry` 真源生成。')
  L.push('> **口径 = DECLARED RESOURCE CLASS（声明式分类），不是实测数值。**')
  L.push('')
  L.push('## 1. 测量现状（诚实声明）')
  L.push('')
  L.push('```text')
  for (const [k, v] of Object.entries(rep.measurement)) {
    if (k === 'note') continue
    L.push(`${k}: ${v}`)
  }
  L.push('```')
  L.push('')
  L.push('> 本项目**没有**按能力的资源实测机制。任何"关闭某功能省了 X MB"的说法都不可靠，')
  L.push('> 本报告只给出声明式分类与可预期释放对象（登记为 Debt-7A-1）。')
  L.push('')
  L.push('## 2. 能力资源总表')
  L.push('')
  L.push('| 能力 | 资源分类(DECLARED) | 可暂停 | 可销毁 | 常驻 | 可组合性 | 关闭后可预期释放 |')
  L.push('|-|-|-|-|-|-|-|')
  for (const r of rep.rows) {
    L.push(
      `| ${r.id} | ${r.resourceClass} | ${r.suspendable ? '✅' : '⬜'} | ${r.destroyable ? '✅' : '⬜'} | ` +
        `${r.resident ? '✅' : '⬜'} | ${r.composability} | ${r.releaseNote} |`,
    )
  }
  L.push('')
  L.push('图例：`CURRENTLY_COMPOSABLE`（真正可独立启停）/ `COMPATIBILITY_WRAPPED`（兼容包装）/ ')
  L.push('`TARGET_COMPOSABLE`（目标态未实现）/ `NOT_COMPOSABLE_BY_DESIGN`（安全或常驻，设计上不参与组合）。')
  L.push('')
  L.push('## 3. Composition Profiles（演示口径）')
  L.push('')
  for (const p of rep.profiles) {
    L.push(`### ${p.name} (\`${p.id}\`)`)
    L.push('')
    L.push(`${p.description}`)
    L.push('')
    L.push('```text')
    L.push(p.capabilities.join('\n'))
    L.push('```')
    L.push('')
    if (p.note) L.push(`> ${p.note}`)
    L.push('')
  }
  L.push('## 4. 演示免责声明')
  L.push('')
  L.push('> ' + String(rep.disclaimer || '').trim().replace(/\s+/g, ' '))
  L.push('')
  return L.join('\n')
}

// ── 自检：护栏断言（防止报告"说谎"） ──────────────────────────────────────

function selfTest() {
  const results = []
  const t = (id, name, fn) => {
    try {
      const r = fn()
      results.push({ id, name, ok: r === true, detail: r === true ? '' : String(r) })
    } catch (e) {
      results.push({ id, name, ok: false, detail: e.message })
    }
  }

  const rep = buildReport()

  t('RPT-01', '每个能力都有资源分类', () => {
    const bad = rep.rows.filter((r) => !r.resourceClass || r.resourceClass === '(未声明)')
    return bad.length === 0 ? true : `缺资源分类: ${bad.map((b) => b.id).join(',')}`
  })

  t('RPT-02', '报告不含任何实测数字（MB/GB/百分比）', () => {
    const md = renderMarkdown(rep)
    const bad = md.match(/\d+(\.\d+)?\s*(MB|GB|KB|%)/g)
    return bad ? `发现疑似实测数字: ${bad.join(',')}` : true
  })

  t('RPT-03', '显式声明 measurement NOT_AVAILABLE', () => {
    const md = renderMarkdown(rep)
    return md.includes('NOT_AVAILABLE') ? true : '报告未声明测量不可用'
  })

  t('RPT-04', '可组合性状态合法且区分三态', () => {
    const ok = ['CURRENTLY_COMPOSABLE', 'COMPATIBILITY_WRAPPED', 'TARGET_COMPOSABLE', 'NOT_COMPOSABLE_BY_DESIGN']
    const bad = rep.rows.filter((r) => !ok.includes(r.composability))
    return bad.length === 0 ? true : `非法状态: ${bad.map((b) => `${b.id}=${b.composability}`).join(',')}`
  })

  t('RPT-05', '不得声称 CURRENTLY_COMPOSABLE（今晚无能力真正可独立启停）', () => {
    const bad = rep.rows.filter((r) => r.composability === 'CURRENTLY_COMPOSABLE')
    return bad.length === 0 ? true : `不应出现 CURRENTLY_COMPOSABLE: ${bad.map((b) => b.id).join(',')}`
  })

  t('RPT-06', 'bookmark 必须标为 COMPATIBILITY_WRAPPED', () => {
    const b = rep.rows.find((r) => r.id === 'bookmark')
    return b && b.composability === 'COMPATIBILITY_WRAPPED' ? true : `实际=${b?.composability}`
  })

  t('RPT-07', '至少三个 Profile', () => {
    return rep.profiles.length >= 3 ? true : `仅 ${rep.profiles.length} 个`
  })

  t('RPT-08', 'registry 本身校验通过（不绕过既有门禁）', () => {
    const { caps, res, prof } = load()
    const findings = validate({
      capabilities: caps.capabilities || [],
      sharedInfrastructure: [{ id: 'bridge' }],
      edges: [],
      forbiddenEdges: [],
      policies: res.policies || [],
      resourceClasses: res.resource_classes || {},
      lifecycleStates: res.resource_lifecycle || [],
    })
    const fails = findings.filter((f) => f.level === 'fail')
    // 注意：此调用未传 edges，CAP_DEPENDENCY_MISMATCH(warn) 必然出现，只校验 fail
    return fails.length === 0 ? true : `registry fail: ${fails.map((f) => f.code).join(',')}`
  })

  const pass = results.filter((r) => r.ok).length
  for (const r of results) {
    console.log(`${r.ok ? 'PASS' : 'FAIL'}  ${r.id}  ${r.name}${r.ok ? '' : `  → ${r.detail}`}`)
  }
  console.log(`\nCAPABILITY_RESOURCE_REPORT_SELFTEST=${pass === results.length ? 'PASS' : 'FAIL'} (${pass}/${results.length})`)
  return pass === results.length ? 0 : 1
}

const HELP = `capability-resource-report.mjs — Capability Resource Report（Phase 7E）

用法:
  node scripts/capability-resource-report.mjs              打印报告
  node scripts/capability-resource-report.mjs --out <file> 写入文件
  node scripts/capability-resource-report.mjs --json        机器可读
  node scripts/capability-resource-report.mjs --self-test   护栏自检（RPT-01..08）
  node scripts/capability-resource-report.mjs --help

口径: DECLARED RESOURCE CLASS（声明式），不含实测数字。
`

function main() {
  const argv = process.argv.slice(2)
  if (argv.includes('--help') || argv.includes('-h')) {
    console.log(HELP)
    return 0
  }
  if (argv.includes('--self-test')) return selfTest()

  const rep = buildReport()
  if (argv.includes('--json')) {
    console.log(JSON.stringify(rep, null, 2))
    return 0
  }
  const md = renderMarkdown(rep)
  const oi = argv.indexOf('--out')
  if (oi >= 0 && argv[oi + 1]) {
    const p = join(ROOT, argv[oi + 1])
    writeFileSync(p, md, 'utf8')
    console.log(`REPORT_WRITTEN path=${p}`)
    return 0
  }
  console.log(md)
  return 0
}

process.exit(main())
