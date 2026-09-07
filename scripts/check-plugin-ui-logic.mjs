#!/usr/bin/env node
// M5-W14 插件管理器 UI 逻辑层自动化测试（headless，无 GUI 依赖）。
// 加载**真实** src/utils/pluginUi.ts（与 check-database-ui-logic.mjs 同一范式），不重实现逻辑。

import * as nodeModule from "node:module";
import { readFile } from "node:fs/promises";

function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__pluginUiResolve(specifier, context, next);
}`,
      ),
  );
  globalThis.__pluginUiResolve = resolveWithExt;
}

const {
  PLUGIN_STATES,
  canTransitionUi,
  enabledActionsFor,
  pluginStateLabel,
  aclLevelLabel,
  aclLevelClass,
  isDangerousGate,
  redactDetail,
  redactSummary,
  redactKeyRecord,
  parseManifestInput,
} = await import("../src/utils/pluginUi.ts");

let pass = 0;
let fail = 0;
function ok(cond, msg) {
  if (cond) {
    pass++;
  } else {
    fail++;
    console.error("FAIL:", msg);
  }
}

// ---- 1. 状态机镜像后端 can_transition ----
ok(canTransitionUi("discovered", "validating") === true, "discovered->validating 合法");
ok(canTransitionUi("validating", "signed_ok") === true, "validating->signed_ok 合法");
ok(canTransitionUi("validating", "signed_failed") === true, "validating->signed_failed 合法");
ok(canTransitionUi("signed_ok", "loaded") === true, "signed_ok->loaded 合法");
ok(canTransitionUi("signed_failed", "uninstalled") === true, "signed_failed->uninstalled 合法");
ok(canTransitionUi("signed_failed", "discovered") === true, "signed_failed->discovered 合法");
ok(canTransitionUi("loaded", "enabled") === true, "loaded->enabled 合法");
ok(canTransitionUi("loaded", "disabled") === true, "loaded->disabled 合法");
ok(canTransitionUi("loaded", "uninstalled") === true, "loaded->uninstalled 合法");
ok(canTransitionUi("enabled", "disabled") === true, "enabled->disabled 合法");
ok(canTransitionUi("enabled", "uninstalled") === true, "enabled->uninstalled 合法");
ok(canTransitionUi("disabled", "enabled") === true, "disabled->enabled 合法");
ok(canTransitionUi("disabled", "uninstalled") === true, "disabled->uninstalled 合法");
// 非法迁移
ok(canTransitionUi("discovered", "enabled") === false, "discovered->enabled 非法");
ok(canTransitionUi("discovered", "loaded") === false, "discovered->loaded 非法");
ok(canTransitionUi("uninstalled", "discovered") === false, "uninstalled 复活 非法");
ok(canTransitionUi("uninstalled", "enabled") === false, "uninstalled->enabled 非法");
ok(canTransitionUi("signed_ok", "enabled") === false, "signed_ok->enabled 非法（需先 loaded）");
ok(canTransitionUi("enabled", "loaded") === false, "enabled->loaded 非法");
ok(canTransitionUi("validating", "loaded") === false, "validating->loaded 非法");

// ---- 2. enabledActionsFor 门控 ----
let g = enabledActionsFor("loaded");
ok(g.enable === true && g.disable === true && g.uninstall === true, "loaded: 可启用/禁用/卸载");
g = enabledActionsFor("enabled");
ok(g.enable === false && g.disable === true && g.uninstall === true, "enabled: 可禁用/卸载，不可启用");
g = enabledActionsFor("disabled");
ok(g.enable === true && g.disable === false && g.uninstall === true, "disabled: 可启用/卸载，不可禁用");
g = enabledActionsFor("discovered");
ok(g.enable === false && g.disable === false && g.uninstall === false, "discovered: 全禁（未 loaded）");
g = enabledActionsFor("uninstalled");
ok(g.enable === false && g.disable === false && g.uninstall === false, "uninstalled: 全禁（终态）");
g = enabledActionsFor("signed_failed");
ok(g.enable === false && g.disable === false && g.uninstall === true, "signed_failed: 仅可卸载");

// ---- 3. 状态中文标签 ----
for (const s of PLUGIN_STATES) {
  ok(typeof pluginStateLabel(s) === "string" && pluginStateLabel(s).length > 0, `状态标签非空: ${s}`);
}
ok(PLUGIN_STATES.length === 8, "PLUGIN_STATES 共 8 态");
ok(
  JSON.stringify(PLUGIN_STATES) ===
    JSON.stringify(["discovered", "validating", "signed_ok", "signed_failed", "loaded", "enabled", "disabled", "uninstalled"]),
  "PLUGIN_STATES 顺序与后端一致",
);

// ---- 4. 能力风险档 ----
ok(aclLevelLabel("safe") === "安全", "safe 标签");
ok(aclLevelLabel("confirm") === "需确认", "confirm 标签");
ok(aclLevelLabel("dangerous") === "危险（已阻止）", "dangerous 标签");
ok(aclLevelClass("safe") === "ok", "safe class");
ok(aclLevelClass("confirm") === "warn", "confirm class");
ok(aclLevelClass("dangerous") === "danger", "dangerous class");
ok(isDangerousGate("dangerous") === true, "dangerous 判定");
ok(isDangerousGate("safe") === false, "safe 非 dangerous");

// ---- 5. redactDetail 脱敏（丢弃签名原文/资源路径/metadata） ----
const detail = {
  id: "com.example.p",
  version: "1.0.0",
  display_name: "示例插件",
  description: "desc",
  min_app_version: "0.1.0",
  state: "loaded",
  capabilities: [{ capability: "workspace.read", reason: "读取", acl_level: "safe" }],
  hash_prefix: "abcd",
  // 故意包含敏感原文，验证投影会丢弃
  signature: { algorithm: "Ed25519", key_id: "k1", status: "structure_ok", value: "SUPER_SECRET_BASE64" },
  installed_at: "2026-09-08T00:00:00Z",
  updated_at: "2026-09-08T00:00:00Z",
  resource: { declared_hash: "deadbeef", path_provided: true, verified: true, path: "/secret/abs/path" },
};
const out = redactDetail(detail);
ok(out.signature && !("value" in out.signature), "redactDetail 丢弃 signature.value");
ok(out.signature.algorithm === "Ed25519" && out.signature.key_id === "k1" && out.signature.status === "structure_ok", "redactDetail 保留签名结构字段");
ok(out.resource && !("path" in out.resource), "redactDetail 丢弃资源绝对路径");
ok(out.resource.declared_hash === "deadbeef" && out.resource.path_provided === true, "redactDetail 保留资源布尔/摘要");
ok(out.capabilities.length === 1 && out.capabilities[0].acl_level === "safe", "redactDetail 保留能力视图");
ok(out.id === "com.example.p" && out.hash_prefix === "abcd", "redactDetail 保留基础字段");
ok(!("metadata" in out), "redactDetail 无 metadata 字段");

// ---- 6. redactSummary 透传（已是脱敏视图） ----
const summary = { id: "com.x", version: "1.0.0", display_name: "X", state: "enabled", capability_count: 0, hash_prefix: "aa", updated_at: "t" };
ok(redactSummary(summary).id === "com.x", "redactSummary 透传");

// ---- 7. redactKeyRecord 仅指纹（无 pubkey） ----
const key = { key_id: "k1", fingerprint: "abcd1234efgh5678", note: "n", added_at: "t" };
const kr = redactKeyRecord(key);
ok(kr.fingerprint === "abcd1234efgh5678" && kr.key_id === "k1", "redactKeyRecord 保留指纹");
ok(!("pubkey" in kr), "redactKeyRecord 无 pubkey 字段");

// ---- 8. parseManifestInput 轻量预检 ----
const goodManifest = JSON.stringify({
  id: "com.example.p",
  version: "1.0.0",
  display_name: "P",
  description: "d",
  min_app_version: "0.1.0",
  entry: { entry_url: "https://e.com/i.html", icon: "https://e.com/i.png" },
  capabilities: [],
  hash: "a".repeat(64),
  signature: { algorithm: "Ed25519", key_id: "k1", value: "QUJD", signed_at: "2026-09-08T00:00:00Z" },
});
ok(parseManifestInput(goodManifest).ok === true, "合法 manifest 解析通过");
ok(parseManifestInput(goodManifest).manifest?.id === "com.example.p", "合法 manifest 回传 manifest");
ok(parseManifestInput("").ok === false, "空文本失败");
ok(parseManifestInput("{ not json").ok === false, "非法 JSON 失败");
ok(parseManifestInput(JSON.stringify({ id: "x" })).ok === false, "缺字段失败");
ok(parseManifestInput(JSON.stringify({ id: "x", version: "1", display_name: "d", description: "d", min_app_version: "0.1", hash: "h", signature: { algorithm: "E" }, capabilities: [], entry: "bad" })).ok === false, "entry 非法失败");

// ---- 9. Pinia setup-store 属性已自动解包，模板不得再取 .value ----
const panelSource = await readFile(new URL("../src/components/plugin/PluginManager.vue", import.meta.url), "utf8");
ok(!/store\.(?:busy|filterState|list|detail|manifestText|resourcePath|keys|actionsFor|error)\.value/.test(panelSource), "PluginManager 不重复解包 Pinia store ref");

console.log(`check-plugin-ui-logic: ${pass} 断言通过, ${fail} 失败`);
if (fail > 0) {
  process.exit(1);
}
