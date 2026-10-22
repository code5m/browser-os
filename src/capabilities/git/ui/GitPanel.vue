<script setup lang="ts">
import GitStudio from "./GitStudio.vue";
const emit = defineEmits<{configure:[]}>();
</script>
<template><GitStudio mode="history" @configure="emit('configure')" /></template>
