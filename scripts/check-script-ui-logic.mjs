#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M2-5.a 脚本库 CRUD UI 逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/utils/scriptUi.ts`（不 mock、不重写逻辑），因此每条
// 断言反映的都是产品代码行为。本卡只做「纯前端 CRUD」，故验证表单逻辑：
// 空表单 / 从后端镜像加载 / name 校验 / 超时边界 / 参数校验（enum options 必填、
// name 形态）/ 序列化（只传正文不传路径）/ 分类树与分组 / 内置脚本不可删。
//
// 用法: node scripts/check-script-ui-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

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
  return globalThis.__m25Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m25Resolve = resolveWithExt;
}

const ROOT = new URL("..", import.meta.url).pathname;

const {
  emptyScriptForm,
  loadFormFromMeta,
  validateScriptForm,
  validateParam,
  serializeScriptForm,
  buildCategoryTree,
  groupScriptsByCategory,
  canDeleteScript,
} = await import(`${ROOT}src/utils/scriptUi.ts`);

let passed = 0;
const failures = [];

function eq(actual, expected, label) {
  if (actual === expected) {
    passed += 1;
  } else {
    failures.push(`${label}\n    期望: ${JSON.stringify(expected)}\n    实际: ${JSON.stringify(actual)}`);
  }
}

function ok(cond, label) {
  if (cond) passed += 1;
  else failures.push(label);
}

// ---------- 1) 空表单：id=null（新建态），默认解释器 bash ----------
const e = emptyScriptForm();
eq(e.id, null, "空表单 id=null（新建态）");
eq(e.interpreter, "bash", "空表单默认解释器 bash");
eq(e.category, "general", "空表单默认分类 general");
eq(e.timeout_secs, 0, "空表单默认超时 0（用全局默认）");
eq(e.builtin, false, "空表单非内置");

// ---------- 2) 从后端镜像加载：保留 id 与全部字段 ----------
const meta = {
  id: "scr-1",
  name: "备份",
  category: "util",
  path: "scr-1.sh",
  interpreter: "bash",
  params: [{ name: "dst", label: "目标", param_type: "string", required: true, default: null, options: [], raw: false, secret: false }],
  description: "desc",
  builtin: false,
  enabled: true,
  timeout_secs: 120,
  created_at: "",
  updated_at: "",
};
const lf = loadFormFromMeta(meta);
eq(lf.id, "scr-1", "加载后保留 id");
eq(lf.name, "备份", "加载后保留 name");
eq(lf.timeout_secs, 120, "加载后保留 timeout");
eq(lf.params.length, 1, "加载后保留 params");
eq(lf.params[0].name, "dst", "加载后 params 内容正确");

// ---------- 3) name 校验（U9） ----------
ok(validateScriptForm(e).some((i) => i.field === "name"), "空 name 应报错");
const badName = { ...e, name: "a/b*c" };
ok(validateScriptForm(badName).some((i) => i.field === "name"), "非法字符 name 应报错");
const goodName = { ...e, name: "my script-1.sh" };
ok(!validateScriptForm(goodName).some((i) => i.field === "name"), "合法 name 不报 name 错");

// ---------- 4) 超时边界（U9） ----------
const negTo = { ...e, name: "x", timeout_secs: -1 };
ok(validateScriptForm(negTo).some((i) => i.field === "timeout_secs"), "负超时应报错");

// ---------- 5) 参数校验：enum 必填 options（U7） ----------
const enumNoOpts = { name: "x", category: "c", interpreter: "bash", description: "", body: "", params: [{ name: "m", label: "M", param_type: "enum", required: false, default: null, options: [], raw: false, secret: false }], timeout_secs: 0, enabled: true, builtin: false, id: null };
ok(validateScriptForm(enumNoOpts).some((i) => i.field.endsWith(".options")), "enum 无 options 应报错");
const enumWithOpts = { ...enumNoOpts, params: [{ name: "m", label: "M", param_type: "enum", required: false, default: null, options: ["a", "b"], raw: false, secret: false }] };
ok(!validateScriptForm(enumWithOpts).some((i) => i.field.endsWith(".options")), "enum 有 options 不报 options 错");

