<script setup lang="ts">
import { computed } from "vue";
import { useGitStore } from "../../stores/useGitStore";

// diff 预览：只读展示 git_diff 结果。
// 后端已做双级截断（单文件 min(max_bytes,256KB) 标 truncated；总量触顶标 more），
// 这里只负责把 truncated / binary / more 三个提示如实呈现给用户，不做二次拼接猜测。
const git = useGitStore();

const hunks = computed(() => git.diff?.hunks ?? []);
const truncatedCount = computed(() => hunks.value.filter((h) => h.truncated).length);
const binaryCount = computed(() => hunks.value.filter((h) => h.binary).length);
const scopeLabel = computed(() => (git.activePath ? `单文件：${git.activePath}` : "全部文件"));
</script>

<template>
  <div class="git-col git-diff-col">
    <div class="git-col-head">
      <span>📄 diff（{{ scopeLabel }}）</span>
      <span class="spacer" />
      <button :disabled="git.diffLoading || !git.repoId" @click="git.loadDiff(null)">全部</button>
      <button
        :disabled="git.diffLoading || !git.repoId || !git.activePath"
        @click="git.loadDiff(git.activePath)"
      >刷新</button>
    </div>

    <div v-if="!git.repoId" class="git-empty">请先在上方选择一个仓库</div>
    <div v-else-if="git.diffLoading" class="git-empty">diff 加载中…</div>
    <div v-else-if="git.diffError" class="git-error">读取 diff 失败：{{ git.diffError }}</div>
    <div v-else-if="!hunks.length" class="git-empty">没有可显示的改动（工作区干净）</div>
    <div v-else class="git-diff-body">
      <div v-if="git.diff?.more" class="git-hint git-more">
        ⚠️ 改动过多，后端已按 256KB 硬上限停止收集，下方仅为部分文件的 diff。
      </div>
      <div v-if="truncatedCount || binaryCount" class="git-hint">
        <span v-if="truncatedCount">⚠️ {{ truncatedCount }} 个文件的补丁已截断（超出单文件上限）。</span>
        <span v-if="binaryCount">⚙️ {{ binaryCount }} 个文件为二进制，不展示内容。</span>
      </div>
      <div v-for="h in hunks" :key="h.file" class="diff-file">
        <div class="diff-file-head">
          <span class="diff-path" :title="h.file">{{ h.file }}</span>
          <span v-if="h.binary" class="gbadge warn">二进制</span>
          <span v-if="h.truncated" class="gbadge warn">已截断</span>
          <button @click="git.loadDiff(h.file)">只看此文件</button>
        </div>
        <pre v-if="!h.binary && h.new_content" class="diff-body">{{ h.new_content }}</pre>
        <div v-else-if="h.binary" class="diff-binary">二进制文件，后端未回传补丁文本</div>
        <div v-else class="diff-binary">该文件没有可展示的补丁内容</div>
      </div>
    </div>
  </div>
</template>
