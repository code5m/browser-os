<script setup lang="ts">
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
const ws = useWorkspaceStore();
const layout = useLayoutStore();
</script>

<template>
  <div class="side-inner">
    <div class="tabs">
      <span>☁ 自有仓库</span>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>
    <div class="repo-panel">
      <div class="form">
        <input v-model="ws.form.name" placeholder="仓库名" />
        <select v-model="ws.form.provider">
          <option value="git">git</option>
          <option value="gitee">码云 gitee</option>
        </select>
        <input v-model="ws.form.remote_url" placeholder="https://.../repo.git" />
        <input v-model="ws.form.branch" placeholder="分支" />
        <input v-model="ws.form.username" placeholder="用户名(可留空)" />
        <input v-model="ws.form.token" type="password" placeholder="Token(仅存密钥库)" />
        <div class="form-actions">
          <button @click="ws.saveRepo">保存仓库</button>
          <button class="ghost" @click="ws.loadGiteeExample">码云示例</button>
        </div>
      </div>
      <ul class="repo-list">
        <li v-for="r in ws.repos" :key="r.id" class="repo">
          <div>
            <div class="title">{{ r.name }} ({{ r.provider }})</div>
            <div class="meta">{{ r.remote_url }}</div>
          </div>
          <button @click="ws.requestSync(r.id)">推送</button>
        </li>
        <li v-if="!ws.repos.length" class="empty">还没配置仓库</li>
      </ul>
    </div>
  </div>
</template>
