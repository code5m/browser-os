#!/usr/bin/env node
// M2-6.d 命令片段库 UI 逻辑层自动化测试（headless，无 GUI 依赖）。

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
  return globalThis.__m26Resolve(specifier, context, next);
}`,
      ),
  );
  globalThis.__m26Resolve = resolveWithExt;
}

globalThis.localStorage = {
  data: new Map(),
  getItem(k) {
    return this.data.has(k) ? this.data.get(k) : null;
  },
  setItem(k, v) {
    this.data.set(k, String(v));
  },
};

const ROOT = new URL("..", import.meta.url).pathname;
const {
  emptySnippetForm,
  loadSnippetForm,
  parseArgvText,
  placeholderOf,
  validateSnippetForm,
  serializeSnippetForm,
  buildSnippetCategoryTree,
  canDeleteSnippet,
  isFavoriteSnippet,
  loadSnippetFavorites,
  toggleSnippetFavorite,
} = await import(`${ROOT}src/utils/snippetUi.ts`);

let passed = 0;
const failures = [];

function eq(actual, expected, label) {
  if (JSON.stringify(actual) === JSON.stringify(expected)) passed += 1;
  else failures.push(`${label}\n    期望: ${JSON.stringify(expected)}\n    实际: ${JSON.stringify(actual)}`);
}

function ok(cond, label) {
  if (cond) passed += 1;
  else failures.push(label);
}

const empty = emptySnippetForm();
eq(empty.category, "general", "默认分类 general");
eq(empty.interpreter, "bash", "默认解释器字段保留为 bash");
eq(empty.dangerous, false, "默认非高风险");

eq(parseArgvText("df\n-h\n\n{PATH}\n"), ["df", "-h", "{PATH}"], "argv 文本按一行一个元素解析");
eq(placeholderOf("{PATH}"), "PATH", "整元素占位符可识别");
eq(placeholderOf("--path={PATH}"), null, "局部占位符不被识别为合法占位");

const valid = {
  ...empty,
  name: "disk usage",
  category: "ops",
  argvText: "df\n-h\n{PATH}",
  params: [{ name: "PATH", label: "路径", param_type: "path", required: true, default: null, options: [], raw: false, secret: false }],
  timeout_secs: 30,
  dangerous: true,
};
eq(validateSnippetForm(valid).length, 0, "合法命令片段零校验问题");
const ser = serializeSnippetForm(valid);
eq(ser.argv, ["df", "-h", "{PATH}"], "序列化保留完整 argv 数组");
eq(ser.timeoutSecs, 30, "序列化保留正超时");
eq(ser.dangerous, true, "序列化保留 dangerous");
ok(!("path" in ser) && !("body" in ser), "命令片段序列化不含 path/body");

// M2-6-fix1（复核 F-1 / T-1）：enabled 必须贯通序列化，否则新建态的启用勾选被后端丢弃
eq(ser.enabled, true, "序列化保留 enabled=true");
eq(serializeSnippetForm({ ...valid, enabled: false }).enabled, false, "序列化保留 enabled=false（F-1 回归）");
eq(serializeSnippetForm({ ...valid, timeout_secs: 0 }).timeoutSecs, null, "超时 0 序列化为 null（走全局默认）");
eq(serializeSnippetForm({ ...valid, description: "" }).description, null, "空描述序列化为 null");

ok(validateSnippetForm({ ...valid, argvText: "" }).some((i) => i.field === "argv"), "空 argv 应报错");
ok(validateSnippetForm({ ...valid, name: "bad/name;rm" }).some((i) => i.field === "name"), "非法命令名应报错");
eq(parseArgvText("df\r\n-h\r\n"), ["df", "-h"], "CRLF 换行按一行一个元素解析");
ok(validateSnippetForm({ ...valid, interpreter: "shebang" }).some((i) => i.field === "interpreter"), "shebang 应报错");
ok(validateSnippetForm({ ...valid, argvText: "grep\n--include={EXT}" }).some((i) => i.field === "argv[1]"), "局部插值应报错");
ok(validateSnippetForm({ ...valid, argvText: "df\n{MISS}" }).some((i) => i.field === "argv[1]"), "未知占位符应报错");
ok(validateSnippetForm({ ...valid, argvText: "df\n-h" }).some((i) => i.field === "params[0].name"), "未使用参数应报错");

const meta = {
  id: "cmd-1",
  name: "disk usage",
  category: "ops",
  interpreter: "bash",
  argv: ["df", "-h"],
  params: [],
  description: "show disks",
  dangerous: false,
  builtin: false,
  enabled: true,
  timeout_secs: 0,
  created_at: "",
  updated_at: "",
};
const loaded = loadSnippetForm(meta);
eq(loaded.argvText, "df\n-h", "加载时 argv 数组转为一行一个元素");
const roundtrip = serializeSnippetForm(loadSnippetForm({ ...meta, enabled: false, builtin: true }));
eq(roundtrip.enabled, false, "load→serialize 往返保留 enabled=false");
eq(roundtrip.argv, ["df", "-h"], "load→serialize 往返保留 argv 数组");

const snippets = [
  { ...meta, id: "b", name: "zeta", category: "z" },
  { ...meta, id: "a", name: "alpha", category: "ops", argv: ["printf", "ok"] },
  { ...meta, id: "c", name: "beta", category: "ops", description: "network" },
];
const tree = buildSnippetCategoryTree(snippets);
eq(tree[0].category, "ops", "分类按字母排序");
eq(tree[0].snippets.map((s) => s.name), ["alpha", "beta"], "分类内按名称排序");
eq(buildSnippetCategoryTree(snippets, "printf")[0].snippets[0].id, "a", "搜索覆盖 argv");

eq(isFavoriteSnippet("a"), false, "初始未收藏");
toggleSnippetFavorite("a");
eq(isFavoriteSnippet("a"), true, "toggle 后收藏");
eq(buildSnippetCategoryTree(snippets, "", true).flatMap((n) => n.snippets).map((s) => s.id), ["a"], "只看收藏过滤生效");
toggleSnippetFavorite("a");
eq(isFavoriteSnippet("a"), false, "再次 toggle 取消收藏");

eq(canDeleteSnippet({ ...meta, builtin: true }), false, "内置命令不可删");
eq(canDeleteSnippet(meta), true, "普通命令可删");

// 收藏存储损坏时须回退为空数组，不得把异常抛给面板
globalThis.localStorage.data.set("browser-os-command-snippet-favorites", "{not json");
eq(loadSnippetFavorites().length, 0, "收藏数据损坏时回退为空数组");
globalThis.localStorage.data.delete("browser-os-command-snippet-favorites");
eq(loadSnippetFavorites().length, 0, "收藏数据缺失时回退为空数组");

if (failures.length === 0) {
  console.log(`check-command-ui-logic: ${passed} 断言全部通过`);
  process.exit(0);
}
console.error(`check-command-ui-logic: ${failures.length} 项失败（通过 ${passed}）`);
for (const f of failures) console.error(`  x ${f}`);
process.exit(1);
