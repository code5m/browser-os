#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M4-8（定时任务 UI）前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/utils/taskUi.ts` 与 `src/stores/useTaskStore.ts`，
// 只把 `src/bridge.ts` 的 task_* / script* / snippet* 方法替换为记录型 mock
// （不 mock 逻辑层与 store 自身）：每条断言反映的都是**产品代码**的行为。
//
// 契约源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`
//   - §3.1 TaskDef / TaskTrigger / RetryPolicy / TaskRunRecord
//   - §3.2 catch_up_limit 仅 CatchUp 生效
//   - §3.3 R-1~R-6 定义期校验（稳定错误码）
//   - §3.5 R-A6-1 新建任务默认 enabled = false
//   - §4.4 cron 仅 5 段；6 段（含秒）、宏一律拒绝（O-A6-4）
//   - §5.2 跳过原因必须可展示（O-A6-9）
//   - §5.3 重试 delay 必须 clamp 到 max_delay_secs
//
// 用法: node scripts/check-scheduler-ui-logic.mjs
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
  return globalThis.__m48Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m48Resolve = resolveWithExt;
}

// ---------- 最小浏览器桩 ----------
globalThis.window = {
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  clearTimeout: (h) => clearTimeout(h),
  setInterval: () => 0,
  clearInterval: () => {},
};
globalThis.localStorage = {
  getItem: () => null,
  setItem: () => {},
  removeItem: () => {},
};

const ROOT = new URL("..", import.meta.url).pathname;

const { bridge } = await import(`${ROOT}src/bridge.ts`);
const taskUi = await import(`${ROOT}src/utils/taskUi.ts`);
const { useTaskStore } = await import(`${ROOT}src/capabilities/task/state/useTaskStore.ts`);
const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);

const {
  CRON_FIELD_COUNT,
  DEFAULT_TASK_ENABLED,
  MIN_INTERVAL_SECS,
  MAX_INTERVAL_SECS,
  MAX_CATCH_UP_LIMIT,
  MAX_ATTEMPTS,
  MAX_TASK_NAME_BYTES,
  MAX_TASKS,
  DEFAULT_CATCH_UP_LIMIT,
  emptyTaskDraft,
  loadTaskDraft,
  serializeTaskDraft,
  validateCron,
  validateIntervalSecs,
  validateTaskDraft,
  isCatchUpLimitEditable,
  retryDelaySeconds,
  retrySummary,
  nextRunState,
  lastRunLabel,
  formatRelative,
  formatTrigger,
  sortRuns,
  runsByTask,
  latestRun,
  skipReasonLabel,
  triggerLabelOf,
} = taskUi;

// ---------- mock bridge（只替换被调用的方法，并记录调用） ----------
let calls = [];
function resetCalls() {
  calls = [];
}
bridge.taskList = async () => {
  calls.push(["task_list"]);
  return [];
};
bridge.taskAdd = async (p) => {
  calls.push(["task_add", p]);
  return { id: "new-id", ...p, created_at: "2026-01-01T00:00:00Z", updated_at: "2026-01-01T00:00:00Z" };
};
bridge.taskUpdate = async (p) => {
  calls.push(["task_update", p]);
  return p;
};
bridge.taskRemove = async (id) => {
  calls.push(["task_remove", id]);
  return true;
};
bridge.taskRunNow = async (id) => {
  calls.push(["task_run_now", id]);
  return { run_id: "run-1", script_id: id, status: "running" };
};
bridge.scriptList = async () => {
  calls.push(["script_list"]);
  return [
    {
      id: "scr-1",
      name: "清理日志",
      category: "ops",
      path: "clean.sh",
      interpreter: "bash",
      params: [
        { name: "DAYS", label: "天数", param_type: "number", required: true, default: null, options: [], raw: false, secret: false },
        { name: "TOKEN", label: "令牌", param_type: "string", required: true, default: null, options: [], raw: false, secret: true },
      ],
      description: "",
      builtin: false,
      enabled: true,
      timeout_secs: 0,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    },
  ];
};
bridge.snippetList = async () => {
  calls.push(["snippet_list"]);
  return [
    {
      id: "cmd-1",
      name: "磁盘占用",
      category: "disk",
      interpreter: "bash",
      argv: ["df", "-h"],
      params: [],
      description: "",
      dangerous: true,
      builtin: false,
      enabled: true,
      timeout_secs: 0,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    },
  ];
};

