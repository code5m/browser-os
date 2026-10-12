<script setup lang="ts">
import { useAgentStore } from "../state/useAgentStore";
import BlueprintStudio from "../../../shared/ui/BlueprintStudio.vue";
const store=useAgentStore();
</script>
<template><BlueprintStudio kind="agent" :validate-definition="store.validateAgent" :validation="store.agentValidation" :preview="store.agentPreview" :backend-ready="store.backendReady" :busy="store.loading"/></template>
