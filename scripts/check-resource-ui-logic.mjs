#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M1-8 资源瀑布前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/stores/useResourceStore.ts`，只把 `src/bridge.ts` 的
// 资源相关方法替换为记录型 mock（不 mock store 自身逻辑）：
// 因此下面每一条断言反映的都是**产品代码**的行为，而非测试替身的行为。
//
// 覆盖：筛选分类正确性（REQUIRE_FRONTEND #3）、本地容量镜像（超限丢最旧）、
// loadTab 权威同步、clear 逻辑（#4）、采集开关（#5）、降级字段展示不伪造（#7）。
//
// 用法: node scripts/check-resource-ui-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

// 前端源码是无扩展名 import（`from "../bridge"`），浏览器由 Vite 解析；
// Node ESM 要求显式扩展名，这里挂一个最小的 resolve 钩子补 `.ts`。
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
  return globalThis.__m18Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m18Resolve = resolveWithExt;
}

const ROOT = new URL("..", import.meta.url).pathname;

const { bridge } = await import(`${ROOT}src/bridge.ts`);
const {
  useResourceStore,
  kindMatchesFilter,
  displayUrl,
  formatSize,
  formatDuration,
  RESOURCE_FILTERS,
} = await import(`${ROOT}src/stores/useResourceStore.ts`);
const { createPinia, setActivePinia } = await import(
  `${ROOT}node_modules/pinia/dist/pinia.mjs`
);

// ---------- mock bridge（只替换资源相关方法，记录调用） ----------
const calls = { list: [], clear: [], set: [], get: 0 };

bridge.listTabResources = async (tabId) => {
  calls.list.push(tabId);
  return { records: [], evicted: 0, enabled: true };
};
bridge.clearTabResources = async (tabId) => {
  calls.clear.push(tabId);
};
bridge.getResourceCaptureSettings = async () => {
  calls.get += 1;
  return { enabled: true, max_per_tab: 200, max_total: 2000, max_url_bytes: 2048 };
};
bridge.setResourceCaptureSettings = async (enabled, maxPerTab) => {
  calls.set.push([enabled, maxPerTab]);
  return { enabled, max_per_tab: maxPerTab ?? 200, max_total: 2000, max_url_bytes: 2048 };
};

let passed = 0;
const failures = [];
function assert(cond, label) {
  if (cond) {
    passed += 1;
  } else {
    failures.push(label);
    console.error(`FAIL: ${label}`);
  }
}

function rec(id, kind, tab = "tab-1", overrides = {}) {
  return {
    id,
    tab_id: tab,
    url: "https://example.com/a?token=***",
    method: "GET",
    status: 200,
    mime: "text/html",
    size_bytes: 1024,
    started_at: 1,
    finished_at: 2,
    duration_ms: 1,
    resource_type: kind,
    ...overrides,
  };
}

// ================= 1) 筛选分类正确性（REQUIRE_FRONTEND #3） =================
const KIND_TO_FILTER = {
  document: "doc",
  script: "js",
  stylesheet: "css",
  image: "image",
  xhr_fetch: "xhr",
  font: "other",
  media: "other",
  other: "other",
};
for (const [kind, expected] of Object.entries(KIND_TO_FILTER)) {
  assert(
    kindMatchesFilter(kind, expected),
    `kindMatchesFilter(${kind}, ${expected}) 应为 true`
  );
  // 每种 kind 只能落入 all 或自己的桶
  for (const f of RESOURCE_FILTERS.map((x) => x.key)) {
    if (f === "all" || f === expected) continue;
    assert(
      !kindMatchesFilter(kind, f),
      `kindMatchesFilter(${kind}, ${f}) 应为 false（kind 只能归 ${expected}）`
    );
  }
}
for (const kind of Object.keys(KIND_TO_FILTER)) {
  assert(kindMatchesFilter(kind, "all"), `all 过滤器必须放行 ${kind}`);
}

// ================= 2) store：实时事件入库 + 本地容量镜像 =================
setActivePinia(createPinia());
const res = useResourceStore();

await res.loadSettings();
assert(calls.get === 1, "loadSettings 必须调用 getResourceCaptureSettings");
assert(res.enabled === true, "默认采集开启");
assert(res.maxPerTab === 200, "默认每 tab 容量 200");

