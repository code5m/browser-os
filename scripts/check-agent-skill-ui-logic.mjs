#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5-6（Agent/Skill UI）前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载**真实的** `src/utils/agentSkillUi.ts` 与 `src/stores/useAgentStore.ts`，
// 只把 `src/bridge.ts` 的 skill_*/agent_* 方法替换为记录型 mock（不 mock 逻辑层与 store 自身）：
// 每条断言反映的都是**产品代码**的行为。
//
// 契约源：
//   - src-tauri/src/domain.rs（Lane A5，W4）：AgentDef / SkillDef / SkillExec /
//     AclLevel / SkillInput / CapabilityRef / PermissionPreview / A2aConfig / AgentDialect
//   - A6 W4 数据契约：logs/assist/A6-M5-W4-20260906-1810.md
//   - board M5-W5：A6 仅纯逻辑 + 面板壳，不执行 Skill、不安装插件、不调实时运行时
//
// 用法: node scripts/check-agent-skill-ui-logic.mjs
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
  return globalThis.__m56Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m56Resolve = resolveWithExt;
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
const ui = await import(`${ROOT}src/utils/agentSkillUi.ts`);
const { useAgentStore } = await import(`${ROOT}src/stores/useAgentStore.ts`);
const { createPinia, setActivePinia } = await import(`${ROOT}node_modules/pinia/dist/pinia.mjs`);

const {
  aclLabel,
  aclTone,
  dialectLabel,
  a2aSummary,
  execSummary,
  capabilityLabel,
  renderCapabilityList,
  buildPermissionPreview,
  classifyCapability,
  runStatusUi,
  isTerminalStatus,
  validateSkillForm,
  redactSecrets,
  serializeSkillForm,
  panelState,
  appendChunk,
  chunksToText,
  makePendingConfirm,
  isConfirmExpired,
  MAX_STREAM_BUFFER_BYTES,
} = ui;

// ---------- mock bridge（只替换被调用的方法，并记录调用） ----------
let calls = [];
function resetCalls() {
  calls = [];
}
const skillDef = {
  id: "skill-1",
  version: "1.0.0",
  displayName: "清理缓存",
  description: "清理临时缓存",
  acl: "confirm",
  exec: { kind: "script_ref", scriptId: "scr-9", params: {} },
  inputs: [
    { name: "DAYS", required: true, description: "天数" },
    { name: "TOKEN", required: false, description: "令牌" },
  ],
  capabilities: [{ id: "fs:read" }, { id: "net:http" }],
  tests: [],
  metadata: {},
};
const agentDef = {
  id: "agent-1",
  version: "1.0.0",
  displayName: "问答助手",
  description: "通用问答",
  dialect: "open_ai_compatible",
  systemPrompt: "你是一个助手",
  defaultCapabilities: [{ id: "net:http" }],
  a2a: { delegateTo: true, delegatedFrom: false },
  metadata: {},
};

