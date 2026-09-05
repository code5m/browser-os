<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import { buildCategoryTree, validateScriptForm, canDeleteScript } from "../../utils/scriptUi";
import type { ScriptMeta } from "../../types";
import ScriptParamForm from "./ScriptParamForm.vue";

const ws = useWorkspaceStore();
const layout = useLayoutStore();

const selectedId = ref<string | null>(null);
const editing = ref(false);

const tree = computed(() => buildCategoryTree(ws.scripts));
const issues = computed(() => validateScriptForm(ws.scriptForm));

onMounted(() => {
  ws.loadScripts();
});

function newScript() {
  ws.openScriptForm(null);
  editing.value = true;
  selectedId.value = null;
}
function editScript(m: ScriptMeta) {
  selectedId.value = m.id;
  ws.openScriptForm(m);
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
  await ws.saveScript();
  editing.value = false;
  selectedId.value = null;
}
async function remove(m: ScriptMeta) {
  if (!canDeleteScript(m)) {
    layout.showToast("内置脚本不可删除");
    return;
  }
  // 删除二次确认（U4/U5）
  if (!confirm(`删除脚本「${m.name}」？此操作不可恢复`)) return;
  await ws.removeScript(m);
}
</script>

<template>
  <div class="side-inner script-panel">
    <div class="tabs">
      <span>📜 脚本库</span>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>

    <!-- 列表态 -->
    <div v-if="!editing" class="list-pane">
      <button class="new-btn" @click="newScript">+ 新建脚本</button>
      <div v-if="!ws.scripts.length" class="empty">暂无脚本，点「新建脚本」添加一个</div>
      <div v-for="node in tree" :key="node.category" class="cat">
        <div class="cat-title">{{ node.category }}（{{ node.scripts.length }}）</div>
        <ul>
          <li v-for="s in node.scripts" :key="s.id" :class="{ builtin: s.builtin }">
            <button class="row" @click="editScript(s)">
              <span class="name">{{ s.name }}</span>
              <span class="meta">{{ s.interpreter }}{{ s.builtin ? " · 内置" : "" }}{{ s.enabled ? "" : " · 已禁用" }}</span>
            </button>
            <button v-if="canDeleteScript(s)" class="del" title="删除" @click="remove(s)">🗑</button>
            <span v-else class="lock" title="内置脚本不可删除">🔒</span>
          </li>
        </ul>
      </div>
    </div>

    <!-- 编辑态 -->
    <div v-else class="edit-pane">
      <button class="back" @click="backToList">← 返回列表</button>
      <!-- 后端调用统一经 bridge.ts，组件不直接 invoke -->
      <form @submit.prevent="save">
        <label>名称<input v-model="ws.scriptForm.name" placeholder="脚本名（字母数字 _ - . 空格）" /></label>
        <label>分类<input v-model="ws.scriptForm.category" placeholder="如 general / ops" /></label>
        <label>解释器
          <select v-model="ws.scriptForm.interpreter">
            <option value="bash">bash</option>
            <option value="sh">sh</option>
            <option value="python3">python3</option>
            <option value="node">node</option>
            <option value="shebang">shebang</option>
          </select>
        </label>
        <label>描述<textarea v-model="ws.scriptForm.description" rows="2" placeholder="可选"></textarea></label>
        <label>正文
          <!-- 前端只传正文，路径由后端按 <id>.<ext> 生成 -->
          <textarea v-model="ws.scriptForm.body" rows="8" placeholder="脚本正文" class="body"></textarea>
        </label>
        <label>超时(秒，0=全局默认)<input type="number" min="0" v-model.number="ws.scriptForm.timeout_secs" /></label>
        <label class="chk"><input type="checkbox" v-model="ws.scriptForm.enabled" /> 启用</label>

        <ScriptParamForm v-model:params="ws.scriptForm.params" />

        <ul v-if="issues.length" class="errors">
          <li v-for="(it, i) in issues" :key="i">⚠ {{ it.message }}</li>
        </ul>

        <div class="form-actions">
          <button type="submit" class="primary">保存</button>
          <button type="button" @click="backToList">取消</button>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.script-panel { display: flex; flex-direction: column; height: 100%; }
.new-btn { margin: 8px; padding: 6px 10px; border: 1px solid #2b6cb0; background: #2b6cb0; color: #fff; border-radius: 5px; cursor: pointer; font-size: 12px; }
.list-pane { overflow: auto; flex: 1; }
.cat { margin: 4px 8px; }
.cat-title { font-size: 12px; color: #86909c; padding: 4px 0; }
.cat ul { list-style: none; margin: 0; padding: 0; }
.cat li { display: flex; align-items: center; gap: 4px; padding: 2px 0; }
.cat li.builtin { opacity: 0.85; }
.row { flex: 1; display: flex; flex-direction: column; align-items: flex-start; text-align: left; background: transparent; border: none; cursor: pointer; padding: 4px 6px; border-radius: 5px; }
.row:hover { background: #eef2fb; }
.name { font-size: 13px; color: #1f2329; }
.meta { font-size: 11px; color: #86909c; }
.del { border: none; background: transparent; cursor: pointer; font-size: 14px; }
.lock { font-size: 12px; opacity: 0.5; }
.empty { color: #86909c; font-size: 12px; padding: 16px; text-align: center; }
.edit-pane { overflow: auto; flex: 1; padding: 8px; }
.edit-pane label { display: block; font-size: 12px; color: #4e5969; margin: 8px 0 2px; }
.edit-pane input, .edit-pane select, .edit-pane textarea { width: 100%; box-sizing: border-box; border: 1px solid #d5dbe7; border-radius: 5px; padding: 5px 7px; font-size: 12px; }
.edit-pane textarea.body { font-family: monospace; }
.chk { display: flex; align-items: center; gap: 6px; }
.chk input { width: auto; }
.errors { color: #c0392b; font-size: 12px; margin-top: 6px; }
.form-actions { margin-top: 10px; display: flex; gap: 8px; }
.form-actions button { padding: 6px 14px; border-radius: 5px; border: 1px solid #d5dbe7; background: #fff; cursor: pointer; font-size: 12px; }
.form-actions .primary { background: #2b6cb0; border-color: #2b6cb0; color: #fff; }
.back { margin: 4px 0; border: none; background: transparent; color: #2b6cb0; cursor: pointer; font-size: 12px; }
</style>