let passed = 0;
const failures = [];
function assert(cond, label) {
  if (cond) {
    passed += 1;
  } else {
    failures.push(label);
    console.error(`  ✗ ${label}`);
  }
}

setActivePinia(createPinia());
const store = useTaskStore();

// ===========================================================================
// 1. cron 方言（A6 §4.4 / O-A6-4）
// ===========================================================================

assert(validateCron("*/15 * * * *") === null, "C-1 合法：*/15 * * * *");
assert(validateCron("0 9 * * 1-5") === null, "C-2 合法：0 9 * * 1-5");
assert(validateCron("0 0 1,15 * *") === null, "C-3 合法：0 0 1,15 * *");
assert(validateCron("0 0 * * 0") === null, "C-4 合法：周日 0");
assert(validateCron("0 0 * * 7") === null, "C-5 合法：周日 7");

const sixField = validateCron("*/15 * * * * *");
assert(typeof sixField === "string" && sixField !== null, "C-6 拒绝 6 段（含秒）");
assert(
  typeof sixField === "string" && sixField.includes(String(CRON_FIELD_COUNT)),
  "C-7 6 段拒绝信息点明需要 5 段（O-A6-4：cron 工具的 6 位模式不兼容）"
);
assert(validateCron("@daily") !== null, "C-8 拒绝宏 @daily");
assert(validateCron("@reboot") !== null, "C-9 拒绝宏 @reboot");
assert(validateCron("") !== null, "C-10 拒绝空表达式");
assert(validateCron("60 * * * *") !== null, "C-11 拒绝分 = 60");
assert(validateCron("0 0 0 * *") !== null, "C-12 拒绝日 = 0");
assert(validateCron("*/0 * * * *") !== null, "C-13 拒绝步长 0");
assert(validateCron("1-10/2 * * * *") !== null, "C-14 拒绝范围步长");
assert(validateCron("a * * * *") !== null, "C-15 拒绝非数字");

// ===========================================================================
// 2. 间隔边界（A6 F-A6-6）
// ===========================================================================

assert(validateIntervalSecs(MIN_INTERVAL_SECS - 1) !== null, "I-1 拒绝 59 秒（下界 60）");
assert(validateIntervalSecs(MIN_INTERVAL_SECS) === null, "I-2 接受 60 秒");
assert(validateIntervalSecs(MAX_INTERVAL_SECS) === null, "I-3 接受 30 天");
assert(validateIntervalSecs(MAX_INTERVAL_SECS + 1) !== null, "I-4 拒绝超过 30 天");
assert(validateIntervalSecs(30.5) !== null, "I-5 拒绝非整数秒");

// ===========================================================================
// 3. 表单态与默认值（R-A6-1）
// ===========================================================================

const draft = emptyTaskDraft();
assert(draft.enabled === false, "D-1 新建任务默认 enabled = false（R-A6-1）");
assert(DEFAULT_TASK_ENABLED === false, "D-2 DEFAULT_TASK_ENABLED 常量为 false");
assert(draft.missed_run_policy === "skip", "D-3 默认错过策略 = skip");
assert(draft.catch_up_limit === DEFAULT_CATCH_UP_LIMIT, `D-4 默认补跑上限 = ${DEFAULT_CATCH_UP_LIMIT}`);
assert(draft.retry_max_attempts === 1, "D-5 默认不重试（max_attempts = 1）");

// ===========================================================================
// 4. 定义期校验（A6 §3.3 R-1~R-6）
// ===========================================================================

