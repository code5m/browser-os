#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-vault-boundary.mjs — Frontend M2 Package Pilot：vault 能力包边界门禁
//
// 设计原则（禁止"grep 固定旧文件名就宣称 PASS"）：
//   - 所有静态检查均基于**真实文件表**（Map<relPath, content>）与**真实 import 解析**。
//   - 检查逻辑全部接受可注入的 files Map → self-test 可用合成 fixture 证明检出能力。
//   - 外部到期路径检测只看真实引用（import 说明符），不看注释里的历史字样。
//
// 门禁码：
//   VB-01 能力外部不得直接 import vault internal（state/ ui/ internal/）
//   VB-02 public API 导出与实际 symbol 一致（无缺失导出）
//   VB-03 禁止反向依赖：capability 不得依赖 App.vue / components/** / main.ts
//   VB-04 禁止跨能力 internal 直连（只能用其它能力的 public/index/manifest）
//   VB-05 state owner 唯一（defineStore('vault') 恰好 1 处；manifest.semanticOwner 指向真实文件）
//   VB-06 native invoke 不得出现在 UI 层（禁止 ui/*.vue 直接用 bridge/invoke）
//   VB-07 UI 注册权威入口唯一（"活"注册恰好 1 处；别名 registerContributions 记为 FG 而非静默放过）
//   VB-08 stale old path = 0（旧的 src/utils/vault.mjs 物理不存在 + 无任何真实引用）
//   VB-09 dead shim = 0（包外不得再导出被下沉的 symbol 做兼容层）
//   VB-10 duplicate semantic implementation = 0（下沉 symbol 全仓定义恰好 1 处）
//
// 用法:
//   node scripts/check-vault-boundary.mjs              # 真实扫描
//   node scripts/check-vault-boundary.mjs --self-test  # 合成 fixture 证明检出能力
//   node scripts/check-vault-boundary.mjs --json
// ---------------------------------------------------------------------------

import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs'
import { join, relative, dirname, normalize, posix } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
const ROOT = join(HERE, '..')

const PKG = 'vault'
const PKG_REL = `src/capabilities/${PKG}`
// 允许外部触达的能力面（唯一稳定出入口）
const PUBLIC_SURFACE = new Set(['index.ts', 'public.ts', 'manifest.ts'])
// 由 Pilot 下沉进能力包的领域 symbol（唯一实现）
const SUNK_SYMBOLS = ['resolveNote', 'noteLinks', 'searchNotes']

// --------------------------------------------------------------------------
// 通用工具
// --------------------------------------------------------------------------

function walkDisk(dir, out) {
  if (!existsSync(dir)) return out
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    const st = statSync(p)
    if (st.isDirectory()) walkDisk(p, out)
    else if (/\.(ts|vue|mjs|js)$/.test(name) && !/\.d\.ts$/.test(name)) out.add(p)
  }
  return out
}

/** 采集真实 src 入口（产品代码 + gates 用到的 scripts，用于 stale path 检测） */
function collectReal(root = ROOT) {
  const files = new Map()
  for (const abs of walkDisk(join(root, 'src'), new Set())) {
    files.set(relative(root, abs).split('\\').join('/'), readFileSync(abs, 'utf8'))
  }
  return files
}

/** 抽取真实 import/export-from 说明符（严格：只认语句，不认注释） */
function parseSpecifiers(code) {
  const specs = []
  const stripComments = code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|\s)\/\/[^\n]*/g, '$1')
  const patterns = [
    /\bimport\s+(?:type\s+)?[\s\S]{0,400}?from\s+['"]([^'"]+)['"]/g,
    /\bimport\s+['"]([^'"]+)['"]/g,
    /\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
    /\bexport\s+\{[^}]*\}\s*from\s+['"]([^'"]+)['"]/g,
    /\bexport\s+type\s+\{[^}]*\}\s*from\s+['"]([^'"]+)['"]/g,
    /\bexport\s+\*\s+from\s+['"]([^'"]+)['"]/g,
  ]
  for (const re of patterns) {
    let m
    while ((m = re.exec(stripComments)) !== null) specs.push(m[1])
  }
  return [...new Set(specs)]
}

