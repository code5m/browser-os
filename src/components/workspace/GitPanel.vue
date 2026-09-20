<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRepoStore } from "../../capabilities/workspace/public";
import { useGitStore, PUSH_NOTICE } from "../../stores/useGitStore";
import GitDiffViewer from "./GitDiffViewer.vue";

// M1-7 Git UI 主面板：仓库选择 + 状态列表 + diff 预览 + 分支信息 + 写操作入口。
// 仓库数据复用既有 useWorkspaceStore.repos（不新造 repo 存储）；
// 所有写操作都经 useGitStore 的双阶段闸门（request → confirm），本组件无直写路径。
const ws = useRepoStore();
const git = useGitStore();

const targetBranch = ref("");

// 分支列表变化时，默认选中当前分支（避免下拉停在空值导致误切）
watch(
  () => [git.currentBranch, git.localBranches] as const,
  ([cur]) => {
    if (cur && !git.localBranches.includes(targetBranch.value)) targetBranch.value = cur;
  },
  { immediate: true }
);

const noRepo = computed(() => !git.repoId);
const disabled = computed(() => noRepo.value || git.busy);
const selectedCount = computed(() => git.selectedPaths.length);

// 勾选为空时 commit 会走「全量提交」，明确告知用户（后端 summary 也会写明）
const commitHint = computed(() =>
  selectedCount.value
    ? `提交勾选的 ${selectedCount.value} 个路径`
    : "未勾选文件时提交 = 提交工作区全部变更（后端预览会写明条数）"
);
</script>

<template>
  <div class="git-panel">
    <!-- 仓库选择 -->
    <div class="git-head">
      <select
        :value="git.repoId ?? ''"
        :disabled="git.busy"
        @change="git.selectRepo(($event.target as HTMLSelectElement).value || null)"
      >
        <option value="">— 选择仓库 —</option>
        <option v-for="r in ws.repos" :key="r.id" :value="r.id">{{ r.name }}（{{ r.branch }}）</option>
      </select>
      <button :disabled="disabled" @click="git.refreshAll()">刷新</button>
    </div>

    <div v-if="!ws.repos.length" class="git-empty">
      还没有配置仓库，请切到「配置」Tab 添加（Git 只读/写能力只能作用于已配置并已同步的仓库）。
    </div>

    <!-- 分支信息 -->
    <div class="git-branch-bar">
      <span>当前分支：<b class="cur">{{ git.currentBranch ?? "—" }}</b></span>
      <span class="spacer" />
      <select v-model="targetBranch" :disabled="disabled || !git.localBranches.length">
        <option v-for="b in git.localBranches" :key="b" :value="b">{{ b }}</option>
      </select>
      <button :disabled="disabled || !targetBranch" @click="git.checkoutBranch(targetBranch)">
        切换
      </button>
    </div>
    <div class="git-branch-bar">
      <input v-model="git.branchName" type="text" placeholder="新分支名" :disabled="disabled" />
      <label class="git-inline">
        <input v-model="git.createCheckout" type="checkbox" :disabled="disabled" />
        创建后切换
      </label>
      <button :disabled="disabled || !git.branchName.trim()" @click="git.createBranch()">
        新建分支
      </button>
      <span v-if="git.remoteBranches.length" class="git-hint">
        远端分支 {{ git.remoteBranches.length }} 个（只读展示，本期不支持 pull/fetch）
      </span>
    </div>

    <div v-if="git.branchError" class="git-error">分支读取失败：{{ git.branchError }}</div>

    <!-- 状态列表 + diff -->
    <div class="git-body">
      <div class="git-col git-status-col">
        <div class="git-col-head">
          <span>🔀 改动（{{ git.status.length }}）</span>
          <span class="spacer" />
          <button :disabled="disabled" @click="git.selectAll()">全选</button>
          <button :disabled="disabled" @click="git.clearSelection()">清空</button>
          <button :disabled="disabled" @click="git.loadStatus()">刷新</button>
        </div>
        <div v-if="git.statusLoading" class="git-empty">状态加载中…</div>
        <div v-else-if="git.statusError" class="git-error">状态读取失败：{{ git.statusError }}</div>
        <div v-else-if="!git.status.length" class="git-empty">工作区干净（无改动）</div>
        <ul v-else class="git-list">
          <li
            v-for="f in git.status"
            :key="f.path"
            class="git-item"
            :class="{ active: git.activePath === f.path }"
            @click="git.loadDiff(f.path)"
          >
            <input
              type="checkbox"
              :checked="git.selected.has(f.path)"
              @click.stop
              @change="git.toggleSelect(f.path)"
            />
            <span class="gbadge" :class="f.status">{{ f.status }}</span>
            <span class="gpath" :title="f.path">{{ f.path }}</span>
          </li>
        </ul>
      </div>

      <GitDiffViewer />
    </div>

    <!-- 写操作入口（全部走 request → confirm 闸门） -->
    <div class="git-actions">
      <div class="git-action-row">
        <button :disabled="disabled || !selectedCount" @click="git.stage()">暂存</button>
        <button :disabled="disabled || !selectedCount" @click="git.unstage()">取消暂存</button>
        <button class="danger" :disabled="disabled || !selectedCount" @click="git.discard()">
          丢弃改动
        </button>
        <span class="git-hint">已勾选 {{ selectedCount }} 项</span>
      </div>

      <div class="git-action-row">
        <input
          v-model="git.commitMessage"
          type="text"
          placeholder="提交信息（必填）"
          :disabled="disabled"
        />
        <button
          class="primary"
          :disabled="disabled || !git.commitMessage.trim()"
          @click="git.commit()"
        >提交</button>
      </div>
      <div class="git-hint">{{ commitHint }}</div>

      <div class="git-action-row">
        <button class="danger" :disabled="disabled" @click="git.push()">推送到 origin</button>
        <span class="git-hint">{{ PUSH_NOTICE }}</span>
      </div>

      <div v-if="git.writeError" class="git-error">{{ git.writeError }}</div>
      <div v-if="git.busy" class="git-hint">写操作执行中…</div>
    </div>
  </div>
</template>
