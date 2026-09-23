#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-native-capability-boundaries.mjs — Native/Rust 能力边界门禁（STAGE H-C）
//
// 让 Rust/native 不再是治理盲区。真源：
//   - Rust 实际命令：src-tauri/src/**/*.rs 的 #[tauri::command]
//   - 归属声明：docs/architecture/native-boundary/native-commands.yaml（唯一真源）
//
//   NATIVE-01 fail  native command 必须有 owner（非空 / 非 UNKNOWN）
//   NATIVE-02 fail  Capability-owned command 不被无关 Capability internal 直调
//   NATIVE-03 fail  Shell/Core 不直接创建 Capability heavy resource
//   NATIVE-04 fail  native resource 与 capability resource declaration 对齐
//   NATIVE-05 fail  credential/keyring command 不泄露 secret
//   NATIVE-06 fail  Rust 中存在的命令必须已登记（unknown command owner → FAIL）
//   NATIVE-07 fail  registry ↔ Rust 命令漂移（双向）
//
// 用法:
//   node scripts/check-native-capability-boundaries.mjs               真实扫描
//   node scripts/check-native-capability-boundaries.mjs --self-test   自检（positive/negative 夹具）
//   node scripts/check-native-capability-boundaries.mjs --json        JSON 输出
//   node scripts/check-native-capability-boundaries.mjs --help
// ---------------------------------------------------------------------------

import { readFileSync, existsSync, statSync, readdirSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join, posix } from 'node:path'

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..')
const RS_GLOB_ROOT = 'src-tauri/src'
const REGISTRY_REL = 'docs/architecture/native-boundary/native-commands.yaml'

const FRAMEWORK_CATEGORIES = new Set([
  'FRAMEWORK_NATIVE_SERVICE',
  'SHARED_NATIVE_INFRASTRUCTURE',
  'SECURITY_INFRASTRUCTURE',
])
// 重资源：Shell 不得直接创建（NATIVE-03）
const HEAVY_RESOURCES = new Set([
  'WEBVIEW', 'GRID_CHILD', 'PTY', 'CHILD_PROCESS',
  'DB_CONNECTION', 'GIT_PROCESS', 'PLUGIN_RUNTIME',
  'AGENT_EXECUTION', 'TOOL_WEBVIEW', 'WATCHER',
])
// native resource → 期望在 capabilities.yaml 声明的 resource class
const RESOURCE_TO_CLASS = {
  WEBVIEW: ['MEDIUM', 'HEAVY'],
  GRID_CHILD: ['MEDIUM', 'HEAVY'],
  PTY: ['PROCESS', 'MEDIUM', 'HEAVY', 'BACKGROUND'],
  CHILD_PROCESS: ['PROCESS', 'MEDIUM', 'HEAVY', 'BACKGROUND'],
  GIT_PROCESS: ['PROCESS', 'MEDIUM', 'HEAVY', 'BACKGROUND'],
  // 资源法（§18）：Database 连接瞬态建连即弃，故声明为 NETWORK/SECRET 而非驻留 PROCESS
  DB_CONNECTION: ['PROCESS', 'MEDIUM', 'HEAVY', 'NETWORK', 'SECRET'],
  PLUGIN_RUNTIME: ['PROCESS', 'MEDIUM', 'HEAVY', 'BACKGROUND'],
  AGENT_EXECUTION: ['PROCESS', 'MEDIUM', 'HEAVY', 'BACKGROUND'],
  TOOL_WEBVIEW: ['MEDIUM', 'HEAVY'],
  WATCHER: ['LIGHT', 'MEDIUM', 'PROCESS', 'BACKGROUND'],
}
// credential/keyring：禁止流入泄露汇（NATIVE-05）
const SECRET_SINKS = /console\.(log|error|warn|debug|info)|localStorage\.setItem|sessionStorage\.setItem/

function walk(dir, exts, out = []) {
  if (!existsSync(dir)) return out
  for (const e of readdirSync(dir)) {
    const p = join(dir, e)
    const st = statSync(p)
    if (st.isDirectory()) walk(p, exts, out)
    else if (exts.some((x) => p.endsWith(x))) out.push(p)
  }
  return out
}

const rel = (p) => posix.normalize(p.split(ROOT + '/')[1] || p)