/** 把相对说明符解析成 repo 根相对、以 src/ 打头的路径；不可解析返回 null */
function resolveSpec(importerRel, spec) {
  if (!spec.startsWith('.')) return null // 裸包/alais：本 Pilot 不涉及
  const base = importerRel.startsWith('src/') ? importerRel : `src/${importerRel}`
  const dir = dirname(base)
  let p = normalize(join(dir, spec)).split('\\').join('/')
  p = p.startsWith('src/') ? p : `src/${p.replace(/^(\.\.\/)+/, '')}`
  return p
}

/** 把缺扩展名的解析结果对齐到真实文件键（.ts/.vue/.mjs/.js） */
function canonicalFile(files, resolved) {
  if (!resolved) return null
  if (files.has(resolved)) return resolved
  for (const ext of ['.ts', '.mjs', '.vue', '.js']) {
    if (files.has(resolved + ext)) return resolved + ext
  }
  return null
}

const inPkg = (rel) => typeof rel === 'string' && rel.startsWith(`${PKG_REL}/`)
/** 路径在包内是否算 internal（非 public surface） */
function pkgTargetIsInternal(resolved) {
  if (!inPkg(resolved)) return false
  const base = resolved.slice(PKG_REL.length + 1)
  return !PUBLIC_SURFACE.has(base)
}

// --------------------------------------------------------------------------
// VB-01..VB-10 检查（全部接受可注入 files Map，供 self-test 复用）
// --------------------------------------------------------------------------

function checkVB01_externalInternalImport(files) {
  const v = []
  for (const [importer, code] of files) {
    if (!/\.(ts|vue|mjs)$/.test(importer)) continue
    for (const spec of parseSpecifiers(code)) {
      const resolved = resolveSpec(importer, spec)
      if (!resolved || !inPkg(resolved)) continue
      if (inPkg(importer)) continue // 包内自引用 = 合法
      const base = (resolved.slice(PKG_REL.length + 1) || '').replace(/\.(ts|vue|mjs)$/, '')
      if (!PUBLIC_SURFACE.has(`${base}.ts`) && !PUBLIC_SURFACE.has(base)) {
        v.push(`VB-01 外部不得 import internal：${importer} -> ${resolved}`)
      }
    }
  }
  return v
}

function checkVB02_publicApiExportConsistency(files) {
  const v = []
  const pubPath = `${PKG_REL}/public.ts`
  const pub = files.get(pubPath)
  if (!pub) return [`VB-02 public API 缺失：${pubPath} 不存在`]
  // 收集 public.ts 的 re-export 映射（symbol -> 目标文件）
  const reExports = []
  const re1 = /export\s+(?:type\s+)?\{([^}]+)\}\s*from\s+['"]([^'"]+)['"]/g
  let m
  while ((m = re1.exec(pub)) !== null) {
    const symbols = m[1].split(',').map((s) => s.trim().replace(/^type\s+/, '')).filter(Boolean)
    reExports.push({ symbols, target: resolveSpec(pubPath, m[2]) })
  }
  if (!reExports.length) v.push(`VB-02 public.ts 未导出任何 symbol（空 façade）`)
  for (const { symbols, target } of reExports) {
    const canonical = canonicalFile(files, target)
    if (!canonical) { v.push(`VB-02 public.ts 导出目标不存在：${target}`); continue }
    const targetCode = files.get(canonical)
    for (const sym of symbols) {
      const defined =
        new RegExp(`export\\s+(?:const|function|class|let|async function)\\s+${sym}\\b`).test(targetCode) ||
        new RegExp(`export\\s*\\{[^}]*\\b${sym}\\b`).test(targetCode) ||
        new RegExp(`defineStore\\s*\\(\\s*['"\`]${PKG}['"\`][\\s\\S]*?${sym}`).test(targetCode)
      if (!defined) v.push(`VB-02 public.ts 导出 ${sym} 但在 ${target} 未真实定义`)
    }
  }
  return v
}

