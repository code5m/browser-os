<script setup lang="ts">
import { useAgentStore } from "../state/useAgentStore";
import { bridge } from "../../../bridge";
import BlueprintStudio from "../../../shared/ui/BlueprintStudio.vue";
const store=useAgentStore();
const localChat = (model:string,message:string,system:string) => bridge.agentLocalChat(model,message,system);
</script>
<template><BlueprintStudio kind="agent" :validate-definition="store.validateAgent" :validation="store.agentValidation" :preview="store.agentPreview" :backend-ready="store.backendReady"  :busy="store.loading" :local-chat="localChat"/></template>
