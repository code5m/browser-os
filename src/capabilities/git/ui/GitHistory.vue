<script setup lang="ts">
import { ref, onUnmounted } from "vue";
import GitStudio from "./GitStudio.vue";
import { currentLocalGitIntent, clearLocalGitIntent, subscribeLocalGitIntent } from "../../../composables/gitLocalNavigation";
const local = ref(currentLocalGitIntent());
const stop = subscribeLocalGitIntent(intent => { local.value = intent; });
onUnmounted(stop);
</script>
<template><GitStudio :path="local?.path" :mode="local?.mode || 'history'" :close-label="local ? '返回配置仓库' : undefined" @close="clearLocalGitIntent()" /></template>
