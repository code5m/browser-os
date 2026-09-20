<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useSnippetStore } from "../../stores/useSnippetStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import {
  buildSnippetCategoryTree,
  canDeleteSnippet,
  isFavoriteSnippet,
  toggleSnippetFavorite,
  validateSnippetForm,
} from "../../utils/snippetUi";
import type { CommandSnippet } from "../../types";
import ScriptParamForm from "./ScriptParamForm.vue";
import ScriptRunDialog from "./ScriptRunDialog.vue";

const sn = useSnippetStore();
const layout = useLayoutStore();

const selectedId = ref<string | null>(null);
const editing = ref(false);
const running = ref<CommandSnippet | null>(null);
const query = ref("");
const favoritesOnly = ref(false);
const favVersion = ref(0);

const tree = computed(() => {
  favVersion.value;
  return buildSnippetCategoryTree(sn.snippets, query.value, favoritesOnly.value);
});
const issues = computed(() => validateSnippetForm(sn.snippetForm));

onMounted(() => {
  sn.loadSnippets();
});

function newSnippet() {
  sn.openSnippetForm(null);
  editing.value = true;
  selectedId.value = null;
}
function editSnippet(m: CommandSnippet) {
  selectedId.value = m.id;
  sn.openSnippetForm(m);
  editing.value = true;
}
function backToList() {
  editing.value = false;
  selectedId.value = null;
}
async function save() {
  if (issues.value.length) {
    layout.showToast("表单校验未通过：" + issues.value[0].message);
    return;
  }
  await sn.saveSnippet();
  editing.value = false;
  selectedId.value = null;
}
async function remove(m: CommandSnippet) {
  if (!canDeleteSnippet(m)) {
    layout.showToast("内置命令不可删除");
    return;
  }
  if (!confirm(`删除命令「${m.name}」？此操作不可恢复`)) return;
  await sn.removeSnippet(m);
}
function toggleFav(id: string) {
  toggleSnippetFavorite(id);
  favVersion.value += 1;
}
function openRun(m: CommandSnippet) {
  if (m.dangerous && !confirm(`运行高风险命令「${m.name}」？`)) return;
  running.value = m;
}
</script>

<template>
  <div class="side-inner snippet-panel">
    <div class="tabs">
      <span>⚡ 命令库</span>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>

    <div v-if="!editing" class="list-pane">
      <div class="toolbar">
        <input v-model="query" placeholder="搜索名称、分类或 argv" />
        <button :class="{ active: favoritesOnly }" title="只看收藏" @click="favoritesOnly = !favoritesOnly">★</button>
        <button class="new-btn" @click="newSnippet">+ 新建</button>
      </div>

      <div v-if="!tree.length" class="empty">暂无匹配命令</div>
      <div v-for="node in tree" :key="node.category" class="cat">
        <div class="cat-title">{{ node.category }}（{{ node.snippets.length }}）</div>
        <ul>
          <li v-for="s in node.snippets" :key="s.id" :class="{ builtin: s.builtin, disabled: !s.enabled }">
            <button class="fav" :class="{ on: isFavoriteSnippet(s.id) }" title="收藏" @click.stop="toggleFav(s.id)">★</button>
            <button class="row" @click="editSnippet(s)">
              <span class="name">{{ s.name }}</span>
              <span class="meta">
                {{ s.argv.join(" ") }}{{ s.dangerous ? " · 高风险" : "" }}{{ s.builtin ? " · 内置" : "" }}{{ s.enabled ? "" : " · 已禁用" }}
              </span>
            </button>
            <button class="run" title="运行" :disabled="!s.enabled" @click.stop="openRun(s)">▶</button>
            <button v-if="canDeleteSnippet(s)" class="del" title="删除" @click.stop="remove(s)">🗑</button>
            <span v-else class="lock" title="内置命令不可删除">🔒</span>
          </li>
        </ul>
      </div>
    </div>

    <div v-else class="edit-pane">
      <button class="back" @click="backToList">← 返回列表</button>
      <form @submit.prevent="save">
        <label>名称<input v-model="sn.snippetForm.name" placeholder="命令名（字母数字 _ - . 空格）" /></label>
        <label>分类<input v-model="sn.snippetForm.category" placeholder="如 general / ops" /></label>
        <label>解释器
          <select v-model="sn.snippetForm.interpreter">
            <option value="bash">bash</option>
            <option value="sh">sh</option>
            <option value="python3">python3</option>
            <option value="node">node</option>
          </select>
        </label>
        <label>描述<textarea v-model="sn.snippetForm.description" rows="2" placeholder="可选"></textarea></label>
        <label>argv（一行一个元素）
          <textarea v-model="sn.snippetForm.argvText" rows="7" class="argv" placeholder="df&#10;-h&#10;{PATH}"></textarea>
        </label>
        <label>超时(秒，0=全局默认)<input type="number" min="0" v-model.number="sn.snippetForm.timeout_secs" /></label>
        <div class="checks">
          <label><input type="checkbox" v-model="sn.snippetForm.enabled" /> 启用</label>
          <label><input type="checkbox" v-model="sn.snippetForm.dangerous" /> 高风险运行前确认</label>
        </div>

        <ScriptParamForm v-model:params="sn.snippetForm.params" />

        <ul v-if="issues.length" class="errors">
          <li v-for="(it, i) in issues" :key="i">⚠ {{ it.message }}</li>
        </ul>

        <div class="form-actions">
          <button type="submit" class="primary">保存</button>
          <button type="button" @click="backToList">取消</button>
        </div>
      </form>
    </div>

    <ScriptRunDialog v-if="running" :script="running" kind="command" @close="running = null" />
  </div>
