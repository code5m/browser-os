<script setup lang="ts">
import { EmptyState } from "../../../shared/ui";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { currentLocalGitIntent, subscribeLocalGitIntent } from "../../../composables/gitLocalNavigation";
import { useRepoStore } from "../state/useRepoStore";
import { contributionRegistry } from "../../../capability/contribution/registry";
import { CONTRIBUTION_SLOTS } from "../../../capability/contribution/types";

const rp = useRepoStore();
const preferHistory = ref(Boolean(currentLocalGitIntent()));
const showConfig = ref(false);
let unsubscribe=()=>{};
onMounted(()=>{unsubscribe=subscribeLocalGitIntent(intent=>{preferHistory.value=Boolean(intent);});});
onUnmounted(()=>unsubscribe());

// Git 工作区经通用 Contribution Registry 的 REPO_SUBVIEW 槽贡献。
// RepoPanel 只按 view 认领渲染，不 import Git 能力内部，避免 workspace→git 反向依赖环。
// Git absent（未注册/未 activate）时槽为空，RepoPanel 仅保留仓库配置能力。
const gitPanelComp = computed(() => contributionRegistry
  .getSurfaceContributions(CONTRIBUTION_SLOTS.REPO_SUBVIEW)
  .find((c) => c.view === "git")?.component)
const gitHistoryComp = computed(() => contributionRegistry
  .getSurfaceContributions(CONTRIBUTION_SLOTS.REPO_SUBVIEW)
  .find((c) => c.view === "history")?.component)
const gitWorkspaceComp = computed(() => preferHistory.value && gitHistoryComp.value ? gitHistoryComp.value : gitPanelComp.value || gitHistoryComp.value)
</script>

<template>
  <div class="side-inner">
    <component :is="gitWorkspaceComp" v-if="gitWorkspaceComp && !showConfig" @configure="showConfig = true" />
    <div v-else class="repo-panel">
      <div v-if="gitWorkspaceComp" class="repo-config-header"><button @click="showConfig = false">← 返回 Git 工作台</button><strong>仓库配置</strong></div>
      <div class="form">
        <input v-model="rp.form.name" placeholder="仓库名" />
        <select v-model="rp.form.provider">
          <option value="git">git</option>
          <option value="gitee">码云 gitee</option>
        </select>
        <input v-model="rp.form.remote_url" placeholder="https://.../repo.git" />
        <input v-model="rp.form.branch" placeholder="分支" />
        <input v-model="rp.form.username" placeholder="用户名(可留空)" />
        <input v-model="rp.form.token" type="password" placeholder="Token(仅存密钥库)" />
        <div class="form-actions">
          <button @click="rp.saveRepo">保存仓库</button>
          <button class="ghost" @click="rp.loadGiteeExample">码云示例</button>
        </div>
      </div>
      <p class="git-hint">这里配置 Git 仓库并同步所选资产。Git 提交、历史、分支与推送操作统一在 Git 工作台中完成。</p>
      <ul class="repo-list">
        <li v-for="r in rp.repos" :key="r.id" class="repo">
          <div>
            <div class="title">{{ r.name }} ({{ r.provider }})</div>
            <div class="meta">{{ r.remote_url }}</div>
          </div>
          <button title="把在资产管理中选中的成果同步到此仓库；不是 Git 分支推送" @click="rp.requestSync(r.id)">同步所选资产</button>
        </li>
        <EmptyState v-if="!rp.repos.length" as="li" text="还没配置仓库" />
      </ul>
    </div>
  </div>
</template>
<style scoped>
.repo-config-header{display:flex;align-items:center;gap:12px;margin:-2px -2px 12px;padding:8px 10px;border-bottom:1px solid var(--border,#dce4ed)}
.repo-config-header button{border:1px solid var(--border,#c6d3e0);border-radius:5px;padding:5px 8px;background:var(--surface,#fff);color:inherit}
</style>
