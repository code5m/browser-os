#!/usr/bin/env node
// check-ui.mjs — UI 规范检查器（Phase 03 Batch A）
//
// 检查 HTML 浮层不覆盖原生浏览器 WebView 区域：
//   - toast / toast-pop / Overlay / Modal / Popover / Tooltip
//   - position: fixed
//   - 非允许区域 z-index
//   - HTML 浮层覆盖 Native WebView
//
// 规则来源：PROJECT-RULES.md 规则 3.8 / §5、01-Detailed-Design.md Error 节、
//           phase-03-checker-specification.md UI Checker 节。
//
// CLI:
//   node scripts/check-ui.mjs              默认（文本输出）
//   node scripts/check-ui.mjs --help       帮助
//   node scripts/check-ui.mjs --json       JSON 输出
//   node scripts/check-ui.mjs --strict     WARN 升级为 FAIL
//   node scripts/check-ui.mjs --self-test  自测
//
// 退出码：0 = PASS；1 = FAIL；2 = USAGE_ERROR

import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join, relative, extname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../", import.meta.url));
const CHECK_NAME = "UI_CHECK";
const SRC_DIR = join(ROOT, "src");

// ===== 已批准的 position:fixed 允许列表 =====
// 每条记录代表一个经源码验证的已批准 fixed 浮层。
// 新增 fixed 浮层必须经 Chief Architect 批准后才能加入此表。
const ALLOWED_FIXED = [
  { file: "src/App.vue", selector: ".boot-overlay", reason: "启动遮罩，应用就绪前显示，此时无 WebView" },
  { file: "src/App.vue", selector: ".shell-error", reason: "外壳错误兜底，子树渲染失败时显示，此时无 WebView" },
  { file: "src/components/layout/StatusBar.vue", selector: ".res-detail", reason: "状态栏区域浮层（bottom:32px），不在浏览器视口上" },
  { file: "src/styles/global.css", selector: ".modal-mask", reason: "通用 modal 遮罩（ConfirmModal/GitWriteConfirmDialog/ImageLightbox）" },
  { file: "src/styles/global.css", selector: ".ctx-menu", reason: "右键菜单" },
  { file: "src/styles/global.css", selector: ".home-modal-mask", reason: "主页编辑弹窗" },
  { file: "src/shared/ui/PermissionPreviewModal.vue", selector: ".modal-mask", reason: "权限预览 modal" },
  { file: "src/components/workspace/RunHistoryModal.vue", selector: ".modal-mask", reason: "运行历史 modal" },
  { file: "src/capabilities/workspace/ui/ScriptRunDialog.vue", selector: ".run-mask", reason: "脚本运行对话框" },
  { file: "src/capabilities/home/ui/HomeShortcutEditor.vue", selector: ".hs-mask", reason: "主页快捷键编辑器" },
  { file: "src/capabilities/plugin/ui/PluginManager.vue", selector: ".pm-modal", reason: "插件管理 modal" },
];

// ===== 已批准的高 z-index（>=1000）允许列表 =====
const ALLOWED_HIGH_ZINDEX = [
  { file: "src/App.vue", selector: ".boot-overlay", reason: "启动遮罩，无 WebView 时显示" },
  { file: "src/App.vue", selector: ".shell-error", reason: "外壳错误兜底，无 WebView 时显示" },
  { file: "src/components/layout/StatusBar.vue", selector: ".res-detail", reason: "状态栏区域浮层，不在浏览器视口上" },
];

const HIGH_ZINDEX_THRESHOLD = 1000;

// ===== 工具函数 =====

function relPath(absPath) {
  return relative(ROOT, absPath).replace(/\\/g, "/");
}

function stripCommentsPreserveLines(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "));
}

function listFilesRecursive(dir, exts, acc = []) {
  if (!existsSync(dir)) return acc;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      listFilesRecursive(full, exts, acc);
    } else if (exts.includes(extname(entry.name))) {
      acc.push(full);
    }
  }
  return acc;
}