// ---------- 6) 参数校验：name 形态（U9） ----------
const badParamName = { name: "x", category: "c", interpreter: "bash", description: "", body: "", params: [{ name: "1bad", label: "M", param_type: "string", required: false, default: null, options: [], raw: false, secret: false }], timeout_secs: 0, enabled: true, builtin: false, id: null };
ok(validateParam(badParamName.params[0], 0).some((p) => p.field === "name"), "参数 name 以数字开头应报错");

// ---------- 7) 整表通过：合法表单零问题（U2/U3） ----------
const validForm = {
  id: null,
  name: "deploy",
  category: "ops",
  interpreter: "bash",
  description: "d",
  body: "echo hi",
  params: [{ name: "env", label: "环境", param_type: "enum", required: true, default: null, options: ["prod", "test"], raw: false, secret: false }],
  timeout_secs: 0,
  enabled: true,
  builtin: false,
};
eq(validateScriptForm(validForm).length, 0, "合法表单零校验问题");

// ---------- 8) 序列化：只传正文不传路径（U4.5 边界，前端不构造路径） ----------
const ser = serializeScriptForm(validForm);
eq(ser.name, "deploy", "序列化 name");
eq(ser.body, "echo hi", "序列化 body（前端只传正文）");
eq(ser.timeoutSecs, null, "超时 0 → null（用全局默认）");
ok(!("id" in ser) && !("path" in ser), "序列化不含 id/path（路径由后端按 <id>.<ext> 生成）");
const ser2 = serializeScriptForm({ ...validForm, timeout_secs: 90 });
eq(ser2.timeoutSecs, 90, "超时 >0 如实传递");

// ---------- 9) 分类树（U1） ----------
const scripts = [
  { ...meta, id: "a", name: "zeta", category: "z", path: "", interpreter: "bash", params: [], description: "", builtin: false, enabled: true, timeout_secs: 0, created_at: "", updated_at: "" },
  { ...meta, id: "b", name: "alpha", category: "a", path: "", interpreter: "bash", params: [], description: "", builtin: false, enabled: true, timeout_secs: 0, created_at: "", updated_at: "" },
  { ...meta, id: "c", name: "beta", category: "a", path: "", interpreter: "bash", params: [], description: "", builtin: false, enabled: true, timeout_secs: 0, created_at: "", updated_at: "" },
];
const tree = buildCategoryTree(scripts);
eq(tree.length, 2, "分类树恰 2 个分类");
eq(tree[0].category, "a", "分类按字母序（a 在前）");
eq(tree[0].scripts.length, 2, "a 分类含 2 个脚本");
eq(tree[0].scripts[0].name, "alpha", "同分类内按名排序（alpha 在 beta 前）");

// ---------- 10) 分组（供列表渲染） ----------
const grouped = groupScriptsByCategory(scripts);
eq(Object.keys(grouped).length, 2, "分组恰 2 个键");
eq(grouped["a"].length, 2, "a 组 2 个");

// ---------- 11) 内置脚本不可删（U5） ----------
const builtinMeta = { ...meta, id: "sys", name: "系统脚本", builtin: true };
eq(canDeleteScript(builtinMeta), false, "内置脚本不可删");
eq(canDeleteScript(meta), true, "普通脚本可删");
eq(canDeleteScript(null), false, "null 不可删");

// ---------- 结果 ----------
if (failures.length === 0) {
  console.log(`check-script-ui-logic: ${passed} 断言全部通过`);
  process.exit(0);
} else {
  console.error(`check-script-ui-logic: ${failures.length} 项失败（通过 ${passed}）`);
  for (const f of failures) console.error(`  ✗ ${f}`);
  process.exit(1);
}