</template>

<style scoped>
.snippet-panel { display: flex; flex-direction: column; height: 100%; }
.list-pane { overflow: auto; flex: 1; }
.toolbar { display: grid; grid-template-columns: 1fr 30px auto; gap: 6px; padding: 8px; border-bottom: 1px solid #e5e6eb; }
.toolbar input { min-width: 0; border: 1px solid #d5dbe7; border-radius: 5px; padding: 5px 7px; font-size: 12px; }
.toolbar button { border: 1px solid #d5dbe7; background: #fff; border-radius: 5px; cursor: pointer; font-size: 12px; }
.toolbar button.active { color: #b7791f; border-color: #b7791f; background: #fff7e6; }
.new-btn { padding: 5px 10px; color: #fff; background: #2b6cb0 !important; border-color: #2b6cb0 !important; }
.cat { margin: 4px 8px; }
.cat-title { font-size: 12px; color: #86909c; padding: 4px 0; }
.cat ul { list-style: none; margin: 0; padding: 0; }
.cat li { display: flex; align-items: center; gap: 4px; padding: 2px 0; }
.cat li.builtin { opacity: 0.9; }
.cat li.disabled { opacity: 0.55; }
.fav, .run, .del { width: 24px; height: 24px; border: none; background: transparent; cursor: pointer; font-size: 14px; }
.fav { color: #c5cad5; }
.fav.on { color: #b7791f; }
.row { flex: 1; min-width: 0; display: flex; flex-direction: column; align-items: flex-start; text-align: left; background: transparent; border: none; cursor: pointer; padding: 4px 6px; border-radius: 5px; }
.row:hover { background: #eef2fb; }
.name { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: #1f2329; }
.meta { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; color: #86909c; }
.run { color: #2b6cb0; }
.run:disabled { opacity: 0.45; cursor: not-allowed; }
.lock { font-size: 12px; opacity: 0.5; }
.empty { color: #86909c; font-size: 12px; padding: 16px; text-align: center; }
.edit-pane { overflow: auto; flex: 1; padding: 8px; }
.edit-pane label { display: block; font-size: 12px; color: #4e5969; margin: 8px 0 2px; }
.edit-pane input, .edit-pane select, .edit-pane textarea { width: 100%; box-sizing: border-box; border: 1px solid #d5dbe7; border-radius: 5px; padding: 5px 7px; font-size: 12px; }
.edit-pane textarea.argv { font-family: monospace; }
.checks { display: flex; flex-direction: column; gap: 4px; margin: 8px 0; }
.checks label { display: flex; align-items: center; gap: 6px; margin: 0; }
.checks input { width: auto; }
.errors { color: #c0392b; font-size: 12px; margin-top: 6px; }
.form-actions { margin-top: 10px; display: flex; gap: 8px; }
.form-actions button { padding: 6px 14px; border-radius: 5px; border: 1px solid #d5dbe7; background: #fff; cursor: pointer; font-size: 12px; }
.form-actions .primary { background: #2b6cb0; border-color: #2b6cb0; color: #fff; }
.back { margin: 4px 0; border: none; background: transparent; color: #2b6cb0; cursor: pointer; font-size: 12px; }
</style>
