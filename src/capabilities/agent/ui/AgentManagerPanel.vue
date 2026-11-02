<script setup lang="ts">
import { useAgentStore } from "../state/useAgentStore";
import { bridge } from "../../../bridge";
import BlueprintStudio from "../../../shared/ui/BlueprintStudio.vue";
import CodingWorkbench from './CodingWorkbench.vue';
import GeneralChatPanel from './GeneralChatPanel.vue';
import { ref } from 'vue';
const tab=ref<'chat'|'coding'|'definition'>('chat');
const store=useAgentStore();
const localChat = (model:string,message:string,system:string) => bridge.agentLocalChat(model,message,system);
</script>
<template><section class="agent-manager"><nav><button :class="{active:tab==='chat'}" @click="tab='chat'">普通聊天</button><button :class="{active:tab==='coding'}" @click="tab='coding'">编码工作区</button><button :class="{active:tab==='definition'}" @click="tab='definition'">智能体定义 / 蓝图</button></nav><GeneralChatPanel v-show="tab==='chat'" :active="tab==='chat'"/><CodingWorkbench v-show="tab==='coding'"/><BlueprintStudio v-if="tab==='definition'" kind="agent" :validate-definition="store.validateAgent" :validation="store.agentValidation" :preview="store.agentPreview" :backend-ready="store.backendReady" :busy="store.loading" :local-chat="localChat"/></section></template>
<style scoped>.agent-manager{height:100%;min-height:0;display:flex;flex-direction:column;overflow:hidden}.agent-manager>nav{display:flex;background:#111821;border-bottom:1px solid #2a3546;flex-shrink:0}.agent-manager>nav button{padding:8px 16px;color:#9cacbf;background:transparent;border:0;cursor:pointer}.agent-manager>nav button.active{color:#dceaff;border-bottom:2px solid #81adf8}.agent-manager>:not(nav){flex:1;min-height:0}</style>
