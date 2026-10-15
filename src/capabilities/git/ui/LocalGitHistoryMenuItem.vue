<script setup lang="ts">
import { ContextMenuItem } from "../../../shared/ui";
import { useFileStore } from "../../workspace/public";
import { useGitStore } from "../public";

const props = defineProps<{entry: DirEntry}>();
const files = useFileStore();
const git = useGitStore();
// Git menu contributions only exist while the capability is ACTIVE.
// Stage/Commit open the scope-specific two-phase workbench; no direct mutation.
function openAction(action: "history" | "stage" | "commit") {
  git.openLocalHistory(props.entry.path,action);
  files.closeFileCtx();
}
</script>
<template>
  <ContextMenuItem @click="openAction('history')">🔀 Git · 查看{{entry.is_dir ? "目录" : "文件"}}提交历史</ContextMenuItem>
  <ContextMenuItem @click="openAction('stage')">Git · 暂存{{entry.is_dir ? "目录改动" : "文件"}}…</ContextMenuItem>
  <ContextMenuItem @click="openAction('commit')">Git · 提交{{entry.is_dir ? "目录" : "文件"}}…</ContextMenuItem>
</template>
