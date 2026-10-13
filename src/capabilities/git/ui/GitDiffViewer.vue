<script setup lang="ts">
import { computed, ref } from "vue";
import { useGitStore } from "../public";
import { parseUnifiedPatch } from "../model/patchView";
const git = useGitStore();
const view = ref<"split" | "unified">("split");
const patches = computed(() => (git.diff?.hunks ?? []).map(hunk => ({
  ...hunk, parsed: hunk.new_content ? parseUnifiedPatch(hunk.new_content) : null,
})));
const truncatedCount = computed(() => patches.value.filter(p => p.truncated || p.parsed?.truncated).length);
const binaryCount = computed(() => patches.value.filter(p => p.binary).length);
const scopeLabel = computed(() => git.activePath ? git.activePath : "全部文件");
</script>

<template>
  <section class="git-col git-diff-col">
    <header class="git-col-head">
      <strong>差异对照 · {{scopeLabel}}</strong>
      <span class="spacer" />
      <button :aria-pressed="view==='split'" @click="view='split'">并排</button>
      <button :aria-pressed="view==='unified'" @click="view='unified'">补丁</button>
      <button :disabled="git.diffLoading || !git.repoId" @click="git.loadDiff(null)">全部</button>
      <button :disabled="git.diffLoading || !git.activePath" @click="git.loadDiff(git.activePath)">刷新</button>
    </header>
    <p class="git-diff-hint">展示 Git 返回的真实变更片段，而非完整文件内容。此处只读，不执行暂存。</p>

    <div v-if="!git.repoId" class="git-empty">请选择仓库后查看差异</div>
    <div v-else-if="git.diffLoading" class="git-empty">正在读取差异…</div>
    <div v-else-if="git.diffError" class="git-error" role="alert">读取失败：{{git.diffError}}</div>
    <div v-else-if="!patches.length" class="git-empty">没有可展示的文本改动</div>
    <div v-else class="git-diff-body">
      <p v-if="git.diff?.more" class="git-more">变更达到后端 256 KiB 上限，内容并不完整。</p>
      <p v-if="truncatedCount || binaryCount" class="git-more">
        {{ truncatedCount ? truncatedCount + ' 个补丁内容已截断；' : "" }}
        {{ binaryCount ? binaryCount + ' 个二进制文件不显示内容。' : "" }}
      </p>
      <article v-for="hunk in patches" :key="hunk.file" class="diff-file">
        <header class="diff-file-head">
          <strong :title="hunk.file">{{hunk.file}}</strong>
          <span v-if="hunk.parsed" class="diff-counter">+{{hunk.parsed.additions}} / −{{hunk.parsed.deletions}}</span>
          <button @click="git.loadDiff(hunk.file)">只看此文件</button>
        </header>
        <p v-if="hunk.binary" class="git-empty">二进制文件 · 无文本补丁</p>
        <p v-else-if="hunk.truncated" class="git-more">该文件的补丁已达到后端限制，以下内容可能不完整。</p>
        <template v-if="hunk.new_content">
          <div v-if="view==='split' && hunk.parsed" class="patch-rows" role="table" :aria-label="'变更片段 '+hunk.file">
            <div class="patch-legend" role="row">
              <span role="columnheader">修改前（HEAD）</span><span role="columnheader">修改后（工作区）</span>
            </div>
            <div v-for="(row,i) in hunk.parsed.rows" :key="i" class="patch-row" role="row" :class="row.kind">
              <div class="patch-side" :class="{removed:row.removed}" role="cell"><span class="line-no">{{row.beforeLine ?? ""}}</span><pre>{{row.before}}</pre></div>
              <div class="patch-side" :class="{added:row.added}" role="cell"><span class="line-no">{{row.afterLine ?? ""}}</span><pre>{{row.after}}</pre></div>
            </div>
          </div>
          <pre v-else class="git-unified-patch">{{hunk.new_content}}</pre>
          <p v-if="hunk.parsed?.truncated" class="git-more">页面渲染数量达到上限，建议使用补丁模式查看更多内容。</p>
        </template>
      </article>
    </div>
  </section>
</template>

<style scoped>
.git-diff-col{min-width:0}
.git-col-head strong{font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.git-col-head button[aria-pressed=true]{color:#2456a6;background:#e7efff;border-color:#c0d3ef}
.git-diff-hint{padding:6px 10px;margin:0;font-size:11px;color:#728399;background:#fafcff;border-bottom:1px solid #e5eaf2}
.diff-file{margin:8px;border:1px solid #dce4ef;border-radius:8px;overflow:hidden}
.diff-file-head{display:flex;align-items:center;gap:9px;padding:9px 10px;background:#f3f6fb;font-size:12px;min-width:0}
.diff-file-head strong{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.diff-counter{color:#56718f;font-variant-numeric:tabular-nums}
.diff-file-head button{border:1px solid #ced9e8;border-radius:6px;background:white;padding:3px 8px;cursor:pointer;font-size:11px}
.patch-legend,.patch-row{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr)}
.patch-legend{background:#e8eef6;color:#536b84;font-size:11px;font-weight:600}
.patch-legend span{padding:6px 12px}
.patch-legend span+span,.patch-side+.patch-side{border-left:1px solid #dae4f0}
.patch-side{min-width:0;display:flex;align-items:stretch;background:#fff}
.patch-side.removed{background:#fff1f2}.patch-side.added{background:#eaf8ef}
.patch-side pre{white-space:pre;overflow:auto;max-width:100%;margin:0;padding:3px 7px;line-height:1.55;font:12px/1.6 ui-monospace,SFMono-Regular,Consolas,monospace;tab-size:2}
.line-no{display:inline-block;min-width:34px;padding:4px 5px;background:#f5f7fa;border-right:1px solid #e5e9f0;text-align:right;color:#8a99ab;user-select:none;font:10px/1.8 ui-monospace,monospace}
.patch-row.header .patch-side{background:#eef5ff;color:#3869a9}
.patch-row.meta .patch-side{background:#f9fafc;color:#8a97a6}
.git-unified-patch{margin:0;padding:12px;max-height:75vh;overflow:auto;background:#fbfcff;font:12px/1.6 ui-monospace,monospace;white-space:pre}
.git-more{color:#a36312;background:#fff8e8;font-size:12px;padding:8px;margin:0}
</style>