/** 扫描 Rust 中的 #[tauri::command] 函数名 */
function scanRustCommands() {
  const out = []
  for (const p of walk(join(ROOT, RS_GLOB_ROOT), ['.rs'])) {
    const src = readFileSync(p, 'utf8')
    const re = /#\[tauri::command\][\s\S]{0,250}?fn\s+([a-z_0-9]+)\s*\(/g
    let m
    while ((m = re.exec(src)) !== null) out.push({ cmd: m[1], file: rel(p) })
  }
  return out
}

/**
 * 极简 YAML 解析（针对 native-commands.yaml 的固定形状：
 * 两/四空格缩进的嵌套 map，标量为引号包裹）。
 */
function parseRegistry(text) {
  const cmds = {}
  let cur = null
  for (const raw of text.split(/\r?\n/)) {
    if (!raw.trim() || raw.trim().startsWith('#')) continue
    const mCmd = raw.match(/^ {2}([a-z_0-9]+):\s*$/)
    if (mCmd) {
      cur = mCmd[1]
      cmds[cur] = {}
      continue
    }
    const mField = raw.match(/^ {4}([a-zA-Z_]+):\s*(.*)$/)
    if (mField && cur) {
      let v = mField[2].trim()
      if (v.startsWith('"') && v.endsWith('"')) v = v.slice(1, -1)
      cmds[cur][mField[1]] = v
    }
  }
  return cmds
}

const camel = (s) => s.split('_').map((x, i) => (i ? x.charAt(0).toUpperCase() + x.slice(1) : x)).join('')

