<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { aiService, type AiProfile } from '../../../settings/public';

type ChatLine = { role: 'user' | 'assistant'; content: string };
const models = ref<AiProfile[]>([]);
const modelId = ref('');
const prompt = ref('');
const lines = ref<ChatLine[]>([]);
const busy = ref(false);
const loading = ref(false);
const error = ref('');
const canSend = computed(() => !!modelId.value && !!prompt.value.trim() && !busy.value && !loading.value);

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
  if (!canSend.value || message.length > 12000) return;
  let remainingHistoryBytes = 16000;
  const history: ChatLine[] = [];
  for (const line of lines.value.slice(-12).reverse()) {
    const content = line.content.slice(0, 3000);
    if (content.length > remainingHistoryBytes) break;
    remainingHistoryBytes -= content.length;
    history.unshift({ role: line.role, content });
  }
  busy.value = true;
  error.value = '';
  try {
    const answer = await aiService.chat('default', message, modelId.value, history);
    lines.value.push({role:'user',content:message},{role:'assistant',content:answer.content});
    prompt.value = '';
  } catch (reason) { error.value = String(reason); }
  finally { busy.value = false; }
}
function newConversation() { if (!busy.value) { lines.value = []; error.value = ''; } }
onMounted(refreshModels);
</script>
<template>
  <section class="plain-chat" aria-label="普通聊天，不需要项目路径">
    <header><strong>普通聊天</strong><span>无需打开项目 · 不会访问文件或执行工具</span></header>
    <div class="toolbar"><select v-model="modelId" :disabled="busy || loading" aria-label="聊天模型"><option value="" disabled>选择聊天模型</option><option v-for="model in models" :key="model.id" :value="model.id">{{ model.name }} · {{ model.model }}</option></select><button :disabled="busy || loading" @click="refreshModels">刷新模型</button><button :disabled="busy" @click="newConversation">新建会话</button></div>
    <main class="history" aria-live="polite"><p v-if="!lines.length" class="hint">选择一个已配置模型即可开始聊天，无需输入项目地址。编码和文件操作请切换到「编码工作区」。</p><article v-for="(line,index) in lines" :key="index" :class="line.role"><strong>{{line.role==='user'?'你':'智能体'}}</strong><p>{{line.content}}</p></article></main>
    <p v-if="error" class="error" role="alert">{{error}}</p>
    <form class="composer" @submit.prevent="send"><textarea v-model="prompt" :disabled="busy" maxlength="12000" rows="3" placeholder="输入想讨论的问题…（无需项目路径）" aria-label="聊天消息" @keydown.ctrl.enter.prevent="send"></textarea><button type="submit" :disabled="!canSend">{{busy?'回复中…':'发送'}}</button></form>
  </section>
</template>
<style scoped>
.plain-chat{display:flex;flex-direction:column;min-height:0;background:#141820;color:#d5def0}.plain-chat header{padding:18px 22px;border-bottom:1px solid #30394b;display:flex;gap:18px;align-items:center}.plain-chat header span,.hint{color:#9dacc3;font-size:12px}.toolbar{display:flex;gap:8px;padding:12px 22px}.toolbar select{min-width:220px;max-width:450px;flex:1}.toolbar select,.toolbar button,.composer button,.composer textarea{background:#1d2431;color:#e1e9f6;border:1px solid #394b62;border-radius:8px;padding:10px}.history{flex:1;min-height:0;overflow:auto;padding:18px 22px}.history article{max-width:850px;margin:0 auto 18px;padding:14px;border-radius:10px;background:#202a3a}.history article.user{background:#25334b}.history article p{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.65}.history article strong{font-size:12px;color:#9ac1fa}.composer{display:flex;gap:12px;padding:15px 22px;border-top:1px solid #30394b}.composer textarea{flex:1;resize:vertical}.composer button{min-width:90px;cursor:pointer}.composer button:disabled{opacity:.45;cursor:default}.error{padding:0 22px;color:#ff9b9b}
</style>
