<script setup lang="ts">
import GitStudio from "./GitStudio.vue";
const props = defineProps<{path:string;mode:string;closeLabel?:string;searchId?:string;initialAction?:string}>();
const emit = defineEmits<{close:[]}>();
</script>
<template><GitStudio v-bind="props" @close="emit('close')" /></template>