/** 扫描前端调用点：命令 → 调用方集合（capability id / SHELL / SHARED / FRAMEWORK_CORE） */
function scanCallers(cmds) {
  const files = walk(join(ROOT, 'src'), ['.ts', '.vue'])
  const callers = {}
  for (const { cmd } of cmds) callers[cmd] = new Set()
  for (const p of files) {
    const rp = rel(p)
    let src
    try {
      src = readFileSync(p, 'utf8')
    } catch {
      continue
    }
    // 归属判定
    let who = 'SHARED'
    const capMatch = rp.match(/^src\/capabilities\/([a-z_0-9]+)\//)
    if (capMatch) who = capMatch[1]
    else if (/^src\/(App\.vue|main\.ts|shell\/)/.test(rp)) who = 'SHELL'
    else if (/^src\/components\/layout\//.test(rp)) who = 'SHELL'
    else if (/^src\/(stores|composables|utils|shared)\//.test(rp)) who = 'FRAMEWORK_CORE'
    // 只把 `bridge.<cmd>(` 计为「直连 native」。
    // 能力 public store 的同名动作（如 browser.tabNew()）是经能力 public 边界的合法调用，
    // 不是绕过边界 —— 计入会造成误报（这是准确性修正，不是放宽门禁）。
    for (const { cmd } of cmds) {
      const re = new RegExp('\\bbridge\\.' + camel(cmd) + '\\s*\\(')
      if (re.test(src)) callers[cmd].add(who)
    }
  }
  return callers
}

/** 极简解析 capabilities.yaml 的 id / resources.class（用于 NATIVE-04） */
function parseCapabilityResources(text) {
  const out = {}
  let cur = null
  let inRes = false
  for (const raw of text.split(/\r?\n/)) {
    const mId = raw.match(/^ {2}- id:\s*([a-zA-Z_0-9]+)\s*$/)
    if (mId) {
      cur = mId[1]
      out[cur] = []
      inRes = false
      continue
    }
    if (cur && /^ {4}resources:\s*$/.test(raw)) {
      inRes = true
      continue
    }
    if (inRes && cur) {
      const mClass = raw.match(/^ {6}class:\s*\[(.*)\]\s*$/)
      if (mClass) {
        out[cur] = mClass[1].split(',').map((s) => s.trim()).filter(Boolean)
        inRes = false
      }
    }
  }
  return out
}

function run(files) {
  const findings = []
  const push = (code, severity, detail, file) =>
    findings.push({ code, severity, detail, file: file || '(global)' })

  const regPath = join(ROOT, REGISTRY_REL)
  if (!files.registryText) {
    push('NATIVE-00', 'fail', `缺少 native 归属真源 ${REGISTRY_REL}`, REGISTRY_REL)
    return findings
  }
  const reg = parseRegistry(files.registryText)
  const rust = files.rust || []

  // NATIVE-01 / NATIVE-06 / NATIVE-07
  for (const { cmd, file } of rust) {
    const e = reg[cmd]
    if (!e) {
      push('NATIVE-06', 'fail', `Rust 命令未登记归属（unknown owner）：${cmd}`, file)
      push('NATIVE-07', 'fail', `registry ↔ Rust 漂移：Rust 有而 registry 无 — ${cmd}`, REGISTRY_REL)
      continue
    }
    const owner = (e.owner || '').trim()
    if (!owner || owner === 'UNKNOWN' || owner === 'LEGACY_UNOWNED') {
      push('NATIVE-01', 'fail', `命令 ${cmd} 缺 owner / owner=UNKNOWN`, REGISTRY_REL)
    }
  }
  for (const cmd of Object.keys(reg)) {
    if (!rust.some((r) => r.cmd === cmd)) {
      push('NATIVE-07', 'fail', `registry ↔ Rust 漂移：registry 有而 Rust 无 — ${cmd}`, REGISTRY_REL)
    }
  }

  const callers = files.callers || {}

  // NATIVE-02 / NATIVE-03
  for (const { cmd, file } of rust) {
    const e = reg[cmd]
    if (!e) continue
    const owner = (e.owner || '').trim()
    const cat = (e.category || '').trim()
    const res = (e.resource || '').trim()
    const cs = [...(callers[cmd] || [])]
    if (FRAMEWORK_CATEGORIES.has(cat)) continue // 框架/安全基础设施允许被能力调用
    // 显式登记的合法跨域调用者（须在 registry 中带理由；未登记一律 FAIL）
    const allowed = (e.allowed_callers || '')
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
    if (allowed.length && !(e.allowed_rationale || '').trim()) {
      push('NATIVE-02', 'fail', `命令 ${cmd} 声明了 allowed_callers 但缺 allowed_rationale`, REGISTRY_REL)
    }
    for (const c of cs) {
      if (c === owner || c === 'SHARED' || c === 'FRAMEWORK_CORE') continue
      if (allowed.includes(c)) continue
      if (c === 'SHELL') {
        // NATIVE-03：Shell 不得直接创建能力重资源
        if (HEAVY_RESOURCES.has(res)) {
          push('NATIVE-03', 'fail', `Shell 直接创建能力重资源：${cmd}（owner=${owner}, resource=${res}）`, file)
        }
        continue
      }
      push('NATIVE-02', 'fail', `能力 ${c} 直调非本能力 native 命令 ${cmd}（owner=${owner}）`, file)
    }
  }

  // NATIVE-04：native resource 与 capability resource declaration 对齐
  if (files.capabilityText) {
    const caps = parseCapabilityResources(files.capabilityText)
    for (const { cmd, file } of rust) {
      const e = reg[cmd]
      if (!e) continue
      const owner = (e.owner || '').trim()
      const res = (e.resource || '').trim()
      const expect = RESOURCE_TO_CLASS[res]
      if (!expect) continue
      const declared = caps[owner]
      if (!declared) {
        push('NATIVE-04', 'fail', `命令 ${cmd} 的 owner ${owner} 在 capabilities.yaml 无 resources.class 声明`, REGISTRY_REL)
        continue
      }
      if (!declared.some((d) => expect.includes(d))) {
        push('NATIVE-04', 'fail', `命令 ${cmd}（resource=${res}）与能力 ${owner} 声明的 resources.class=[${declared.join(',')}] 不匹配`, REGISTRY_REL)
      }
    }
  }

  // NATIVE-05：credential/keyring 命令结果不得流入泄露汇
  for (const { cmd } of rust) {
    const e = reg[cmd]
    if (!e) continue
    const owner = (e.owner || '').trim()
    const res = (e.resource || '').trim()
    if (owner !== 'credential' && res !== 'KEYRING') continue
    for (const f of files.srcFiles || []) {
      const re = new RegExp('\\.' + camel(cmd) + '\\s*\\(')
      if (!re.test(f.content)) continue
      // 取该调用点附近 6 行，检查是否流入泄露汇
      const lines = f.content.split(/\r?\n/)
      lines.forEach((line, i) => {
        if (!re.test(line)) return
        const window = lines.slice(Math.max(0, i - 2), i + 7).join('\n')
        if (SECRET_SINKS.test(window)) {
          push('NATIVE-05', 'fail', `credential 命令 ${cmd} 的结果流入泄露汇（console/localStorage）`, f.path)
        }
      })
    }
  }

  return findings
}

// ---------------------------- 真实扫描 ----------------------------
function loadReal() {
  const rust = scanRustCommands()
  const registryText = existsSync(join(ROOT, REGISTRY_REL))
    ? readFileSync(join(ROOT, REGISTRY_REL), 'utf8')
    : null
  const srcFiles = walk(join(ROOT, 'src'), ['.ts', '.vue']).map((p) => ({
    path: rel(p),
    content: readFileSync(p, 'utf8'),
  }))
  const capabilityText = existsSync(join(ROOT, 'docs/architecture/capability-registry/capabilities.yaml'))
    ? readFileSync(join(ROOT, 'docs/architecture/capability-registry/capabilities.yaml'), 'utf8')
    : null
  return { rust, registryText, srcFiles, callers: scanCallers(rust), capabilityText }
}

// ---------------------------- fixtures / self-test ----------------------------
function fxRegistry(cmds) {
  let s = 'version: 1\ncommands:\n'
  for (const [name, owner, cat, res] of cmds) {
    s += `  ${name}:\n    file: "src-tauri/src/bridge.rs"\n    owner: "${owner}"\n    category: "${cat}"\n`
    s += `    callers: "-"\n    side_effect: "-"\n    resource: "${res}"\n    permission: "-"\n`
    s += `    current_boundary: "-"\n    target_boundary: "-"\n`
  }
  return s
}

function runSelfTest() {
  let bad = 0
  const t = (name, expectCodes, files) => {
    const got = run(files).map((f) => f.code)
    const uniqGot = [...new Set(got)]
    const ok = expectCodes.every((c) => uniqGot.includes(c)) &&
      (expectCodes.length === 0 ? uniqGot.length === 0 : true)
    if (ok) console.log(`  ✓ ${name}`)
    else {
      bad++
      console.log(`  ✗ ${name} — expect [${expectCodes}] got [${uniqGot}]`)
    }
  }

  // POSITIVE：正常登记 + 本能力调用 + 资源声明匹配 + 无泄露
  t('POSITIVE: 已登记且本能力调用', [], {
    rust: [{ cmd: 'term_spawn', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([['term_spawn', 'terminal', 'CAPABILITY_NATIVE_ADAPTER', 'PTY']]),
    callers: { term_spawn: new Set(['terminal']) },
    capabilityText: '  - id: terminal\n    resources:\n      class: [PROCESS]\n',
    srcFiles: [{ path: 'src/capabilities/terminal/x.ts', content: 'await bridge.termSpawn(1)' }],
  })

  // NATIVE-01：owner 缺失
  t('NATIVE-01: 缺 owner', ['NATIVE-01'], {
    rust: [{ cmd: 'orphan_cmd', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([['orphan_cmd', 'UNKNOWN', 'LEGACY_UNOWNED', 'NONE']]),
    callers: {}, capabilityText: '', srcFiles: [],
  })

  // NATIVE-02：跨能力直调
  t('NATIVE-02: 无关能力直调', ['NATIVE-02'], {
    rust: [{ cmd: 'term_spawn', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([['term_spawn', 'terminal', 'CAPABILITY_NATIVE_ADAPTER', 'PTY']]),
    callers: { term_spawn: new Set(['git']) },
    capabilityText: '  - id: terminal\n    resources:\n      class: [PROCESS]\n',
    srcFiles: [],
  })

  // NATIVE-03：Shell 直接创建重资源
  t('NATIVE-03: Shell 直接创建重资源', ['NATIVE-03'], {
    rust: [{ cmd: 'create_grid', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([['create_grid', 'browser', 'CAPABILITY_NATIVE_ADAPTER', 'GRID_CHILD']]),
    callers: { create_grid: new Set(['SHELL']) },
    capabilityText: '  - id: browser\n    resources:\n      class: [MEDIUM]\n',
    srcFiles: [],
  })

  // NATIVE-04：资源声明不匹配
  t('NATIVE-04: 资源声明不匹配', ['NATIVE-04'], {
    rust: [{ cmd: 'db_connect', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([['db_connect', 'database', 'CAPABILITY_NATIVE_ADAPTER', 'DB_CONNECTION']]),
    callers: { db_connect: new Set(['database']) },
    capabilityText: '  - id: database\n    resources:\n      class: [LIGHT]\n',
    srcFiles: [],
  })

  // NATIVE-05：credential 结果泄露
  t('NATIVE-05: credential 流入泄露汇', ['NATIVE-05'], {
    rust: [{ cmd: 'list_browser_credentials', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([['list_browser_credentials', 'credential', 'SECURITY_INFRASTRUCTURE', 'KEYRING']]),
    callers: {},
    capabilityText: '',
    srcFiles: [{ path: 'src/x.ts', content: 'const c = await bridge.listBrowserCredentials()\nconsole.log(c)' }],
  })

  // NATIVE-06/07：Rust 有 registry 无
  t('NATIVE-06/07: 未登记命令', ['NATIVE-06', 'NATIVE-07'], {
    rust: [{ cmd: 'ghost_cmd', file: 'src-tauri/src/bridge.rs' }],
    registryText: fxRegistry([]),
    callers: {}, capabilityText: '', srcFiles: [],
  })

  // NATIVE-07：registry 有 Rust 无
  t('NATIVE-07: registry 冗余条目', ['NATIVE-07'], {
    rust: [],
    registryText: fxRegistry([['stale_cmd', 'terminal', 'CAPABILITY_NATIVE_ADAPTER', 'PTY']]),
    callers: {}, capabilityText: '', srcFiles: [],
  })

  console.log(bad === 0 ? '\nNATIVE_BOUNDARY_SELF_TEST=ALL_PASS' : `\nNATIVE_BOUNDARY_SELF_TEST=FAIL (${bad})`)
  return bad
}

// ---------------------------- CLI ----------------------------
function printHelp() {
  console.log(`check-native-capability-boundaries.mjs — Native/Rust 能力边界门禁（STAGE H-C）

用法:
  node scripts/check-native-capability-boundaries.mjs               真实扫描
  node scripts/check-native-capability-boundaries.mjs --self-test   自检
  node scripts/check-native-capability-boundaries.mjs --json        JSON 输出
  node scripts/check-native-capability-boundaries.mjs --help

规则:
  NATIVE-01 native command 有 owner
  NATIVE-02 Capability-owned command 不被无关 Capability internal 直调
  NATIVE-03 Shell/Core 不直接创建 Capability heavy resource
  NATIVE-04 native resource 与 capability resource declaration 对齐
  NATIVE-05 credential/keyring command 不泄露 secret
  NATIVE-06 Rust 命令必须已登记（unknown owner → FAIL）
  NATIVE-07 registry ↔ Rust 命令漂移（双向）

真源: ${REGISTRY_REL} + ${RS_GLOB_ROOT}/**/*.rs`)
}

const args = process.argv.slice(2)
if (args.includes('--help')) {
  printHelp()
  process.exit(0)
}
if (args.includes('--self-test')) {
  process.exit(runSelfTest() === 0 ? 0 : 1)
}

const files = loadReal()
const findings = run(files)
const fail = findings.filter((f) => f.severity === 'fail')
const warn = findings.filter((f) => f.severity === 'warn')

if (args.includes('--json')) {
  console.log(JSON.stringify({ total: files.rust.length, findings }, null, 1))
} else {
  console.log(`\n== NATIVE CAPABILITY BOUNDARY ==\ncommands=${files.rust.length}`)
  for (const f of findings) {
    console.log(`  [${f.severity.toUpperCase().padEnd(4)}] ${f.code} ${f.detail}  (${f.file})`)
  }
  console.log(`\nfail=${fail.length} warn=${warn.length}`)
  console.log(`NATIVE_CAPABILITY_BOUNDARY_RESULT=${fail.length ? 'FAIL' : 'PASS'}`)
}
process.exit(fail.length ? 1 : 0)
