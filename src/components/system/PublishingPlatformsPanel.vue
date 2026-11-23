<script setup lang="ts">
import { computed, ref } from "vue";
import { useBrowserStore } from "../../capabilities/browser/public";
import { bridge } from "../../bridge";
import { PUBLISHING_PLATFORMS, officialPublisherUrl, platformById } from "../../capabilities/wechat/model/publishingPlatforms.mjs";
const props = defineProps<{ selectedId: string }>();
const emit = defineEmits<{ (e:"update:selectedId", id:string):void }>();
const browser=useBrowserStore();
const search=ref("");
const opening=ref("");
const checking=ref(false);
const account=ref("未核验");
const permission=ref<"unknown"|"allowed"|"denied">("unknown");
const draftCount=ref<number|null>(null);
const notice=ref("");
const selected=computed(()=>platformById(props.selectedId)??PUBLISHING_PLATFORMS[0]);
const filtered=computed(()=>PUBLISHING_PLATFORMS.filter(p=>(p.name+" "+p.category).toLowerCase().includes(search.value.trim().toLowerCase())));
function choose(id:string){emit("update:selectedId",id);notice.value="";}
async function openOfficial(id:string) {
  const url=officialPublisherUrl(id);
  if(!url || opening.value)return;
  opening.value=id;notice.value="";
  try {
    if (!await browser.openBrowserAt(url)) notice.value="官方后台未能打开，请检查浏览器能力。";
    else if (id === "wechat") void inspectWechat();
  }
  catch{notice.value="打开官方后台失败，请检查网络与登录状态。";}
  finally{opening.value="";}
}
async function inspectWechat() {
  if(props.selectedId!=="wechat"||checking.value)return;
  checking.value=true;notice.value="";
  try {
    account.value=String(await bridge.wechatAccountStatus()).slice(0,160);
    const result=await bridge.wechatDraftPermissionCheck();
    permission.value=result.draft_read_access?"allowed":"denied";
    draftCount.value=typeof result.draft_count==="number"?result.draft_count:null;
    notice.value=permission.value==="allowed"?"已通过微信官方 API 核验账号及草稿读取权限。":"微信草稿读取权限未通过，检查接口权限与 IP 白名单。";
  }catch(error){
    account.value="未核验";permission.value="denied";
    notice.value=String(error).match(/WECHAT_API_-?\d+|WECHAT_[A-Z0-9_]+/)?.[0]??"WECHAT_ACCOUNT_CHECK_FAILED";
  }finally{checking.value=false;}
}
</script>
<template>
  <section class="publisher-hub" aria-label="多平台内容中心">
    <header class="publisher-head"><div><h3>多平台创作中心</h3><p>选择官方后台并登录；只有已获授权的接口才可以读取配置。</p></div>
    <label>搜索平台 <input v-model="search" type="search" maxlength="40" placeholder="平台名或类型"/></label></header>
    <div class="publisher-grid">
      <button v-for="platform in filtered" :key="platform.id" type="button" :class="{active: selectedId===platform.id}" :aria-pressed="selectedId===platform.id" @click="choose(platform.id)">
        <span aria-hidden="true">{{platform.icon}}</span><strong>{{platform.name}}</strong><small>{{platform.category}}</small>
      </button>
      <p v-if="!filtered.length">没有匹配的平台</p>
    </div>
    <div v-if="selected" class="publisher-details">
      <div><strong>{{selected.name}}</strong><p>官方后台：{{selected.officialHost}}</p>
        <p>{{selected.id==="wechat"?"微信原有官方草稿与素材 API 已接入":"尚未接入官方自动化 API；账号配置不从网页中抓取"}}</p></div>
      <div class="publisher-buttons">
        <button type="button" :disabled="!!opening" @click="openOfficial(selected.id)">{{opening===selected.id?"打开中…":"↗ 一键打开官方后台"}}</button>
        <button v-if="selected.id==='wechat'" type="button" :disabled="checking" @click="inspectWechat">{{checking?"核验中…":"授权检测与账号信息"}}</button>
      </div>
      <p v-if="selected.id==='wechat'" class="publisher-status">账号：{{account}} · 草稿读取：{{permission==="allowed"?"通过":permission==="denied"?"未通过":"待核验"}} · 草稿数：{{draftCount??"待核验"}}</p>
      <p v-else class="publisher-status">授权：待接入官方 API；请在已打开的创作者后台使用平台自身的登录方式。</p>
      <p v-if="notice" role="status" class="publisher-status">{{notice}}</p>
    </div>
  </section>
</template>
<style scoped>
.publisher-hub{margin:12px 20px 0;padding:16px;background:#062842;border:1px solid #256a8c;border-radius:12px}
.publisher-head{display:flex;justify-content:space-between;gap:14px;flex-wrap:wrap}
.publisher-head h3{font-size:17px;margin:0 0 7px}.publisher-head p,.publisher-details p{font-size:12px;color:#bfd7e8;margin:5px 0}
.publisher-head label{font-size:11px;display:grid;gap:4px}
.publisher-head input{padding:9px;border:1px solid #4888a8;border-radius:7px;background:#041f34;color:white}
.publisher-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(120px,1fr));gap:8px;margin:12px 0}
.publisher-grid button{display:grid;justify-items:start;gap:5px;padding:10px;background:#0b3758;color:#eefaff;border:1px solid #27688e;border-radius:10px;cursor:pointer}
.publisher-grid button.active{border-color:#4cf6ff;background:#13557a}
.publisher-grid span{font-size:19px}.publisher-grid small{font-size:10px;color:#b4d7eb}
.publisher-details{padding:12px;border:1px solid #34708f;border-radius:9px;display:flex;align-items:center;gap:12px;justify-content:space-between;flex-wrap:wrap}
.publisher-buttons{display:flex;gap:8px;flex-wrap:wrap}.publisher-buttons button{padding:9px 11px;background:#0876a3;color:white;border:1px solid #54d3f0;border-radius:8px;cursor:pointer}
.publisher-buttons button:disabled{opacity:.5;cursor:wait}
.publisher-details .publisher-status{width:100%}
@media(max-width:650px){.publisher-hub{margin:8px}.publisher-head input{width:100%}}
</style>
