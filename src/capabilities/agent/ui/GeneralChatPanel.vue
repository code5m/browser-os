<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { selectChatHistory, utf8Bytes, createChatRequestId } from '../model/chatHistory.mjs';
import { aiService, type AiProfile, type TextRunRecord, type TextRunSummary } from '../../../settings/public';

const props = defineProps<{ active?: boolean }>();
type ChatLine = { role: 'user' | 'assistant'; content: string };
const models = ref<AiProfile[]>([]);
const modelId = ref('');
const prompt = ref('');
const lines = ref<ChatLine[]>([]);
const busy = ref(false);
const loading = ref(false);
const stopping = ref(false);
let currentRequestId: string | null = null;
type PendingTurn = { id: string; model: string; message: string; history: ChatLine[]; session: number; previous?:string };
const historyOpen = ref(false);
const historyLoading = ref(false);
const historyRuns = ref<TextRunSummary[]>([]);
const historyCursor = ref<string|null>(null);
const historyUnavailable = ref(0);
const historyError = ref('');
const restored = ref<TextRunRecord|null>(null);
const lastRun = ref<string|null>(null);
let historyRevision = 0;
const pendingRetry = ref<PendingTurn | null>(null);
let sessionVersion = 0;
const error = ref('');
const canSend = computed(() => !!modelId.value && !!prompt.value.trim() && utf8Bytes(prompt.value.trim()) <= 24000 && !busy.value && !loading.value && !historyLoading.value
  && (!restored.value || restored.value.can_continue && restored.value.kind === 'chat'
      && restored.value.profile_id === modelId.value
      && models.value.some(model => model.id === modelId.value && model.model === restored.value?.model)));

async function refreshModels() {
  loading.value = true;
  try {
    const config = await aiService.list();
    models.value = config.profiles.filter(profile => profile.kind === 'chat');
    if (!models.value.some(profile => profile.id === modelId.value)) {
      modelId.value = models.value.find(profile => profile.id === config.bindings.default)?.id ?? models.value[0]?.id ?? '';
    }
    error.value = models.value.length ? '' : '没有聊天模型，请先前往设置添加模型。';
  } catch (reason) {
    error.value = String(reason);
  } finally {
    loading.value = false;
  }
}

