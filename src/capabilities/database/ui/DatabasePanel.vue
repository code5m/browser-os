<script setup lang="ts">
import DatabaseWorkspace from "./DatabaseWorkspace.vue";
import { bridge } from "../../../bridge";
import { discoverIdeaProjects } from "../model/ideaDiscovery.mjs";
import type { DbConnectionConfig } from "../../../types";
import { EmptyState } from "../../../shared/ui";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { takeIdeaProjectPath, subscribeIdeaProjectScan } from "../../../composables/ideaProjectNavigation";
import { Plus, X, Square, ChevronLeft, ChevronRight, RefreshCw } from '@lucide/vue';
import { useDatabaseStore } from "../state/useDatabaseStore";
import { parseDatabaseConnectionUrl, parseDatabaseHostAddress } from "../../../utils/dbConnectionUrl";
import { previewIdeaDataSources, type IdeaMysqlCandidate } from "../model/jetbrainsImport";
import { readAuthorizedWorkspaceTextFile, listAuthorizedWorkspaceDirectory, useFileStore } from "../../workspace/public";
import { ideaProjectDataSourcesPath, parseIdeaDataSourcesXml, type IdeaSource } from "../../../utils/ideaDataSources";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useWorkbenchStore } from "../../../stores/useWorkbenchStore";
import { open as openNativeDialog } from "@tauri-apps/plugin-dialog";
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
const files = useFileStore();