const targets = [
  {
    id: "scr-1",
    name: "清理日志",
    kind: "script",
    dangerous: false,
    params: [
      { name: "DAYS", label: "天数", param_type: "number", required: true, default: null, options: [], raw: false, secret: false },
      { name: "TOKEN", label: "令牌", param_type: "string", required: true, default: null, options: [], raw: false, secret: true },
    ],
  },
  { id: "cmd-1", name: "磁盘占用", kind: "command", dangerous: true, params: [] },
];

function draftWith(over = {}) {
  const d = emptyTaskDraft();
  d.name = "每晚清理";
  d.kind = "script";
  d.target_id = "scr-1";
  d.params = { DAYS: "7" };
  return Object.assign(d, over);
}

function codesOf(issues) {
  return issues.map((i) => i.code);
}

const okIssues = validateTaskDraft(draftWith(), { targets, taskCount: 0 });
assert(okIssues.length === 0, `V-1 合法草稿零问题（实得 ${JSON.stringify(codesOf(okIssues))}）`);

assert(
  codesOf(validateTaskDraft(draftWith({ name: "  " }), { targets, taskCount: 0 })).includes("TASK_PARAM_INVALID"),
  "V-2 空名称 → TASK_PARAM_INVALID"
);
assert(
  codesOf(
    validateTaskDraft(draftWith({ name: "x".repeat(MAX_TASK_NAME_BYTES + 1) }), { targets, taskCount: 0 })
  ).includes("TASK_NAME_TOO_LONG"),
  "V-3 超长名称 → TASK_NAME_TOO_LONG（按 UTF-8 字节计）"
);
assert(
  codesOf(validateTaskDraft(draftWith({ target_id: "" }), { targets, taskCount: 0 })).includes(
    "TASK_TARGET_NOT_FOUND"
  ),
  "V-4 未选目标 → TASK_TARGET_NOT_FOUND"
);
assert(
  codesOf(validateTaskDraft(draftWith({ target_id: "nope" }), { targets, taskCount: 0 })).includes(
    "TASK_TARGET_NOT_FOUND"
  ),
  "V-5 目标不存在 → TASK_TARGET_NOT_FOUND"
);
assert(
  codesOf(validateTaskDraft(draftWith({ kind: "command" }), { targets, taskCount: 0 })).includes(
    "TASK_TARGET_NOT_FOUND"
  ),
  "V-6 目标类型不匹配（command 指向脚本 id）→ TASK_TARGET_NOT_FOUND"
);
assert(
  codesOf(
    validateTaskDraft(draftWith({ params: { DAYS: "7", TOKEN: "abc" } }), { targets, taskCount: 0 })
  ).includes("TASK_SECRET_PARAM_FORBIDDEN"),
  "V-7 secret 参数值 → TASK_SECRET_PARAM_FORBIDDEN（R-3：不落 tasks.json）"
);
assert(
  codesOf(validateTaskDraft(draftWith({ params: {} }), { targets, taskCount: 0 })).includes(
    "TASK_PARAM_INVALID"
  ),
  "V-8 缺必填参数 → TASK_PARAM_INVALID"
);
assert(
  codesOf(
    validateTaskDraft(draftWith({ params: { DAYS: "7", NOPE: "1" } }), { targets, taskCount: 0 })
  ).includes("TASK_PARAM_INVALID"),
  "V-9 未知参数键 → TASK_PARAM_INVALID"
);
assert(
  codesOf(validateTaskDraft(draftWith({ cron_expr: "*/1 * * * * *" }), { targets, taskCount: 0 })).includes(
    "TASK_INVALID_CRON"
  ),
  "V-10 非法 cron → TASK_INVALID_CRON"
);
assert(
  codesOf(
    validateTaskDraft(draftWith({ trigger_kind: "interval", interval_secs: 10 }), { targets, taskCount: 0 })
  ).includes("TASK_INTERVAL_OUT_OF_RANGE"),
  "V-11 间隔越界 → TASK_INTERVAL_OUT_OF_RANGE"
);
for (const bad of [0, MAX_CATCH_UP_LIMIT + 1]) {
  assert(
    codesOf(validateTaskDraft(draftWith({ catch_up_limit: bad }), { targets, taskCount: 0 })).includes(
      "TASK_PARAM_INVALID"
    ),
    `V-12 补跑上限 ${bad} 越界 → TASK_PARAM_INVALID`
  );
}
assert(
  codesOf(validateTaskDraft(draftWith({ timeout_secs: 601 }), { targets, taskCount: 0 })).includes(
    "TASK_PARAM_INVALID"
  ),
  "V-13 超时 601 → TASK_PARAM_INVALID"
);
assert(
  codesOf(validateTaskDraft(draftWith({ retry_max_attempts: MAX_ATTEMPTS + 1 }), { targets, taskCount: 0 })).includes(
    "TASK_PARAM_INVALID"
  ),
  "V-14 重试次数越界 → TASK_PARAM_INVALID"
);
assert(
  codesOf(validateTaskDraft(draftWith(), { targets, taskCount: MAX_TASKS })).includes("TASK_LIMIT_REACHED"),
  "V-15 新建且已达上限 → TASK_LIMIT_REACHED"
);
assert(
  !codesOf(validateTaskDraft(draftWith({ id: "t-1" }), { targets, taskCount: MAX_TASKS })).includes(
    "TASK_LIMIT_REACHED"
  ),
  "V-16 编辑已有任务不受总数上限约束"
);

