<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { MODULE_META, useLayoutStore, type MainView } from "../../../stores/useLayoutStore";
import { hostServices, type BrowserContextPort } from "../../../capability/platform/host-services";
import { useToolsStore } from "../state/useToolsStore";

const store = useToolsStore();
const layout = useLayoutStore();
function openModule(id: string) {
  if (id === "browser") { layout.activateBrowser(); return; }
  layout.openModule(id as MainView);
  if (id === "grid") {
    const browser = hostServices.get<BrowserContextPort>("browser-context");
    if (browser) void browser.activateGrid().catch(() => layout.showToast("宫格启动失败"));
  }
}
const query = ref("");
const groups = [
 { title: "日常工作", ids: ["browser","files","home","clip","apps"] },
 { title: "AI 与比较", ids: ["grid","agents","skills","graph"] },
 { title: "开发和自动化", ids: ["term","repo","db","scripts","commands","tasks"] },
 { title: "知识与管理", ids: ["vault","arts","audit","plugin","settings"] },
];
const visibleGroups = computed(() => groups.map(g => ({ ...g, items: g.ids.filter(id => (MODULE_META[id]?.label || id).toLowerCase().includes(query.value.toLowerCase())) })).filter(g => g.items.length));
const visibleTools = computed(() => store.tools.filter(t => (t.name + " " + t.category + " " + (t.description || "")).toLowerCase().includes(query.value.toLowerCase())));
onMounted(store.load);
</script>

<template>
  <main class="toolbox">
    <header class="tb-hero"><div><span class="tb-eyebrow">WORKSPACE</span><h1>工具</h1><p>需要时再打开。浏览器和文件始终是工作中心。</p></div></header>
    <label class="tb-search"><span>查找工具</span><input v-model="query" type="search" placeholder="搜索终端、数据库、宫格、知识库…" aria-label="搜索工具" /></label>
    <section v-for="group in visibleGroups" :key="group.title" class="tb-section"><h2>{{ group.title }}</h2>
      <div class="tb-grid"><button v-for="id in group.items" :key="id" class="tb-card" @click="openModule(id)"><strong>{{ MODULE_META[id]?.label || id }}</strong><small>打开工作区 <span aria-hidden="true">↗</span></small></button></div>
    </section>
    <section class="tb-section" v-if="visibleTools.length"><h2>内置与个人工具</h2><div class="tb-grid">
      <button v-for="t in visibleTools" :key="t.id" class="tb-card" :title="t.description || t.id" @click="store.open(t)"><strong>{{ t.name }}</strong><small>{{ t.category }}</small></button>
    </div></section>
    <p v-if="store.error" class="tb-err" role="alert">工具加载失败：{{ store.error }} <button @click="store.load">重试</button></p>
    <p v-if="!visibleGroups.length && !visibleTools.length && !store.error" class="tb-empty">没有找到匹配的工具。试试搜索“终端”或“宫格”。</p>
  </main>
</template>

<style scoped>
.toolbox { box-sizing:border-box; height:100%; overflow:auto; background:#f8fafc; color:#1d2939; padding:30px clamp(16px,4vw,48px); }
.tb-hero { display:flex; justify-content:space-between; margin-bottom:22px; }
.tb-hero h1 { font-size:28px; margin:4px 0; letter-spacing:-.03em; }
.tb-hero p { color:#667085; margin:6px 0; font-size:13px; }
.tb-eyebrow { color:#64748b; font-size:10px; letter-spacing:.14em; font-weight:700; }
.tb-search { display:flex; flex-direction:column; gap:5px; font-size:12px; color:#475467; max-width:500px; margin-bottom:24px; }
.tb-search input { box-sizing:border-box; width:100%; padding:10px 14px; border:1px solid #d0d8e3; border-radius:10px; font:inherit; background:#fff; }
.tb-search input:focus-visible,.tb-card:focus-visible { outline:2px solid #2563eb; outline-offset:2px; }
.tb-section { margin:0 0 26px; }
.tb-section h2 { font-size:14px; color:#344054; font-weight:650; margin:0 0 11px; }
.tb-grid { display:grid; grid-template-columns:repeat(auto-fill,minmax(175px,1fr)); gap:10px; }
.tb-card { display:flex; flex-direction:column; align-items:flex-start; justify-content:center; min-height:76px; gap:8px; background:#fff; border:1px solid #e2e8f0; border-radius:12px; padding:12px 14px; text-align:left; cursor:pointer; color:#243244; }
.tb-card:hover { border-color:#94b5df; box-shadow:0 2px 8px #0f172a0c; }
.tb-card strong { font-size:13px; font-weight:650; }
.tb-card small { font-size:11px; color:#64748b; }
.tb-err { color:#b42318; }.tb-empty { color:#667085; }
</style>
