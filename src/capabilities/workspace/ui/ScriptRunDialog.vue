<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from "vue";
import { bridge } from "../../../bridge";
import type {
  CommandSnippet,
  ScriptMeta,
  RunStatus,
  ScriptOutputEvent,
  ScriptFinishedEvent,
} from "../../../types";

const props = withDefaults(defineProps<{ script: ScriptMeta | CommandSnippet; kind?: "script" | "command" }>(), {
  kind: "script",
});
const emit = defineEmits<{ close: [] }>();

// 参数值收集（统一字符串形态；bool 用 "true"/"false"）
const values = reactive<Record<string, string>>({});
const runId = ref<string | null>(null);
const running = ref(false);
const output = ref("");
const status = ref<RunStatus | null>(null);
const exitCode = ref<number | null>(null);
const errorMsg = ref<string | null>(null);
const truncated = ref(false);

// rAF 合并输出：高频 script-output 先入 pending，下一帧一次性 flush（D14 前端层）
const pendingChunks = ref<string[]>([]);
let rafId: number | null = null;
function flushOutput() {
  rafId = null;
  if (!pendingChunks.value.length) return;
  output.value += pendingChunks.value.join("");
  pendingChunks.value = [];
}

function onOutput(e: ScriptOutputEvent) {
  if (e.run_id !== runId.value) return;
  pendingChunks.value.push(e.chunk);
  if (rafId === null) rafId = requestAnimationFrame(flushOutput);
}

function onFinished(e: ScriptFinishedEvent) {
  if (e.snapshot.run_id !== runId.value) return;
  status.value = e.snapshot.status;
  exitCode.value = e.snapshot.exit_code ?? null;
  errorMsg.value = e.snapshot.error ?? null;
  truncated.value = e.snapshot.truncated;
  running.value = false;
}

// 在组件卸载时 unlisten 这两个订阅，否则多次运行会泄漏监听器
const unlisteners: Array<() => void> = [];
onMounted(async () => {
  // 初始化参数默认值（bool 默认 false，避免必填 bool 永远“缺失”）
  for (const p of props.script.params) {
    if (!(p.name in values)) {
      values[p.name] = p.default ?? (p.param_type === "bool" ? "false" : "");
    }
  }
  unlisteners.push(await bridge.onScriptOutput(onOutput));
  unlisteners.push(await bridge.onScriptFinished(onFinished));
});
onUnmounted(() => {
  if (rafId !== null) cancelAnimationFrame(rafId);
  unlisteners.forEach((fn) => fn());
});

const requiredMissing = computed(() =>
  props.script.params.some((p) => p.required && (values[p.name] ?? "") === ""),
);

async function run() {
  if (running.value) return;
  // 轻量前端必填校验；真正的 fail-closed 校验在后端
  const missing = props.script.params.filter(
    (p) => p.required && (values[p.name] ?? "") === "",
  );
  if (missing.length) {
    errorMsg.value = "必填参数缺失：" + missing.map((p) => p.label || p.name).join(", ");
    return;
  }
  output.value = "";
  status.value = null;
  errorMsg.value = null;
  pendingChunks.value = [];
  try {
    const snap = props.kind === "command"
      ? await bridge.runCommand(props.script.id, { ...values })
      : await bridge.runScript(props.script.id, { ...values });
    runId.value = snap.run_id;
    running.value = true;
    status.value = snap.status;
  } catch (e: unknown) {
    errorMsg.value = String(e);
    running.value = false;
  }
}

async function cancel() {
  if (runId.value) await bridge.cancelScript(runId.value);
}
</script>