// 从 CSS 内容中提取选择器列表（处理逗号分隔）
function extractSelectors(selectorText) {
  return selectorText
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

// 检测选择器是否匹配允许列表条目
function selectorMatches(selector, allowedSelector) {
  if (selector === allowedSelector) return true;
  if (selector.includes(allowedSelector)) return true;
  return false;
}

// 在允许列表中查找匹配条目
function findAllowedEntry(rel, selector, allowList) {
  return allowList.find(
    (e) => e.file === rel && extractSelectors(e.selector).some((as) => selectorMatches(selector, as))
  );
}

// ===== CSS 扫描 =====

// 扫描 CSS 内容，检测 position:fixed 和高 z-index
// baseLine: CSS 内容在原文件中的起始行号（用于 Vue SFC <style> 块）
function scanCssContent(css, rel, baseLine = 1) {
  const findings = [];
  const stripped = stripCommentsPreserveLines(css);
  const ruleRe = /([^{}]+)\{([^{}]*)\}/g;
  let m;
  while ((m = ruleRe.exec(stripped)) !== null) {
    const selectorText = m[1].trim();
    const body = m[2];
    const bodyStartOffset = m.index + m[1].length + 1;

    const selectors = extractSelectors(selectorText);
    if (selectors.length === 0) continue;

    // 检测 position: fixed
    const posRe = /position:\s*fixed/gi;
    let posMatch;
    while ((posMatch = posRe.exec(body)) !== null) {
      const charOffset = bodyStartOffset + posMatch.index;
      const line = baseLine + stripped.substring(0, charOffset).split("\n").length - 1;
      for (const sel of selectors) {
        findings.push({ kind: "fixed", selector: sel, line, file: rel });
      }
    }

    // 检测 z-index >= 阈值
    const zRe = /z-index:\s*(\d+)/gi;
    let zMatch;
    while ((zMatch = zRe.exec(body)) !== null) {
      const value = parseInt(zMatch[1], 10);
      if (value < HIGH_ZINDEX_THRESHOLD) continue;
      const charOffset = bodyStartOffset + zMatch.index;
      const line = baseLine + stripped.substring(0, charOffset).split("\n").length - 1;
      for (const sel of selectors) {
        findings.push({ kind: "zindex", selector: sel, line, file: rel, value });
      }
    }
  }
  return findings;
}

// 扫描 Vue SFC：提取 <style> 块 + 检测 DEPRECATED 模式
function scanVueFile(content, rel) {
  const findings = [];

  // 提取 <style> 块并扫描 CSS
  const styleRe = /<style[^>]*>([\s\S]*?)<\/style>/g;
  let sm;
  while ((sm = styleRe.exec(content)) !== null) {
    const styleContent = sm[1];
    const styleStartLine = content.substring(0, sm.index).split("\n").length + 1;
    findings.push(...scanCssContent(styleContent, rel, styleStartLine));
  }

  // 逐行检测 DEPRECATED 模式
  const lines = content.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNo = i + 1;

    // toast-pop（DEPRECATED，PROJECT-RULES 规则 3.8）
    if (/toast-pop/.test(line)) {
      findings.push({ kind: "deprecated-toast-pop", line: lineNo, file: rel });
    }

    // "已新建页签" Toast（DEPRECATED，PROJECT-RULES [DEPRECATED]）
    if (/已新建页签/.test(line) && /toast|Toast|showToast/i.test(line)) {
      findings.push({ kind: "deprecated-new-tab-toast", line: lineNo, file: rel });
    }
  }

  return findings;
}

// 扫描独立 CSS 文件
function scanCssFile(content, rel) {
  const findings = scanCssContent(content, rel, 1);
  const lines = content.split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (/toast-pop/.test(lines[i])) {
      findings.push({ kind: "deprecated-toast-pop", line: i + 1, file: rel });
    }
  }
  return findings;
}

// ===== 结构检查 =====