// 密码只作**组件本地瞬时态**：不进 store、不进表单模型、不持久化（F2）。
const password = ref("");
const connectionUrl = ref("");
const connectionUrlError = ref("");
const connectionUrlNotice = ref("");
function applyConnectionUrl() {
  const raw = connectionUrl.value;
  connectionUrl.value = ""; // Never keep a pasted DSN or accidentally persist it in the form.
  connectionUrlError.value = "";
  connectionUrlNotice.value = "";
  const address = parseDatabaseHostAddress(raw);
  if (address) {
    db.form.host = address.host;
    if (address.portText) db.form.portText = address.portText;
    connectionUrlNotice.value = "已填入主机地址；请核对数据库名和账号。MySQL 加密方式会自动识别。";
    showConnection.value = true;
    return;
  }
  try {
    const parsed = parseDatabaseConnectionUrl(raw);
    db.form.kind = parsed.kind;
    db.form.host = parsed.host;
    db.form.portText = parsed.portText;
    db.form.database = parsed.database;
    db.form.sslMode = parsed.kind === "mysql" ? "prefer" : parsed.sslMode;
    if (parsed.ignoredOptions?.length) connectionUrlNotice.value = `已识别地址，但未执行 JDBC 专有参数：${parsed.ignoredOptions.join("、")}。多语句及批处理开关不会绕过应用安全门禁。`;
    // Parsing an address must not erase the name, account, or the user's
    // explicit read/write and production choices elsewhere in the form.
    password.value = "";
    showConnection.value = true;
  } catch (error) {
    // Never display the raw URL in errors; it might include credentials.
    connectionUrlError.value = error instanceof Error ? error.message : "连接 URL 解析失败";
  }
}
const showConnection = ref(false);
const connectionDialogOpen = ref(false);
const connectionDialogTab = ref<"manual" | "idea">("manual");
const tablesOpen = ref(true);
const viewsOpen = ref(true);
const objectFilter = ref("");
const connectionFilter = ref("");
const ideaImportOpen = ref(false);
const ideaProjectPath = ref("");
// Only prefill a REAL user-opened location. Do not invent ~/IdeaProjects.
const ideaSuggestedBase = computed(() => files.filePath || files.startDirs[0]?.path || "");
watch(ideaSuggestedBase, path => {
  if (path && !ideaProjectPath.value.trim()) ideaProjectPath.value = path;
}, {immediate:true});
const ideaDiscoveryBusy = ref(false);
const ideaDiscoverySummary = ref("");
const ideaDiscovered = ref<{path:string;source:IdeaSource;selected:boolean}[]>([]);
async function chooseIdeaProjectDirectory() {
  try {
    const selected = await openNativeDialog({ directory: true, multiple: false, title: "选择 IntelliJ IDEA 项目目录" });
    if (typeof selected === "string" && selected) {
      ideaProjectPath.value = selected;
      await discoverIdeaSources();
    }
  } catch (_) {
    ideaDiscoverySummary.value = "无法打开目录选择器；也可以从文件工作区打开项目目录后重试";
  }
}
let ideaDiscoveryGeneration = 0;
async function discoverIdeaSources() {
  if (ideaDiscoveryBusy.value) return;
  ideaDiscoveryBusy.value = true;
  const generation = ++ideaDiscoveryGeneration;
  ideaDiscoverySummary.value = "正在检查已经授权的工作区目录（最多 1200 个目录，最多 8 层）…";
  ideaDiscovered.value = [];
  try {
    const roots = [ideaProjectPath.value,files.filePath,...files.startDirs.map(entry => entry.path)].filter(Boolean);
    const result = await discoverIdeaProjects(roots, {
      listDir: listAuthorizedWorkspaceDirectory,
      readFile: readAuthorizedWorkspaceTextFile,
      parse: parseIdeaDataSourcesXml,
    }, () => generation !== ideaDiscoveryGeneration);
    if (generation !== ideaDiscoveryGeneration) return;
    ideaDiscovered.value = result.sources.map((item:{path:string;source:IdeaSource}) =>
      ({...item,selected:item.source.supported}));
    ideaDiscoverySummary.value = `已检查 ${result.scanned} 个目录，发现 ${result.sources.length} 条数据源` +
      (result.partial ? "；已达到范围上限，请选择更具体的目录重试" : "") +
      (result.permissionErrors ? `；${result.permissionErrors} 个目录无法读取` : "");
  } catch (_) {
    if (generation === ideaDiscoveryGeneration) ideaDiscoverySummary.value = "发现失败：请从文件工作区选择已授权的 IDEA 项目目录";
  } finally {
    if (generation === ideaDiscoveryGeneration) ideaDiscoveryBusy.value = false;
  }
}
function cancelIdeaDiscovery() { ideaDiscoveryGeneration++; ideaDiscoveryBusy.value = false; }
function selectAllIdeaSources(selected: boolean) {
  ideaDiscovered.value.forEach(item => { if (item.source.supported) item.selected = selected; });
}
async function importIdeaSelection() {
  const items = ideaDiscovered.value.filter(item => item.selected && item.source.supported && item.source.kind!=="unsupported");
  if (!items.length || ideaScanning.value) return;
  if (items.length > 32) {
    ideaImportError.value = "一次最多导入 32 条；请取消多余的勾选后重试。";
    return;
  }
  ideaScanning.value = true;
  ideaImportError.value = "";
  try {
    const payload = items.map(({source}) => ({
      name:source.name,kind:source.kind as "mysql"|"postgres"|"sqlite",
      host:source.kind!=="sqlite"?source.host:null,
      port:source.kind!=="sqlite"?Number(source.portText||(source.kind==="postgres"?5432:3306)):null,
      database:source.database,username:source.kind!=="sqlite"?source.username:null,
    }));
    const outcome = await bridge.dbImportProfiles(payload);
    await db.refreshConnections();
    ideaDiscoverySummary.value = `已保存 ${outcome.imported} 条连接配置，跳过 ${outcome.duplicates} 条重复项；连接仍需逐条验证`;
    ideaDiscovered.value.forEach(item => {item.selected=false;});
    showConnection.value = false;
    connectionDialogOpen.value = false;
  } catch (_) {
    ideaImportError.value = "无法保存导入配置；原始 IDEA 数据未修改，请检查连接配置并重试";
  } finally { ideaScanning.value = false; }
}
function selectPendingIdeaConfig(config:DbConnectionConfig) {
  editConnection(config.id);
  // Keep the original production/read-only labels on imported profiles.
  db.form.enabled = false;
}
function editConnection(id:string) {
  const config=db.configs.find(item=>item.id===id);if(!config||db.busy)return;
  fillIdeaForm({kind:config.kind,name:config.name,database:config.database,host:config.host||'',portText:config.port?String(config.port):'',username:config.username||''},'prefer');
  db.form.id=config.id;db.form.enabled=config.enabled;db.form.allowWrite=config.allow_write;
  db.form.sslMode=config.ssl_mode;db.form.productionHint=config.production_hint===true?'yes':config.production_hint===false?'no':'';
  password.value='';connectionUrl.value='';connectionUrlError.value='';connectionUrlNotice.value='';
}
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
// in one place; MySQL always uses automatic TLS negotiation.
function fillIdeaForm(candidate: {
  kind: "mysql" | "postgres" | "sqlite";
  name: string; database: string; host: string;
  portText: string; username: string;
}, tls: "prefer" | "require") {
  const port = candidate.portText || (candidate.kind === "mysql" ? "3306" : candidate.kind === "postgres" ? "5432" : "");
  const existing = db.configs.find(cfg =>
    cfg.kind === candidate.kind && cfg.database === candidate.database &&
    (cfg.host || "") === candidate.host &&
    String(cfg.port ?? (candidate.kind === "mysql" ? 3306 : candidate.kind === "postgres" ? 5432 : "")) === port &&
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
  db.form.sslMode = candidate.kind === "mysql" ? "prefer" : tls;
  db.form.allowWrite = false;
  db.form.productionHint = "";
  password.value = "";
  ideaImportOpen.value = false;
  showConnection.value = true;
  connectionDialogOpen.value = true;
  connectionDialogTab.value = "manual";
}
function chooseManualIdeaSource(candidate: IdeaMysqlCandidate) {
  fillIdeaForm({...candidate, kind: "mysql", portText: candidate.port}, "require");
  manualIdeaCandidates.value = [];
  layout.showToast("只填入地址；请核对配置、输入密码并验证连接");
}
const visibleConnections = computed(() => {
  const needle = connectionFilter.value.trim().toLowerCase();
  if (!needle) return db.connections;
  return db.connections.filter(item => `${item.name} ${item.host || ""} ${item.database || ""} ${item.username || ""}`.toLowerCase().includes(needle));
});

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
const config = computed(() => db.configs.find(c => c.id === db.activeId));
const detailHeaders = computed(() => {
  if (db.objectDetailMode === 'columns') return ['字段', '类型', '可空', '键', '默认值'];
  if (db.objectDetailMode === 'indexes') return ['索引', '字段', '非唯一', '序号'];
  if (db.objectDetailMode === 'foreign_keys') return ['字段', '引用表', '引用字段'];
  return ['DDL'];
});
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
  showConnection.value=true;
  connectionDialogOpen.value=true;
  connectionDialogTab.value="idea";
  void scanIdeaProject();
}
function openIdeaImport() {
  showConnection.value=true;
  connectionDialogOpen.value=true;
  connectionDialogTab.value="idea";
  ideaImportOpen.value=true;
}
onMounted(async () => {
  stopIdeaProjectScan=subscribeIdeaProjectScan(handleIdeaProjectScan);
  const project=takeIdeaProjectPath();
  if(project)handleIdeaProjectScan(project);
  await db.refreshConnections();
  const ready = db.connections.filter(connection => connection.enabled);
  if (!db.activeId && ready.length === 1) db.selectConnection(ready[0]);
  if (!db.connections.length) showConnection.value = false;
});
onUnmounted(()=>{stopIdeaProjectScan();cancelIdeaDiscovery();});

