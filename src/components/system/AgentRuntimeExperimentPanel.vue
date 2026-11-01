<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { bridge, type AgentRuntimeExperimentConfig, type AgentRuntimeBenchmarkReport, type AgentRuntimeBenchmarkSample, type AgentRuntimeMode } from "../../bridge";

const persisted = ref<AgentRuntimeExperimentConfig | null>(null);
const loading = ref(true);
const saving = ref(false);
const running = ref(false);
const reading = ref(false);
const routing = ref(false);
const fixtureId = ref("tab_summary");
const routedResult = ref<{fixture_id:string;sample:AgentRuntimeBenchmarkSample;output:string}|null>(null);
const error = ref("");
const message = ref("");
const enabled = ref(false);
const mode = ref<AgentRuntimeMode>("native");
const canary = ref(10);
const benchEnabled = ref(false);
const emergencyFallback = ref(true);
const maxTurns = ref(3);
const repeats = ref(3);
const report = ref<AgentRuntimeBenchmarkReport | null>(null);
const toolCommand = ref("tab_list");
const toolRepo = ref("");
const toolRoot = ref("");
const confirmRead = ref(false);
const result = ref("");
const showResults = ref(false);

const supported = computed(() => persisted.value !== null && !loading.value);
const runtimeOptions: {value:AgentRuntimeMode;label:string;hint:string}[] = [
  {value:"native",label:"Native",hint:"现有单轮引擎"},
  {value:"rig",label:"Rig AgentRun",hint:"官方多轮状态机"},
  {value:"canary",label:"Canary",hint:"新任务按比例选择 Rig"}
];
const tools = [
  {value:"tab_list",label:"Browser · 当前页签元数据"},
  {value:"git_status",label:"Git · 工作区文件状态"},
  {value:"git_diff",label:"Git · 已授权仓库差异"},
  {value:"db_list_connections",label:"Database · 连接名称与类型"},
  {value:"coding_lsp_status",label:"Coding · LSP 状态"}
];
const needsRepo = computed(() => toolCommand.value.startsWith("git_"));
const needsRoot = computed(() => toolCommand.value === "coding_lsp_status");

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    const config = await bridge.agentRuntimeGetConfig();
    persisted.value = config;
    enabled.value = config.enabled;
    mode.value = config.mode;
    canary.value = config.canary_percent;
    benchEnabled.value = config.benchmark_enabled;
    emergencyFallback.value = config.emergency_fallback_to_native;
    maxTurns.value = config.max_turns;
  } catch (reason) {
    persisted.value = null;
    error.value = "原生 Agent Runtime 接口不可用，请检查安装版构建与权限配置：" + String(reason);
  } finally {
    loading.value = false;
  }
}
async function save() {
  if (!persisted.value || saving.value) return;
  saving.value = true;
  error.value = "";message.value = "";
  const config: AgentRuntimeExperimentConfig = {
    revision:persisted.value.revision + 1,
    enabled:enabled.value,
    mode:enabled.value ? mode.value : "native",
    canary_percent:Math.min(100,Math.max(0,Math.round(canary.value))),
    emergency_fallback_to_native:emergencyFallback.value,
    benchmark_enabled:benchEnabled.value,
    max_turns:Math.min(32,Math.max(1,Math.round(maxTurns.value)))
  };
  try {
    const updated = await bridge.agentRuntimeSetConfig(config,persisted.value.revision);
    persisted.value = updated;
    mode.value = updated.mode;
    message.value = "已由原生 Host 原子保存（版本 " + updated.revision + "）。仅影响新实验任务。";
  } catch (reason) {
    error.value = "保存失败：" + String(reason);
    // Refresh on a stale revision instead of quietly overwriting another writer.
    await refresh();
  } finally {
    saving.value = false;
  }
}
async function benchmark() {
  if (!supported.value || running.value) return;
  running.value = true;
  error.value = "";message.value = "";
  report.value = null;
  try {
    report.value = await bridge.agentRuntimeBenchmark(Number(repeats.value));
    showResults.value = true;
  } catch (reason) {
    error.value = "对照测评失败：" + String(reason);
  } finally {
    running.value = false;
  }
}
async function runFixture() {
  if (!persisted.value?.enabled || routing.value) return;
  routing.value = true;
  error.value = "";message.value = "";routedResult.value=null;
  try {
    routedResult.value = await bridge.agentRuntimeRunFixture(fixtureId.value);
  } catch (reason) {
    error.value = "按策略执行失败：" + String(reason);
  } finally {
    routing.value = false;
  }
}
async function readTool() {
  if (!enabled.value || !persisted.value?.enabled || !confirmRead.value || reading.value) return;
  reading.value = true;
  error.value = "";message.value = "";result.value = "";
  try {
    const args:Record<string,unknown> = needsRepo.value
      ? {repo_id:toolRepo.value.trim()}
      : needsRoot.value ? {root:toolRoot.value.trim()} : {};
    const response = await bridge.agentRuntimeReadTool(toolCommand.value,args);
    result.value = JSON.stringify(response,null,2).slice(0,64000);
    message.value = "已通过主窗口授权和原业务 Public API 只读执行。";
  } catch (reason) {
    error.value = "业务工具验证失败：" + String(reason);
  } finally {
    reading.value = false;
    confirmRead.value = false;
  }
}
function summary(engine:"native_turn"|"rig_agent_run") {
  const values=(report.value?.samples ?? []).filter(item=>item.engine===engine);
  if (!values.length) return {count:0,median:"—",pass:"—",tokens:"—"};
  const durations=values.map(item=>item.total_duration_us).sort((a,b)=>a-b);
  const median=durations[Math.floor(durations.length/2)];
  const input=values.reduce((n,item)=>n+(item.input_tokens ?? 0),0);
  const output=values.reduce((n,item)=>n+(item.output_tokens ?? 0),0);
  return {count:values.length,median:median+" μs",
    pass:Math.round(values.filter(item=>item.completed).length/values.length*100)+"%",
    tokens:input+" / "+output};
}
const nativeStats=computed(()=>summary("native_turn"));
const rigStats=computed(()=>summary("rig_agent_run"));
onMounted(refresh);
</script>