<template>
  <div class="run-mask" @click.self="emit('close')">
    <div class="run-dialog">
      <div class="run-head">
        <span>▶ 运行：{{ script.name }}</span>
        <button class="x" @click="emit('close')">✕</button>
      </div>

      <div v-if="script.params.length" class="params">
        <div v-for="p in script.params" :key="p.name" class="p-row">
          <label :for="`rp-${p.name}`">{{ p.label || p.name }}<em v-if="p.required">*</em></label>
          <input
            v-if="p.param_type === 'string' && !p.secret"
            :id="`rp-${p.name}`"
            v-model="values[p.name]"
            :placeholder="p.default ?? ''"
          />
          <input
            v-else-if="p.param_type === 'string' && p.secret"
            :id="`rp-${p.name}`"
            type="password"
            v-model="values[p.name]"
            autocomplete="new-password"
            placeholder="敏感值不回显"
          />
          <input
            v-else-if="p.param_type === 'int'"
            :id="`rp-${p.name}`"
            type="number"
            v-model="values[p.name]"
            :placeholder="p.default ?? ''"
          />
          <input
            v-else-if="p.param_type === 'path'"
            :id="`rp-${p.name}`"
            v-model="values[p.name]"
            :placeholder="p.default ?? ''"
          />
          <select
            v-else-if="p.param_type === 'enum'"
            :id="`rp-${p.name}`"
            v-model="values[p.name]"
          >
            <option value="" disabled>请选择</option>
            <option v-for="o in p.options" :key="o" :value="o">{{ o }}</option>
          </select>
          <label v-else-if="p.param_type === 'bool'" class="bool">
            <input type="checkbox" v-model="values[p.name]" true-value="true" false-value="false" /> 启用
          </label>
        </div>
      </div>

      <div class="actions">
        <button v-if="!running" class="run-btn" :disabled="requiredMissing" @click="run">运行</button>
        <button v-else class="cancel-btn" @click="cancel">取消</button>
        <button class="close-btn" @click="emit('close')">关闭</button>
      </div>

      <pre class="out">{{ output }}</pre>

      <div v-if="status" class="result" :class="status">
        状态：{{ status }}<span v-if="exitCode !== null"> · 退出码 {{ exitCode }}</span>
        <span v-if="truncated"> · 输出已截断</span>
      </div>
      <div v-if="errorMsg && !running" class="err">⚠ {{ errorMsg }}</div>
    </div>
  </div>
</template>

<style scoped>
.run-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}
.run-dialog {
  width: 520px;
  max-width: 92vw;
  max-height: 86vh;
  overflow: auto;
  background: #fff;
  border-radius: 8px;
  padding: 14px 16px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25);
}
.run-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 14px;
  color: #1f2329;
  margin-bottom: 10px;
}
.run-head .x { border: none; background: transparent; cursor: pointer; font-size: 15px; }
.params { display: flex; flex-direction: column; gap: 8px; }
.p-row { display: flex; flex-direction: column; }
.p-row label { font-size: 12px; color: #4e5969; margin-bottom: 2px; }
.p-row label em { color: #c0392b; font-style: normal; }
.p-row input, .p-row select {
  border: 1px solid #d5dbe7; border-radius: 5px; padding: 5px 7px; font-size: 12px;
}
.p-row .bool { flex-direction: row; align-items: center; gap: 6px; }
.actions { display: flex; gap: 8px; margin: 12px 0; }
.run-btn { background: #2b6cb0; border-color: #2b6cb0; color: #fff; }
.cancel-btn { background: #c0392b; border-color: #c0392b; color: #fff; }
.actions button {
  padding: 6px 16px; border-radius: 5px; border: 1px solid #d5dbe7; background: #fff; cursor: pointer; font-size: 12px;
}
.actions button:disabled { opacity: 0.5; cursor: not-allowed; }
.out {
  background: #1e1e1e; color: #d4d4d4; border-radius: 6px; padding: 10px;
  font-family: monospace; font-size: 12px; white-space: pre-wrap; word-break: break-all;
  max-height: 320px; overflow: auto; margin: 0;
}
.result { font-size: 12px; margin-top: 8px; }
.result.succeeded { color: #1a7f37; }
.result.failed { color: #c0392b; }
.result.cancelled { color: #b7791f; }
.result.timeout { color: #b7791f; }
.result.running { color: #2b6cb0; }
.err { color: #c0392b; font-size: 12px; margin-top: 6px; }
</style>
