<script setup lang="ts">
import { EmptyState } from "../../../shared/ui";
import { computed, onMounted, ref, watch } from "vue";
import { Plus, X, Square, ChevronLeft, ChevronRight, RefreshCw } from '@lucide/vue';
import { useDatabaseStore } from "../../../stores/useDatabaseStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useWorkbenchStore } from "../../../stores/useWorkbenchStore";
import {
  databaseFieldLabel,
  defaultPortFor,
  kindLabel,
  looksLikeMultipleStatements,
  productionLabel,
  riskLabel,
  toCsv,
  truncateText,
  visibleFields,
  type DbKind,
} from "../../../utils/dbUi";

const db = useDatabaseStore();
const layout = useLayoutStore();
const workbench = useWorkbenchStore();

// 密码只作**组件本地瞬时态**：不进 store、不进表单模型、不持久化（F2）。
const password = ref("");
const copied = ref(false);
const page = ref(0);
const filter = ref('');
const sortColumn = ref(-1);
const sortAscending = ref(true);
const showConnection = ref(true);
const pageSize = 50;
const filteredRows = computed(() => {
  const rows = (db.result?.rows || []).filter(row => row.some(cell => cell.toLowerCase().includes(filter.value.toLowerCase())));
  return sortColumn.value < 0 ? rows : [...rows].sort((a,b) => (a[sortColumn.value] || '').localeCompare(b[sortColumn.value] || '', undefined, {numeric:true}) * (sortAscending.value ? 1 : -1));
});
watch(() => [db.activeDocument,db.result,filter.value], () => page.value = 0);
function sort(column:number) { sortAscending.value = column === sortColumn.value ? !sortAscending.value : true; sortColumn.value = column; }
/** 仅限制 DOM 渲染行数（首期无虚拟滚动）；与后端取数上限 DB_MAX_ROWS=1000 是两回事，UI 须说清。 */
const DISPLAY_ROW_CAP = 200;

const fields = computed(() => visibleFields(db.form.kind));
const issues = computed(() => db.formIssues);
const portPlaceholder = computed(() => {
  const p = defaultPortFor(db.form.kind);
  return p ? String(p) : "默认端口";
});
const displayRows = computed(() => filteredRows.value.slice(page.value * pageSize, (page.value+1)*pageSize));
const hiddenRows = computed(() => Math.max(0, (db.result?.rowCount ?? 0) - displayRows.value.length));
const multiStatementHint = computed(() => looksLikeMultipleStatements(db.sql));

onMounted(() => {
  db.refreshConnections();
});

async function doConnect() {
  const okConnected = await db.connect(password.value);
  // 无论成败都立即清空本地密码，避免密码在内存中多停留一轮渲染
  password.value = "";
  if (okConnected) layout.showToast("已连接");
}

async function copyCsv() {
  if (!db.result) return;
  const text = toCsv(db.document.raw?.columns || db.result.columns, db.document.raw?.rows || db.result.rows);
  try {
    await navigator.clipboard.writeText(text);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1500);
  } catch {
    layout.showToast("复制失败，请手动选择结果");
  }
}
</script>

