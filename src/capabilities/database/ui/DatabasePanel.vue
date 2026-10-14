<script setup lang="ts">
import { EmptyState } from "../../../shared/ui";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { takeIdeaProjectPath, subscribeIdeaProjectScan } from "../../../composables/ideaProjectNavigation";
import { Plus, X, Square, ChevronLeft, ChevronRight, RefreshCw } from '@lucide/vue';
import { useDatabaseStore } from "../state/useDatabaseStore";
import { readAuthorizedWorkspaceTextFile } from "../../workspace/public";
import { ideaProjectDataSourcesPath, parseIdeaDataSourcesXml, type IdeaSource } from "../../../utils/ideaDataSources";
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
const showConnection = ref(false);
const showHistory = ref(false);
const tablesOpen = ref(true);
const viewsOpen = ref(true);
const objectFilter = ref("");
const ideaImportOpen = ref(false);
const ideaProjectPath = ref("");
const ideaCandidates = ref<IdeaSource[]>([]);
const ideaScanning = ref(false);
const ideaImportError = ref("");
const ideaScannedFile = ref("");
async function scanIdeaProject() {
  if (ideaScanning.value) return;
  ideaImportError.value = "";
  ideaCandidates.value = [];
  ideaScanning.value = true;
  try {
    // The user selected a concrete project/path; only check up to six nearest
    // ancestor project roots for .idea/dataSources.xml, never scan $HOME.
    const requested=ideaProjectDataSourcesPath(ideaProjectPath.value);
    let dir=requested.slice(0,-"/.idea/dataSources.xml".length);
    let lastError:unknown=new Error("IDEA 配置文件未找到");
    let content:string|null=null,found="";
    for(let depth=0;depth<6;depth++){
      const path=ideaProjectDataSourcesPath(dir);
      try{
        content=await readAuthorizedWorkspaceTextFile(path); // Workspace public facade enforces native allowed-roots.
        found=path;break;
      }catch(err){lastError=err;}
      const last=dir.lastIndexOf("/");
      if(last<=0)break;
      dir=dir.slice(0,last);
    }
    if(content===null)throw lastError;
    ideaCandidates.value = parseIdeaDataSourcesXml(content);
    ideaScannedFile.value = found;
    if (!ideaCandidates.value.length) ideaImportError.value = "没有发现 IDEA 项目数据库连接";
  } catch (e) {
    ideaImportError.value = e instanceof Error ? e.message : "无法读取项目数据库配置；请检查目录权限";
  } finally { ideaScanning.value = false; }
}
function chooseIdeaSource(candidate: IdeaSource) {
  if (!candidate.supported || candidate.kind === "unsupported") return;
  // Reuse the same backend connection identity when an equivalent connection exists.
  const existing = db.configs.find(cfg =>
    cfg.kind === candidate.kind && cfg.database === candidate.database &&
    (cfg.host || "") === candidate.host && String(cfg.port ?? (candidate.kind === "mysql" ? 3306 : "")) === (candidate.portText || (candidate.kind === "mysql" ? "3306" : "")) &&
    (cfg.username || "") === candidate.username
  );
  db.resetForm();
  db.form.id = existing?.id ?? null;
  db.form.name = existing?.name ?? candidate.name;
  db.form.kind = candidate.kind;
  db.form.database = candidate.database;
  db.form.host = candidate.host;
  db.form.portText = candidate.portText;
  db.form.username = candidate.username;
  db.form.sslMode = "prefer"; // Never downgrade TLS based on imported IDEA URL.
  db.form.allowWrite = false; // Imported configs MUST start read-only.
  db.form.productionHint = ""; // Unknown production status must remain fail-closed.
  password.value = "";
  ideaImportOpen.value = false;
  showConnection.value = true;
  layout.showToast("已填充 IDEA 数据源；请补充密码并点击连接。原配置未修改。");
}
const visibleSchema = computed(() => db.schema.filter(object =>
  object.name.toLowerCase().includes(objectFilter.value.trim().toLowerCase())
));
const tables = computed(() => visibleSchema.value.filter(object => !object.type.toLowerCase().includes("view")));
const views = computed(() => visibleSchema.value.filter(object => object.type.toLowerCase().includes("view")));
const activeConnection = computed(() => db.connections.find(c => c.id === db.activeId));
const connectionStatus = computed(() =>
  !activeConnection.value ? "未选择连接" : db.recentlyVerified ? "最近验证成功" : "已选择 · 查询时建连"
);
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

