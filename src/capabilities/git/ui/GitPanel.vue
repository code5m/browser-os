<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRepoStore } from "../../workspace/public";
import { useGitStore, PUSH_NOTICE } from "../public";
import GitDiffViewer from "./GitDiffViewer.vue";

// M1-7 Git UI 主面板：仓库选择 + 状态列表 + diff 预览 + 分支信息 + 写操作入口。
// 仓库数据复用既有 useWorkspaceStore.repos（不新造 repo 存储）；
// 所有写操作都经 useGitStore 的双阶段闸门（request → confirm），本组件无直写路径。
const ws = useRepoStore();
const git = useGitStore();

const targetBranch = ref("");
watch(() => git.currentBranch, branch => { if (branch) targetBranch.value = branch }, { immediate: true });
const noRepo = computed(() => !git.repoId);
const disabled = computed(() => noRepo.value || git.busy);
const selectedCount = computed(() => git.selectedPaths.length);
// GitFileStatus currently does not include an index/staging bit. Do not
// misrepresent these categories as "staged" vs "unstaged".
const changeGroups = computed(() => [
  { key:"conflicted", label:"需要解决的冲突", files:git.status.filter(f => f.status==="conflicted") },
  { key:"untracked", label:"未跟踪文件", files:git.status.filter(f => f.status==="untracked") },
  { key:"tracked", label:"已跟踪的改动（含可能已暂存）",
    files:git.status.filter(f => f.status!=="conflicted" && f.status!=="untracked") },
].filter(g => g.files.length));
const fileStateLabel: Record<string,string> = {
  modified:"修改", added:"新增", deleted:"删除", renamed:"重命名", untracked:"未跟踪", conflicted:"冲突"
};

// 仅勾选文件允许提交；拒绝静默全量提交。
const commitHint = computed(() =>
  selectedCount.value
    ? "只提交明确勾选的 " + selectedCount.value + " 个路径（最终以原生预览为准）"
    : "请先勾选文件；想提交全部时点击全选。"
);
</script>

<template>
  <div class="git-panel">
    <!-- 仓库选择 -->
    <div class="git-head">
      <strong class="git-studio-title">Git Studio</strong>
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

    <!-- Branch status, with explicit non-force remote-push semantics. -->
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
          <strong>改动文件（{{ git.status.length }}）</strong>
          <span class="spacer" />
          <button :disabled="disabled" @click="git.selectAll()">全选</button>
          <button :disabled="disabled" @click="git.clearSelection()">清空</button>
          <button :disabled="disabled" @click="git.loadStatus()">刷新</button>
        </div>
        <div v-if="git.statusLoading" class="git-empty">状态加载中…</div>
        <div v-else-if="git.statusError" class="git-error">状态读取失败：{{ git.statusError }}</div>
        <div v-else-if="!git.status.length" class="git-empty">工作区干净（无改动）</div>
        <div v-else class="git-change-groups">
          <section v-for="group in changeGroups" :key="group.key" class="git-change-group">
            <h4>{{group.label}} <span>({{group.files.length}})</span></h4>
            <ul class="git-list">
              <li v-for="f in group.files" :key="f.path" class="git-item"
                :class="{active:git.activePath===f.path}" @click="git.loadDiff(f.path)"
                @keydown.enter.prevent="git.loadDiff(f.path)" tabindex="0">
                <input type="checkbox" :aria-label="'选择 '+f.path"
                  :checked="git.selected.has(f.path)" @click.stop @change="git.toggleSelect(f.path)" />
                <span class="gbadge" :class="f.status">{{fileStateLabel[f.status] || f.status}}</span>
                <span class="gpath" :title="f.path">{{f.path}}</span>
              </li>
            </ul>
          </section>
        </div>
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
          :disabled="disabled || !git.commitMessage.trim() || !selectedCount"
          @click="git.commit()"
        >提交</button>
      </div>
      <div class="git-hint">{{commitHint}}</div>
      <p class="git-hint">当前原生接口未单独返回暂存状态，因此文件分组不会冒充「已暂存/未暂存」。所有写操作仍经过预览和确认。</p>

      <div class="git-action-row">
        <button class="danger" :disabled="disabled" @click="git.push()">推送 Git 分支到 origin</button>
        <span class="git-hint">{{ PUSH_NOTICE }}</span>
      </div>

      <div v-if="git.writeError" class="git-error">{{ git.writeError }}</div>
      <div v-if="git.busy" class="git-hint">写操作执行中…</div>
    </div>
  </div>
</template>

<style scoped>
.git-panel{background:#f9fbfe;color:#273a50}
.git-studio-title{color:#26589f;font-size:13px;white-space:nowrap}
.git-branch-bar{background:#fff;border:1px solid #e4eaf2;border-radius:7px;padding:7px 9px}
.git-body{gap:8px}
.git-change-groups{flex:1;overflow:auto}
.git-change-group{border-bottom:1px solid #e8edf5}
.git-change-group h4{background:#f3f6fa;color:#486079;padding:8px 9px;margin:0;font-size:12px}
.git-change-group h4 span{color:#8799a9;font-weight:400}
.git-item{padding:7px 9px}
.git-item:focus-visible{outline:2px solid #7ea6e6}
.git-col-head strong{font-size:12px}
.git-actions{background:#fff;border-radius:9px;border:1px solid #e4eaf2;padding:10px}
.git-head select{background:#fff}
</style>