function checkVB03_reverseDependency(files) {
  const v = []
  const forbidden = [/^src\/App\.vue$/, /^src\/main\.ts$/, /^src\/components\//]
  for (const [importer, code] of files) {
    if (!inPkg(importer)) continue
    for (const spec of parseSpecifiers(code)) {
      const resolved = resolveSpec(importer, spec)
      if (!resolved) continue
      if (forbidden.some((re) => re.test(resolved))) {
        v.push(`VB-03 反向依赖：${PKG} 包的 ${importer} 依赖 shell ${resolved}`)
      }
    }
  }
  return v
}

function checkVB04_crossCapabilityInternal(files) {
  const v = []
  for (const [importer, code] of files) {
    if (!inPkg(importer)) continue
    for (const spec of parseSpecifiers(code)) {
      const resolved = resolveSpec(importer, spec)
      if (!resolved) continue
      const m = resolved.match(/^src\/capabilities\/([a-z]+)\/(.+)$/)
      if (!m) continue
      const [, other, rest] = m
      if (other === PKG) continue // 自身包内合法
      const isPublicSurface = PUBLIC_SURFACE.has(rest)
      if (!isPublicSurface) v.push(`VB-04 跨能力 internal 直连：${importer} -> ${resolved}`)
    }
  }
  return v
}

function checkVB05_stateOwnerUnique(files) {
  const v = []
  const defs = []
  for (const [p, code] of files) {
    const count = (code.match(new RegExp(`defineStore\\s*\\(\\s*['"\`]${PKG}['"\`]`, 'g')) || []).length
    if (count) defs.push(`${p}×${count}`)
  }
  if (defs.length !== 1) v.push(`VB-05 state owner 不唯一：defineStore('${PKG}') 出现于 ${defs.join(', ') || '（无）'}`)
  const man = files.get(`${PKG_REL}/manifest.ts`)
  if (man) {
    const owner = (man.match(/semanticOwner:\s*["']([^"']+)["']/) || [])[1]
    if (owner && owner !== 'useVaultStore') v.push(`VB-05 manifest.semanticOwner='${owner}' 与真实 store 不一致`)
  } else v.push(`VB-05 manifest 缺失`)
  return v
}

function checkVB06_nativeInvokeLayer(files) {
  const v = []
  for (const [p, code] of files) {
    if (!inPkg(p) || !/ui\/.+\.vue$/.test(p)) continue
    if (/\bbridge\.[a-zA-Z]/.test(code) || /@tauri-apps\/api/.test(code) || /\binvoke\s*\(/.test(code)) {
      v.push(`VB-06 UI 层直连 native：${p} 使用 bridge/invoke（须经 state owner）`)
    }
  }
  return v
}

function checkVB07_uiRegistrationAuthoritative(files) {
  const v = []
  const findings = []
  const idxPath = `${PKG_REL}/index.ts`
  const idx = files.get(idxPath)
  if (!idx) return [`VB-07 index.ts 缺失`]
  // 真实注册块：contributionRegistry.registerContribution({ ... view: "vault" ... })
  const blocks = (idx.match(/registerContribution\s*\(\s*\{[\s\S]*?\}\s*\)/g) || []).filter((b) =>
    /view:\s*["'`]vault["'`]/.test(b),
  )
  // 注意：`export function registerVaultContributions()` 的定义签名本身也匹配调用样式，需扣掉定义那一次
  const liveCalls = (idx.match(/registerVaultContributions\s*\(\s*\)/g) || []).length
  const hasDef = /export\s+function\s+registerVaultContributions\s*\(/.test(idx)
  const executed = Math.max(0, liveCalls - (hasDef ? 1 : 0))
  if (blocks.length === 0) v.push(`VB-07 无 vault UI 注册条目`)
  if (executed !== 1) v.push(`VB-07 活注册调用数=${executed}（应恰好 1）`)
  // 潜在别名：对象方法 registerContributions 也是一条注册路径（本 Pilot 不改语义，诚实记为 FG）
  if (blocks.length > 1) {
    findings.push(
      `VB-07 FG（非阻断/既有架构）：存在 ${blocks.length} 个注册块，其中之一为 def.registerContributions 别名；` +
        `运行时仅 registerVaultContributions() 被调用，故当前无双注册，但具备潜在双注册面。`,
    )
  }
  return [...v, ...findings.map((f) => `FG:${f}`)]
}

function checkVB08_staleOldPath(files, root = ROOT) {
  const v = []
  const staleRel = 'src/utils/vault.mjs'
  if (existsSync(join(root, staleRel))) v.push(`VB-08 stale old path 仍存在：${staleRel}`)
  for (const [p, code] of files) {
    for (const spec of parseSpecifiers(code)) {
      const resolved = resolveSpec(p, spec)
      if (resolved && posix.normalize(resolved).endsWith(staleRel)) {
        v.push(`VB-08 stale old path 被引用：${p} -> ${resolved}`)
      }
    }
  }
  return v
}

function checkVB09_deadShim(files) {
  const v = []
  for (const [p, code] of files) {
    if (inPkg(p)) continue // 包内自身实现不算 shim
    const re = /export\s+(?:const|function|class|async function)\s+(resolveNote|noteLinks|searchNotes)\b/
    const re2 = /export\s*\{[^}]*\b(resolveNote|noteLinks|searchNotes)\b/
    if (re.test(code) || re2.test(code)) {
      v.push(`VB-09 dead shim：${p} 在能力包外再导出已下沉的 symbol`)
    }
  }
  return v
}

function checkVB10_duplicateSemanticImplementation(files) {
  const v = []
  for (const sym of SUNK_SYMBOLS) {
    const sites = []
    for (const [p, code] of files) {
      // 任何形式的 function 定义（含未 export 的私有副本）都算一份实现，避免摄像式复制绕开门禁
      if (new RegExp(`(?:export\\s+)?function\\s+${sym}\\s*\\(`).test(code)) sites.push(p)
    }
    if (sites.length !== 1) {
      v.push(`VB-10 ${sym} 定义处=${sites.length}（${sites.join(', ') || '无'}），应恰好 1 处`)
    }
  }
  return v
}

// --------------------------------------------------------------------------
// 汇总
// --------------------------------------------------------------------------

export function runChecks(files, root = ROOT) {
  const all = [
    ...checkVB01_externalInternalImport(files),
    ...checkVB02_publicApiExportConsistency(files),
    ...checkVB03_reverseDependency(files),
    ...checkVB04_crossCapabilityInternal(files),
    ...checkVB05_stateOwnerUnique(files),
    ...checkVB06_nativeInvokeLayer(files),
    ...checkVB07_uiRegistrationAuthoritative(files),
    ...checkVB08_staleOldPath(files, root),
    ...checkVB09_deadShim(files),
    ...checkVB10_duplicateSemanticImplementation(files),
  ]
  const failures = all.filter((x) => !x.startsWith('FG:'))
  const findings = all.filter((x) => x.startsWith('FG:'))
  return { failures, findings, total: all.length }
}

// --------------------------------------------------------------------------
// self-test：用合成 fixture 证明每条门禁真的能检出（不是摆设）
// --------------------------------------------------------------------------

const BASE = new Map([
  [`${PKG_REL}/public.ts`, 'export { useVaultStore } from "./state/useVaultStore"\n'],
  [
    `${PKG_REL}/state/useVaultStore.ts`,
    'import { defineStore } from "pinia";\nimport { noteLinks } from "../internal/vault.mjs";\n' +
      'export const useVaultStore = defineStore("vault", () => {\n  return {};\n});\n',
  ],
  [
    `${PKG_REL}/internal/vault.mjs`,
    SUNK_SYMBOLS.map((s) => `export function ${s}() {}`).join('\n') + '\n',
  ],
  [`${PKG_REL}/ui/VaultPanel.vue`, '<template><div>vault</div></template>\n'],
  [
    `${PKG_REL}/index.ts`,
    'export function registerVaultContributions() {\n  contributionRegistry.registerContribution({ id: "vault.main", capabilityId: "vault", view: "vault" });\n}\nregisterVaultContributions();\n',
  ],
  [`${PKG_REL}/manifest.ts`, 'export const vaultManifest = { id: "vault", semanticOwner: "useVaultStore" };\n'],
])

function withFixture(patch) {
  const m = new Map(BASE)
  for (const [k, v] of Object.entries(patch)) m.set(k, v)
  return m
}

// 负面样例必须"只"引入目标违约：追加而非替换 store 内容，保持 defineStore 等基线完好，
// 否则会连带触发 VB-05/VB-10 等非预期失败，掩盖本样例真正要证明的检出能力。
const STORE_BASE = BASE.get(`${PKG_REL}/state/useVaultStore.ts`)
function withStoreAppend(extra) {
  return withFixture({ [`${PKG_REL}/state/useVaultStore.ts`]: STORE_BASE + extra })
}

function selfTest() {
  const cases = [
    {
      id: 'ST-01',
      name: 'valid fixture → 零失败',
      files: BASE,
      expectFail: [],
    },
    {
      id: 'ST-02',
      name: '外部 direct import internal → VB-01',
      files: withFixture({
        'src/components/layout/MainArea.vue':
          '<script>import { useVaultStore } from "../../capabilities/vault/state/useVaultStore";</script>\n',
      }),
      expectFail: ['VB-01'],
    },
    {
      id: 'ST-03',
      name: 'capability 反向依赖 App.vue → VB-03',
      files: withStoreAppend('import App from "../../../App.vue";\n'),
      expectFail: ['VB-03'],
    },
    {
      id: 'ST-04',
      name: '跨能力 internal 直连 → VB-04',
      files: withStoreAppend('import { x } from "../../git/state/useGitStore";\n'),
      expectFail: ['VB-04'],
    },
    {
      id: 'ST-05',
      name: '双 state owner → VB-05',
      files: withFixture({
        [`${PKG_REL}/ui/ShadowStore.ts`]: 'export const useShadow = defineStore("vault", () => ({}));\n',
      }),
      expectFail: ['VB-05'],
    },
    {
      id: 'ST-06',
      name: 'UI 层直连 native → VB-06',
      files: withFixture({ [`${PKG_REL}/ui/VaultPanel.vue`]: '<script>bridge.vaultOpen("x");</script>\n' }),
      expectFail: ['VB-06'],
    },
    {
      id: 'ST-07',
      name: 'stale old path 仍被引用 → VB-08',
      files: withStoreAppend('import { noteLinks } from "../../../utils/vault.mjs";\n'),
      expectFail: ['VB-08'],
    },
    {
      id: 'ST-08',
      // 注意：包外 shim 要再导出已下沉 symbol，必然越过 public.ts 直连包内 internal，
      // 因此同一真实缺陷会同时触发 VB-09（shim）与 VB-01（外部越权）。两条规则各自成立，均为预期。
      name: '包外 dead shim 再导出下沉 symbol → VB-09 + VB-01',
      files: withFixture({ 'src/utils/vault.mjs': 'export { noteLinks } from "../capabilities/vault/internal/vault.mjs";\n' }),
      expectFail: ['VB-09', 'VB-01'],
    },
    {
      id: 'ST-09',
      name: '重复语义实现 → VB-10',
      files: withFixture({ 'src/utils/vaultDup.mjs': 'function searchNotes() { /* 未 export 的私有重复实现 */ }\n' }),
      expectFail: ['VB-10'],
    },
    {
      id: 'ST-10',
      name: 'public API 缺失导出 → VB-02',
      files: withFixture({ [`${PKG_REL}/public.ts`]: 'export { useGhostStore } from "./state/useVaultStore";\n' }),
      expectFail: ['VB-02'],
    },
  ]

  let pass = 0
  let fail = 0
  const lines = []
  for (const c of cases) {
    const { failures } = runChecks(c.files, ROOT)
    const missing = c.expectFail.filter((code) => !failures.some((f) => f.startsWith(code)))
    const unexpected = failures.filter((f) => !c.expectFail.some((code) => f.startsWith(code)))
    const ok = missing.length === 0 && (c.expectFail.length === 0 ? failures.length === 0 : unexpected.length === 0)
    if (ok) pass++
    else fail++
    lines.push(
      `${ok ? 'PASS' : 'FAIL'} ${c.id} ${c.name}` +
        (ok ? '' : `\n     缺少检出: ${missing.join(',') || '无'}\n     非预期失败: ${unexpected.join(' | ') || '无'}`),
    )
  }
  return { pass, fail, lines }
}

// --------------------------------------------------------------------------
// CLI
// --------------------------------------------------------------------------

const argv = process.argv.slice(2)
if (argv.includes('--self-test')) {
  const { pass, fail, lines } = selfTest()
  console.log(lines.join('\n'))
  console.log(`\nVAULT_BOUNDARY_SELFTEST_RESULT=${fail === 0 ? 'PASS' : 'FAIL'} (${pass}/${pass + fail})`)
  process.exit(fail === 0 ? 0 : 1)
}

const files = collectReal(ROOT)
const { failures, findings, total } = runChecks(files, ROOT)

if (argv.includes('--json')) {
  console.log(JSON.stringify({ total, failures, findings }, null, 2))
} else {
  for (const f of failures) console.log(`  FAIL ${f}`)
  for (const f of findings) console.log(`  * ${f}`)
  console.log(
    `\nVAULT_BOUNDARY_RESULT=${failures.length === 0 ? 'PASS' : 'FAIL'} ` +
      `(fail=${failures.length}, finding=${findings.length}, files_scanned=${files.size})`,
  )
}
process.exit(failures.length === 0 ? 0 : 1)