async function doConnect() {
  // The UI owns automatic MySQL negotiation, including older saved profiles.
  if (db.form.kind === "mysql") db.form.sslMode = "prefer";
  const wasEnabled=db.form.enabled;
  db.form.enabled=true;
  const okConnected = await db.connect(password.value);
  if(!okConnected)db.form.enabled=wasEnabled;
  // 无论成败都立即清空本地密码，避免密码在内存中多停留一轮渲染
  password.value = "";
  if (okConnected) {
    showConnection.value = false;
    connectionDialogOpen.value = false;
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
      <button class="conn-toggle" @click="showConnection=true;connectionDialogOpen=true;connectionDialogTab='manual'">连接设置</button>
      <button class="close" @click="layout.sidebarOpen = false">✕</button>
    </div>

    <div v-if="!db.backendReady" class="gate">后端数据库命令未就绪，连接与查询暂不可用</div>

    <div class="db-workspace">
    <section v-if="!workbench.collapsed" class="conn">
      <div class="conn-head">
        <strong>数据源</strong>
        <span class="badge" :class="{on:db.recentlyVerified}">{{ connectionStatus }}</span>
        <button class="conn-add" title="新增数据库连接" aria-label="新增数据库连接" @click="db.resetForm(); password=''; showConnection=true; connectionDialogOpen=true; connectionDialogTab='manual'">＋</button>
      </div>
      <button class="idea-import-toggle" @click="openIdeaImport">从 IntelliJ IDEA 导入连接…</button>

      <div v-if="db.connections.length" class="connection-filter"><input v-model="connectionFilter" type="search" placeholder="搜索名称、主机、数据库…" aria-label="筛选数据库连接"/><span>{{visibleConnections.length}} / {{db.connections.length}}</span></div>
      <ul v-if="db.connections.length" class="conn-list">
        <li v-for="c in visibleConnections" :key="c.id" :class="{ active: c.id === db.form.id, disabled: !c.enabled }">
          <button class="row" @click="c.enabled ? db.selectConnection(c) : selectPendingIdeaConfig(db.configs.find(cfg=>cfg.id===c.id)!)">
            <span class="name">{{ c.name }}</span>
            <span class="meta">{{ c.label }}{{ c.allowWrite ? " · 允许写" : "" }}{{ c.enabled ? "" : " · 待验证（点击配置或验证）" }}</span>
          </button>
          <button class="connection-edit" :aria-label="'编辑连接 '+c.name" :disabled="db.busy" @click="editConnection(c.id)">编辑</button>
        </li>
      </ul>
      <p v-if="db.connections.length && !visibleConnections.length" class="hint">没有匹配的数据库连接。</p>

      <div v-if="connectionDialogOpen" class="connection-dialog-backdrop" @click.self="connectionDialogOpen=false; showConnection=false"></div>
      <div v-if="showConnection" class="form" :class="{'connection-dialog-form':connectionDialogOpen, 'idea-dialog':connectionDialogOpen&&connectionDialogTab==='idea'}" :role="connectionDialogOpen ? 'dialog' : undefined" :aria-modal="connectionDialogOpen ? 'true' : undefined" aria-labelledby="database-connection-title">
        <div class="connection-form-title"><strong id="database-connection-title">{{ db.form.id ? '连接设置' : '新增数据库连接' }}</strong><span>测试并保存后才会加入可用连接</span></div>
        <button v-if="connectionDialogOpen" class="connection-dialog-close" aria-label="关闭新增连接" title="关闭" @click="connectionDialogOpen=false; showConnection=false">×</button>
        <div v-if="connectionDialogOpen" class="connection-dialog-tabs" role="tablist" aria-label="连接方式"><button role="tab" :aria-selected="connectionDialogTab==='manual'" @click="connectionDialogTab='manual'">手动连接</button><button role="tab" :aria-selected="connectionDialogTab==='idea'" @click="connectionDialogTab='idea';ideaImportOpen=true">从 IDEA 导入</button></div>
        <template v-if="connectionDialogOpen&&connectionDialogTab==='idea'">
      <section v-if="ideaImportOpen" class="idea-import" aria-label="IDEA 数据源导入">
        <p>不会猜测 IDEA 固定安装或项目目录。可直接发现已授权工作区中的项目级配置；其他目录需要先在文件工作区打开并授权。</p>
        <div class="toolbar"><button :disabled="ideaDiscoveryBusy" @click="discoverIdeaSources">发现 IDEA 项目数据源</button><button v-if="ideaDiscoveryBusy" @click="cancelIdeaDiscovery">停止扫描</button></div>
        <p v-if="ideaDiscoverySummary" role="status">{{ideaDiscoverySummary}}</p>
        <div v-for="(item,index) in ideaDiscovered" :key="item.path+item.source.id+index" class="idea-source">
          <label><input type="checkbox" v-model="item.selected" :disabled="!item.source.supported||ideaScanning"/>
            <strong>{{item.source.name}}</strong>
            <small>{{item.source.kind}} · {{item.path}} · {{item.source.note}}</small>
          </label>
        </div>
        <button :disabled="ideaScanning||!ideaDiscovered.some(item=>item.selected)" @click="importIdeaSelection">一键导入选中的连接配置（最多 32 条）</button>
        <p>导入只保存非敏感配置，默认禁用且只读；在下面“待验证”列表选择条目、输入密码并测试连接后才能使用。</p>
        <p>不读取 IDEA 密码、SSH 凭据、<code>~/.config/JetBrains</code> 用户配置或 <code>dataSources.local.xml</code>。</p>
        <label>IDEA 项目目录
          <input v-model="ideaProjectPath" type="text" spellcheck="false" placeholder="/home/你的用户名/IdeaProjects/项目名" />
        </label>
        <button :disabled="ideaScanning || !ideaProjectPath.trim()" @click="scanIdeaProject">
          {{ ideaScanning ? "读取中…" : "预览此项目的数据源" }}
        </button>
        <p class="hint">此路径来自当前文件工作区，可修改为具体 IDEA 项目目录；不读取隐藏凭据。</p>
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
        </template>
        <template v-else>
        <p class="form-note">连接设置仅在需要时展开。密码不保存到页面或浏览器存储。</p>
        <p v-if="db.form.id && !db.form.enabled" class="hint">这是待验证配置。密码留空会尝试本机密钥库中的凭据；点击“验证并启用”，仅连接成功后才启用。</p>
        <label>连接名称<input v-model="db.form.name" placeholder="analytics" /></label>
        <label>数据库类型
          <select :value="db.form.kind" @change="(e) => db.setKind(((e.target as HTMLSelectElement).value) as DbKind)">
            <option value="sqlite">SQLite</option>
            <option value="mysql">MySQL</option>
            <option value="postgres">PostgreSQL</option>
          </select>
        </label>
        <div class="url-import" role="group" aria-label="连接 URL">
          <label>连接地址（主机名或 URL）
            <input v-model="connectionUrl" type="text" autocomplete="off" spellcheck="false" placeholder="db.example.com:3306 或 jdbc:mysql://db.example.com:3306/app" @keydown.enter.prevent="applyConnectionUrl" />
          </label>
          <button type="button" :disabled="!connectionUrl.trim() || db.busy" @click="applyConnectionUrl">解析 URL</button>
          <p class="hint">主机名仅填充主机/端口，保留当前库名；完整 URL 可填充数据库类型和库名。地址不保存；密码请单独输入。</p>
          <p v-if="connectionUrlError" class="errors" role="alert">{{ connectionUrlError }}</p>
          <p v-if="connectionUrlNotice" class="connection-url-notice" role="status">{{ connectionUrlNotice }}</p>
        </div>
        <label v-if="fields.host">主机<input v-model="db.form.host" placeholder="127.0.0.1" /></label>
        <label v-if="fields.port">端口<input v-model="db.form.portText" :placeholder="portPlaceholder" /></label>
        <label>{{ databaseFieldLabel(db.form.kind) }}<input v-model="db.form.database" placeholder="/data/app.db 或 app" /></label>
        <label v-if="fields.username">用户名<input v-model="db.form.username" placeholder="reader" /></label>
        <label v-if="fields.username">密码（仅交给系统密钥库，配置中不保存）<input v-model="password" type="password" autocomplete="off" /></label>
        <p v-if="db.form.id" class="hint">编辑时密码留空会复用系统密钥库中的凭据；填写新密码才会替换。必须先关闭此连接的事务/查询会话。</p>
        <label v-if="fields.ssl && db.form.kind !== 'mysql'">SSL
          <select v-model="db.form.sslMode">
            <option value="disable">disable</option>
            <option value="prefer">prefer</option>
            <option value="require">require</option>
          </select>
        </label>
        <p v-if="db.form.kind === 'mysql'" class="hint">MySQL 自动尝试 TLS；仅当服务器明确不支持 TLS 时回退到非加密连接。远程连接请使用可信网络和只读账号。</p>
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
          <button class="primary" :disabled="db.busy || !db.backendReady" @click="doConnect">{{!db.form.enabled?'验证并启用':db.form.id?'测试并保存':'连接'}}</button>
          <button :disabled="db.busy || !db.connected" @click="db.deselectConnection()">断开当前会话</button>
          <button :disabled="db.busy || !db.connected" @click="deleteConnection">删除连接及凭据</button>
          <button :disabled="db.busy" @click="db.resetForm(); password = ''">新建连接</button>
        </div>
        </template>
      </div>
      <div class="schema-head"><strong>对象资源树</strong>
        <button title="重新读取数据库对象" aria-label="刷新对象" :disabled="db.schemaBusy || !db.activeId" @click="db.refreshSchema()"><RefreshCw :size="14"/></button>
      </div>
      <p class="catalog-root">▾ {{ db.form.database || '请选择数据库' }}</p>
      <input v-model="objectFilter" class="schema-search" type="search" aria-label="筛选数据库表和视图" placeholder="搜索表和视图…" />
      <div class="schema-head tree-group">
        <button :aria-expanded="tablesOpen" @click="tablesOpen=!tablesOpen">{{ tablesOpen ? '▾' : '▸' }} 表 <small>({{db.schemaLoaded ? tables.length : '—'}})</small></button>
      </div>
      <template v-if="tablesOpen">
        <button v-for="table in tables" :key="table.name" class="schema-row"
          :class="{selected:db.selectedObject===table.name}" :aria-selected="db.selectedObject===table.name"
          :title="'单击查看字段，双击预览数据：'+table.name"
          @click="db.selectObject(table.name)" @dblclick="db.previewTable(table.name)"
          @keydown.enter.prevent="db.previewTable(table.name)">▦ {{ table.name }}</button>
      </template>
      <div class="schema-head tree-group">
        <button :aria-expanded="viewsOpen" @click="viewsOpen=!viewsOpen">{{ viewsOpen ? '▾' : '▸' }} 视图 <small>({{db.schemaLoaded ? views.length : '—'}})</small></button>
      </div>
      <template v-if="viewsOpen">
        <button v-for="table in views" :key="table.name" class="schema-row"
          :class="{selected:db.selectedObject===table.name}" :aria-selected="db.selectedObject===table.name"
          :title="'单击查看字段，双击预览视图：'+table.name"
          @click="db.selectObject(table.name)" @dblclick="db.previewTable(table.name)"
          @keydown.enter.prevent="db.previewTable(table.name)">◈ {{ table.name }}</button>
      </template>
      <p v-if="db.schema.length && !visibleSchema.length" class="hint">没有匹配的对象</p>
      <p v-if="db.schemaBusy" class="hint" role="status">正在连接并读取表和视图…</p>
      <div v-else-if="db.schemaError" class="schema-load-error" role="alert"><p>表列表加载失败：{{db.schemaError}}</p><button @click="db.refreshSchema()">重新连接并加载</button></div>
      <p v-else-if="!db.schema.length" class="hint">{{ db.schemaLoaded ? '该数据库没有可见的表或视图。' : '请选择数据源，加载表和视图。' }}</p>
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
          <div class="column-item column-header" role="row">
            <span v-for="header in detailHeaders" :key="header">{{ header }}</span>
          </div>
          <div v-for="(col,i) in db.objectDetailRows" :key="i" class="column-item" :title="col.join(' · ')" role="row">
            <strong>{{col[0]}}</strong><span v-for="(cell,j) in col.slice(1)" :key="j">{{cell}}</span>
          </div>
        </div>
      </div>
    </section>

    <DatabaseWorkspace />
    <aside class="inspector" aria-label="数据库检查器">
      <div class="inspector-head"><strong>Inspector</strong><button class="icon-button" title="检查器操作" @click="layout.showToast('检查器展示当前真实连接元数据')">⋯</button></div>
      <article class="inspector-card">
        <h3>当前连接</h3>
        <div class="inspector-row"><span>名称</span><b>{{ activeConnection?.name || '未选择' }}</b></div>
        <div class="inspector-row"><span>引擎</span><b>{{ config?.kind?.toUpperCase() || '—' }}</b></div>
        <div class="inspector-row"><span>状态</span><b :class="{ok: db.recentlyVerified && !db.schemaError, warn: !!db.schemaError}">{{ db.schemaBusy ? '连接中' : db.schemaError ? '连接失败' : db.recentlyVerified ? '已验证 · 按需连接' : '未选择' }}</b></div>
        <div class="inspector-row"><span>写入</span><b>{{ config?.allow_write ? '已允许' : '只读' }}</b></div>
      </article>
      <article class="inspector-card">
        <h3>对象结构 · {{ db.selectedObject || '未选择' }}</h3>
        <div class="inspector-row"><span>字段</span><b>{{ db.selectedObject ? db.objectDetailRows.length : '—' }}</b></div>
        <div class="inspector-row"><span>详情</span><b>{{ db.selectedObject ? db.objectDetailMode : '请从左侧选择表' }}</b></div>
        <button class="inspector-action" :disabled="!db.selectedObject" @click="db.selectedObject && db.previewTable(db.selectedObject)">查看数据</button>
      </article>
      <article class="inspector-card assistant-card">
        <h3>Database Assistant</h3>
        <p>查询结果、结构和安全门禁均来自当前 BrowserOS 数据库会话。</p>
        <div class="assistant-state">{{ !db.backendReady ? '原生数据库桥接未启动；启动 Tauri 客户端后可连接数据库' : db.document.busy ? '正在执行查询…' : db.error || '等待查询' }}</div>
      </article>
    </aside>
    </div>
  </div>
</template>

<style scoped>
.schema-load-error{margin:8px 0;padding:9px;border:1px solid #70434c;border-radius:6px;background:#35232b;color:#f0b9c0;font-size:12px;overflow-wrap:anywhere}
.schema-load-error p{margin:0 0 8px}.schema-load-error button{width:100%;background:#493541;color:#ffe1e7;border-color:#8b5a68}
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
.connection-filter{display:flex;align-items:center;gap:6px;margin:7px 0}.connection-filter input{min-width:0;flex:1}.connection-filter span{font-size:10px;color:#718196;white-space:nowrap}.idea-import-toggle{width:100%;text-align:left;margin:7px 0;padding:7px;border-radius:6px;border:1px solid #bdd2ed;background:#eff6ff;color:#245889;cursor:pointer;font-size:12px}
.idea-import{padding:10px;margin-bottom:8px;border:1px solid #d3e0f2;border-radius:8px;background:#fff;max-height:340px;overflow:auto}
.idea-import p{font-size:11px;line-height:1.5;color:#596d83;overflow-wrap:anywhere}
.idea-import label{display:block;font-size:11px}
.idea-import input{width:100%;font:11px ui-monospace,monospace;border:1px solid #cad8e6;border-radius:6px;padding:6px;margin:5px 0;box-sizing:border-box}
.idea-import button{border:1px solid #aac6e8;border-radius:6px;background:#f0f6ff;color:#2a5990;padding:6px;cursor:pointer}
.idea-import button:disabled{opacity:.5;cursor:default}
.idea-source-list{max-height:190px;overflow-y:auto;border:1px solid #dbe6f2;border-radius:6px;padding:0 7px;background:#fbfdff;scrollbar-width:thin}
.idea-source{display:flex;align-items:center;justify-content:space-between;gap:8px;border-top:1px solid #e4e9f1;padding:7px 0}
.idea-source:first-child{border-top:0}.idea-source.unsupported{background:#fffaf4}.idea-source.unsupported strong{color:#7c5a25}
.idea-source div{min-width:0}.idea-source strong{font-size:12px;display:block}.idea-source small{color:#62768f;display:block;word-break:break-all}
</style>

<style scoped>
/* DBX visual contract: keep the real database handlers, but match the approved dark workspace. */
.db-panel{position:relative;background:#111722;color:#d8dee9;font:13px/1.5 system-ui,-apple-system,"Segoe UI","Noto Sans CJK SC",sans-serif}
.db-panel>.tabs{height:48px;flex:0 0 48px;background:#131a27;border-bottom:1px solid #2a3546;color:#d8dee9;padding:0 14px;gap:12px}
.db-panel>.tabs span{color:#8794a8;font-size:12px}.studio-brand{color:#fff}
.db-workspace{display:grid;grid-template-columns:280px minmax(0,1fr) 292px;grid-template-rows:minmax(0,1fr);background:#111722;min-height:0}
.db-workspace>.conn{grid-column:1;grid-row:1;border-right:1px solid #2a3546;background:#151c29;padding:8px;overflow:auto;min-width:0}
.db-workspace>:deep(.database-workspace){grid-column:2;grid-row:1;min-width:0;background:#111722;color:#d8dee9;border:0;border-radius:0;padding:0 14px 14px;overflow:auto}
.db-workspace>.inspector{grid-column:3;grid-row:1;min-width:0;border-left:1px solid #2a3546;background:#151c29;padding:14px;overflow:auto;color:#d8dee9}
.db-panel .conn-head,.db-panel .schema-head{color:#f4f7fb}.db-panel .badge{border-color:#40516a;color:#8794a8}.db-panel .badge.on{color:#46d18d;border-color:#2d8d64}
.db-panel .connection-filter input,.db-panel .schema-search,.db-panel .form input,.db-panel .form select,.db-panel .url-import input{background:#111722;color:#d8dee9;border-color:#2a3546}
.db-panel .conn-list .row{color:#bbc7d8}.db-panel .conn-list .name{color:#e4ebf5}.db-panel .conn-list .meta{color:#91a0b5}.db-panel .conn-list li.active,.db-panel .schema-row.selected{background:#243247;color:#fff}.db-panel .schema-row{color:#bbc7d8;background:transparent}.db-panel .schema-row:hover{background:#1d293b}.db-panel .hint,.db-panel .form-note,.db-panel .tree-help{color:#8794a8}
.db-panel .conn-list{max-height:min(36vh,360px);padding:3px 0}.db-panel .conn-list li{margin:2px 0;border:1px solid transparent}.db-panel .conn-list li:hover{background:#202b3c;border-color:#33445b}.db-panel .conn-list li.disabled{opacity:1}.db-panel .conn-list li.disabled .meta{color:#e7b76b}.db-panel .conn-list .row{padding:9px 10px;gap:3px}.db-panel .conn-list .name{font-size:12px;font-weight:650;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%}.db-panel .conn-list .meta{font-size:10px;line-height:1.45;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%}
.db-panel .conn-head{position:relative}.db-panel .conn-add{margin-left:auto;width:24px;height:24px;padding:0;border-radius:6px;font-size:17px;line-height:1}.db-panel .connection-form-title{display:flex;align-items:baseline;justify-content:space-between;gap:8px;margin-bottom:8px}.db-panel .connection-form-title strong{font-size:13px;color:#f4f7fb}.db-panel .connection-form-title span{font-size:10px;color:#8794a8;text-align:right}
.db-panel .connection-dialog-backdrop{position:absolute;inset:48px 0 0;background:rgba(3,8,16,.68);z-index:20}
.db-panel .connection-dialog-form{position:absolute;z-index:21;top:50%;left:50%;transform:translate(-50%,-50%);width:min(460px,calc(100% - 48px));max-height:calc(100% - 48px);overflow:auto;margin:0;padding:18px;background:#1b2432;border:1px solid #40516a;border-radius:12px;box-shadow:0 16px 42px rgba(0,0,0,.5)}
.db-panel .connection-dialog-form{color-scheme:dark}
.db-panel .connection-dialog-form.idea-dialog{width:min(760px,calc(100% - 48px))}
.db-panel .connection-dialog-tabs{display:flex;gap:6px;margin:12px 0;border-bottom:1px solid #35445a}
.db-panel .connection-dialog-tabs button{padding:9px 15px;border:0;border-bottom:2px solid transparent;border-radius:0;background:transparent;color:#aebbd0}
.db-panel .connection-dialog-tabs button[aria-selected=true]{border-bottom-color:#65aeff;color:#fff;font-weight:700}
.db-panel .connection-warning{padding:10px 12px;border:1px solid #a46d42;border-radius:8px;background:#3a2b26;color:#ffd7b0;font-size:12px;line-height:1.5}
.db-panel .connection-url-notice{grid-column:1/-1;margin:0;color:#a7caee;font-size:11px;line-height:1.45}
.db-panel .idea-dialog .idea-import{max-height:none;margin:0;padding:4px 0;border:0;background:transparent}
.db-panel .idea-dialog .idea-source-list{max-height:240px;background:#141d2a;border-color:#35445a}
.db-panel .idea-dialog .idea-source{padding:11px 7px;border-color:#35445a}
.db-panel .idea-dialog .idea-source strong{color:#eef4ff}
.db-panel .idea-dialog .idea-source small{color:#9bb0c9}
.db-panel .connection-dialog-form .form-note{font-size:11px;line-height:1.5}.db-panel .connection-dialog-form label{font-size:12px;color:#aebbd0;margin:11px 0 6px}.db-panel .connection-dialog-form input,.db-panel .connection-dialog-form select{padding:9px;background:#121a26;border-color:#2a3546;border-radius:7px;color:#e7edf6}
.db-panel .connection-dialog-form .url-import{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:8px;align-items:end}.db-panel .connection-dialog-form .url-import label{margin-bottom:0}.db-panel .connection-dialog-form .url-import button{height:36px;padding:0 11px;border-radius:7px}
.db-panel .connection-dialog-form .form-actions{justify-content:flex-end;margin-top:18px}.db-panel .connection-dialog-form .form-actions button{padding:7px 11px;border-radius:7px}.db-panel .connection-dialog-form .form-actions .primary{background:#5aa9ff;border-color:#5aa9ff;color:#07111e;font-weight:700}
.db-panel .connection-dialog-form .connection-form-title{padding-right:26px;border-bottom:1px solid #2a3546;padding-bottom:12px}.db-panel .connection-dialog-close{position:absolute;right:12px;top:12px;width:28px;height:28px;padding:0;border:0;background:transparent!important;color:#9eacc0;font-size:22px;line-height:1}.db-panel .connection-dialog-close:hover{background:#243247!important;color:#fff}
.db-panel .form,.db-panel .idea-import{background:#1b2432;border-color:#2a3546;color:#d8dee9}.db-panel .idea-import button,.db-panel .idea-import-toggle{background:#182131;border-color:#40516a;color:#cbd6e4}
.db-panel .primary,.db-panel .form-actions .primary{background:#5aa9ff;border-color:#5aa9ff;color:#07111e}.db-panel button{background:#182131;border-color:#2a3546;color:#cbd6e4}.db-panel button:hover{background:#243247;color:#fff}.db-panel button:disabled{opacity:.45}
.db-panel .object-details{border-color:#2a3546}.db-panel .object-tabs button[aria-pressed=true]{background:#243751;color:#a9d4ff}
.db-panel .local-er{background:#1b2432;border-color:#2a3546}.db-panel .object-ddl{background:#101722;color:#bdc8d7;border-color:#2a3546}
.inspector-head{display:flex;align-items:center;justify-content:space-between;margin:-2px 0 12px;color:#f4f7fb;font-size:12px;letter-spacing:.08em;text-transform:uppercase}.icon-button{background:transparent;border:1px solid transparent;color:#8794a8;border-radius:6px;padding:5px 8px}.icon-button:hover{background:#1d293b;color:#fff}
.inspector-card{background:#1b2432;border:1px solid #2a3546;border-radius:9px;padding:12px;margin-bottom:10px}.inspector-card h3{font-size:12px;margin:0 0 10px;color:#dfe8f5}.inspector-row{display:flex;justify-content:space-between;gap:8px;font-size:12px;margin:8px 0;color:#aebbd0}.inspector-row b{color:#f0f4fa;font-weight:600;max-width:150px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.inspector-row b.ok{color:#46d18d}.inspector-row b.warn{color:#f4b860}.inspector-action{width:100%;margin-top:7px}.assistant-card p{font-size:11px;line-height:1.5;color:#8794a8}.assistant-state{padding:8px;border:1px solid #2a3546;border-radius:7px;background:#101722;color:#bdc8d7;font-size:11px;overflow-wrap:anywhere}
.db-panel .column-list{display:flex;flex-direction:column;gap:1px;overflow:auto;border:1px solid #273447;border-radius:6px;background:#273447}
.db-panel .column-item{display:grid;grid-template-columns:minmax(120px,1.2fr) repeat(4,minmax(76px,1fr));gap:8px;align-items:center;padding:7px 9px;background:#151e2b;color:#cbd5e1;font-size:12px;min-width:0}
.db-panel .column-item>*{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.db-panel .column-item.column-header{background:#202d40;color:#91a4bc;font-weight:600;position:sticky;top:0;z-index:1}
.db-panel .column-item strong{color:#f3f6fa;font-weight:600}
@media(max-width:1200px){.db-workspace{grid-template-columns:220px minmax(0,1fr)}.db-workspace>.inspector{display:none}}
</style>