// This route is intentionally read-only and independent from CodingWorkbench roots.
// A bounded transcript is supplied as untrusted context; no tools or workspace permissions are granted.
async function send() {
  const message = prompt.value.trim();
  if (!canSend.value || message.length > 12000) { error.value = '消息不能超过 12000 字符或 24000 UTF-8 字节'; return; }
  pendingRetry.value = null;
  await submitTurn({
    id: createChatRequestId(), model: modelId.value, message,
    history: selectChatHistory(lines.value).map(line => ({ ...line })), session: sessionVersion,
    previous: lastRun.value ?? undefined,
  });
}
async function retryOriginal() {
  const turn = pendingRetry.value;
  if (!turn || busy.value || loading.value || turn.session !== sessionVersion || turn.model !== modelId.value) return;
  await submitTurn(turn);
}
async function submitTurn(turn: PendingTurn) {
  if (busy.value || turn.session !== sessionVersion || turn.model !== modelId.value) return;
  const requestId = turn.id;
  currentRequestId = requestId;
  busy.value = true;
  stopping.value = false;
  error.value = '';
  pendingRetry.value = null;
  try {
    const answer = turn.previous
      ? await aiService.continueTextRun(turn.previous, requestId, turn.message)
      : await aiService.chat('default', turn.message, turn.model, turn.history, requestId);
    if (sessionVersion !== turn.session || modelId.value !== turn.model) return;
    if (turn.message) lines.value.push({role:'user',content:turn.message});
    lines.value.push({role:'assistant',content:answer.content});
    lastRun.value = requestId;
    restored.value = null;
    if (lines.value.length > 100) lines.value.splice(0, lines.value.length - 100);
    if (prompt.value.trim() === turn.message) prompt.value = '';
  } catch (reason) {
    if (sessionVersion === turn.session && modelId.value === turn.model) {
      const detail = String(reason);
      error.value = /AGENT_RUN_RECOVERY_REQUIRED|AGENT_RUN_RESULT_UNAVAILABLE|AGENT_RUN_SUCCESSOR_EXISTS/.test(detail)
        ? '原请求已有记录或缺少可恢复结果。请从运行记录恢复，不会自动再次调用模型。'
        : detail;
      // Only retry the immutable original turn, never a modified prompt
      // with a reused request ID. No plaintext conversation in web storage.
      if (!/AGENT_RUN_RECOVERY_REQUIRED|AGENT_RUN_RESULT_UNAVAILABLE|AGENT_RUN_SUCCESSOR_EXISTS|AGENT_RUN_INPUT_MISMATCH|AI_PROVIDER_CANCELLED/.test(detail)) {
        pendingRetry.value = turn;
      }
    }
  } finally {
    if (currentRequestId === requestId) {
      currentRequestId = null;
      busy.value = false;
      stopping.value = false;
    }
  }
}
async function stopReply() {
  if (!currentRequestId || stopping.value) return;
  const requestId = currentRequestId;
  stopping.value = true;
  sessionVersion++;
  try { await aiService.cancelChat(requestId); }
  catch (reason) { error.value = `取消请求失败：${String(reason)}`; }
  // Keep busy until the worker has actually exited; no overlapping native requests.
}
function newConversation() {
  if (busy.value) return;
  sessionVersion++;
  pendingRetry.value = null;
  lastRun.value = null;
  restored.value = null;
  lines.value = [];
  prompt.value = '';
  error.value = '';
}
// A failed/cancelled chain or a result intentionally excluded from
// persistence must not permanently disable ordinary text chat. Detach
// only after an explicit user choice, preserving visible in-memory lines.
function startNewChainWithContext() {
  if (busy.value || historyLoading.value) return;
  sessionVersion++;
  lastRun.value = null;
  restored.value = null;
  pendingRetry.value = null;
  error.value = '';
}
async function loadRunHistory(reset=true) {
  if (historyLoading.value || busy.value) return;
  historyOpen.value = true;
  const revision = ++historyRevision;
  historyLoading.value = true;
  historyError.value = '';
  try {
    const page = await aiService.textRuns(reset ? '' : (historyCursor.value ?? ''));
    if (revision !== historyRevision) return;
    historyRuns.value = reset ? page.runs : [...historyRuns.value, ...page.runs.filter(row => !historyRuns.value.some(old => old.id === row.id))];
    historyCursor.value = page.next_cursor;
    historyUnavailable.value = reset ? page.unavailable : historyUnavailable.value + page.unavailable;
  } catch (reason) { if (revision === historyRevision) historyError.value = String(reason); }
  finally { if (revision === historyRevision) historyLoading.value = false; }
}
async function restoreRun(id:string) {
  if (historyLoading.value || busy.value) return;
  const revision = ++historyRevision;
  historyLoading.value = true;
  historyError.value = '';
  try {
    const record = await aiService.restoreTextRun(id);
    if (revision !== historyRevision) return;
    if (record.profile_id && models.value.some(model => model.id === record.profile_id)) {
      modelId.value = record.profile_id;
    }
    sessionVersion++;
    pendingRetry.value = null;
    lastRun.value = record.id;
    lines.value = record.messages;
    prompt.value = '';
    restored.value = record;
    error.value = '';
  } catch (reason) { if (revision === historyRevision) historyError.value = String(reason); }
  finally { if (revision === historyRevision) historyLoading.value = false; }
}
async function resumeRestoredRun() {
  const record = restored.value;
  if (!record || busy.value || historyLoading.value) return;
  if (record.successor) {
    // Read a published successor before attempting an explicitly reserved
    // but unpublished one. An unavailable/corrupt receipt is not absence.
    historyLoading.value = true;
    const revision = ++historyRevision;
    try {
      const child = await aiService.restoreTextRun(record.successor);
      if (revision !== historyRevision) return;
      historyLoading.value = false;
      await restoreRun(child.id);
      return;
    } catch (reason) {
      if (revision !== historyRevision) return;
      historyLoading.value = false;
      if (!record.reserved_input_available || !String(reason).includes('AGENT_RUN_NOT_FOUND')) {
        historyError.value = String(reason); return;
      }
    }
  } else if (!record.can_resume) return;
  const id = record.successor || createChatRequestId();
  const turn: PendingTurn = {id,model:modelId.value,message:'',history:[],session:sessionVersion,previous:record.id};
  await submitTurn(turn);
  if (lastRun.value === id && sessionVersion === turn.session) await restoreRun(id);
}
async function forgetRestoredRun() {
  const record = restored.value;
  if (!record || busy.value || historyLoading.value ||
      !['completed','failed','cancelled'].includes(record.phase)) return;
  const revision = ++historyRevision;
  historyLoading.value = true;
  let archived = false;
  try { archived = (await aiService.forgetTextRun(record.id)).archived; }
  catch (reason) { if (revision === historyRevision) historyError.value = String(reason); return; }
  finally { if (revision === historyRevision) historyLoading.value = false; }
  if (revision !== historyRevision) return;
  await restoreRun(record.id);
  await loadRunHistory();
  if (!archived && revision + 2 === historyRevision) {
    historyError.value = '本条内容已清理，归档尚未确认；可再次归档，原运行 ID 保留。';
  }
}
watch(() => props.active, active => {
  if (active === false && currentRequestId) void stopReply();
});
watch(modelId, (next, previous) => {
  if (next !== previous) newConversation(); // No cross-model history or state leakage.
}, {flush:'sync'});
onBeforeUnmount(() => {
  historyRevision++;
  sessionVersion++;
  if (currentRequestId) void aiService.cancelChat(currentRequestId).catch(() => {});
});
onMounted(refreshModels);
</script>
<template>
  <section class="plain-chat" aria-label="普通聊天，不需要项目路径">
    <header><strong>普通聊天</strong><span>无需打开项目 · 不会访问文件或执行工具</span></header>
    <div class="toolbar"><select v-model="modelId" :disabled="busy || loading || historyLoading" aria-label="聊天模型"><option value="" disabled>选择聊天模型</option><option v-for="model in models" :key="model.id" :value="model.id">{{ model.name }} · {{ model.model }}</option></select><button :disabled="busy || loading || historyLoading" @click="refreshModels">刷新模型</button><button :disabled="busy || historyLoading" @click="newConversation">新建会话</button><button v-if="lastRun&&(!restored||restored.kind==='chat')" :disabled="busy||historyLoading" @click="startNewChainWithContext">保留当前消息并新建会话链</button><button v-if="pendingRetry" :disabled="busy || loading || historyLoading" @click="retryOriginal">重试原请求 / 恢复已保存结果</button><button :disabled="busy || historyLoading" @click="historyOpen ? historyOpen=false : loadRunHistory()">运行记录</button></div>
    <section v-if="historyOpen" class="run-history" aria-label="加密运行记录">
      <p class="hint">按记录 ID 分页读取，每次只解密有界记录。正在执行或无法认证的记录不显示为已结束。</p>
      <button :disabled="busy||historyLoading" @click="loadRunHistory()">刷新</button>
      <p v-if="historyUnavailable">本次未能读取 {{historyUnavailable}} 条记录（运行中或存储不可读）。</p>
      <article v-for="item in historyRuns" :key="item.id">
        <span>{{item.kind}} · {{item.phase}} · {{item.id.slice(0,8)}} · {{item.updated_at_ms ? new Date(item.updated_at_ms).toLocaleString() : '旧记录'}}</span>
        <button :disabled="busy||historyLoading" @click="restoreRun(item.id)">恢复查看</button>
      </article>
      <button v-if="historyCursor" :disabled="busy||historyLoading" @click="loadRunHistory(false)">下一页</button>
      <p v-if="historyError" role="status">{{historyError}}</p>
    </section>
    <section v-if="restored" class="restored-run" aria-label="恢复运行">
      <p>会话 {{restored.session_id.slice(0,8)}} · 运行 {{restored.id.slice(0,8)}} · {{restored.phase}} · {{restored.model || restored.kind}}</p>
      <button v-if="restored.can_resume||restored.successor" :disabled="busy||historyLoading" @click="resumeRestoredRun">{{restored.successor?'恢复已登记的后续运行（未发布的请求可能重新计费）':'明确重新执行此未完成的模型请求（可能再次计费）'}}</button>
      <p v-if="!restored.can_continue&&!restored.can_resume&&!restored.successor" class="hint">此记录仅供查看；如需新任务请新建会话。</p>
      <p v-if="restored.can_continue&&restored.profile_id!==modelId" class="hint">原模型未配置，不能通过其他模型接续此记录。</p>
      <p v-if="restored.archived" class="hint">终态回执已归档，活跃记录容量已释放；运行 ID 仍保留。</p>
      <details v-if="['completed','failed','cancelled'].includes(restored.phase)&&(restored.has_payload||restored.forgotten&&!restored.archived)"><summary>{{restored.forgotten?'重试终态归档':'清理本条终态记录的内容并归档'}}</summary><p>清理后此条内容不可恢复，保留运行 ID；其他记录可能仍保存上下文副本。归档发布确认前不会移除活跃记录。</p><button :disabled="busy||historyLoading" @click="forgetRestoredRun">{{restored.forgotten?'确认重试归档':'确认清理并归档'}}</button></details>
    </section>
    <main class="history" aria-live="polite"><p v-if="!lines.length" class="hint">选择一个已配置模型即可开始聊天，无需输入项目地址。编码和文件操作请切换到「编码工作区」。</p><article v-for="(line,index) in lines" :key="index" :class="line.role"><strong>{{line.role==='user'?'你':'智能体'}}</strong><p>{{line.content}}</p></article></main>
    <p v-if="error" class="error" role="alert">{{error}}</p><p v-if="utf8Bytes(prompt.trim()) > 24000" class="error">消息超过后端 24000 字节限制</p>
    <form class="composer" @submit.prevent="send"><textarea v-model="prompt" :disabled="busy" maxlength="12000" rows="3" placeholder="输入想讨论的问题…（无需项目路径）" aria-label="聊天消息" @keydown.ctrl.enter.prevent="send"></textarea><button v-if="busy" type="button" :disabled="stopping" @click="stopReply">{{stopping?'停止中…':'停止生成'}}</button><button v-else type="submit" :disabled="!canSend">发送</button></form>
  </section>