<template>
  <section class="agent-experiment" aria-label="Agent Runtime 实验配置">
    <div class="section-heading">
      <div>
        <strong>Agent Channel · Runtime Lab</strong>
        <p>Native 与 Rig 使用同一任务／审批 Owner，默认关闭，配置只影响新任务。</p>
      </div>
      <span class="state" :class="{ready:supported}" aria-live="polite">{{ loading ? "读取中" : supported ? "Host 已连接" : "不可用" }}</span>
    </div>
    <div v-if="error" class="notice error" role="alert">{{ error }}</div>
    <div v-if="message" class="notice" role="status">{{ message }}</div>

    <div class="settings-grid">
      <div class="field wide switch-field">
        <div><b>启用实验能力</b><small>不会替换现有 Coding Agent 的默认生产链路</small></div>
        <input v-model="enabled" type="checkbox" :disabled="!supported" aria-label="启用 Agent Runtime 实验" />
      </div>
      <div class="field wide">
        <label for="runtime-mode">运行策略</label>
        <select id="runtime-mode" v-model="mode" :disabled="!supported || !enabled">
          <option v-for="option in runtimeOptions" :key="option.value" :value="option.value">{{ option.label }} · {{ option.hint }}</option>
        </select>
      </div>
      <div class="field">
        <label for="runtime-canary">Rig 灰度比例 {{ canary }}%</label>
        <input id="runtime-canary" v-model.number="canary" type="range" min="0" max="100" step="5" :disabled="!enabled || mode !== 'canary'" />
      </div>
      <div class="field">
        <label for="runtime-turns">最大模型轮次</label>
        <input id="runtime-turns" v-model.number="maxTurns" type="number" min="1" max="32" :disabled="!supported || !enabled" />
      </div>
      <div class="field check"><label><input v-model="benchEnabled" type="checkbox" :disabled="!supported || !enabled" /> 开启可重复基准测评</label></div>
      <div class="field check"><label><input v-model="emergencyFallback" type="checkbox" :disabled="!supported || !enabled" /> 应急回退策略（仅预留，尚未自动执行）</label></div>
    </div>
    <div class="actions">
      <button type="button" class="primary" :disabled="!supported || saving" @click="save">{{ saving ? "保存中…" : "保存实验设置" }}</button>
      <button type="button" :disabled="loading" @click="refresh">重新读取</button>
      <small>运行中任务不会热切换；存在未知副作用时禁止自动重放。</small>
    </div>

    <div class="lab-divider"></div>
    <div class="subheader"><strong>运行一次新任务 · 实际策略选择</strong><small>用于验证 Native / Rig / Canary 是否生效</small></div>
    <div class="actions">
      <select v-model="fixtureId" aria-label="选择固定测试任务" :disabled="routing || !persisted?.enabled">
        <option value="tab_summary">Browser 标签示例</option>
        <option value="git_review">Git 变更示例</option>
        <option value="database_report">数据库结果示例</option>
      </select>
      <button type="button" class="primary" :disabled="!persisted?.enabled || routing" @click="runFixture">{{ routing ? "执行中…" : "按当前策略运行" }}</button>
    </div>
    <div v-if="routedResult" class="notice" role="status">
      运行时：{{ routedResult.sample.engine === "rig_agent_run" ? "Rig AgentRun" : "Native" }} ·
      耗时：{{ routedResult.sample.total_duration_us }} μs ·
      {{ routedResult.sample.completed ? "模拟任务完成" : "失败" }}
    </div>
    <p class="disclaimer">每次由持久化路由策略创建一个新任务。Canary 采用任务 ID 的确定性分桶；不切换运行中任务。这里仍是离线模型 Fixture。</p>
    <div class="lab-divider"></div>
    <div class="subheader"><strong>同任务集 A/B 对照</strong><small>固定三类离线 Fixture · 每次运行两套执行策略</small></div>
    <p class="disclaimer">只测官方 Rig 状态机与 Native 调度开销。使用模拟模型响应，不请求真实 AI 模型、不调用真实工具，也不衡量答案质量或 GUI 性能。</p>
    <div class="actions">
      <label for="bench-repeat">每组重复</label>
      <select id="bench-repeat" v-model.number="repeats" :disabled="!supported || running">
        <option :value="1">1 次</option><option :value="3">3 次</option><option :value="5">5 次</option>
      </select>
      <button type="button" class="primary" :disabled="!persisted?.enabled || !persisted.benchmark_enabled || running" @click="benchmark">{{ running ? "测评运行中…" : "运行对照测评" }}</button>
    </div>
    <div v-if="report && showResults" class="results" aria-live="polite">
      <table><thead><tr><th>运行时</th><th>成功率</th><th>耗时中位数</th><th>输入/输出 Tokens（模拟）</th></tr></thead>
        <tbody>
          <tr><td>Native</td><td>{{ nativeStats.pass }}</td><td>{{ nativeStats.median }}</td><td>{{ nativeStats.tokens }}</td></tr>
          <tr><td>Rig AgentRun</td><td>{{ rigStats.pass }}</td><td>{{ rigStats.median }}</td><td>{{ rigStats.tokens }}</td></tr>
        </tbody></table>
      <small>{{ report.suite_id }} · {{ nativeStats.count + rigStats.count }} 条样本 · 所有耗时以本机即时测量为准</small>
    </div>

    <div class="lab-divider"></div>
    <div class="subheader"><strong>真实 Public API 只读验证</strong><small>人工触发 · 经原业务 Owner 验证 · 不执行写入／Shell</small></div>
    <div class="actions">
      <label for="public-tool">工具</label>
      <select id="public-tool" v-model="toolCommand" :disabled="!supported || reading">
        <option v-for="item in tools" :key="item.value" :value="item.value">{{ item.label }}</option>
      </select>
    </div>
    <div v-if="needsRepo" class="field">
      <label for="tool-repo">已配置的 Repo ID</label>
      <input id="tool-repo" v-model="toolRepo" type="text" maxlength="100" placeholder="Git 管理中已有的 repo id" />
    </div>
    <div v-if="needsRoot" class="field">
      <label for="tool-root">已授权工作区根目录</label>
      <input id="tool-root" v-model="toolRoot" type="text" maxlength="4096" placeholder="/home/.../your-project" />
    </div>
    <div class="actions">
      <label class="confirm"><input v-model="confirmRead" type="checkbox" :disabled="!persisted?.enabled" /> 我确认执行这一次只读检查</label>
      <button type="button" class="primary" :disabled="!persisted?.enabled || !confirmRead || reading || (needsRepo && !toolRepo.trim()) || (needsRoot && !toolRoot.trim())" @click="readTool">{{ reading ? "验证中…" : "执行已授权检查" }}</button>
    </div>
    <pre v-if="result" class="payload" aria-label="业务接口执行结果">{{ result }}</pre>
    <p class="disclaimer">只读 API 仅向受信任的主窗口开放。Coding Agent 的完整工作流、数据库 SQL、Git 写入及浏览器导航仍沿用各业务模块的原有权限体系，本实验室不允许绕过。</p>
  </section>
