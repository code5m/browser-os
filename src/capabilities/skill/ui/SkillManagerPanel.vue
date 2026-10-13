<script setup lang="ts">
import { useSkillStore } from "../state/useSkillStore";
import BlueprintStudio from "../../../shared/ui/BlueprintStudio.vue";
const store=useSkillStore();
</script>
<template><BlueprintStudio kind="skill" :validate-definition="store.validateSkill" :validation="store.skillValidation" :preview="store.skillPreview" :backend-ready="store.backendReady"  :busy="store.loading" :run-safe-reference="store.runSafeReference"/></template>