let stopIdeaProjectScan=()=>{};
function handleIdeaProjectScan(path:string) {
  ideaProjectPath.value=path;
  ideaImportOpen.value=true;
  void scanIdeaProject();
}
onMounted(async () => {
  stopIdeaProjectScan=subscribeIdeaProjectScan(handleIdeaProjectScan);
  const project=takeIdeaProjectPath();
  if(project)handleIdeaProjectScan(project);
  await db.refreshConnections();
  if (!db.connections.length) showConnection.value = true;
});
onUnmounted(()=>stopIdeaProjectScan());

async function doConnect() {
  const okConnected = await db.connect(password.value);
  // 无论成败都立即清空本地密码，避免密码在内存中多停留一轮渲染
  password.value = "";
  if (okConnected) {
    showConnection.value = false;
    layout.showToast("连接已验证；后续查询会按需创建数据库连接");
  }
}

function deleteConnection() {
  if (!db.connected || db.busy) return;
  if (!window.confirm("永久删除 BrowserOS 保存的数据库连接及系统密钥库凭据？IDEA 原始配置不会修改。")) return;
  void db.disconnect();
}

const sqlEditor = ref<HTMLTextAreaElement | null>(null);
function insertSqlKeyword(fragment: string) {
  const element = sqlEditor.value;
  if (!element || db.document.busy || db.confirmOpen) return;
  const start = element.selectionStart;
  const end = element.selectionEnd;
  const text = db.sql;
  const preceding = start && !/\s/.test(text[start-1]) ? " " : "";
  const following = end >= text.length || /\s/.test(text[end]) ? " " : " ";
  const insert = preceding + fragment + following;
  db.sql = text.slice(0,start) + insert + text.slice(end);
  requestAnimationFrame(() => {
    element.focus();
    element.setSelectionRange(start+insert.length,start+insert.length);
  });
}
function downloadCsv() {
  if (!db.result || db.document.busy) return;
  try {
    const csv = toCsv(db.document.raw?.columns || db.result.columns, db.document.raw?.rows || db.result.rows);
    if (new TextEncoder().encode(csv).length > 5 * 1024 * 1024) {
      layout.showToast("CSV 超过安全导出上限"); return;
    }
    const blob = new Blob(["\uFEFF",csv],{type:"text/csv;charset=utf-8"});
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "browseros-query-" + new Date().toISOString().slice(0,10) + ".csv";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  } catch { layout.showToast("导出失败，请使用复制 CSV"); }
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
      <strong class="studio-brand">Database Studio</strong>
      <span>{{ activeConnection?.name || '数据库资源管理器' }}</span>
      <button class="conn-toggle" @click="showConnection = !showConnection">{{ showConnection ? '隐藏连接设置' : '连接设置' }}</button>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>

    <div v-if="!db.backendReady" class="gate">后端数据库命令未就绪，连接与查询暂不可用</div>

    <div class="db-workspace">
    <section v-if="!workbench.collapsed" class="conn">
      <div class="conn-head">
        <strong>数据源</strong>
        <span class="badge" :class="{on:db.recentlyVerified}">{{ connectionStatus }}</span>
      </div>
      <button class="idea-import-toggle" :aria-expanded="ideaImportOpen" @click="ideaImportOpen=!ideaImportOpen">
        {{ ideaImportOpen ? "收起 IDEA 导入" : "从 IntelliJ IDEA 导入连接…" }}
      </button>
      <section v-if="ideaImportOpen" class="idea-import" aria-label="IDEA 数据源导入">
        <p>仅读取你指定项目的 <code>.idea/dataSources.xml</code>。不读取 IDEA 密码、SSH 凭据或 <code>dataSources.local.xml</code>。</p>
        <label>IDEA 项目目录
          <input v-model="ideaProjectPath" type="text" spellcheck="false" placeholder="/home/you/IdeaProjects/project" />
        </label>
        <button :disabled="ideaScanning || !ideaProjectPath.trim()" @click="scanIdeaProject">
          {{ ideaScanning ? "扫描中…" : "扫描该项目的数据库配置" }}
        </button>
        <p v-if="ideaImportError" role="alert" class="errors">{{ ideaImportError }}</p>
        <p v-if="ideaCandidates.length" class="hint">发现 {{ ideaCandidates.length }} 条连接 · {{ ideaScannedFile }}</p>
        <div v-for="candidate in ideaCandidates" :key="candidate.id" class="idea-source">
          <div><strong>{{candidate.name}}</strong>
            <small>{{candidate.kind==="unsupported"?"不支持的驱动":candidate.kind.toUpperCase()}} · {{candidate.host ? candidate.host+":"+candidate.portText+"/" : ""}}{{candidate.database}}</small>
            <p class="hint">{{candidate.note}}</p>
          </div>
          <button :disabled="!candidate.supported" @click="chooseIdeaSource(candidate)">选用</button>
        </div>
      </section>

      <ul v-if="db.connections.length" class="conn-list">
        <li v-for="c in db.connections" :key="c.id" :class="{ active: c.id === db.form.id, disabled: !c.enabled }">
          <button class="row" :disabled="!c.enabled" @click="db.selectConnection(c); showConnection = false">
            <span class="name">{{ c.name }}</span>
            <span class="meta">{{ c.label }}{{ c.allowWrite ? " · 允许写" : "" }}{{ c.enabled ? "" : " · 已禁用" }}</span>
          </button>
        </li>
      </ul>

      <div v-if="showConnection" class="form">
        <p class="form-note">连接设置仅在需要时展开。密码不保存到页面或浏览器存储。</p>
        <label>名称<input v-model="db.form.name" placeholder="如 local-sqlite" /></label>
        <label>驱动
          <select :value="db.form.kind" @change="(e) => db.setKind(((e.target as HTMLSelectElement).value) as DbKind)">
            <option value="sqlite">SQLite</option>
            <option value="mysql">MySQL</option>
            <option value="postgres" disabled>PostgreSQL（查询尚未接通）</option>
          </select>
        </label>
        <label v-if="fields.host">主机<input v-model="db.form.host" placeholder="127.0.0.1" /></label>
        <label v-if="fields.port">端口<input v-model="db.form.portText" :placeholder="portPlaceholder" /></label>
        <label>{{ databaseFieldLabel(db.form.kind) }}<input v-model="db.form.database" placeholder="/data/app.db 或 app" /></label>
        <label v-if="fields.username">用户名<input v-model="db.form.username" placeholder="reader" /></label>
        <label v-if="fields.username">密码（仅交给系统密钥库，配置中不保存）<input v-model="password" type="password" autocomplete="off" /></label>
        <label v-if="fields.ssl">SSL
          <select v-model="db.form.sslMode">
            <option value="disable">disable</option>
            <option value="prefer">prefer</option>
            <option value="require">require</option>
          </select>
        </label>
        <p v-if="db.form.kind === 'mysql'" class="hint">MySQL：prefer/require 强制校验 TLS；无 TLS 请选择 disable。建议使用只读账号。</p>
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
          <button :disabled="db.busy || !db.connected" @click="db.deselectConnection()">断开当前会话</button>
          <button :disabled="db.busy || !db.connected" @click="deleteConnection">删除连接及凭据</button>
          <button :disabled="db.busy" @click="db.resetForm(); password = ''">新建连接</button>
        </div>
      </div>
      <div class="schema-head"><strong>对象资源树</strong>
        <button title="重新读取数据库对象" aria-label="刷新对象" :disabled="db.schemaBusy || !db.activeId" @click="db.refreshSchema()"><RefreshCw :size="14"/></button>
      </div>
      <p class="catalog-root">▾ {{ db.form.database || '请选择数据库' }}</p>
      <input v-model="objectFilter" class="schema-search" type="search" aria-label="筛选数据库表和视图" placeholder="搜索表和视图…" />
      <div class="schema-head tree-group">
        <button :aria-expanded="tablesOpen" @click="tablesOpen=!tablesOpen">{{ tablesOpen ? '▾' : '▸' }} 表 <small>({{tables.length}})</small></button>
      </div>
      <template v-if="tablesOpen">
        <button v-for="table in tables" :key="table.name" class="schema-row"
          :class="{selected:db.selectedObject===table.name}" :aria-selected="db.selectedObject===table.name"
          :title="'单击查看字段，双击预览数据：'+table.name"
          @click="db.selectObject(table.name)" @dblclick="db.previewTable(table.name)"
          @keydown.enter.prevent="db.previewTable(table.name)">▦ {{ table.name }}</button>
      </template>
      <div class="schema-head tree-group">
        <button :aria-expanded="viewsOpen" @click="viewsOpen=!viewsOpen">{{ viewsOpen ? '▾' : '▸' }} 视图 <small>({{views.length}})</small></button>
      </div>
      <template v-if="viewsOpen">
        <button v-for="table in views" :key="table.name" class="schema-row"
          :class="{selected:db.selectedObject===table.name}" :aria-selected="db.selectedObject===table.name"
          :title="'单击查看字段，双击预览视图：'+table.name"
          @click="db.selectObject(table.name)" @dblclick="db.previewTable(table.name)"
          @keydown.enter.prevent="db.previewTable(table.name)">◈ {{ table.name }}</button>
      </template>
      <p v-if="db.schema.length && !visibleSchema.length" class="hint">没有匹配的对象</p>
      <p v-if="!db.schema.length" class="hint">{{ db.schemaBusy ? '读取对象中…' : '请先验证数据库连接以查看对象' }}</p>
      <p class="tree-help">单击表查看字段，双击直接读取前 100 行。查询遵循原生安全门禁。</p>
      <div v-if="db.selectedObject" class="object-details">
        <div class="schema-head object-header"><strong :title="db.selectedObject">{{ db.selectedObject }}</strong>
          <button :disabled="!db.activeId" @click="db.previewTable(db.selectedObject)">查看数据</button>
        </div>
        <div class="object-tabs" aria-label="数据库对象详情">
          <button :aria-pressed="db.objectDetailMode==='columns'" @click="db.selectObject(db.selectedObject,'columns')">字段</button>
          <button :aria-pressed="db.objectDetailMode==='indexes'" @click="db.selectObject(db.selectedObject,'indexes')">索引</button>
          <button :aria-pressed="db.objectDetailMode==='foreign_keys'" @click="db.selectObject(db.selectedObject,'foreign_keys')">外键 / ER</button>
          <button :aria-pressed="db.objectDetailMode==='ddl'" @click="db.selectObject(db.selectedObject,'ddl')">DDL</button>
        </div>
        <div v-if="db.objectDetailMode==='foreign_keys' && db.objectDetailRows.length" class="local-er">
          <strong>当前表 → 引用表（局部 ER，只展示真实外键关系）</strong>
          <svg :viewBox="'0 0 520 '+Math.min(310,75+Math.min(db.objectDetailRows.length,6)*38)"
             role="img" :aria-label="db.selectedObject+' 的真实外键关系图'">
            <rect x="4" y="7" width="182" height="44" rx="6" fill="#dceaff" stroke="#6b95cd"/>
            <text x="14" y="34" fill="#24568a" font-size="12">{{db.selectedObject.slice(0,20)}}</text>
            <g v-for="(rel,i) in db.objectDetailRows.slice(0,6)" :key="i">
              <line x1="186" y1="29" x2="263" :y2="35+i*38" stroke="#638ab8" stroke-width="1.5"/>
              <path :d="'M256 '+(29+i*38)+' L263 '+(35+i*38)+' L256 '+(41+i*38)" stroke="#638ab8" fill="none"/>
              <rect x="267" :y="17+i*38" width="245" height="32" rx="5" fill="#eaf7ef" stroke="#8ebca1"/>
              <text x="275" :y="38+i*38" fill="#2b6642" font-size="10">{{String(rel[1]||'').slice(0,24)}}.{{String(rel[2]||'').slice(0,14)}}</text>
            </g>
          </svg>
          <p class="hint">最多绘制 6 条关系；完整关系清单显示在下方。</p>
        </div>
        <p v-if="db.columnsBusy" class="hint">正在读取对象元数据…</p>
        <p v-else-if="!db.objectDetailRows.length" class="hint">暂无记录；此项可能没有索引或当前账号没有元数据权限。</p>
        <pre v-else-if="db.objectDetailMode==='ddl'" class="object-ddl">{{db.objectDetailRows.map(row => row.at(-1)).join('\n')}}</pre>
        <div v-else class="column-list">
          <div v-for="(col,i) in db.objectDetailRows" :key="i" class="column-item" :title="col.join(' · ')">
            <strong>{{col[0]}}</strong><small>{{col.slice(1).join(' · ')}}</small>
          </div>
        </div>
      </div>
    </section>

    <section class="query">
      <div class="sql-documents" role="tablist"><div v-for="doc in db.documents" :key="doc.id" :class="{active:doc.id === db.activeDocument}"><button role="tab" :aria-selected="doc.id === db.activeDocument" @click="db.activeDocument = doc.id">{{ doc.name }}{{ doc.busy ? ' · 执行中' : '' }}</button><button title="关闭查询" :disabled="doc.busy" @click="db.closeDocument(doc.id)"><X :size="12"/></button></div><button title="新建查询" @click="db.newDocument()"><Plus :size="16"/></button></div>
      <div v-if="db.closePending" class="confirm">此查询包含 SQL 文本，关闭后会丢失该草稿。<button @click="db.closeDocument(db.closePending!, true)">丢弃并关闭</button><button @click="db.closePending = null">取消</button></div>
      <div class="query-head">
        <strong class="sec-title">SQL 控制台</strong>
        <span class="tag">{{ activeConnection?.allowWrite ? '写权限已启用' : '只读保护' }}</span>
        <span class="tag" :class="db.risk">{{ riskLabel(db.risk) }}</span>
        <span class="tag warn" v-if="db.verdict !== 'NonProduction'">{{ productionLabel(db.verdict) }}</span>
      </div>

      <div class="sql-snippets" aria-label="SQL 快捷输入">
        <span>快速插入：</span>
        <button v-for="fragment in ['SELECT','FROM','WHERE','ORDER BY','LIMIT']" :key="fragment"
          :disabled="db.document.busy || db.confirmOpen" @click="insertSqlKeyword(fragment)">{{fragment}}</button>
      </div>
      <textarea ref="sqlEditor" v-model="db.sql" rows="9" class="sql" aria-label="SQL 编辑器"
        placeholder="-- 选择表格或输入 SELECT 语句。Ctrl/Cmd + Enter 执行。"
        spellcheck="false" :disabled="db.document.busy || db.confirmOpen"
        @keydown.ctrl.enter.prevent="db.requestRun()" @keydown.meta.enter.prevent="db.requestRun()"></textarea>
      <p v-if="multiStatementHint" class="hint">检测到多条语句，后端会拒绝执行</p>

      <div class="form-actions">
        <button class="primary" :disabled="db.busy || !db.runGate.ok" @click="db.requestRun()">▶ 运行 SQL</button>
        <button :disabled="!db.document.busy" @click="db.cancelQuery()"><Square :size="12"/>取消查询</button>
        <button :disabled="!db.result" @click="copyCsv">复制 CSV</button>
        <button :disabled="db.busy || !db.sql.trim() || db.form.kind!=='mysql'" title="仅构造非 ANALYZE 的 MySQL EXPLAIN SELECT" @click="db.explainCurrentSql()">EXPLAIN</button>
        <button :disabled="!db.result" @click="downloadCsv">导出 CSV</button>
        <button :disabled="!db.result && !db.error" @click="db.clearResult()">清空结果</button>
        <button :aria-pressed="showHistory" @click="showHistory = !showHistory">{{showHistory?'收起':'查询'}}历史</button>
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

      <section v-if="showHistory" class="query-history">
        <label><input type="checkbox" :checked="db.rememberQueries" @change="db.setRememberQueries(($event.target as HTMLInputElement).checked)" /> 本次会话记录成功的 SQL（最多 30 条，不写入磁盘）</label>
        <button :disabled="!db.queryHistory.length" @click="db.clearQueryHistory()">清空</button>
        <p v-if="!db.queryHistory.length" class="hint">尚无本次会话记录；开启后执行 SQL 才会加入历史。</p>
        <button v-for="item in db.queryHistory" :key="item.id" class="history-item"
          :title="item.sql" @click="db.reuseHistory(item)">{{ item.at }} · {{item.connection}} · {{item.elapsed}} — {{truncateText(item.sql,90)}}</button>
      </section>
      <p v-if="db.error" class="errors" role="alert">⚠ {{ db.error }}</p>

      <div v-if="db.result" class="result">
        <div v-if="db.document.tablePreview" class="server-page">
          <span>表 {{db.document.tablePreview.name}} · 数据库分页：
            第 {{Math.floor(db.document.tablePreview.offset/100)+1}} 页（每页最多 100 行）
          </span>
          <button :disabled="db.busy || db.document.tablePreview.offset===0" @click="db.previousTablePage()">上一批 100 行</button>
          <button :disabled="db.busy || db.result.rowCount<100 || db.document.tablePreview.offset>=50000" @click="db.nextTablePage()">下一批 100 行</button>
          <small>无总行数承诺；最终由数据库返回记录为准</small>
        </div>
        <div class="result-head">
          <span>{{ db.result.rowLabel }}</span>
          <span>耗时 {{ db.result.elapsedLabel }}</span>
          <span v-if="db.result.state !== 'completed'" class="tag warn">{{ db.result.stateLabel }}</span>
        </div>
        <ul v-if="db.result.warnings.length" class="warn-list">
          <li v-for="(w, i) in db.result.warnings" :key="i">⚠ {{ w }}</li>
        </ul>
        <p class="hint">当前仅筛选和排序已读取的 {{db.result.rows.length}} 行；并非整个数据库的服务器端分页。</p>
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

<style scoped>
.schema-search{width:100%;box-sizing:border-box;border:1px solid #cbd5e1;border-radius:6px;padding:6px;margin:3px 0 8px}
</style>

<style scoped>
/* Database Studio: hierarchy-first layout, without overlaying native WebViews. */
.studio-brand{font-size:13px;letter-spacing:.015em;color:#2461ae}
.conn-toggle{border:1px solid #d9e2ef;border-radius:7px;background:#fff;padding:5px 9px;font-size:12px}
.conn-head{justify-content:space-between}
.db-workspace>.conn{flex-basis:250px;background:#f9fbff}
.db-workspace>.query{display:flex;flex-direction:column;gap:4px;min-width:0}
.conn-list{max-height:185px}
.form{background:#fff;padding:11px;border:1px solid #dde6f0;border-radius:9px;margin:9px 0}
.form-note,.tree-help{color:#75859a;line-height:1.5;font-size:11px}
.tree-help{padding:5px 2px}
.catalog-root{font-weight:600;font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.tree-group{margin:7px 0 3px}
.tree-group button{border:0;background:transparent;font-weight:600;color:#33465e;text-align:left;cursor:pointer}
.schema-row{padding:7px 12px;margin:2px 0;border-radius:6px;background:transparent;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.schema-row:hover{background:#e8f1ff}.schema-row.selected{background:#dceaff;color:#214f99;font-weight:600}
.object-details{margin-top:12px;border-top:1px solid #dce4ee;padding-top:9px}
.column-list{max-height:210px;overflow:auto}
.column-item{display:flex;gap:7px;justify-content:space-between;padding:5px;font-size:11px;border-bottom:1px solid #e9edf4}
.column-item strong{overflow:hidden;text-overflow:ellipsis}.column-item small{color:#63748a;overflow:hidden;text-overflow:ellipsis}
.sql{resize:vertical;min-height:125px;font:12px/1.6 ui-monospace,Consolas,monospace;background:#fbfcff}
.query-history{padding:9px;border-radius:8px;border:1px solid #dce6f1;background:#f9fbfd}
.query-history label{font-size:12px}.query-history button{margin:4px;cursor:pointer}
.history-item{display:block;width:100%;text-align:left;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.grid-wrap td{max-width:350px;overflow:hidden;text-overflow:ellipsis}
@media(max-width:940px){.db-workspace>.conn{flex:0 0 195px}.db-workspace>.query{min-width:0}}
</style>

<style scoped>
.object-header strong{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;min-width:0;flex:1}
.object-tabs{display:flex;gap:4px;padding-bottom:8px}
.object-tabs button{flex:1;border:1px solid #d4dfef;background:#fff;border-radius:5px;color:#536d8b;cursor:pointer;font-size:11px;padding:5px 2px}
.object-tabs button[aria-pressed=true]{background:#e5f0ff;border-color:#9cbce8;color:#2358a8;font-weight:700}
.object-ddl{margin:2px 0 8px;white-space:pre-wrap;overflow-wrap:anywhere;overflow:auto;max-height:240px;background:#f5f8fc;border:1px solid #e2eaf5;border-radius:6px;padding:10px;font:11px/1.6 ui-monospace,monospace}
.sql-snippets{display:flex;align-items:center;gap:5px;flex-wrap:wrap;color:#789;font-size:11px;margin:5px 0}
.sql-snippets button{border:1px solid #dfe6f2;background:#f7faff;border-radius:4px;font:10px ui-monospace,monospace;padding:3px 6px;cursor:pointer;color:#31588b}
.server-page{display:flex;align-items:center;flex-wrap:wrap;gap:7px;margin:6px 0;padding:7px;background:#eef5ff;color:#2f5887;border-radius:7px;font-size:11px}
.server-page button{font-size:11px;border:1px solid #bed3ef;background:#fff;border-radius:5px;padding:4px 7px;cursor:pointer}
.server-page button:disabled{opacity:.5;cursor:default}.server-page small{color:#617c9a}
</style>

<style scoped>
.local-er{padding:9px;border:1px solid #d9e6f1;border-radius:8px;background:#f9fbff;font-size:11px}
.local-er strong{font-size:12px;color:#2a5484}
.local-er svg{width:100%;height:auto;max-height:320px;display:block;margin-top:8px;overflow:hidden}
</style>

<style scoped>
.idea-import-toggle{width:100%;text-align:left;margin:7px 0;padding:7px;border-radius:6px;border:1px solid #bdd2ed;background:#eff6ff;color:#245889;cursor:pointer;font-size:12px}
.idea-import{padding:10px;margin-bottom:8px;border:1px solid #d3e0f2;border-radius:8px;background:#fff;max-height:340px;overflow:auto}
.idea-import p{font-size:11px;line-height:1.5;color:#596d83;overflow-wrap:anywhere}
.idea-import label{display:block;font-size:11px}
.idea-import input{width:100%;font:11px ui-monospace,monospace;border:1px solid #cad8e6;border-radius:6px;padding:6px;margin:5px 0;box-sizing:border-box}
.idea-import button{border:1px solid #aac6e8;border-radius:6px;background:#f0f6ff;color:#2a5990;padding:6px;cursor:pointer}
.idea-import button:disabled{opacity:.5;cursor:default}
.idea-source{display:flex;align-items:center;justify-content:space-between;gap:8px;border-top:1px solid #e4e9f1;padding:7px 0}
.idea-source div{min-width:0}.idea-source strong{font-size:12px;display:block}.idea-source small{color:#62768f;display:block;word-break:break-all}
</style>
