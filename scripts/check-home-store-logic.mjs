#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5-W17（主页 Home）前端逻辑层自动化测试（headless，无 GUI / store / invoke 依赖）
//
// 直接加载真实的 src/utils/homeUi.ts（纯逻辑层），只把产品代码行为作为断言对象，
// 不 mock 逻辑层自身。覆盖 board §M5-W17 A3 的四项要求：
//   有界（bounded）/ 已校验（validated）/ 稳定默认（stable defaults）/
//   迁移安全（migration-safe），以及共享验收 #4 的「零敏感披露」。
//
// 用法: node scripts/check-home-store-logic.mjs
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
  return globalThis.__homeResolve(specifier, context, next);
}`
      )
  );
  globalThis.__homeResolve = resolveWithExt;
}

const ROOT = new URL("..", import.meta.url).pathname;
const homeUi = await import(`${ROOT}src/utils/homeUi.ts`);

let passed = 0;
let failed = 0;
function ok(name, cond) {
  if (cond) {
    passed++;
    console.log("  ✓ " + name);
  } else {
    failed++;
    console.error("  ✗ " + name);
  }
}
function eq(name, a, b) {
  ok(name + ` (got=${JSON.stringify(a)})`, JSON.stringify(a) === JSON.stringify(b));
}

const SECRET_RE = /token|secret|api[_-]?key|password|Bearer|sk-|AKIA/i;

// ---------------------------------------------------------------------------
// 容量常量（有界）
// ---------------------------------------------------------------------------
ok("HOME_MAX_SHORTCUTS=24 正数", homeUi.HOME_MAX_SHORTCUTS === 24);
ok("HOME_MAX_RECENTS=12 正数", homeUi.HOME_MAX_RECENTS === 12);
ok(
  "常量合理(0<RECENTS<=SHORTCUTS)",
  homeUi.HOME_MAX_RECENTS > 0 && homeUi.HOME_MAX_RECENTS <= homeUi.HOME_MAX_SHORTCUTS
);
ok("HOME_MAX_NAME_LEN/TARGET_LEN 正数", homeUi.HOME_MAX_NAME_LEN > 0 && homeUi.HOME_MAX_TARGET_LEN > 0);

// ---------------------------------------------------------------------------
// 类型判别 / 文本规整
// ---------------------------------------------------------------------------
ok("isShortcutType url", homeUi.isShortcutType("url") === true);
ok("isShortcutType dir", homeUi.isShortcutType("dir") === true);
ok("isShortcutType 拒绝未知", homeUi.isShortcutType("file") === false);
ok("isShortcutType 拒绝 null/undefined", homeUi.isShortcutType(null) === false && homeUi.isShortcutType(undefined) === false);

eq("clampText 非字符串→''", homeUi.clampText(123, 10), "");
eq("clampText null→''", homeUi.clampText(null, 10), "");
eq("clampText 去空白", homeUi.clampText("  ab  ", 10), "ab");
eq("clampText 超长截断", homeUi.clampText("abcdefghij", 4), "abcd");

// ---------------------------------------------------------------------------
// 确定性 id（迁移安全：同 type+target ⇒ 同 id；不用随机）
// ---------------------------------------------------------------------------
const sid1 = homeUi.stableId("url", "https://a.example/x");
const sid2 = homeUi.stableId("url", "https://a.example/x");
const sid3 = homeUi.stableId("dir", "https://a.example/x");
ok("stableId 确定性(同输入同输出)", sid1 === sid2);
ok("stableId 区分 type", sid1 !== sid3);
ok("stableId 前缀 home-", sid1.startsWith("home-"));

// ---------------------------------------------------------------------------
// sanitizeShortcut：单条校验 + 迁移修补
// ---------------------------------------------------------------------------
const sOk = homeUi.sanitizeShortcut({ id: "x1", type: "url", name: "A", target: "https://a.example", icon: "🔍" });
ok("sanitize 合法条目保留", sOk && sOk.id === "x1" && sOk.type === "url");
eq("sanitize 拒绝非法 type", homeUi.sanitizeShortcut({ type: "file", name: "A", target: "/tmp" }), null);
eq("sanitize 拒绝缺 name", homeUi.sanitizeShortcut({ type: "url", name: "", target: "https://a" }), null);
eq("sanitize 拒绝缺 target", homeUi.sanitizeShortcut({ type: "url", name: "A", target: "" }), null);
eq("sanitize 拒绝非对象(null)", homeUi.sanitizeShortcut(null), null);
eq("sanitize 拒绝非对象(字符串)", homeUi.sanitizeShortcut("nope"), null);
eq("sanitize name 非字符串→拒绝", homeUi.sanitizeShortcut({ type: "url", name: 123, target: "https://a" }), null);

const sNoId = homeUi.sanitizeShortcut({ type: "dir", name: "文档", target: "/home/u/Documents" });
ok("sanitize 缺 id→派生确定性 id", sNoId && sNoId.id.startsWith("home-dir-"));
eq(
  "sanitize 缺 id 两次派生一致(迁移稳定)",
  homeUi.sanitizeShortcut({ type: "dir", name: "文档", target: "/home/u/Documents" }).id,
  sNoId.id
);
eq(
  "sanitize 空 icon→默认 🔗",
  homeUi.sanitizeShortcut({ type: "url", name: "A", target: "https://a", icon: "" }).icon,
  "🔗"
);
const sLong = homeUi.sanitizeShortcut({
  type: "url",
  name: "N".repeat(500),
  target: "https://a.example/" + "p".repeat(5000),
});
ok("sanitize 超长 name 截断", sLong.name.length === homeUi.HOME_MAX_NAME_LEN);
ok("sanitize 超长 target 截断", sLong.target.length === homeUi.HOME_MAX_TARGET_LEN);

// ---------------------------------------------------------------------------
// normalizeShortcuts：列表校验 + 去重 + 有界 + 稳定默认
// ---------------------------------------------------------------------------
eq("normalizeShortcuts 非数组→[]", homeUi.normalizeShortcuts(null), []);
eq("normalizeShortcuts 字符串→[]", homeUi.normalizeShortcuts("[]"), []);
eq("normalizeShortcuts 空数组→[]", homeUi.normalizeShortcuts([]), []);

const mixed = homeUi.normalizeShortcuts([
  { id: "a", type: "url", name: "A", target: "https://a.example", icon: "🔍" },
  { id: "b", type: "file", name: "B", target: "/tmp" },           // 非法 type → 丢弃
  { id: "c", type: "url", name: "", target: "https://c.example" }, // 缺 name → 丢弃
  null,                                                            // 非对象 → 丢弃
  "junk",                                                          // 非对象 → 丢弃
  { id: "d", type: "dir", name: "D", target: "/home/u/D", icon: "📁" },
]);
eq("normalizeShortcuts 仅保留合法(4 脏→2 合法)", mixed.length, 2);
ok("normalizeShortcuts 保留项均合法", mixed.every((s) => homeUi.isShortcutType(s.type) && !!s.name && !!s.target && !!s.id));

const dup = homeUi.normalizeShortcuts([
  { id: "a", type: "url", name: "A", target: "https://dup.example", icon: "🔍" },
  { id: "b", type: "url", name: "A2", target: "https://dup.example", icon: "🔍" },
]);
eq("normalizeShortcuts 去重(type|target)", dup.length, 1);

const many = [];
for (let i = 0; i < 200; i++) {
  many.push({ id: "m" + i, type: "url", name: "N" + i, target: "https://x.example/" + i, icon: "🔍" });
}
const bounded = homeUi.normalizeShortcuts(many);
eq("normalizeShortcuts 有界封顶(200→24)", bounded.length, homeUi.HOME_MAX_SHORTCUTS);

const idDup = homeUi.normalizeShortcuts([
  { id: "same", type: "url", name: "A", target: "https://a1.example", icon: "🔍" },
  { id: "same", type: "url", name: "B", target: "https://b1.example", icon: "🔍" },
]);
ok("normalizeShortcuts 重复 id 被改写(渲染 key 唯一)", idDup.length === 2 && idDup[0].id !== idDup[1].id);

// 稳定默认
const defs = homeUi.defaultShortcuts();
ok("defaultShortcuts 非空且合法", defs.length > 0 && defs.every((s) => homeUi.isShortcutType(s.type)));
eq(
  "defaultShortcuts 确定性(两次调用同结果)",
  JSON.stringify(homeUi.defaultShortcuts()),
  JSON.stringify(defs)
);
ok(
  "defaultShortcuts 全部通过校验(零漂移)",
  homeUi.normalizeShortcuts(defs).length === defs.length
);

// ---------------------------------------------------------------------------
// normalizeRecents / pushRecent：有界、去重、时间倒序
// ---------------------------------------------------------------------------
eq("normalizeRecents 非数组→[]", homeUi.normalizeRecents(null), []);
const recents = homeUi.normalizeRecents([
  { id: "r1", type: "dir", name: "D1", target: "/home/u/D1", icon: "📁", at: 10 },
  { id: "r2", type: "url", name: "U1", target: "https://u1.example", icon: "🔍", at: 30 },
  { id: "r3", type: "bad", name: "X", target: "/x", at: 5 },       // 非法 → 丢弃
  { id: "r4", type: "url", name: "U1", target: "https://u1.example", icon: "🔍", at: 20 }, // 重复
]);
eq("normalizeRecents 去重+丢弃非法", recents.length, 2);
ok("normalizeRecents 时间倒序(新的在前)", recents[0].at >= recents[1].at);
eq("normalizeRecents at 非数字→0", homeUi.normalizeRecents([{ type: "url", name: "A", target: "https://a.example", at: "nope" }])[0].at, 0);

const recMany = [];
for (let i = 0; i < 50; i++) {
  recMany.push({ id: "z" + i, type: "url", name: "N" + i, target: "https://z.example/" + i, icon: "🔍", at: i });
}
eq("normalizeRecents 有界封顶(50→12)", homeUi.normalizeRecents(recMany).length, homeUi.HOME_MAX_RECENTS);

const base = { id: "k", type: "url", name: "K", target: "https://k.example", icon: "🔍" };
let rl = [];
for (let i = 0; i < 20; i++) {
  rl = homeUi.pushRecent(rl, { ...base, id: "k" + i, target: "https://k.example/" + i, at: i }, homeUi.HOME_MAX_RECENTS);
}
eq("pushRecent 有界封顶(20→12)", rl.length, homeUi.HOME_MAX_RECENTS);
ok("pushRecent 最新在前", rl[0].at === 19);
const rl2 = homeUi.pushRecent(rl, { ...rl[5], at: 999 }, homeUi.HOME_MAX_RECENTS);
ok("pushRecent 去重(同 type|target 不增项)", rl2.length === homeUi.HOME_MAX_RECENTS);
ok("pushRecent 去重后该项置顶", rl2[0].at === 999);

// boundedPush：超上限保留最新
const bp = homeUi.boundedPush(["a", "b"], ["c", "d", "a"], 3, (x) => x);
eq("boundedPush 去重+保留最新(cap=3)", bp, ["b", "c", "d"]);

// ---------------------------------------------------------------------------
// 零敏感披露（共享验收 #4）
// ---------------------------------------------------------------------------
ok("looksSensitive 命中 URL userinfo", homeUi.looksSensitive("https://u:p@h.example/x") === true);
ok("looksSensitive 命中 token 参数", homeUi.looksSensitive("https://h.example?access_token=abc") === true);
ok("looksSensitive 命中 Bearer", homeUi.looksSensitive("Authorization: Bearer abcdefghijkl") === true);
ok("looksSensitive 正常串不误伤", homeUi.looksSensitive("https://github.com/a/b") === false);

const leakyUrl = homeUi.homeDisplayTarget({
  id: "1", type: "url", name: "Leak",
  target: "https://user:pw@h.example/docs/index?access_token=ghp_abcdefghijklmnop&x=1#frag",
  icon: "🔍",
});
ok("homeDisplayTarget(url) 不含 query", !leakyUrl.includes("access_token"));
ok("homeDisplayTarget(url) 不含 userinfo", !leakyUrl.includes("user:pw") && !leakyUrl.includes("pw@"));
ok("homeDisplayTarget(url) 不含 fragment", !leakyUrl.includes("frag"));
ok("homeDisplayTarget(url) 无敏感关键字", !SECRET_RE.test(leakyUrl));

const dirLabel = homeUi.homeDisplayTarget({
  id: "2", type: "dir", name: "文档", target: "/home/alice/secret-project/docs", icon: "📁",
});
ok("homeDisplayTarget(dir) 只给末段名(不回显完整路径)", !dirLabel.includes("/home/alice"));
eq("homeDisplayTarget(dir) 末段名", dirLabel, "docs");

const appLabel = homeUi.homeDisplayTarget({
  id: "3", type: "app", name: "编辑器", target: "/usr/bin/code --flag", icon: "🚀",
});
ok("homeDisplayTarget(app) 不回显完整路径", !appLabel.includes("/usr/bin"));
eq("homeDisplayTarget(app) 可执行名", appLabel, "code");

eq("homeDisplayTarget 非法 URL→类型标签(不回显原文)", homeUi.homeDisplayTarget({ id: "4", type: "url", name: "A", target: "not a url at all", icon: "🔍" }), "网页");
eq("homeDisplayTarget null→''", homeUi.homeDisplayTarget(null), "");
eq("homeDisplayTarget 空 target→类型标签", homeUi.homeDisplayTarget({ id: "5", type: "dir", name: "A", target: "", icon: "📁" }), "目录");

const a11y = homeUi.homeAccessibleLabel({ id: "6", type: "dir", name: "文档", target: "/home/alice/secret/docs", icon: "📁" });
ok("homeAccessibleLabel 非空", a11y.length > 0);
ok("homeAccessibleLabel 不含完整路径", !a11y.includes("/home/alice"));
ok("homeAccessibleLabel 无敏感", !SECRET_RE.test(a11y));

// ---------------------------------------------------------------------------
// 主要工作区（共享验收 #2/#3：既有模块可发现，不新增路由）
// ---------------------------------------------------------------------------
const areas = homeUi.HOME_PRINCIPAL_AREAS;
ok("HOME_PRINCIPAL_AREAS 非空", areas.length > 0);
ok("HOME_PRINCIPAL_AREAS 有界(<=12)", areas.length <= 12);
ok("area 字段齐全(icon/label/hint/view)", areas.every((a) => !!a.view && !!a.icon && !!a.label && !!a.hint));
ok("area view 唯一(无重复入口)", new Set(areas.map((a) => a.view)).size === areas.length);
ok(
  "area view 均为已知 MainView 子集(不新增视图)",
  areas.every((a) =>
    ["browser", "files", "clip", "arts", "grid", "apps", "term", "repo", "audit", "scripts",
     "commands", "tools", "db", "tasks", "skills", "agents", "graph", "plugin", "editor", "home"]
      .includes(a.view)
  )
);
ok(
  "areaAccessibleLabel 全部非空且无敏感",
  areas.every((a) => {
    const t = homeUi.areaAccessibleLabel(a);
    return t.length > 0 && !SECRET_RE.test(t);
  })
);
eq("areaAccessibleLabel null→'工作区'", homeUi.areaAccessibleLabel(null), "工作区");

// ---------------------------------------------------------------------------
// 面板三态（共享验收 #2：连贯空/载入/错误态，且错误不回显原始串）
// ---------------------------------------------------------------------------
eq("panelStateHome loading", homeUi.panelStateHome({ loading: true, count: 0, error: null }).state, "loading");
eq("panelStateHome empty", homeUi.panelStateHome({ loading: false, count: 0, error: null }).state, "empty");
eq("panelStateHome ready", homeUi.panelStateHome({ loading: false, count: 3, error: null }).state, "ready");
eq("panelStateHome error", homeUi.panelStateHome({ loading: false, count: 3, error: "boom" }).state, "error");
ok("panelStateHome 空态文案非空", homeUi.panelStateHome({ loading: false, count: 0, error: null }).message.length > 0);
ok(
  "panelStateHome 错误态不回显原始 error(零敏感)",
  !homeUi.panelStateHome({ loading: false, count: 0, error: "/home/alice token=sk-abcdef" }).message.includes("alice") &&
    !SECRET_RE.test(homeUi.panelStateHome({ loading: false, count: 0, error: "token=sk-abcdef" }).message)
);
ok(
  "panelStateHome loading 优先于 error/empty",
  homeUi.panelStateHome({ loading: true, count: 0, error: "x" }).state === "loading"
);
eq("panelStateHome count 非数字→按 0(空态)", homeUi.panelStateHome({ loading: false, count: "x", error: null }).state, "empty");

// ---------------------------------------------------------------------------
// 全部用户可见文案：零敏感、非空（纵深断言）
// ---------------------------------------------------------------------------
const VISIBLE = [
  homeUi.HOME_EMPTY_MESSAGE,
  homeUi.HOME_RECENTS_EMPTY_MESSAGE,
  homeUi.HOME_ERROR_FALLBACK,
  homeUi.HOME_LOADING_MESSAGE,
  homeUi.HOME_SHORTCUT_LIMIT_MESSAGE,
  ...areas.map((a) => homeUi.areaAccessibleLabel(a)),
];
ok("用户可见文案全部非空", VISIBLE.every((t) => typeof t === "string" && t.length > 0));
ok("用户可见文案零敏感披露", VISIBLE.every((t) => !SECRET_RE.test(t)));
ok("空态文案不是营销页(无外部推广语)", !/立即|免费注册|升级|试用|官网/.test(homeUi.HOME_EMPTY_MESSAGE));

// ---------------------------------------------------------------------------
// M5-W17 Acceptance Closeout 回归：HOME_NO_SECRET_PERSIST
// （应用启动命令体不得写入浏览器存储；只保留非敏感主页元数据）
// ---------------------------------------------------------------------------
ok("HOME_PERSISTED_TYPES 不含 app", !homeUi.HOME_PERSISTED_TYPES.includes("app"));
ok(
  "HOME_PERSISTED_TYPES 含 url/dir",
  homeUi.HOME_PERSISTED_TYPES.includes("url") && homeUi.HOME_PERSISTED_TYPES.includes("dir")
);

const appItem = {
  id: "app1", type: "app", name: "终端工具",
  target: "myapp --token=ghp_abcdefghijklmnop --user=root", icon: "🚀",
};
const urlItem = { id: "u1", type: "url", name: "A", target: "https://a.example", icon: "🔍" };
const dirItem = { id: "d1", type: "dir", name: "文档", target: "/home/u/Documents", icon: "📁" };

ok("isStorageSafe(app)=false", homeUi.isStorageSafe(appItem) === false);
ok("isStorageSafe(url)=true", homeUi.isStorageSafe(urlItem) === true);
ok("isStorageSafe(dir)=true", homeUi.isStorageSafe(dirItem) === true);

const mixedList = [urlItem, appItem, dirItem];
const persisted = homeUi.toPersisted(mixedList);
eq("toPersisted 剔除 app(3→2)", persisted.length, 2);
ok("toPersisted 结果不含 app 条目", !persisted.some((s) => s.type === "app"));
eq("toPersisted 不改动原数组", mixedList.length, 3);

const payload = homeUi.persistedJson(mixedList);
ok("persistedJson 不含命令体", !payload.includes("myapp"));
ok("persistedJson 不含 token 参数", !payload.includes("token=") && !payload.includes("ghp_"));
ok("persistedJson 不含 --user 参数", !payload.includes("--user=root"));
ok(
  "persistedJson 保留 url/dir 非敏感元数据",
  payload.includes("https://a.example") && payload.includes("文档")
);
ok("persistedJson 零敏感", !SECRET_RE.test(payload));

// 最近访问（HomeRecent 带 at）同样不落库 app，且保留时间字段
const recPayload = JSON.stringify(
  homeUi.toPersisted([{ ...urlItem, at: 100 }, { ...appItem, at: 200 }])
);
ok("toPersisted(最近访问) 剔除 app 命令体", !recPayload.includes("myapp"));
ok("toPersisted(最近访问) 保留 at 字段", recPayload.includes('"at":100'));

// 迁移期：旧存储里的 app 条目被剔除（不进内存、不回显）
const legacy = homeUi.normalizeShortcuts([appItem, urlItem]).filter(homeUi.isStorageSafe);
eq("迁移：旧存储 app 条目被剔除", legacy.length, 1);
ok("迁移：剩余为 url 条目", legacy[0].type === "url");
eq(
  "迁移：仅含 app 的旧存储→空(回落稳定默认)",
  homeUi.normalizeShortcuts([appItem]).filter(homeUi.isStorageSafe).length,
  0
);

ok("HOME_APP_SESSION_ONLY_NOTICE 非空", homeUi.HOME_APP_SESSION_ONLY_NOTICE.length > 0);
ok("HOME_APP_SESSION_ONLY_NOTICE 零敏感", !SECRET_RE.test(homeUi.HOME_APP_SESSION_ONLY_NOTICE));
ok("HOME_APP_SESSION_ONLY_NOTICE 明示会话内有效", homeUi.HOME_APP_SESSION_ONLY_NOTICE.includes("会话"));

// 结果
console.log(`\n主页 Home 逻辑测试：通过 ${passed}，失败 ${failed}`);
process.exit(failed === 0 ? 0 : 1);