<template>
  <div class="side-inner db-panel">
    <div class="tabs">
      <button @click="showConnection = !showConnection">连接 / 对象</button>
      <span>数据库 · {{ db.connections.find(c => c.id === db.activeId)?.name || '未连接' }}</span>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>

    <div v-if="!db.backendReady" class="gate">后端数据库命令未就绪，连接与查询暂不可用</div>

    <div class="db-workspace">
    <section v-if="showConnection && !workbench.collapsed" class="conn">
      <div class="conn-head">
        <span class="sec-title">连接</span>
        <span v-if="db.connected" class="badge on">已连接 · {{ db.activeId }}</span>
        <span v-else class="badge">未连接</span>
      </div>

      <ul v-if="db.connections.length" class="conn-list">
        <li v-for="c in db.connections" :key="c.id" :class="{ active: c.id === db.form.id, disabled: !c.enabled }">
          <button class="row" @click="db.selectConnection(c)">
            <span class="name">{{ c.name }}</span>
            <span class="meta">{{ c.label }}{{ c.allowWrite ? " · 允许写" : "" }}{{ c.enabled ? "" : " · 已禁用" }}</span>
          </button>
        </li>
      </ul>

      <div class="form">
        <label>名称<input v-model="db.form.name" placeholder="如 local-sqlite" /></label>
        <label>驱动
          <select :value="db.form.kind" @change="(e) => db.setKind(((e.target as HTMLSelectElement).value) as DbKind)">
            <option value="sqlite">SQLite</option>
            <option value="mysql" disabled>MySQL（查询尚未接通）</option>
            <option value="postgres" disabled>PostgreSQL（查询尚未接通）</option>
          </select>
        </label>
        <label v-if="fields.host">主机<input v-model="db.form.host" placeholder="127.0.0.1" /></label>
        <label v-if="fields.port">端口<input v-model="db.form.portText" :placeholder="portPlaceholder" /></label>
        <label>{{ databaseFieldLabel(db.form.kind) }}<input v-model="db.form.database" placeholder="/data/app.db 或 app" /></label>
        <label v-if="fields.username">用户名<input v-model="db.form.username" placeholder="reader" /></label>
        <label v-if="fields.username">密码（仅本次提交，不保存）<input v-model="password" type="password" autocomplete="off" /></label>
        <label v-if="fields.ssl">SSL
          <select v-model="db.form.sslMode">
            <option value="disable">disable</option>
            <option value="prefer">prefer</option>
            <option value="require">require</option>
          </select>
        </label>
        <label>生产标记
          <select v-model="db.form.productionHint">
            <option value="">未标记</option>
            <option value="yes">是生产</option>
            <option value="no">非生产</option>
          </select>
        </label>

        <div class="checks">
          <label><input type="checkbox" v-model="db.form.allowWrite" /> 允许写操作（默认关闭，后端仍会再判一次）</label>
          <label><input type="checkbox" v-model="db.form.enabled" /> 启用该连接</label>
        </div>

        <ul v-if="issues.length" class="errors">
          <li v-for="(it, i) in issues" :key="i">⚠ {{ it.message }}</li>
        </ul>

        <div class="form-actions">
          <button class="primary" :disabled="db.busy || !db.backendReady" @click="doConnect">连接</button>
          <button :disabled="db.busy || !db.connected" @click="db.disconnect()">断开</button>
          <button :disabled="db.busy" @click="db.resetForm()">重置</button>
        </div>
      </div>
      <div class="schema-head"><strong>表 / 视图</strong><button title="刷新对象" :disabled="db.schemaBusy || !db.connected" @click="db.refreshSchema()"><RefreshCw :size="14"/></button></div>
      <button v-for="table in db.schema" :key="table.name" class="schema-row" @click="db.previewTable(table.name)">{{ table.name }} <small>{{ table.type }}</small></button>
      <p v-if="!db.schema.length" class="hint">{{ db.schemaBusy ? '读取对象中' : '没有可显示对象' }}</p>
    </section>

    <section class="query">
      <div class="sql-documents" role="tablist"><div v-for="doc in db.documents" :key="doc.id" :class="{active:doc.id === db.activeDocument}"><button role="tab" :aria-selected="doc.id === db.activeDocument" @click="db.activeDocument = doc.id">{{ doc.name }}{{ doc.busy ? ' …' : doc.sql.trim() ? ' *' : '' }}</button><button title="关闭查询" :disabled="doc.busy" @click="db.closeDocument(doc.id)"><X :size="12"/></button></div><button title="新建查询" @click="db.newDocument()"><Plus :size="16"/></button></div>
      <div v-if="db.closePending" class="confirm">此查询有未保存的 SQL。<button @click="db.closeDocument(db.closePending!, true)">丢弃并关闭</button><button @click="db.closePending = null">取消</button></div>
      <div class="query-head">
        <span class="sec-title">查询</span>
        <span class="tag" :class="db.risk">{{ riskLabel(db.risk) }}</span>
        <span class="tag warn" v-if="db.verdict !== 'NonProduction'">{{ productionLabel(db.verdict) }}</span>
      </div>

      <textarea v-model="db.sql" rows="7" class="sql" aria-label="SQL 编辑器" placeholder="SELECT 1" spellcheck="false" :disabled="db.document.busy || db.confirmOpen" @keydown.ctrl.enter.prevent="db.requestRun()"></textarea>
      <p v-if="multiStatementHint" class="hint">检测到多条语句，后端会拒绝执行</p>

      <div class="form-actions">
        <button class="primary" :disabled="db.busy || !db.runGate.ok" @click="db.requestRun()">运行</button>
        <button :disabled="!db.document.busy" @click="db.cancelQuery()"><Square :size="12"/>取消查询</button>
        <button :disabled="!db.result" @click="copyCsv">复制 CSV</button>
        <button :disabled="!db.result && !db.error" @click="db.clearResult()">清空结果</button>
      </div>

      <div v-if="db.confirmOpen" class="confirm">
        <div class="confirm-text">
          确认执行以下语句？<code>{{ truncateText(db.pendingSql ?? "", 120) }}</code>
        </div>
        <div class="form-actions">
          <button class="danger" @click="db.confirmRun()">确认执行</button>
          <button @click="db.cancelConfirm()">取消</button>
        </div>
      </div>

      <p v-if="db.error" class="errors">⚠ {{ db.error }}</p>

      <div v-if="db.result" class="result">
        <div class="result-head">
          <span>{{ db.result.rowLabel }}</span>
          <span>耗时 {{ db.result.elapsedLabel }}</span>
          <span v-if="db.result.state !== 'completed'" class="tag warn">{{ db.result.stateLabel }}</span>
        </div>
        <ul v-if="db.result.warnings.length" class="warn-list">
          <li v-for="(w, i) in db.result.warnings" :key="i">⚠ {{ w }}</li>
        </ul>
        <div class="result-paging"><input v-model="filter" aria-label="筛选结果" placeholder="筛选当前结果"/><button title="上一页" :disabled="page === 0" @click="page--"><ChevronLeft :size="14"/></button><span>{{ page+1 }} / {{ Math.max(1,Math.ceil(filteredRows.length/pageSize)) }} · {{ filteredRows.length }} 行</span><button title="下一页" :disabled="(page+1)*pageSize >= filteredRows.length" @click="page++"><ChevronRight :size="14"/></button></div>
        <div class="grid-wrap">
          <table v-if="db.result.columns.length">
            <thead>
              <tr>
                <th v-for="(c, i) in db.result.columns" :key="i"><button @click="sort(i)">{{ c }}{{ sortColumn === i ? sortAscending ? ' ↑' : ' ↓' : '' }}</button></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, ri) in displayRows" :key="ri">
                <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
              </tr>
            </tbody>
          </table>
          <EmptyState v-else text="无列信息" />
        </div>
        <p v-if="copied" class="hint">已复制</p>
      </div>
    </section>
    </div>
  </div>