</template>
<style scoped>
.plain-chat{display:flex;flex-direction:column;min-height:0;background:#141820;color:#d5def0}.plain-chat header{padding:18px 22px;border-bottom:1px solid #30394b;display:flex;gap:18px;align-items:center}.plain-chat header span,.hint{color:#9dacc3;font-size:12px}.toolbar{display:flex;gap:8px;padding:12px 22px}.toolbar select{min-width:220px;max-width:450px;flex:1}.toolbar select,.toolbar button,.composer button,.composer textarea{background:#1d2431;color:#e1e9f6;border:1px solid #394b62;border-radius:8px;padding:10px}.history{flex:1;min-height:0;overflow:auto;padding:18px 22px}.history article{max-width:850px;margin:0 auto 18px;padding:14px;border-radius:10px;background:#202a3a}.history article.user{background:#25334b}.history article p{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.65}.history article strong{font-size:12px;color:#9ac1fa}.composer{display:flex;gap:12px;padding:15px 22px;border-top:1px solid #30394b}.composer textarea{flex:1;resize:vertical}.composer button{min-width:90px;cursor:pointer}.composer button:disabled{opacity:.45;cursor:default}.error{padding:0 22px;color:#ff9b9b}
.run-history,.restored-run{flex:0 0 auto;padding:8px 22px;border-bottom:1px solid #30394b;max-height:220px;overflow:auto}.run-history article{display:flex;align-items:center;justify-content:space-between;gap:8px;margin:5px 0}.run-history button,.restored-run button{background:#1d2431;color:#e1e9f6;border:1px solid #394b62;border-radius:6px;padding:6px}
</style>