// ===========================================================================
// 5. catch_up_limit 生效范围（A6 §3.2）
// ===========================================================================

assert(isCatchUpLimitEditable("catch_up") === true, "M-1 CatchUp 策略下补跑上限可编辑");
assert(isCatchUpLimitEditable("skip") === false, "M-2 Skip 策略下置灰");
assert(isCatchUpLimitEditable("run_once") === false, "M-3 RunOnce 策略下置灰");

// ===========================================================================
// 6. 序列化（存储态）
// ===========================================================================

const serCron = serializeTaskDraft(draftWith({ cron_expr: "*/15 * * * *" }));
assert(
  JSON.stringify(serCron.trigger) === JSON.stringify({ cron: { expr: "*/15 * * * *" } }),
  "S-1 cron 序列化为 { cron: { expr } }"
);
const serInterval = serializeTaskDraft(
  draftWith({ trigger_kind: "interval", interval_secs: 3600 })
);
assert(
  JSON.stringify(serInterval.trigger) === JSON.stringify({ interval: { every_secs: 3600 } }),
  "S-2 interval 序列化为 { interval: { every_secs } }"
);
assert(serCron.retry.max_attempts === 1, "S-3 默认重试 1 次");
assert(
  Object.prototype.hasOwnProperty.call(serCron, "id") === false &&
    Object.prototype.hasOwnProperty.call(serCron, "next_run_at") === false,
  "S-4 提交载荷不含 id / 时间戳（由服务端生成与维护）"
);
const src = draftWith();
const copied = serializeTaskDraft(src);
src.params.DAYS = "999";
assert(copied.params.DAYS === "7", "S-5 序列化对 params 做拷贝（后续改动不污染已提交载荷）");

const roundTripDef = {
  id: "t-1",
  name: "每晚清理",
  kind: "script",
  target_id: "scr-1",
  params: { DAYS: "7" },
  enabled: true,
  trigger: { interval: { every_secs: 3600 } },
  missed_run_policy: "catch_up",
  catch_up_limit: 5,
  misfire_grace_secs: 120,
  retry: { max_attempts: 3, backoff: "exponential", base_delay_secs: 30, max_delay_secs: 300 },
  timeout_secs: 120,
  last_fired_at: "2026-01-01T00:00:00Z",
  next_run_at: "2026-01-01T01:00:00Z",
  created_at: "2025-12-31T00:00:00Z",
  updated_at: "2025-12-31T00:00:00Z",
};
const rt = serializeTaskDraft(loadTaskDraft(roundTripDef));
assert(
  rt.kind === roundTripDef.kind &&
    rt.target_id === roundTripDef.target_id &&
    rt.missed_run_policy === "catch_up" &&
    rt.catch_up_limit === 5 &&
    rt.misfire_grace_secs === 120 &&
    rt.timeout_secs === 120 &&
    rt.enabled === true &&
    JSON.stringify(rt.trigger) === JSON.stringify({ interval: { every_secs: 3600 } }),
  "S-6 TaskDef → 表单态 → 提交态 往返无损"
);