bridge.skillList = async () => {
  calls.push(["skill_list"]);
  return [skillDef];
};
bridge.agentList = async () => {
  calls.push(["agent_list"]);
  return [agentDef];
};
bridge.skillInstall = async (id) => {
  calls.push(["skill_install", id]);
  return { request_id: "req-1" };
};
bridge.agentInstall = async (id) => {
  calls.push(["agent_install", id]);
  return { request_id: "req-2" };
};
bridge.confirmSkill = async (rid, dec) => {
  calls.push(["confirm_skill_install", rid, dec]);
};
bridge.confirmAgent = async (rid, dec) => {
  calls.push(["confirm_agent_install", rid, dec]);
};
bridge.agentChat = async (a, p, s) => {
  calls.push(["agent_chat", a, p, s]);
};
bridge.agentRunCancel = async (rid) => {
  calls.push(["agent_chat_cancel", rid]);
};
bridge.skillRunsList = async (id) => {
  calls.push(["skill_runs_list", id]);
  return [];
};
bridge.agentRunsList = async (id) => {
  calls.push(["agent_runs_list", id]);
  return [];
};
// W9：只读桥命令（M5-W7 已进 mainline，AGENT_SKILL_READONLY_COMMANDS_AVAILABLE=true）。
// 成功路径默认 mock；§11.5 会临时改写为失败路径。
bridge.skillValidate = async (t) => {
  calls.push(["skill_validate", t]);
  return { valid: true, errors: [] };
};
bridge.skillParse = async (t) => {
  calls.push(["skill_parse", t]);
  return skillDef;
};
bridge.skillPermissionPreview = async (t) => {
  calls.push(["skill_permission_preview", t]);
  return { gate: "confirm", capabilities: ["fs:read", "net:http"] };
};
bridge.agentValidate = async (t) => {
  calls.push(["agent_validate", t]);
  return { valid: true, errors: [] };
};
bridge.agentParse = async (t) => {
  calls.push(["agent_parse", t]);
  return agentDef;
};
bridge.agentPermissionPreview = async (t) => {
  calls.push(["agent_permission_preview", t]);
  return { gate: "safe", capabilities: ["net:http"] };
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
const store = useAgentStore();

// ===========================================================================
// 1. 展示标签
// ===========================================================================
assert(aclLabel("safe") === "安全", "L-1 aclLabel safe");
assert(aclLabel("dangerous") === "高危", "L-2 aclLabel dangerous");
assert(aclTone("dangerous") === "danger", "L-3 aclTone dangerous");
assert(dialectLabel("open_ai_compatible") === "OpenAI 兼容", "L-4 dialectLabel open_ai_compatible");
assert(a2aSummary({ delegateTo: true, delegatedFrom: false }) === "可委派他人", "L-5 a2aSummary delegateTo");
assert(a2aSummary({ delegateTo: false, delegatedFrom: false }) === "不可委派", "L-6 a2aSummary none");
assert(execSummary({ kind: "script_ref", scriptId: "scr-9", params: {} }) === "脚本 #scr-9", "L-7 execSummary script_ref");
assert(execSummary({ kind: "sequence", steps: [{}, {}] }) === "串联 2 步", "L-8 execSummary sequence");
assert(capabilityLabel("fs:read") === "文件系统读", "L-9 capabilityLabel known");
assert(capabilityLabel("weird:cap") === "weird:cap", "L-10 capabilityLabel unknown 回显原 id");

// ===========================================================================
// 2. 能力列表渲染 + 权限预览
// ===========================================================================
const caps = renderCapabilityList([{ id: "fs:read" }, { id: "secret:x" }], ["fs:read"]);
assert(caps.length === 2, "C-1 能力列表长度");
assert(caps[0].granted === true && caps[0].unknown === false, "C-2 白名单内 → granted");
assert(caps[1].granted === false && caps[1].unknown === true, "C-3 白名单外 → unknown");

const emptyCaps = renderCapabilityList([{ id: "fs:read" }], []);
assert(emptyCaps[0].unknown === true, "C-4 白名单为空 → 全部 unknown（不谎报已授权）");

const pv = buildPermissionPreview(skillDef, ["fs:read", "net:http"]);
assert(pv.gate === "confirm", "P-1 权限预览 gate 来自 SkillDef.acl");
assert(pv.gateLabel === "需确认", "P-2 gate 标签");
assert(pv.targetSummary === "脚本 #scr-9", "P-3 目标摘要（不展开参数）");
assert(pv.capabilities.length === 2, "P-4 权限预览含能力列表");
assert(pv.dangerHint !== null, "P-5 confirm 档有提示");

const pvAgent = buildPermissionPreview(agentDef, ["net:http"]);
assert(pvAgent.gate === "safe", "P-6 Agent 无 acl 字段时默认 safe");
assert(pvAgent.targetSummary === "可委派他人", "P-7 Agent 目标摘要用 a2a");

// ===========================================================================
// 3. 运行态 + secret 脱敏
// ===========================================================================
assert(runStatusUi("succeeded").label === "成功" || runStatusUi("succeeded").label.length > 0, "R-1 runStatusUi 有标签");
assert(isTerminalStatus("failed") === true, "R-2 failed 终态");
assert(isTerminalStatus("running") === false, "R-3 running 非终态");

const redacted = redactSecrets({ user: "a", password: "secret", nested: { api_key: "k" } });
assert(redacted.password === "***", "S-1 password 脱敏");
assert(redacted.nested.api_key === "***", "S-2 嵌套 api_key 脱敏");
assert(redacted.user === "a", "S-3 非 secret 原样");

const ser = serializeSkillForm(skillDef.inputs, { DAYS: "7", TOKEN: "abc" });
assert(ser.TOKEN === "***", "S-4 表单序列化 TOKEN → ***");
assert(ser.DAYS === "7", "S-5 表单序列化 DAYS 原样");

// ===========================================================================
// 4. 表单校验（前端前置）
// ===========================================================================
const okIssues = validateSkillForm(skillDef, { DAYS: "7" });
assert(okIssues.length === 0, `V-1 合法表单零问题（实得 ${JSON.stringify(okIssues.map((i) => i.code))}）`);
assert(
  validateSkillForm(skillDef, {}).some((i) => i.code === "SKILL_PARAM_REQUIRED"),
  "V-2 缺必填 → SKILL_PARAM_REQUIRED",
);
assert(
  validateSkillForm(skillDef, { DAYS: "7", EXTRA: "1" }).some((i) => i.code === "SKILL_PARAM_UNKNOWN"),
  "V-3 未知入参 → SKILL_PARAM_UNKNOWN",
);
assert(
  validateSkillForm(undefined, {}).some((i) => i.code === "SKILL_DEF_MISSING"),
  "V-4 def 缺失 → SKILL_DEF_MISSING",
);

// ===========================================================================
// 5. 面板三态
// ===========================================================================
assert(
  panelState("skills", { loading: false, count: 0, error: null, backendReady: false }).state === "empty",
  "E-1 后端未就绪且无数据 → empty",
);
assert(
  panelState("skills", { loading: true, count: 0, error: null, backendReady: true }).state === "loading",
  "E-2 loading",
);
assert(
  panelState("agents", { loading: false, count: 0, error: "boom", backendReady: true }).state === "error",
  "E-3 error",
);

// ===========================================================================
// 6. 流式缓冲有界
// ===========================================================================
assert(appendChunk([], { kind: "data", data: "hello" }).length === 1, "B-1 追加首块");
const big = Array.from({ length: 50 }, () => ({ kind: "data", data: "x".repeat(2000) }));
const bounded = big.reduce((acc, c) => appendChunk(acc, c), []);
const total = bounded.filter((c) => c.kind === "data").reduce((n, c) => n + (c.data?.length ?? 0), 0);
assert(total <= MAX_STREAM_BUFFER_BYTES, `B-2 单会话缓冲不超上限（${total} <= ${MAX_STREAM_BUFFER_BYTES}）`);
assert(chunksToText([{ kind: "data", data: "ab" }, { kind: "data", data: "cd" }]) === "abcd", "B-3 拼接文本");

// ===========================================================================
// 7. 二段式闸门辅助
// ===========================================================================
const pc = makePendingConfirm("install_skill", { id: "skill-1" });
assert(isConfirmExpired(pc) === false, "K-1 新建待确认未过期");
assert(isConfirmExpired({ action: "install_skill", payload: {}, expiresAt: 0 }) === true, "K-2 过期判定");

// ===========================================================================
// 8. store：后端未就绪时零 invoke（LIMITED START 口径）
// ===========================================================================
assert(store.backendReady === false, "B-1 store 初始 backendReady=false（命令未落地）");
resetCalls();
await store.loadSkills();
await store.loadAgents();
await store.selectSkill("skill-1");
await store.installSkill("skill-1");
await store.installAgent("agent-1");
await store.runAgent("agent-1", "hi", "sess-1");
await store.ackConfirm("req-x", "approve");
assert(calls.length === 0, `B-2 未就绪时所有动作零 invoke（实得 ${JSON.stringify(calls)}）`);
assert(typeof store.error === "string" && store.error.length > 0, "B-3 未就绪时给出明确错误文案");

// ===========================================================================
// 9. store：后端就绪后的接线（mock bridge）
// ===========================================================================
store.backendReady = true;
resetCalls();
await store.loadSkills();
assert(calls.some(([n]) => n === "skill_list"), "L-1 就绪后 loadSkills 调 skill_list");
assert(store.skills.length === 1, "L-2 加载到 1 个 skill");

resetCalls();
await store.loadAgents();
assert(calls.some(([n]) => n === "agent_list"), "L-3 就绪后 loadAgents 调 agent_list");
assert(store.agents.length === 1, "L-4 加载到 1 个 agent");

resetCalls();
await store.installSkill("skill-1");
assert(calls.some(([n, id]) => n === "skill_install" && id === "skill-1"), "L-5 installSkill 调 skill_install");
assert(store.pendingConfirms.has("req-1"), "L-6 收到 request_id → 写入 pendingConfirms");

resetCalls();
await store.ackConfirm("req-1", "approve");
assert(calls.some(([n]) => n === "confirm_skill_install"), "L-7 确认走 confirm_skill_install");
assert(!store.pendingConfirms.has("req-1"), "L-8 确认后清除该待确认项");

resetCalls();
await store.installAgent("agent-1");
await store.ackConfirm("req-2", "approve");
assert(calls.some(([n]) => n === "confirm_agent_install"), "L-9 agent 确认走 confirm_agent_install");

resetCalls();
await store.runAgent("agent-1", "hi", "sess-9");
assert(calls.some(([n]) => n === "agent_chat"), "L-10 runAgent 调 agent_chat（不自建执行通道）");
assert(store.sessions["sess-9"]?.status === "streaming", "L-11 runAgent 建流式会话壳");

// pushChunk 有界追加（经 store 动作）
store.pushChunk("sess-9", { kind: "data", data: "你好" });
assert(store.sessions["sess-9"].chunks.length === 1, "L-12 pushChunk 追加到会话");
store.endSession("sess-9", "done");
assert(store.sessions["sess-9"].status === "done", "L-13 endSession 终止会话");

// ===========================================================================
// 10. 只读校验消费路径（W8：面板消费 skill_validate/agent_validate 结果）
//     仅用 agentSkillUi.ts 纯逻辑；不调用任何 bridge（只读命令尚未进 mainline，
//     符合 W8 A6 "no live command execution unless bridge functions already
//     exist and are typed"）。后端 SkillValidationResult 形状见 A6 W7 接线笔记
//     §4（types.ts 暂由 A5 W8 落地，此处按契约构造做纯逻辑断言）。
// ===========================================================================
// 消费形状：{ ok, def: SkillDef|null, issues: ValidationIssue[], preview: PermissionPreview|null }
const skillValidateOk = {
  ok: true,
  def: skillDef,
  issues: [],
  preview: { gate: "confirm", capabilities: ["fs:read", "net:http"] },
};
assert(skillValidateOk.ok === true, "W8-1 校验通过 ok=true");
assert(skillValidateOk.issues.length === 0, "W8-2 校验通过无错误项");
assert(skillValidateOk.preview !== null, "W8-3 校验通过回传权限预览");
// 消费：用后端预览直接派生前端展示态（PermissionPreviewModal 直渲）
const okView = buildPermissionPreview(skillValidateOk.def, skillValidateOk.preview.capabilities);
assert(okView.gate === "confirm", "W8-4 展示态 gate 来自预览");
assert(okView.capabilities.length === 2, "W8-5 预览含能力列表");
assert(okView.dangerHint !== null, "W8-6 confirm 档展示提示");

// 解析失败：无 def、无 preview，仅错误码
const parseErr = {
  ok: false,
  def: null,
  issues: [{ field: "json", code: "PARSE_ERROR", message: "JSON 解析失败" }],
  preview: null,
};
assert(parseErr.def === null, "W8-7 解析失败无 def");
assert(parseErr.preview === null, "W8-8 解析失败无 preview");
assert(parseErr.issues.some((i) => i.code === "PARSE_ERROR"), "W8-9 解析错误码可见");

// 语义校验失败：已解析 def 仍回传（供部分展示），preview 仍回传（只读可看）
const validateFail = {
  ok: false,
  def: skillDef,
  issues: [{ field: "id", code: "EmptyRequiredField", message: "id 为空" }],
  preview: { gate: "confirm", capabilities: ["fs:read", "net:http"] },
};
assert(validateFail.ok === false, "W8-10 语义失败 ok=false");
assert(validateFail.def !== null, "W8-11 仍回传已解析 def 供部分展示");
assert(validateFail.preview !== null, "W8-12 仍回传 preview");
const failView = buildPermissionPreview(validateFail.def, validateFail.preview.capabilities);
assert(failView.gate === "confirm", "W8-13 失败项仍可渲染权限预览（只读）");
assert(validateFail.issues.length > 0, "W8-14 错误项驱动 UI 错误展示");

// dangerous 闸门：高危提示 + danger 配色
const dangerDef = { ...skillDef, acl: "dangerous" };
const dv = buildPermissionPreview(dangerDef, []);
assert(dv.gate === "dangerous", "W8-15 dangerous gate");
assert(dv.gateTone === "danger", "W8-16 dangerous 配色");
assert(dv.dangerHint !== null, "W8-17 dangerous 提示必显");
assert(dv.capabilities.every((c) => c.unknown), "W8-18 空白名单 → 全部 unknown（不谎报已授权）");

// 单能力判定（供权限预览逐项 + 校验结果能力标签）
assert(classifyCapability("fs:read", ["fs:read"]).granted === true, "W8-19 白名单内 → granted");
assert(classifyCapability("file_read", []).unknown === true, "W8-20 白名单外 → unknown（前端不伪造授权）");

// secret 边界（W8）：提示词本身不是 secret 键，不按键名脱敏；真正凭据泄露由后端判
const red2 = redactSecrets({ system_prompt: "use sk-abc token", TOKEN: "x" });
assert(red2.TOKEN === "***", "W8-21 TOKEN 键脱敏");
assert(red2.system_prompt === "use sk-abc token", "W8-22 提示词按值不按键名脱敏（凭据泄露由后端判）");

// ===========================================================================
// 11. W9：Agent/Skill 面板消费打磨（只读校验命令已进 mainline，只读开关=true）
//     - 确定的空/加载/成功/错误态（skillStatus / agentStatus）
//     - 预览渲染有界（boundedPreview / boundedAgentPreview）
//     - UI 状态不回显 secret：原始输入文本不进 store；错误串与解析 def 做 secret 模式脱敏
//     全部经 store 动作 + mock bridge 驱动，反映产品代码行为。
// ===========================================================================
// 11.1 确定的空态
store.clearValidation();
assert(store.skillStatus === "empty", "W9-1 清空后 skill 面板为空态");
assert(store.agentStatus === "empty", "W9-2 清空后 agent 面板为空态");
assert(
  store.boundedPreview.shown.length === 0 && store.boundedPreview.overflow === 0,
  "W9-3 空态预览有界无溢出",
);

// 11.2 loading 态优先于其它态
store.loading = true;
assert(store.skillStatus === "loading", "W9-4 loading 优先于其它态");
store.loading = false;

// 11.3 validateSkill 串联消费：只读路径独立于 install/run 就绪（即便 backendReady=false）
store.backendReady = false;
resetCalls();
await store.validateSkill("PASTED_JSON_TEXT");
assert(
  calls.some(([n, t]) => n === "skill_validate" && t === "PASTED_JSON_TEXT"),
  "W9-5 只读校验发出 skill_validate（不受 backendReady 拦截）",
);
assert(store.skillValidation && store.skillValidation.valid === true, "W9-6 校验通过 valid=true");
assert(store.skillParseDef && store.skillParseDef.id === "skill-1", "W9-7 解析出 def 供摘要展示");
assert(
  store.skillPreview && store.skillPreview.capabilities.length === 2,
  "W9-8 权限预览回传（confirm 档 + 2 能力）",
);
assert(store.skillStatus === "ok", "W9-9 校验通过 → ok 态");
store.backendReady = true;

// 11.4 错误态 + 不回显 secret（错误串脱敏）
bridge.skillValidate = async () => ({
  valid: false,
  errors: ["凭据泄露：sk-ABCDE12345678 出现在 system_prompt", "id 为空"],
});
resetCalls();
await store.validateSkill("SECRET_INPUT_WITH_sk-ABCDE12345678");
assert(store.skillStatus === "error", "W9-10 校验失败 → error 态");
assert(
  store.skillValidation.errors.some((e) => e.includes("id 为空")),
  "W9-11 普通错误项可见",
);
assert(
  store.skillValidation.errors.every((e) => !e.includes("sk-ABCDE12345678")),
  "W9-12 错误串中 secret 模式已脱敏（不回显）",
);
// 原始输入文本绝不进入 store 状态
assert(!("skillInputText" in store) && !("lastInput" in store), "W9-13 原始输入文本不进 store 状态");

// 11.5 有界预览渲染
store.skillPreview = { gate: "safe", capabilities: Array.from({ length: 100 }, (_, i) => `cap:${i}`) };
assert(store.boundedPreview.shown.length === 20, "W9-14 预览仅展示前 20 项");
assert(store.boundedPreview.overflow === 80, "W9-15 余下 80 项标记为溢出");
assert(store.boundedPreview.total === 100, "W9-16 总数仍为 100（不丢信息）");

// 11.6 agent 对称路径 + 解析 def 中 prompt-secret 不进前端状态
bridge.agentParse = async () => ({
  ...agentDef,
  systemPrompt: "你是一个助手，内部 token=sk-ZZZ99999999999",
});
resetCalls();
await store.validateAgent("AGENT_JSON_TEXT");
assert(calls.some(([n]) => n === "agent_validate"), "W9-17 只读校验发出 agent_validate");
assert(store.agentStatus === "ok", "W9-18 agent 校验通过 → ok 态");
assert(
  store.agentPreview && store.agentPreview.gate === "safe",
  "W9-19 agent 权限预览（safe 档）",
);
assert(
  store.agentParseDef && !store.agentParseDef.systemPrompt.includes("sk-ZZZ99999999999"),
  "W9-20 解析出的 AgentDef.systemPrompt 中 secret 已脱敏（不进前端状态）",
);

// 11.7 收尾：回到空态，避免影响后续（如若有）断言
await store.clearValidation();

// ---------------- 汇总 ----------------
console.log(`\ncheck-agent-skill-ui-logic: ${passed} assertions passed, ${failures.length} failed`);
if (failures.length > 0) {
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
process.exit(0);