res.applyReceived(rec("r1", "document"));
res.applyReceived(rec("r2", "script"));
res.applyReceived(rec("r3", "image", "tab-2"));
assert(res.recordsOf("tab-1").length === 2, "applyReceived 按 tab 入库");
assert(res.recordsOf("tab-2").length === 1, "不同 tab 记录隔离");

// 本地容量镜像：max_per_tab=2，推 4 条 → 保留最新 2 条 + evicted=2
res.settings = { enabled: true, max_per_tab: 2, max_total: 2000, max_url_bytes: 2048 };
res.applyReceived(rec("r4", "script"));
res.applyReceived(rec("r5", "script"));
assert(res.recordsOf("tab-1").length === 2, "本地镜像守住 max_per_tab=2");
assert(
  res.recordsOf("tab-1").map((r) => r.id).join(",") === "r4,r5",
  "本地镜像 FIFO 丢最旧（r1/r2 被丢弃）"
);
assert(res.evictedOf("tab-1") === 2, "本地镜像驱逐计数 evicted=2");

// ================= 3) loadTab：后端为权威数据源 =================
bridge.listTabResources = async (tabId) => {
  calls.list.push(tabId);
  return {
    records: [rec("s1", "document", tabId), rec("s2", "xhr_fetch", tabId)],
    evicted: 7,
    enabled: false,
  };
};
await res.loadTab("tab-9");
assert(
  res.recordsOf("tab-9").map((r) => r.id).join(",") === "s1,s2",
  "loadTab 以后端记录为权威并替换本地"
);
assert(res.evictedOf("tab-9") === 7, "loadTab 同步后端 evicted 计数");
assert(res.enabled === false, "loadTab 同步后端 enabled 状态（settings 已加载时）");

// ================= 4) 筛选应用（filteredOf） =================
res.filter = "xhr";
assert(
  res.filteredOf("tab-9").map((r) => r.id).join(",") === "s2",
  "xhr 过滤器只保留 xhr_fetch 记录"
);
res.filter = "all";
assert(res.filteredOf("tab-9").length === 2, "all 过滤器返回全部记录");

// ================= 5) clear 逻辑（REQUIRE_FRONTEND #4） =================
await res.clearTab("tab-9");
assert(calls.clear.join(",") === "tab-9", "clearTab 必须调用后端 clear_tab_resources");
assert(res.recordsOf("tab-9").length === 0, "clearTab 后本地记录清空");
assert(res.evictedOf("tab-9") === 0, "clearTab 后驱逐计数复位");

// ================= 6) 采集开关（REQUIRE_FRONTEND #5） =================
await res.setEnabled(false);
assert(calls.set.length === 1 && calls.set[0][0] === false, "setEnabled(false) 调后端开关");
assert(res.enabled === false, "开关关闭后 enabled=false");
await res.setEnabled(true);
assert(res.enabled === true, "开关重新打开后 enabled=true");

// ================= 7) 降级字段展示不伪造（REQUIRE_FRONTEND #7） =================
assert(formatSize(null) === "-", "size 未知必须显示 '-'，不得伪造为 0");
assert(formatDuration(null) === "-", "duration 未知必须显示 '-'，不得伪造为 0");
assert(formatSize(512) === "512 B", "formatSize 字节级展示");
assert(formatSize(2048) === "2.0 KB", "formatSize KB 级展示");
assert(formatDuration(1500) === "1.50 s", "formatDuration 秒级展示");

// ================= 8) 脱敏 URL 展示 =================
const shown = displayUrl("https://user:pw@example.com/p?token=***&ok=1");
assert(!shown.includes("user:pw"), "displayUrl 展示不得含 userinfo");
assert(shown.includes("token=***"), "displayUrl 保留已脱敏的 *** 形态");
assert(shown.includes("example.com/p"), "displayUrl 展示 host+path");
assert(displayUrl("not a url") === "not a url", "解析失败原样返回（原样串已是脱敏形态）");

// ================= 汇总 =================
console.log(`\nassertions passed: ${passed}`);
if (failures.length) {
  console.error(`assertions failed: ${failures.length}`);
  process.exit(1);
}
console.log("RESOURCE_UI_LOGIC_RESULT=ALL_PASS");