// ===========================================================================
// 7. 重试（A6 §5.3）
// ===========================================================================

const expo = { max_attempts: 5, backoff: "exponential", base_delay_secs: 30, max_delay_secs: 300 };
assert(retryDelaySeconds(expo, 1) === 0, "R-1 首次执行无延迟");
assert(retryDelaySeconds(expo, 2) === 30, "R-2 第 2 次 = base");
assert(retryDelaySeconds(expo, 3) === 60, "R-3 第 3 次 = base * 2");
assert(retryDelaySeconds(expo, 4) === 120, "R-4 第 4 次 = base * 4");
// 30 * 2^4 = 480 > max 300 → 必须 clamp（A6 §5.3）
assert(retryDelaySeconds(expo, 6) === 300, "R-5 指数退避必须 clamp 到 max_delay_secs");
const fixed = { max_attempts: 3, backoff: "fixed", base_delay_secs: 45, max_delay_secs: 20 };
assert(retryDelaySeconds(fixed, 2) === 20, "R-6 固定间隔同样 clamp 到 max_delay_secs");
assert(retrySummary({ ...expo, max_attempts: 1 }).includes("不重试"), "R-7 1 次 = 不重试");
assert(
  retrySummary({ ...expo, max_attempts: 3 }).includes("3") &&
    retrySummary({ ...expo, max_attempts: 3 }).includes("指数退避"),
  "R-8 重试摘要含次数与退避方式"
);

// ===========================================================================
// 8. 时间展示
// ===========================================================================

const NOW = Date.parse("2026-01-01T00:00:00Z");
assert(
  nextRunState({ enabled: false, next_run_at: "2026-01-01T01:00:00Z" }, NOW).kind === "disabled",
  "T-1 未启用 → disabled（不展示下次执行）"
);
assert(nextRunState({ enabled: true, next_run_at: null }, NOW).kind === "none", "T-2 无计划 → none");
assert(
  nextRunState({ enabled: true, next_run_at: "2025-12-31T23:00:00Z" }, NOW).kind === "stale",
  "T-3 计划已过期 → stale（等后端重算，前端不自造时间）"
);
const future = nextRunState({ enabled: true, next_run_at: "2026-01-01T01:00:00Z" }, NOW);
assert(future.kind === "scheduled" && future.label.includes("后"), "T-4 未来计划 → 相对时间");
assert(lastRunLabel({ last_fired_at: null }, NOW) === "从未执行", "T-5 从未执行");
assert(lastRunLabel({ last_fired_at: "2025-12-31T23:00:00Z" }, NOW).includes("前"), "T-6 上次执行用过去时");
assert(formatRelative(null, NOW) === "—", "T-7 空时间 → —");
assert(formatRelative("not-a-date", NOW) === "—", "T-8 非法时间 → —（不抛错、不显示 NaN）");
assert(formatTrigger({ cron: { expr: "*/15 * * * *" } }).includes("*/15"), "T-9 cron 展示含原表达式");
assert(formatTrigger({ interval: { every_secs: 3600 } }).includes("小时"), "T-10 间隔按小时归并展示");

// ===========================================================================
// 9. 历史与跳过原因（O-A6-9）
// ===========================================================================

