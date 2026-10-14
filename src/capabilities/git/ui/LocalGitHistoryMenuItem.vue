<script setup lang="ts">
import { ContextMenuItem } from "../../../shared/ui";
import { useFileStore } from "../../workspace/public";
import { useGitStore } from "../public";

const props = defineProps<{ entry: DirEntry }>();
const files = useFileStore();
const git = useGitStore();

function openHistory() {
  git.openLocalHistory(props.entry.path);
  files.closeFileCtx();
}
</script>

<template>
  <ContextMenuItem @click="openHistory">🔀 Git → 查看{{entry.is_dir ? "目录" : "文件"}}提交历史</ContextMenuItem>
</template>
