#!/usr/bin/env node
// check-module-documentation.mjs
//
// Phase D3 文档门禁（机器可证明事实，不靠 NLP 猜自然语言正确性）。
//
// 检查项（仅机器可证明）：
//   1. README existence        每个被文档化模块必须有 README.md
//   2. module id               README 必须包含该模块 id（来自 manifest.id）
//   3. manifest path          manifest.ts 存在，且 README 引用之
//   4. public entry           public.ts 存在，且 README 引用之
//   5. source-of-truth links  README 必须有 "Source of Truth" 段，并列出 manifest/public 路径
//   6. referenced paths        README 中列为真源的本地路径（.ts/.vue）必须真实存在
//
// 运行：
//   node scripts/check-module-documentation.mjs            # 检查仓库真实模块
//   node scripts/check-module-documentation.mjs --self-test # 跑 positive/negative fixture 自测
//
// 退出码：全过 0；有失败非 0。

import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const REPO_ROOT = path.resolve(__dirname, "..")
const SRC = path.join(REPO_ROOT, "src")

// Pilot 模块（已跳过，不在本阶段文档化范围）
const PILOT = new Set(["skill", "git", "workspace"])

// 文档化范围：所有含 manifest.ts 的 capability 目录（排除 Pilot）+ settings（SERVICE 候选）
function discoverModules() {
  const capsDir = path.join(SRC, "capabilities")
  const mods = []
  for (const name of fs.readdirSync(capsDir)) {
    const dir = path.join(capsDir, name)
    if (!fs.statSync(dir).isDirectory()) continue
    const manifest = path.join(dir, "manifest.ts")
    if (fs.existsSync(manifest) && !PILOT.has(name)) mods.push({ id: name, dir })
  }
  // settings（SERVICE 候选，单独文档化）
  const settingsDir = path.join(SRC, "settings")
  if (fs.existsSync(path.join(settingsDir, "manifest.ts"))) {
    mods.push({ id: "settings", dir: settingsDir })
  }
  return mods
}

function readManifestId(dir) {
  const p = path.join(dir, "manifest.ts")
  const txt = fs.readFileSync(p, "utf8")
  const m = txt.match(/id:\s*["']([a-z][a-z0-9.]*)["']/)
  return m ? m[1] : null
}

// 从 README 抓取 "Source of Truth" 段之后列出的本地路径（.ts/.vue/.py/.mjs/.rs，仓库根相对）
function extractLocalPaths(md) {
  const lines = md.split(/\r?\n/)
  let inSOT = false
  const paths = []
  for (const line of lines) {
    if (/^#+\s*(?:\d+\.\s*)?Source of Truth/i.test(line)) { inSOT = true; continue }
    if (inSOT && /^#+\s/.test(line)) break // 下一个标题段结束
    if (inSOT) {
      const m = line.match(/`([^`]+\.(?:ts|vue|py|mjs|rs))`/g)
      if (m) for (const x of m) paths.push(x.replace(/[`]/g, ""))
    }
  }
  return [...new Set(paths)]
}

function checkModule({ id, dir }) {
  const errors = []
  const readme = path.join(dir, "README.md")
  if (!fs.existsSync(readme)) { errors.push(`${id}: README.md 缺失`); return errors }

  const md = fs.readFileSync(readme, "utf8")

  // 2. module id
  const manifestId = readManifestId(dir)
  if (!manifestId) errors.push(`${id}: manifest.ts 中无法解析 id`)
  else if (!md.includes(manifestId)) errors.push(`${id}: README 未包含模块 id "${manifestId}"`)

  // 3. manifest path
  const manifestTs = path.join(dir, "manifest.ts")
  if (!fs.existsSync(manifestTs)) errors.push(`${id}: manifest.ts 缺失`)
  else if (!md.includes("manifest.ts")) errors.push(`${id}: README 未引用 manifest.ts`)

  // 4. public entry
  const publicTs = path.join(dir, "public.ts")
  if (!fs.existsSync(publicTs)) errors.push(`${id}: public.ts 缺失`)
  else if (!md.includes("public.ts")) errors.push(`${id}: README 未引用 public.ts`)

  // 5. source-of-truth section
  if (!/^#+\s*(?:\d+\.\s*)?Source of Truth/mi.test(md)) errors.push(`${id}: README 缺少 "Source of Truth" 段`)

  // 6. referenced paths exist（Source of Truth 中的路径以仓库根为基准）
  for (const p of extractLocalPaths(md)) {
    const abs = path.resolve(REPO_ROOT, p)
    if (!abs.startsWith(REPO_ROOT)) continue // 跳过异常/绝对式
    if (!fs.existsSync(abs)) {
      errors.push(`${id}: Source of Truth 引用的本地路径不存在: ${p}`)
    }
  }
  return errors
}

// ---------- fixtures ----------
function positiveFixture() {
  // 真实模块 bookmark 应全部通过
  const dir = path.join(SRC, "capabilities", "bookmark")
  if (!fs.existsSync(dir)) return ["fixture: bookmark 目录缺失（环境不符预期）"]
  const errs = checkModule({ id: "bookmark", dir })
  return errs // 期望空
}

function negativeFixture() {
  // 构造一个缺 README 的假模块目录，断言应报错
  const tmp = fs.mkdtempSync(path.join(REPO_ROOT, ".docgate-neg-"))
  try {
    fs.writeFileSync(path.join(tmp, "manifest.ts"), 'export const x = { id: "negdummy" }')
    fs.writeFileSync(path.join(tmp, "public.ts"), "export {}")
    // 故意缺失 README.md
    const errs = checkModule({ id: "negdummy", dir: tmp })
    const ok = errs.some((e) => e.includes("README.md 缺失"))
    return ok ? [] : ["fixture: 负例未检出缺失 README（门禁逻辑失效）"]
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true })
  }
}

// ---------- runner ----------
function main() {
  const args = process.argv.slice(2)
  if (args.includes("--self-test")) {
    const fErrs = [...positiveFixture(), ...negativeFixture()]
    if (fErrs.length) {
      console.log("SELF_TEST: FAIL")
      for (const e of fErrs) console.log("  - " + e)
      process.exit(1)
    }
    console.log("SELF_TEST: PASS (positive+negative fixtures OK)")
    process.exit(0)
  }

  const mods = discoverModules()
  let totalErrs = []
  for (const m of mods) totalErrs.push(...checkModule(m))

  if (totalErrs.length) {
    console.log(`MODULE_DOCUMENTATION: FAIL (${mods.length} modules checked, ${totalErrs.length} errors)`)
    for (const e of totalErrs) console.log("  - " + e)
    process.exit(1)
  }
  console.log(`MODULE_DOCUMENTATION: PASS (${mods.length} modules documented & verified)`)
  process.exit(0)
}

main()
