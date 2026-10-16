<script setup lang="ts">
import DatabaseWorkspace from "./DatabaseWorkspace.vue";
import { EmptyState } from "../../../shared/ui";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { takeIdeaProjectPath, subscribeIdeaProjectScan } from "../../../composables/ideaProjectNavigation";
import { Plus, X, Square, ChevronLeft, ChevronRight, RefreshCw } from '@lucide/vue';
import { useDatabaseStore } from "../state/useDatabaseStore";
import { parseDatabaseConnectionUrl } from "../../../utils/dbConnectionUrl";
import { previewIdeaDataSources, type IdeaMysqlCandidate } from "../model/jetbrainsImport";
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
const connectionUrl = ref("");
const connectionUrlError = ref("");
function applyConnectionUrl() {
  const raw = connectionUrl.value;
  connectionUrl.value = ""; // Never keep a pasted DSN or accidentally persist it in the form.
  connectionUrlError.value = "";
  try {
    const parsed = parseDatabaseConnectionUrl(raw);
    if (parsed.kind === "postgres") throw new Error("PostgreSQL 的常规查询界面尚未开放，请勿误认为可以连接");
    db.resetForm();
    db.form.kind = parsed.kind;
    db.form.host = parsed.host;
    db.form.portText = parsed.portText;
    db.form.database = parsed.database;
    db.form.sslMode = parsed.sslMode;
    db.form.allowWrite = false;
    db.form.productionHint = "";
    password.value = "";
    showConnection.value = true;
  } catch (error) {
    // Never display the raw URL in errors; it might include credentials.
    connectionUrlError.value = error instanceof Error ? error.message : "连接 URL 解析失败";
  }
}
const showConnection = ref(false);
const tablesOpen = ref(true);
const viewsOpen = ref(true);
const objectFilter = ref("");
const ideaImportOpen = ref(false);
const ideaProjectPath = ref("");
const ideaCandidates = ref<IdeaSource[]>([]);
const ideaScanning = ref(false);
const ideaImportError = ref("");
const ideaScannedFile = ref("");
// Alternative user-selected XML file import remains local to this panel, never sent to a server.
const manualIdeaInput = ref<HTMLInputElement | null>(null);
const manualIdeaCandidates = ref<IdeaMysqlCandidate[]>([]);
const manualIdeaError = ref("");
const manualIdeaSkipped = ref(0);
function openManualIdeaPicker() { manualIdeaInput.value?.click(); }
async function readManualIdeaFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  manualIdeaCandidates.value = [];
  manualIdeaError.value = "";
  manualIdeaSkipped.value = 0;
  if (!file) return;
  try {
    if (file.size > 2 * 1024 * 1024) throw new Error("IDEA 配置超过 2 MiB");
    const result = previewIdeaDataSources(await file.text());
    manualIdeaCandidates.value = result.candidates;
    manualIdeaSkipped.value = result.skipped;
    manualIdeaError.value = result.warnings.join("；");
  } catch (error) {
    manualIdeaError.value = error instanceof Error ? error.message : "无法解析 IDEA XML 配置";
  } finally { input.value = ""; }
}
// Both project scans and manual XML imports configure the same BrowserOS
// connection form. Keep the source selection, identity and fail-closed writes
// in one place; only the manual picker insists on stricter TLS.
function fillIdeaForm(candidate: {
  kind: "mysql" | "sqlite";
  name: string; database: string; host: string;
  portText: string; username: string;
}, tls: "prefer" | "require") {
  const port = candidate.portText || (candidate.kind === "mysql" ? "3306" : "");
  const existing = db.configs.find(cfg =>
    cfg.kind === candidate.kind && cfg.database === candidate.database &&
    (cfg.host || "") === candidate.host &&
    String(cfg.port ?? (candidate.kind === "mysql" ? 3306 : "")) === port &&
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
  db.form.sslMode = tls;
  db.form.allowWrite = false;
  db.form.productionHint = "";
  password.value = "";
  ideaImportOpen.value = false;
  showConnection.value = true;
}
function chooseManualIdeaSource(candidate: IdeaMysqlCandidate) {
  fillIdeaForm({...candidate, kind: "mysql", portText: candidate.port}, "require");
  manualIdeaCandidates.value = [];
  layout.showToast("只填入地址；请核对配置、输入密码并验证连接");
}

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
  fillIdeaForm(candidate, "prefer");
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
const fields = computed(() => visibleFields(db.form.kind));
const issues = computed(() => db.formIssues);
const portPlaceholder = computed(() => {
  const p = defaultPortFor(db.form.kind);
  return p ? String(p) : "默认端口";
});
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
        <div class="ctx-sep"></div>
        <p class="hint">也可以手动选择 IDEA / DataGrip 的 dataSources.xml 文件（仅本地解析，不读取密码或 SSH 配置）。</p>
        <input ref="manualIdeaInput" type="file" accept=".xml,text/xml" style="display:none" @change="readManualIdeaFile" />
        <button @click="openManualIdeaPicker">选择 XML 文件预览</button>
        <p v-if="manualIdeaError" role="alert" class="errors">{{ manualIdeaError }}</p>
        <p v-if="manualIdeaSkipped" class="hint">已跳过 {{ manualIdeaSkipped }} 项无法安全导入的连接</p>
        <div v-for="candidate in manualIdeaCandidates" :key="'manual-'+candidate.id" class="idea-source">
          <div><strong>{{ candidate.name }}</strong>
            <small>MySQL · {{ candidate.host }}:{{ candidate.port }}/{{ candidate.database }}</small>
            <p class="hint">{{ candidate.warning }}</p>
          </div>
          <button @click="chooseManualIdeaSource(candidate)">选用</button>
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
        <div class="url-import" role="group" aria-label="连接 URL">
          <label>通过 URL / JDBC 地址填写连接参数
            <input v-model="connectionUrl" type="text" autocomplete="off" spellcheck="false" placeholder="jdbc:mysql://127.0.0.1:3306/app" @keydown.enter.prevent="applyConnectionUrl" />
          </label>
          <button type="button" :disabled="!connectionUrl.trim() || db.busy" @click="applyConnectionUrl">解析 URL</button>
          <p class="hint">支持 MySQL 与 SQLite 的标准 URI、IDEA JDBC 地址。URL 不保存；URL 中禁止包含账号密码，密码请单独输入。</p>
          <p v-if="connectionUrlError" class="errors" role="alert">{{ connectionUrlError }}</p>
        </div>
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

    <DatabaseWorkspace />
    </div>
  </div>
</template>

<style scoped>
.db-panel { display: flex; flex-direction: column; height: 100%; overflow: hidden; background:#fff; color:#293944; }
.db-workspace{display:flex;flex:1;min-height:0}.db-workspace>.conn{flex:0 0 260px;overflow:auto;border-right:1px solid #d8dfe4;box-sizing:border-box}.schema-head{display:flex;align-items:center;gap:6px;margin:12px 0 6px;font-size:12px}.schema-row{display:block;width:100%;text-align:left;padding:6px;border:0;background:#f2f6f4;cursor:pointer;overflow-wrap:anywhere}.schema-row small{color:#728681}@media(max-width:1000px){.db-workspace>.conn{flex-basis:210px}}
.gate { margin: 6px 8px; padding: 6px 8px; border-radius: 5px; background: #fff7e6; color: #b7791f; font-size: 12px; }
.conn { padding: 8px; border-bottom: 1px solid #e5e6eb; }
.conn-head { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
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
.hint { font-size: 11px; color: #86909c; margin: 4px 0 0; }
table { border-collapse: collapse; width: 100%; font-size: 12px; }
th, td { border-bottom: 1px solid #f0f1f3; padding: 4px 6px; text-align: left; white-space: nowrap; }
th { position: sticky; top: 0; background: #f7f8fa; color: #4e5969; font-weight: 600; }
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
@media(max-width:940px){.db-workspace>.conn{flex:0 0 195px}}
</style>

<style scoped>
.object-header strong{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;min-width:0;flex:1}
.object-tabs{display:flex;gap:4px;padding-bottom:8px}
.object-tabs button{flex:1;border:1px solid #d4dfef;background:#fff;border-radius:5px;color:#536d8b;cursor:pointer;font-size:11px;padding:5px 2px}
.object-tabs button[aria-pressed=true]{background:#e5f0ff;border-color:#9cbce8;color:#2358a8;font-weight:700}
.object-ddl{margin:2px 0 8px;white-space:pre-wrap;overflow-wrap:anywhere;overflow:auto;max-height:240px;background:#f5f8fc;border:1px solid #e2eaf5;border-radius:6px;padding:10px;font:11px/1.6 ui-monospace,monospace}
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
