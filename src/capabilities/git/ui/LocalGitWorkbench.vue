<script setup lang="ts">
// Compatibility entrypoint only; no second operation implementation.
import GitStudio from "./GitStudio.vue";
const props = defineProps<{path:string;initialAction?:string}>();
</script>
<template><GitStudio v-bind="props" /></template>
