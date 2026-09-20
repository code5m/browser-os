<script setup lang="ts">
import { ref } from "vue";
import { useRepoStore } from "../../stores/useRepoStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import GitPanel from "./GitPanel.vue";
import GitHistory from './GitHistory.vue';

const rp = useRepoStore();
const layout = useLayoutStore();
// M1-7：默认进「状态」Tab（Git UI）；「配置」仍是既有仓库配置表单（含 token 录入）。
const tab = ref<"git" | "history" | "config">("git");
</script>

<template>
  <div class="side-inner">
    <div class="tabs">
      <button :class="{ active: tab === 'git' }" @click="tab = 'git'">🔀 状态</button>
      <button :class="{ active: tab === 'history' }" @click="tab = 'history'">提交历史</button>
      <button :class="{ active: tab === 'config' }" @click="tab = 'config'">⚙️ 配置</button>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>
    <GitPanel v-if="tab === 'git'" />
    <GitHistory v-else-if="tab === 'history'" />
    <div v-else class="repo-panel">
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
      <ul class="repo-list">
        <li v-for="r in rp.repos" :key="r.id" class="repo">
          <div>
            <div class="title">{{ r.name }} ({{ r.provider }})</div>
            <div class="meta">{{ r.remote_url }}</div>
          </div>
          <button @click="rp.requestSync(r.id)">推送</button>
        </li>
        <li v-if="!rp.repos.length" class="empty">还没配置仓库</li>
      </ul>
    </div>
  </div>
</template>