</template>

<style scoped>
.agent-experiment { border:1px solid #e4e8ee;border-radius:12px;padding:16px;background:#fcfdff;color:#26364d; }
.section-heading,.subheader {display:flex;align-items:flex-start;justify-content:space-between;gap:14px;}
.section-heading strong {font-size:15px;letter-spacing:-.2px;}
.section-heading p,.section-heading small,.subheader small {font-size:12px;color:#778396;margin:5px 0 0;line-height:1.5;}
.state {flex:none;background:#f3f4f6;padding:5px 9px;border-radius:16px;font-size:11px;color:#637084;}
.state.ready {background:#eaf4ef;color:#246446;}
.notice {padding:9px 12px;background:#edf4fc;border-radius:8px;color:#275f91;font-size:12px;margin:12px 0;}
.notice.error {background:#fef0ee;color:#a33529;}
.settings-grid {display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:13px;margin:17px 0;}
.field {display:flex;flex-direction:column;gap:7px;font-size:12px;}
.field.wide {grid-column:1/-1;}
.field label {font-weight:600;color:#455267;}
.field input:not([type=checkbox]):not([type=range]),.field select,.actions select {
  border:1px solid #d8dfe9;border-radius:7px;background:#fff;padding:8px;min-width:0;font-size:12px;color:#293a54;
}
.field small {font-size:11px;color:#8490a1;}
.field.check label {display:flex;gap:8px;align-items:center;font-weight:400;}
.switch-field {flex-direction:row;justify-content:space-between;align-items:center;}
.switch-field div {display:flex;flex-direction:column;gap:3px;}
.actions {display:flex;gap:10px;align-items:center;flex-wrap:wrap;margin:12px 0;font-size:12px;}
.actions button {background:#fff;border:1px solid #d7deea;border-radius:7px;color:#2a3a53;padding:8px 12px;font-size:12px;cursor:pointer;}
.actions button.primary {background:#275fa0;border-color:#275fa0;color:white;}
.actions button:disabled {opacity:.48;cursor:not-allowed;}
.actions small {color:#8995a3;line-height:1.5;}
.lab-divider {border-top:1px solid #e7ebf2;margin:17px 0 14px;}
.subheader strong {font-size:13px;}
.disclaimer {font-size:11px;line-height:1.65;color:#8995a3;margin:9px 0;}
.confirm {display:flex;gap:6px;align-items:center;}
.results {overflow-x:auto;border:1px solid #e6eaf1;border-radius:8px;margin-top:12px;}
.results table {width:100%;border-collapse:collapse;text-align:left;font-size:12px;}
.results th,.results td {padding:10px;border-bottom:1px solid #e8edf4;}
.results th {font-weight:600;background:#f3f6fa;color:#596679;}
.results small {display:block;color:#8a96a8;font-size:10px;padding:8px;}
.payload {white-space:pre-wrap;overflow:auto;max-height:240px;background:#f1f4f9;border-radius:8px;padding:12px;font-size:11px;color:#40506b;}
@media(max-width:650px) {.settings-grid{grid-template-columns:1fr;}.field.wide{grid-column:auto;}}
</style>