</template>

<style scoped>
.db-panel { display: flex; flex-direction: column; height: 100%; overflow: hidden; background:#fff; color:#293944; }
.db-workspace{display:flex;flex:1;min-height:0}.db-workspace>.conn{flex:0 0 260px;overflow:auto;border-right:1px solid #d8dfe4;box-sizing:border-box}.db-workspace>.query{flex:1;min-width:0;overflow:auto}.sql-documents{display:flex;gap:4px;overflow:auto;margin-bottom:8px}.sql-documents>div{display:flex;flex:none;border-bottom:2px solid transparent}.sql-documents .active{border-color:#198163}.sql-documents button{border:0;background:#f0f4f5;padding:5px;color:inherit;cursor:pointer}.schema-head,.result-paging{display:flex;align-items:center;gap:6px;margin:12px 0 6px;font-size:12px}.schema-row{display:block;width:100%;text-align:left;padding:6px;border:0;background:#f2f6f4;cursor:pointer;overflow-wrap:anywhere}.schema-row small{color:#728681}.result-paging input{min-width:0;flex:1;padding:5px}.result-paging button{display:grid;place-items:center}.result-paging span{white-space:nowrap}.grid-wrap th button{border:0;background:none;font:inherit;color:inherit;cursor:pointer}.query .grid-wrap{max-height:calc(100vh - 430px);min-height:120px}@media(max-width:1000px){.db-workspace>.conn{flex-basis:210px}}
.gate { margin: 6px 8px; padding: 6px 8px; border-radius: 5px; background: #fff7e6; color: #b7791f; font-size: 12px; }
.conn, .query { padding: 8px; border-bottom: 1px solid #e5e6eb; }
.conn-head, .query-head { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
.sec-title { font-size: 12px; color: #4e5969; }
.badge { font-size: 11px; color: #86909c; border: 1px solid #e5e6eb; border-radius: 10px; padding: 1px 8px; }
.badge.on { color: #2b6cb0; border-color: #2b6cb0; }
.conn-list { list-style: none; margin: 0 0 6px; padding: 0; max-height: 120px; overflow: auto; }
.conn-list li { border-radius: 5px; }
.conn-list li.active { background: #eef2fb; }
.conn-list li.disabled { opacity: 0.55; }
.conn-list .row { width: 100%; display: flex; flex-direction: column; align-items: flex-start; text-align: left; background: transparent; border: none; cursor: pointer; padding: 4px 6px; border-radius: 5px; }
.conn-list .name { font-size: 13px; color: #1f2329; }
.conn-list .meta { font-size: 11px; color: #86909c; }
.form label { display: block; font-size: 12px; color: #4e5969; margin: 6px 0 2px; }
.form input, .form select { width: 100%; box-sizing: border-box; border: 1px solid #d5dbe7; border-radius: 5px; padding: 5px 7px; font-size: 12px; }
.checks { display: flex; flex-direction: column; gap: 4px; margin: 8px 0; }
.checks label { display: flex; align-items: center; gap: 6px; margin: 0; }
.checks input { width: auto; }
.errors { color: #c0392b; font-size: 12px; margin: 6px 0 0; }
.form-actions { margin-top: 8px; display: flex; gap: 8px; flex-wrap: wrap; }
.form-actions button { padding: 6px 14px; border-radius: 5px; border: 1px solid #d5dbe7; background: #fff; cursor: pointer; font-size: 12px; }
.form-actions button:disabled { opacity: 0.5; cursor: not-allowed; }
.form-actions .primary { background: #2b6cb0; border-color: #2b6cb0; color: #fff; }
.form-actions .danger { background: #c0392b; border-color: #c0392b; color: #fff; }
.tag { font-size: 11px; color: #4e5969; border: 1px solid #e5e6eb; border-radius: 10px; padding: 1px 8px; }
.tag.warn { color: #b7791f; border-color: #b7791f; }
.sql { width: 100%; box-sizing: border-box; border: 1px solid #d5dbe7; border-radius: 5px; padding: 6px 8px; font-size: 12px; font-family: monospace; }
.hint { font-size: 11px; color: #86909c; margin: 4px 0 0; }
.confirm { margin-top: 8px; padding: 8px; border: 1px solid #f0c36d; border-radius: 5px; background: #fffaf0; }
.confirm-text { font-size: 12px; color: #7a5c00; }
.confirm-text code { display: block; margin-top: 4px; font-size: 11px; word-break: break-all; color: #4e5969; }
.result { margin-top: 8px; }
.result-head { display: flex; gap: 12px; font-size: 12px; color: #4e5969; }
.warn-list { margin: 6px 0; padding-left: 16px; }
.warn-list li { font-size: 11px; color: #b7791f; }
.grid-wrap { max-height: 320px; overflow: auto; border: 1px solid #e5e6eb; border-radius: 5px; }
table { border-collapse: collapse; width: 100%; font-size: 12px; }
th, td { border-bottom: 1px solid #f0f1f3; padding: 4px 6px; text-align: left; white-space: nowrap; }
th { position: sticky; top: 0; background: #f7f8fa; color: #4e5969; font-weight: 600; }
.empty { color: #86909c; font-size: 12px; padding: 12px; text-align: center; }
</style>