function structuralChecks(findings, warnings) {
  const statusBarPath = join(SRC_DIR, "components/layout/StatusBar.vue");
  const mainAreaPath = join(SRC_DIR, "components/layout/MainArea.vue");
  const activityBarPath = join(SRC_DIR, "components/layout/ActivityBar.vue");
  const contributionRegistryPath = join(SRC_DIR, "capability/contribution/registry.ts");
  const globalCssPath = join(SRC_DIR, "styles/global.css");

  // 1. StatusBar.vue 必须使用状态栏模式
  if (existsSync(statusBarPath)) {
    const sb = readFileSync(statusBarPath, "utf8");
    if (!/<footer[^>]*class="status"/.test(sb)) {
      findings.push({
        severity: "P1",
        file: "src/components/layout/StatusBar.vue",
        line: 0,
        message: "StatusBar 必须使用 <footer class=\"status\"> 状态栏模式",
        rule: "PROJECT-RULES.md 规则 3.8 / [APPROVED] 错误和状态只显示在状态栏",
      });
    }
    if (!/<span\s+v-if="layout\.msg"[^>]*class="msg"/.test(sb)) {
      findings.push({
        severity: "P1",
        file: "src/components/layout/StatusBar.vue",
        line: 0,
        message: "StatusBar 必须保留 <span v-if=\"layout.msg\" class=\"msg\"> 状态栏消息",
        rule: "PROJECT-RULES.md 规则 3.8",
      });
    }
    if (!/contributionRegistry\.getSurfaceContributions\(CONTRIBUTION_SLOTS\.WORKBENCH_MAIN\)/.test(sb)) {
      findings.push({
        severity: "P1",
        file: "src/components/layout/StatusBar.vue",
        line: 0,
        message: "StatusBar 的视图名称必须包含 WORKBENCH_MAIN 贡献，禁止仅靠硬编码视图表判定未知视图",
        rule: "Contribution/Slot 可插拔视图契约",
      });
    }
  }

  // 2. MainArea 未知视图兜底必须以贡献解析结果为准，避免新增贡献同时显示“当前视图不可用”。
  if (existsSync(mainAreaPath)) {
    const mainArea = readFileSync(mainAreaPath, "utf8");
    if (!/!viewOf\(layout\.mainView\)/.test(mainArea)) {
      findings.push({
        severity: "P1",
        file: "src/components/layout/MainArea.vue",
        line: 0,
        message: "MainArea 未知视图兜底必须检查 viewOf(layout.mainView) 的贡献解析结果",
        rule: "Contribution/Slot 可插拔视图契约",
      });
    }
    if (!/const\s+workbenchMainContributions\s*=\s*computed\(/.test(mainArea)) {
      findings.push({
        severity: "P1",
        file: "src/components/layout/MainArea.vue",
        line: 0,
        message: "MainArea 必须响应 Contribution Registry 运行时增删",
        rule: "Capability 停用后贡献必须即时从 UI 消失",
      });
    }
  }

  if (existsSync(activityBarPath)) {
    const activityBar = readFileSync(activityBarPath, "utf8");
    if (!/const\s+addressBarActions\s*=\s*computed\(/.test(activityBar)
      || !/const\s+trailingActions\s*=\s*computed\(/.test(activityBar)) {
      findings.push({
        severity: "P1",
        file: "src/components/layout/ActivityBar.vue",
        line: 0,
        message: "ActivityBar 的能力入口必须响应运行时增删",
        rule: "Capability 停用后不得残留死入口",
      });
    }
  }

  if (existsSync(contributionRegistryPath)) {
    const registry = readFileSync(contributionRegistryPath, "utf8");
    if (!/shallowReactive\(new Map/.test(registry)) {
      findings.push({
        severity: "P1",
        file: "src/capability/contribution/registry.ts",
        line: 0,
        message: "Contribution Registry 必须保持 Vue 可观察性",
        rule: "Capability 贡献运行时增删契约",
      });
    }
  }

  // 3. global.css 的 html, body, #app 必须 overflow: hidden
  if (existsSync(globalCssPath)) {
    const css = readFileSync(globalCssPath, "utf8");
    if (!/html,\s*body,\s*#app\s*\{[^}]*overflow:\s*hidden/.test(css)) {
      findings.push({
        severity: "P1",
        file: "src/styles/global.css",
        line: 0,
        message: "html, body, #app 必须保留 overflow:hidden（滚动只属于内容区或网页）",
        rule: "PROJECT-RULES.md 规则 3.8",
      });
    }
  }
}

// ===== 主检查逻辑 =====

function runCheck() {
  const findings = [];
  const warnings = [];

  const files = listFilesRecursive(SRC_DIR, [".vue", ".css"]);
  let filesScanned = 0;

  for (const filePath of files) {
    const rel = relPath(filePath);
    const content = readFileSync(filePath, "utf8");
    filesScanned++;

    let rawFindings;
    if (extname(filePath) === ".vue") {
      rawFindings = scanVueFile(content, rel);
    } else {
      rawFindings = scanCssFile(content, rel);
    }

    for (const f of rawFindings) {
      if (f.kind === "deprecated-toast-pop") {
        findings.push({
          severity: "P1",
          file: f.file,
          line: f.line,
          message: "toast-pop 已废弃（DEPRECATED），禁止恢复覆盖浏览器区域的固定浮层",
          rule: "PROJECT-RULES.md 规则 3.8 / [DEPRECATED]",
        });
      } else if (f.kind === "deprecated-new-tab-toast") {
        findings.push({
          severity: "P1",
          file: f.file,
          line: f.line,
          message: "\"已新建页签\" Toast 已废弃（DEPRECATED），成功操作不得弹浮窗",
          rule: "PROJECT-RULES.md [DEPRECATED] / [APPROVED] 成功操作不显示 Toast",
        });
      } else if (f.kind === "fixed") {
        const allowed = findAllowedEntry(f.file, f.selector, ALLOWED_FIXED);
        if (!allowed) {
          findings.push({
            severity: "P2",
            file: f.file,
            line: f.line,
            message: `新的 position:fixed 浮层 "${f.selector}" 不在已批准允许列表中；HTML fixed 浮层无法覆盖原生 WebView`,
            rule: "PROJECT-RULES.md 规则 3.8 / [APPROVED] HTML 浮层禁止覆盖原生浏览器 WebView 区域",
          });
        }
      } else if (f.kind === "zindex") {
        const allowed = findAllowedEntry(f.file, f.selector, ALLOWED_HIGH_ZINDEX);
        if (!allowed) {
          findings.push({
            severity: "P2",
            file: f.file,
            line: f.line,
            message: `高 z-index:${f.value} 选择器 "${f.selector}" 不在已批准允许列表中；不得用高 z-index 覆盖原生 WebView`,
            rule: "PROJECT-RULES.md [APPROVED] 原生 WebView 的层级问题不能使用无限增大 z-index 解决",
          });
        }
      }
    }
  }

  structuralChecks(findings, warnings);

  return { findings, warnings, summary: { filesScanned } };
}

// ===== 自测 =====

function runSelfTest() {
  const failures = [];

  // 测试 1: scanCssContent 检测 position:fixed
  const css1 = ".test-fixed { position: fixed; inset: 0; z-index: 500; }";
  const r1 = scanCssContent(css1, "test.vue");
  if (!r1.some((f) => f.kind === "fixed" && f.selector === ".test-fixed")) {
    failures.push("scanCssContent 未能检测 position:fixed");
  }

  // 测试 2: scanCssContent 检测高 z-index
  const css2 = ".test-high-z { position: absolute; z-index: 9999; }";
  const r2 = scanCssContent(css2, "test.vue");
  if (!r2.some((f) => f.kind === "zindex" && f.value === 9999)) {
    failures.push("scanCssContent 未能检测高 z-index");
  }

  // 测试 3: 低 z-index 不被检测
  const css3 = ".test-low-z { z-index: 50; }";
  const r3 = scanCssContent(css3, "test.vue");
  if (r3.some((f) => f.kind === "zindex")) {
    failures.push("scanCssContent 不应检测低 z-index");
  }

  // 测试 4: toast-pop 检测
  const vue4 = "<template><div class=\"toast-pop\">x</div></template>";
  const r4 = scanVueFile(vue4, "test.vue");
  if (!r4.some((f) => f.kind === "deprecated-toast-pop")) {
    failures.push("scanVueFile 未能检测 toast-pop");
  }

  // 测试 5: "已新建页签" toast 检测
  const vue5 = "<script>layout.showToast('已新建页签');</script>";
  const r5 = scanVueFile(vue5, "test.vue");
  if (!r5.some((f) => f.kind === "deprecated-new-tab-toast")) {
    failures.push("scanVueFile 未能检测 \"已新建页签\" toast");
  }

  // 测试 6: 允许列表匹配
  const allowed = findAllowedEntry("src/App.vue", ".boot-overlay", ALLOWED_FIXED);
  if (!allowed) {
    failures.push("findAllowedEntry 未能匹配允许列表中的条目");
  }

  // 测试 7: 允许列表不匹配未知条目
  const notAllowed = findAllowedEntry("src/App.vue", ".unknown-selector", ALLOWED_FIXED);
  if (notAllowed) {
    failures.push("findAllowedEntry 不应匹配未知条目");
  }

  // 测试 8: 注释移除保持行号
  const css8 = "/* line 1 */\n.test { position: fixed; }";
  const r8 = scanCssContent(css8, "test.css");
  if (!r8.some((f) => f.kind === "fixed" && f.line === 2)) {
    failures.push(`stripCommentsPreserveLines 行号不正确: ${JSON.stringify(r8)}`);
  }

  return failures;
}

// ===== 输出 =====

function printHelp() {
  console.log(`check-ui.mjs — UI 规范检查器（Phase 03）

用法:
  node scripts/check-ui.mjs              默认（文本输出）
  node scripts/check-ui.mjs --help       显示本帮助
  node scripts/check-ui.mjs --json       JSON 输出
  node scripts/check-ui.mjs --strict     WARN 升级为 FAIL
  node scripts/check-ui.mjs --self-test  自测

检查项:
  toast-pop               DEPRECATED 固定浮层（P1）
  "已新建页签" Toast       DEPRECATED 成功提示浮窗（P1）
  position: fixed         新的 fixed 浮层覆盖浏览器区域（P2）
  高 z-index (>=1000)     新的高 z-index 覆盖原生 WebView（P2）
  StatusBar 结构          状态栏模式不变量（P1）
  global.css 结构         html/body/#app overflow:hidden 不变量（P1）

退出码: 0 = PASS；1 = FAIL；2 = USAGE_ERROR`);
}

function printText(result, strict) {
  const blockingFindings = result.findings.filter(
    (f) => f.severity === "P1" || f.severity === "P2"
  );
  const allWarnings = result.warnings;
  const effectiveWarnings = strict ? [] : allWarnings;
  const strictUpgraded = strict ? allWarnings : [];

  const totalBlocking = blockingFindings.length + strictUpgraded.length;

  if (totalBlocking === 0 && effectiveWarnings.length === 0) {
    console.log(`${CHECK_NAME}: PASS`);
    console.log(`(扫描 ${result.summary.filesScanned} 个文件; ${ALLOWED_FIXED.length} 个已批准 fixed 模式)`);
    return;
  }

  if (totalBlocking > 0) {
    console.log(`${CHECK_NAME}: FAIL`);
    console.log();
    const allBlocking = [...blockingFindings, ...strictUpgraded];
    for (const f of allBlocking) {
      const loc = f.line > 0 ? `${f.file}:${f.line}` : f.file;
      console.log(`[${f.severity}] ${loc} ${f.message}`);
      if (f.rule) console.log(`       规则: ${f.rule}`);
    }
  } else {
    console.log(`${CHECK_NAME}: PASS`);
  }

  for (const w of effectiveWarnings) {
    const loc = w.line > 0 ? `${w.file}:${w.line}` : w.file;
    console.log(`[WARN] ${loc} ${w.message}`);
  }
}

function printJson(result, strict) {
  const blockingFindings = result.findings.filter(
    (f) => f.severity === "P1" || f.severity === "P2"
  );
  const allWarnings = result.warnings;
  const strictUpgraded = strict ? allWarnings.map((w) => ({ ...w, severity: "P2" })) : [];
  const effectiveWarnings = strict ? [] : allWarnings;

  const totalBlocking = blockingFindings.length + strictUpgraded.length;

  const output = {
    check: "check-ui",
    status: totalBlocking > 0 ? "FAIL" : "PASS",
    findings: [...blockingFindings, ...strictUpgraded].map((f) => ({
      severity: f.severity,
      file: f.file,
      line: f.line,
      message: f.message,
      rule: f.rule || "",
    })),
    warnings: effectiveWarnings.map((w) => ({
      file: w.file,
      line: w.line,
      message: w.message,
    })),
    summary: {
      filesScanned: result.summary.filesScanned,
      allowedFixedPatterns: ALLOWED_FIXED.length,
      allowedHighZindexPatterns: ALLOWED_HIGH_ZINDEX.length,
    },
  };
  console.log(JSON.stringify(output, null, 2));
}

// ===== CLI 入口 =====

function main() {
  const args = process.argv.slice(2);
  const wantHelp = args.includes("--help");
  const wantJson = args.includes("--json");
  const wantStrict = args.includes("--strict");
  const wantSelfTest = args.includes("--self-test");

  const known = new Set(["--help", "--json", "--strict", "--self-test"]);
  const unknown = args.filter((a) => !known.has(a));
  if (unknown.length > 0) {
    console.error(`未知参数: ${unknown.join(", ")}`);
    console.error("使用 --help 查看可用参数");
    process.exit(2);
  }

  if (wantHelp) {
    printHelp();
    process.exit(0);
  }

  if (wantSelfTest) {
    const failures = runSelfTest();
    if (failures.length === 0) {
      console.log(`${CHECK_NAME}_SELF_TEST: PASS`);
      process.exit(0);
    } else {
      console.log(`${CHECK_NAME}_SELF_TEST: FAIL`);
      for (const f of failures) console.log(`  - ${f}`);
      process.exit(1);
    }
  }

  const result = runCheck();

  if (wantJson) {
    printJson(result, wantStrict);
  } else {
    printText(result, wantStrict);
  }

  const blockingCount = result.findings.filter(
    (f) => f.severity === "P1" || f.severity === "P2"
  ).length;
  const strictUpgradedCount = wantStrict ? result.warnings.length : 0;

  process.exit(blockingCount + strictUpgradedCount > 0 ? 1 : 0);
}

main();
