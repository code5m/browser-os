<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { selectChatHistory, utf8Bytes, createChatRequestId } from '../model/chatHistory.mjs';
import { aiService, type AiProfile } from '../../../settings/public';

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
let sessionVersion = 0;
const error = ref('');
const canSend = computed(() => !!modelId.value && !!prompt.value.trim() && utf8Bytes(prompt.value.trim()) <= 24000 && !busy.value && !loading.value);

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
  const history: ChatLine[] = selectChatHistory(lines.value);
  const requestId = createChatRequestId();
  const selectedModel = modelId.value;
  const currentSession = sessionVersion;
  currentRequestId = requestId;
  busy.value = true;
  stopping.value = false;
  error.value = '';
  try {
    const answer = await aiService.chat('default', message, selectedModel, history, requestId);
    // Discard late provider results after switching model/session or unmounting.
    if (sessionVersion !== currentSession || modelId.value !== selectedModel) return;
    lines.value.push({role:'user',content:message},{role:'assistant',content:answer.content});
    if (lines.value.length > 100) lines.value.splice(0, lines.value.length - 100);
    prompt.value = '';
  } catch (reason) {
    if (sessionVersion === currentSession) error.value = String(reason);
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
  lines.value = [];
  prompt.value = '';
  error.value = '';
}
watch(() => props.active, active => {
  if (active === false && currentRequestId) void stopReply();
});
watch(modelId, (next, previous) => {
  if (next !== previous) newConversation(); // No cross-model history or state leakage.
});
onBeforeUnmount(() => {
  sessionVersion++;
  if (currentRequestId) void aiService.cancelChat(currentRequestId).catch(() => {});
});
onMounted(refreshModels);
</script>
<template>
  <section class="plain-chat" aria-label="普通聊天，不需要项目路径">
    <header><strong>普通聊天</strong><span>无需打开项目 · 不会访问文件或执行工具</span></header>
    <div class="toolbar"><select v-model="modelId" :disabled="busy || loading" aria-label="聊天模型"><option value="" disabled>选择聊天模型</option><option v-for="model in models" :key="model.id" :value="model.id">{{ model.name }} · {{ model.model }}</option></select><button :disabled="busy || loading" @click="refreshModels">刷新模型</button><button :disabled="busy" @click="newConversation">新建会话</button></div>
    <main class="history" aria-live="polite"><p v-if="!lines.length" class="hint">选择一个已配置模型即可开始聊天，无需输入项目地址。编码和文件操作请切换到「编码工作区」。</p><article v-for="(line,index) in lines" :key="index" :class="line.role"><strong>{{line.role==='user'?'你':'智能体'}}</strong><p>{{line.content}}</p></article></main>
    <p v-if="error" class="error" role="alert">{{error}}</p><p v-if="utf8Bytes(prompt.trim()) > 24000" class="error">消息超过后端 24000 字节限制</p>
    <form class="composer" @submit.prevent="send"><textarea v-model="prompt" :disabled="busy" maxlength="12000" rows="3" placeholder="输入想讨论的问题…（无需项目路径）" aria-label="聊天消息" @keydown.ctrl.enter.prevent="send"></textarea><button v-if="busy" type="button" :disabled="stopping" @click="stopReply">{{stopping?'停止中…':'停止生成'}}</button><button v-else type="submit" :disabled="!canSend">发送</button></form>
  </section>
</template>
<style scoped>
.plain-chat{display:flex;flex-direction:column;min-height:0;background:#141820;color:#d5def0}.plain-chat header{padding:18px 22px;border-bottom:1px solid #30394b;display:flex;gap:18px;align-items:center}.plain-chat header span,.hint{color:#9dacc3;font-size:12px}.toolbar{display:flex;gap:8px;padding:12px 22px}.toolbar select{min-width:220px;max-width:450px;flex:1}.toolbar select,.toolbar button,.composer button,.composer textarea{background:#1d2431;color:#e1e9f6;border:1px solid #394b62;border-radius:8px;padding:10px}.history{flex:1;min-height:0;overflow:auto;padding:18px 22px}.history article{max-width:850px;margin:0 auto 18px;padding:14px;border-radius:10px;background:#202a3a}.history article.user{background:#25334b}.history article p{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.65}.history article strong{font-size:12px;color:#9ac1fa}.composer{display:flex;gap:12px;padding:15px 22px;border-top:1px solid #30394b}.composer textarea{flex:1;resize:vertical}.composer button{min-width:90px;cursor:pointer}.composer button:disabled{opacity:.45;cursor:default}.error{padding:0 22px;color:#ff9b9b}
</style>