const runs = [
  { task_id: "t1", run_id: "r1", trigger: "scheduled", scheduled_at: "2026-01-01T00:00:00Z", attempt: 1, started_at: "2026-01-01T00:00:00Z", finished_at: "2026-01-01T00:00:10Z", status: "failed", exit_code: 1, error_code: null },
  { task_id: "t2", run_id: "r2", trigger: "manual", scheduled_at: "2026-01-01T00:05:00Z", attempt: 1, started_at: "2026-01-01T00:05:00Z", finished_at: "2026-01-01T00:05:05Z", status: "succeeded", exit_code: 0, error_code: null },
  { task_id: "t1", run_id: "r3", trigger: "retry", scheduled_at: "2026-01-01T00:00:00Z", attempt: 2, started_at: "2026-01-01T00:10:00Z", finished_at: null, status: "running", exit_code: null, error_code: null },
];
const sorted = sortRuns(runs);
assert(sorted[0].run_id === "r3" && sorted[2].run_id === "r1", "H-1 历史按开始时间倒序");
assert(runs.length === 3 && sorted.length === 3, "H-2 排序不修改入参");
assert(runsByTask(runs)["t1"].length === 2, "H-3 按任务分组");
assert((latestRun(runs, "t1") || {}).run_id === "r3", "H-4 latestRun 取最新一条");
assert(latestRun(runs, "nope") === null, "H-5 无记录 → null");
assert(skipReasonLabel("reentrant").length > 0, "H-6 跳过原因 reentrant 有文案");
assert(skipReasonLabel("target_busy").includes("占用"), "H-7 跳过原因 target_busy 有文案（O-A6-9）");
assert(skipReasonLabel("global_limit").length > 0, "H-8 跳过原因 global_limit 有文案");
assert(skipReasonLabel(null).length > 0, "H-9 未知原因也有兜底文案");
assert(
  triggerLabelOf("scheduled") === "定时" &&
    triggerLabelOf("manual") === "手工" &&
    triggerLabelOf("catch_up") === "补跑" &&
    triggerLabelOf("retry") === "重试",
  "H-10 四种触发来源均有中文标签"
);

// ===========================================================================
// 10. store：后端未就绪时零 invoke（LIMITED START 口径）
//   A7 已落地 5 条命令，flag 默认解锁；此处显式切回未就绪态，验证降级路径
//   （只读壳契约仍成立，不假借未实现的命令名假装可用）。
// ===========================================================================

store.backendReady = false;
assert(store.backendReady === false, "B-1 可显式切回未就绪态（降级路径保留）");
resetCalls();
await store.loadAll();
assert(calls.length === 0, `B-2 未就绪时 loadAll 零 invoke（实得 ${calls.length}）`);
assert(typeof store.error === "string" && store.error.length > 0, "B-3 未就绪时给出明确错误文案");

await store.openCreate();
store.draft.name = "未就绪保存";
store.draft.target_id = "scr-1";
await store.saveDraft();
assert(calls.length === 0, "B-4 未就绪时 saveDraft 零 invoke");

await store.setEnabled({ id: "t-1", name: "x" }, true);
await store.removeTask("t-1");
await store.runNow("t-1");
assert(calls.length === 0, "B-5 未就绪时 setEnabled / removeTask / runNow 全部零 invoke");
store.closeEditor();

// ===========================================================================
// 11. store：后端就绪后的接线（mock bridge）
// ===========================================================================

store.backendReady = true;
resetCalls();
await store.loadAll();
assert(calls.some(([n]) => n === "task_list"), "L-1 就绪后 loadAll 调 task_list");
assert(calls.some(([n]) => n === "script_list") && calls.some(([n]) => n === "snippet_list"), "L-2 一并加载脚本与命令片段作目标候选");
assert(store.targets.length === 2, "L-3 目标候选含脚本与命令片段");
assert(store.targets.some((t) => t.dangerous === true), "L-4 高风险目标被标记（供二次确认）");

