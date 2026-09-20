#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-capability-pilot.mjs — 试点能力（Bookmark）集成门禁（Phase 7D）
//
// 证明六件事：
//   PLT-01 manifest 可以注册
//   PLT-02 Runtime 可以发现
//   PLT-03 Capability 可以 activate
//   PLT-04 不加载其它可选 Capability 时也能工作
//   PLT-05 不破坏原业务 Owner（源码 + git 双重断言）
//   PLT-06 manifest 与 capability-registry/capabilities.yaml 一致
//   PLT-07 suspend 后再 activate 可用
//   PLT-08 inspect 不含任何业务状态字段
//
// 加载真实 src/capability/index.ts（esbuild bundle），不是复制逻辑另测。
//
// 用法:
//   node scripts/check-capability-pilot.mjs [--self-test] [--json] [--help]
// ---------------------------------------------------------------------------

import { readFileSync, writeFileSync, unlinkSync, existsSync, readdirSync, statSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { dirname, join } from 'node:path'
import { execSync } from 'node:child_process'
import { build } from 'esbuild'
import { parseYaml, REG_DIR } from './check-capability-registry.mjs'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const INDEX_TS = join(ROOT, 'src/capability/index.ts')
const CAP_DIR = join(ROOT, 'src/capability')
const BASELINE_TAG = 'semantic-governance-v1'

async function loadBootstrap() {
  const res = await build({
    entryPoints: [INDEX_TS],
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    target: 'es2020',
    // .vue 是能力内部 UI 组件，经 defineAsyncComponent 懒加载；pilot 仅做 import 扫描，
    // 不解析/不执行 .vue，故标记为 external（与 vite 的 .vue 解析解耦，不弱化 PLT 检查）。
    external: ['*.vue'],
    write: false,
  })
  const code = res.outputFiles[0].text
  const tmp = join(ROOT, '.tmp-capability-pilot.mjs')
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

async function runTests() {
  // (b)(pre) 在真正断言前解析 bookmark 业务文件当前规范路径（跟随 8A.1 locator，兼容物理迁移）
  const { resolveOwnerFile } = await import(pathToFileURL(join(ROOT, 'scripts/check-semantic-registry.mjs')).href)
  const results = []
  const t = async (id, name, fn) => {
    try {
      const r = await fn()
      results.push({ id, name, ok: r === true, detail: r === true ? '' : String(r) })
    } catch (e) {
      results.push({ id, name, ok: false, detail: e && e.message ? e.message : String(e) })
    }
  }

  const mod = await loadBootstrap()
  const { bootstrapCapabilityRuntime, getCapabilityRuntime } = mod
  const boot = bootstrapCapabilityRuntime()
  const rt = boot.runtime ?? getCapabilityRuntime()

  await t('PLT-01', 'manifest 可以注册', () => {
    const rec = rt.get('bookmark')
    return rec && rec.definition.id === 'bookmark' ? true : `注册失败: ${JSON.stringify(rec?.id)}`
  })

  await t('PLT-02', 'Runtime 可以发现该能力', () => {
    const list = rt.inspect()
    const hit = list.find((x) => x.id === 'bookmark')
    if (!hit) return 'inspect() 找不到 bookmark'
    const need = ['id', 'name', 'state', 'enabled', 'governanceStatus', 'status', 'resourceClass']
    const miss = need.filter((k) => hit[k] === undefined)
    return miss.length === 0 ? true : `inspect 缺字段: ${miss.join(',')}`
  })

  await t('PLT-03', 'Capability 可以 activate', () => {
    const rec = rt.get('bookmark')
    return rec.state === 'ACTIVE' ? true : `状态非 ACTIVE: ${rec.state}（error=${boot.error}）`
  })

  await t('PLT-04', '不加载其它可选 Capability 时也能工作', () => {
    const list = rt.inspect()
    if (list.length !== 1) return `注册项应为 1（仅 bookmark），实际 ${list.length}`
    const def = rt.get('bookmark').definition
    if (def.dependsOn.length !== 0) return `强依赖应为空（可选依赖允许缺失），实际 ${def.dependsOn}`
    return true
  })

  await t('PLT-05', '不破坏原业务 Owner（源码 + git 双重断言）', () => {
    // (a) 源码：capability 层不得 import 业务 store
    //     注意：manifest 中以字符串形式 "引用" semanticOwner 是设计允许的，
    //     因此必须先剥离注释、再只匹配真实 import/require 语句，避免误报。
    const files = collectTs(CAP_DIR)
    const banned = ['useBookmarkStore', 'useBrowserStore', 'useSystemStore', 'useWorkspaceStore']
    for (const f of files) {
      const src = stripComments(readFileSync(f, 'utf8'))
      for (const b of banned) {
        const re = new RegExp(
          `(from\\s+['"][^'"]*${b}\\b|import\\(\\s*['"][^'"]*${b}\\b|require\\(\\s*['"][^'"]*${b}\\b)`,
        )
        if (re.test(src)) return `${f} 真实引入了业务 store: ${b}`
      }
    }
    // (b) git：既有 bookmark 业务文件相对冻结基线无「逻辑改动」。
    //     物理迁移（git mv）只是 rename，内容不变；--diff-filter=M 仅匹配真正的内容修改（M），
    //     忽略 rename(纯 R)/add/delete，避免 Phase 8B 迁移误报。
    //     路径经 owner_implementations locator 解析，使断言跟随物理路径迁移（与 8A.1 一致）。
    try {
      const bmPath = resolveOwnerFile('useBookmarkStore')
      const gitPaths = [bmPath, 'src/components/home'].filter(Boolean)
      const out = execSync(
        `git diff --name-only --diff-filter=M ${BASELINE_TAG} HEAD -- ${gitPaths.join(' ')}`,
        { cwd: ROOT, encoding: 'utf8' },
      ).trim()
      if (out !== '') return `既有业务文件被改动: ${out}`
    } catch (e) {
      return `git 断言执行失败: ${e.message}`
    }
    return true
  })

  await t('PLT-06', 'manifest 与 capabilities.yaml 一致', () => {
    const yamlPath = join(REG_DIR, 'capabilities.yaml')
    if (!existsSync(yamlPath)) return '缺少 capabilities.yaml'
    const reg = parseYaml(readFileSync(yamlPath, 'utf8'))
    const y = (reg.capabilities || []).find((c) => c && c.id === 'bookmark')
    if (!y) return 'capabilities.yaml 无 bookmark'
    const m = rt.get('bookmark').definition
    const cmp = [
      ['id', y.id, m.id],
      ['semanticOwner', y.semanticOwner, m.semanticOwner],
      ['governanceStatus', y.governanceStatus, m.governanceStatus],
      ['resources.class', JSON.stringify(y.resources?.class), JSON.stringify(m.resources?.class)],
      ['lifecycle.activatable', y.lifecycle?.activatable, m.lifecycle?.activatable],
    ]
    const bad = cmp.filter(([, a, b]) => String(a) !== String(b))
    return bad.length === 0
      ? true
      : `不一致: ${bad.map(([k, a, b]) => `${k}(yaml=${a} manifest=${b})`).join('; ')}`
  })

  await t('PLT-07', 'suspend 后可再次 activate', () => {
    const rt2 = rt
    rt2.suspend('bookmark')
    if (rt2.get('bookmark').state !== 'SUSPENDED') return 'suspend 失败'
    rt2.activate('bookmark')
    return rt2.get('bookmark').state === 'ACTIVE' ? true : '再次 activate 失败'
  })

  await t('PLT-08', 'inspect 不含业务状态字段', () => {
    const keys = Object.keys(rt.inspect()[0])
    const banned = ['items', 'panelOpen', 'loaded', 'busy', 'error', 'sorted', 'termPanes']
    const hit = keys.filter((k) => banned.includes(k))
    return hit.length === 0 ? true : `inspect 含业务状态字段: ${hit.join(',')}`
  })

  return results
}

/** 剥离注释，避免把注释里的 "import xxxStore" 当成真实引入（误报防护） */
function stripComments(s) {
  return s
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(^|[^:])\/\/.*$/gm, '$1')
}

function collectTs(dir) {
  const out = []
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) out.push(...collectTs(p))
    else if (p.endsWith('.ts')) out.push(p)
  }
  return out
}

const HELP = `check-capability-pilot.mjs — 试点能力（Bookmark）集成门禁

用法:
  node scripts/check-capability-pilot.mjs              运行 PLT-01..PLT-08
  node scripts/check-capability-pilot.mjs --self-test   同上
  node scripts/check-capability-pilot.mjs --json        机器可读
  node scripts/check-capability-pilot.mjs --help

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
    console.log(`\nCAPABILITY_PILOT_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${results.length})`)
  }
  return fail === 0 ? 0 : 1
}

main().then((c) => process.exit(c))