// 复合返回形态：{ tasks, runs }
resetCalls();
bridge.taskList = async () => ({ tasks: [roundTripDef], runs });
await store.loadAll();
assert(store.tasks.length === 1 && store.runs.length === 3, "L-5 支持 { tasks, runs } 复合返回");
resetCalls();
bridge.taskList = async () => [roundTripDef];
await store.loadAll();
assert(store.tasks.length === 1, "L-6 支持纯数组返回");

// 新建：默认关闭、secret 不入参
resetCalls();
store.openCreate();
store.draft.name = "每晚清理";
store.draft.kind = "script";
store.draft.target_id = "scr-1";
store.draft.params = { DAYS: "7" };
const saved = await store.saveDraft();
assert(saved === true, "L-7 合法草稿保存成功");
const addCall = calls.find(([n]) => n === "task_add");
assert(Boolean(addCall), "L-8 新建走 task_add");
assert(addCall && addCall[1].enabled === false, "L-9 新建任务提交时 enabled = false（R-A6-1 端到端）");
assert(addCall && !Object.prototype.hasOwnProperty.call(addCall[1].params, "TOKEN"), "L-10 secret 参数不出现在提交载荷");
// DTO 形态：task_add 是平铺 camelCase 入参（Tauri 默认转换），多词键必须 camelCase。
assert(addCall && addCall[1].targetId === "scr-1", "L-10a task_add 入参使用 camelCase 键 targetId");
assert(
  addCall && addCall[1].catchUpLimit !== undefined && addCall[1].misfireGraceSecs !== undefined &&
    addCall[1].timeoutSecs !== undefined && addCall[1].missedRunPolicy !== undefined,
  "L-10b task_add 入参多词键均为 camelCase（catchUpLimit/misfireGraceSecs/timeoutSecs/missedRunPolicy）"
);
assert(
  addCall && addCall[1].target_id === undefined && addCall[1].catch_up_limit === undefined &&
    addCall[1].misfire_grace_secs === undefined && addCall[1].timeout_secs === undefined,
  "L-10c task_add 入参不含 snake_case 多词键（避免被 Tauri 转换吞掉）"
);

// 校验失败时不得发出 task_add
resetCalls();
store.openCreate();
store.draft.name = "";
store.draft.target_id = "scr-1";
const blocked = await store.saveDraft();
assert(blocked === false, "L-11 校验失败 → 保存被拒");
assert(calls.length === 0, "L-12 校验失败时零 invoke（前端前置校验生效）");
store.closeEditor();

// 启停 / 立即执行 / 删除
resetCalls();
await store.setEnabled(roundTripDef, false);
const updCall = calls.find(([n]) => n === "task_update");
assert(updCall && updCall[1].enabled === false, "L-13 停用走 task_update 且传 enabled=false");
// 注：task_update 的 { task: TaskDef } 单结构体包装属 bridge.ts 职责（Tauri 收 camelCase/
// snake_case 转换），由 policy 夹具 SCHEDUI_TASK_UPDATE_WRAPPED 守护；本层只验证
// store 确实把合并后的 TaskDef（含 enabled）交给了 bridge。

resetCalls();
await store.runNow("t-1");
assert(calls.some(([n]) => n === "task_run_now"), "L-14 立即执行走 task_run_now（不自建执行通道）");

resetCalls();
store.tasks = [roundTripDef];
await store.removeTask("t-1");
assert(calls.some(([n]) => n === "task_remove"), "L-15 删除走 task_remove");
assert(store.tasks.length === 0, "L-16 删除后本地列表同步剔除");

await store.loadTargets();
assert(store.isDangerous({ ...roundTripDef, kind: "command", target_id: "cmd-1" }) === true, "L-17 高风险命令片段可被识别");
assert(store.isDangerous(roundTripDef) === false, "L-18 普通脚本不误判为高风险");

// ---------------- 汇总 ----------------
console.log(`\ncheck-scheduler-ui-logic: ${passed} assertions passed, ${failures.length} failed`);
if (failures.length > 0) {
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
process.exit(0);
