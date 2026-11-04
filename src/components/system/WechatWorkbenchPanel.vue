<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { bridge } from '../../bridge';
import WechatDraftPanel from './WechatDraftPanel.vue';

// No simulated provider metrics. Only the existing WeChat draft/material bridge is live.
type Section = 'overview' | 'topics' | 'draft' | 'layout' | 'calendar' | 'analytics' | 'messages' | 'assets' | 'viral';
const navigation: {id:Section; name:string; icon:string; subtitle:string; english:string}[] = [
  {id:'overview',name:'工作台总览',icon:'⌂',subtitle:'选题 · 成稿 · 排版 · 排期 · 复盘，一页掌握',english:'WORKBENCH OVERVIEW'},
  {id:'topics',name:'AI 选题库',icon:'✧',subtitle:'记录选题灵感，形成有依据的文章主题',english:'TOPIC DISCOVERY'},
  {id:'draft',name:'AI 一键成稿',icon:'✎',subtitle:'编辑图文、上传素材、安全创建公众号草稿',english:'DRAFT STUDIO'},
  {id:'layout',name:'排版与封面',icon:'▦',subtitle:'封面与正文图片管理、草稿校验',english:'LAYOUT & COVER'},
  {id:'calendar',name:'发布排期',icon:'▣',subtitle:'记录本机排期计划，不自动发布',english:'PUBLISH CALENDAR'},
  {id:'analytics',name:'数据复盘',icon:'▥',subtitle:'阅读数据与完读表现的真实接口待接入',english:'DATA REVIEW'},
  {id:'messages',name:'留言与私信',icon:'▤',subtitle:'评论与回复接口待接入，不会自动回复',english:'COMMENTS & MESSAGES'},
  {id:'assets',name:'素材库',icon:'▱',subtitle:'本次草稿的封面与正文素材上传',english:'ASSET LIBRARY'},
  {id:'viral',name:'爆文拆解',icon:'✦',subtitle:'拆解标题结构，不生成虚假数据',english:'HIT ARTICLE ANALYSIS'}
];
const current = ref<Section>('overview');
const selected = computed(() => navigation.find(item=>item.id===current.value) ?? navigation[0]!);
const pageNo = computed(() => String(navigation.findIndex(item=>item.id===current.value)+1).padStart(2,'0'));
const account = ref('检查中');
const permission = ref<'unknown'|'allowed'|'denied'>('unknown');
const draftCount = ref<number|null>(null);
const checking = ref(false);
const accountNotice = ref('');
const topic = ref('');
const angle = ref('');
const topics = ref<{title:string;angle:string}[]>([]);
const draftTitle = ref('');
const planTitle = ref('');
const planDate = ref('');
const plans = ref<{title:string;date:string}[]>([]);
const viralTitle = ref('');
const titleSignals = computed(()=>{
  const value = viralTitle.value.trim();
  if (!value) return [];
  const result:string[] = [];
  if (/\d|[一二三四五六七八九十]个/.test(value)) result.push('数字标题');
  if (/[？?]|如何|为何|为什么/.test(value)) result.push('问题导向');
  if (/却|但是|反而|vs|比/i.test(value)) result.push('冲突 / 对比');
  if (/AI|模型|GPT|Codex|Claude/i.test(value)) result.push('AI 领域');
  return result.length ? result : ['建议人工分析悬念与利益点'];
});
function go(id:Section){current.value=id;}
function addTopic(){
  if (!topic.value.trim()) return;
  topics.value.unshift({title:topic.value.trim().slice(0,120),angle:angle.value.trim().slice(0,300)});
  topic.value=''; angle.value='';
}
function toDraft(value:string){draftTitle.value=value.trim().slice(0,32);go('draft');}
function addPlan(){
  if (!planTitle.value.trim() || !planDate.value) return;
  plans.value.push({title:planTitle.value.trim().slice(0,80),date:planDate.value});
  plans.value.sort((a,b)=>a.date.localeCompare(b.date));
  planTitle.value='';planDate.value='';
}
function safeCode(error:unknown) {
  return String(error).match(/WECHAT_API_-?\d+|WECHAT_[A-Z0-9_]+/)?.[0] ?? 'WECHAT_REQUEST_FAILED';
}
async function refreshAccount(){
  if (checking.value) return;
  checking.value=true;accountNotice.value='';
  try{ account.value=await bridge.wechatAccountStatus(); }
  catch(error){account.value='尚未配置';accountNotice.value=safeCode(error);}
  finally{checking.value=false;}
}
async function checkPermission(){
  if (checking.value) return;
  checking.value=true;accountNotice.value='';
  try{
    const response=await bridge.wechatDraftPermissionCheck();
    permission.value=response.draft_read_access?'allowed':'denied';
    draftCount.value=typeof response.draft_count==='number'?response.draft_count:null;
    accountNotice.value=response.draft_read_access?'草稿读取权限已验证；写入仍须通过真实草稿核验。':'草稿读取权限未通过，请检查公众号权限及 IP 白名单。';
  }catch(error){permission.value='denied';accountNotice.value='官方权限检查失败：'+safeCode(error);}
  finally{checking.value=false;}
}
onMounted(()=>{void refreshAccount();});
</script>

<template>
  <section class="wxwb" aria-label="公众号个人工作台">
    <header class="wxwb-hero">
      <div class="wxwb-brand">✦ AI WORKBENCH <span>/ WECHAT</span></div>
      <div class="wxwb-hero-line">
        <div><h2>公众号<span>个人工作台</span></h2><p>{{selected.subtitle}}</p></div>
        <div class="wxwb-counter"><strong>{{pageNo}}</strong><span> / 09</span><small>{{selected.english}}</small></div>
      </div>
      <div class="wxwb-art" aria-hidden="true"><span>◈</span><span>▤</span></div>
    </header>
    <div class="wxwb-layout">
      <aside class="wxwb-rail">
        <div class="wxwb-profile"><span class="wxwb-avatar">☏</span><div><strong>我的公众号</strong><small>{{account}}</small></div></div>
        <nav aria-label="公众号工作台导航" class="wxwb-nav">
          <button v-for="item in navigation" :key="item.id" type="button" :class="{active:current===item.id}" :aria-current="current===item.id?'page':undefined" @click="go(item.id)">
            <span aria-hidden="true" class="wxwb-icon">{{item.icon}}</span><span>{{item.name}}</span>
          </button>
        </nav>
        <div class="wxwb-rail-bottom"><strong>草稿优先</strong><small>官方接口 · 人工确认</small><p>不自动发布、不群发、不定时发送。</p></div>
      </aside>
      <main class="wxwb-main" :aria-label="selected.name">
        <div class="wxwb-toolbar"><strong>{{selected.name}}</strong><span><i></i>{{permission==='allowed'?'权限已验证':permission==='denied'?'权限未通过':'权限尚未检查'}}</span></div>
        <div v-if="current==='overview'" class="wxwb-view">
          <div class="wxwb-kpis">
            <article><span>◈ 账号状态</span><strong class="wxwb-account-state">{{account}}</strong><small>来自本机系统凭据</small></article>
            <article><span>▣ 官方草稿数</span><strong>{{draftCount===null?'—':draftCount}}</strong><small>{{draftCount===null?'尚未核实':'来自微信草稿读取 API'}}</small></article>
            <article><span>✧ 本次选题</span><strong>{{topics.length}}</strong><small>本次会话，非 AI 抓取</small></article>
            <article><span>◷ 本机排期</span><strong>{{plans.length}}</strong><small>不会发送至微信</small></article>
          </div>
          <article class="wxwb-card"><div class="wxwb-card-head"><h3>创作流程</h3><small>点击任一步进入相应工作区</small></div>
            <div class="wxwb-flow">
              <button type="button" @click="go('topics')"><span>01</span><strong>选题</strong><small>提出问题</small></button>
              <button type="button" @click="go('draft')"><span>02</span><strong>成稿</strong><small>填写正文</small></button>
              <button type="button" @click="go('layout')"><span>03</span><strong>排版</strong><small>上传图片</small></button>
              <button type="button" @click="go('draft')"><span>04</span><strong>草稿</strong><small>微信 API</small></button>
              <button type="button" @click="go('draft')"><span>05</span><strong>复核</strong><small>后台确认</small></button>
            </div>
          </article>
          <div class="wxwb-cols">
            <article class="wxwb-card"><div class="wxwb-card-head"><h3>今日待办</h3><small>不模拟完成状态</small></div><div class="wxwb-todo">
              <button type="button" @click="go('topics')"><span>□</span> 新增文章选题 <em>进入 →</em></button>
              <button type="button" @click="go('draft')"><span>□</span> 编辑图文草稿 <em>进入 →</em></button>
              <button type="button" @click="go('layout')"><span>□</span> 上传封面与正文图 <em>进入 →</em></button>
              <button type="button" :disabled="checking" @click="checkPermission"><span>{{permission==='allowed'?'✓':'□'}}</span> 核对微信草稿权限 <em>{{checking?'检查中…':'检查 →'}}</em></button>
            </div></article>
            <article class="wxwb-card"><div class="wxwb-card-head"><h3>账号连接与权限</h3><button type="button" class="wxwb-link" @click="refreshAccount" :disabled="checking">刷新 ↻</button></div>
              <p class="wxwb-muted">草稿读取接口</p><div class="wxwb-big-state">{{permission==='allowed'?'已通过':permission==='denied'?'未通过':'待验证'}}</div>
              <button type="button" class="wxwb-cta" @click="checkPermission" :disabled="checking">{{checking?'检查中…':'验证草稿读取权限'}}</button>
              <p class="wxwb-muted" role="status">{{accountNotice||'粉丝数、阅读量和分享数据未接入，不展示虚构指标。'}}</p>
            </article>
          </div>
          <div class="wxwb-suggestion"><span aria-hidden="true">✦</span><div><strong>AI 今日建议</strong><p>当前没有接入 AI 推荐及官方阅读数据。请从选题开始，上传素材后只创建草稿，最后到微信后台人工检查。</p></div><button type="button" @click="go('draft')">进入草稿工作区 →</button></div>
        </div>
        <div v-else-if="current==='topics'" class="wxwb-view">
          <article class="wxwb-card"><div class="wxwb-card-head"><h3>待写选题</h3><small>仅本次会话 · 不外传</small></div>
            <form class="wxwb-form" @submit.prevent="addTopic"><label>主题 / 标题 <input v-model="topic" required maxlength="120" placeholder="例如：AI 工具如何改变内容创作？"/></label><label>选题依据 / 论证角度<textarea v-model="angle" rows="3" maxlength="300" placeholder="事实来源、用户痛点、因果机制、反例…"/></label><button class="wxwb-cta" type="submit">＋ 加入待写选题</button></form>
          </article>
          <article class="wxwb-card"><div class="wxwb-card-head"><h3>待写清单</h3><small>共 {{topics.length}} 项</small></div>
            <p v-if="topics.length===0" class="wxwb-muted">还没有选题。新增后可以直接将标题带入草稿编辑器。</p>
            <div v-for="(item,i) in topics" :key="i" class="wxwb-list-item"><div><strong>{{item.title}}</strong><p>{{item.angle||'尚未填写选题依据'}}</p></div><button type="button" @click="toDraft(item.title)">带入草稿 →</button><button type="button" aria-label="删除选题" @click="topics.splice(i,1)">×</button></div>
          </article><p class="wxwb-disclaimer">AI 推荐、热点抓取与阅读量预测尚未接入，当前不提供虚构排行。</p>
        </div>
        <div v-else-if="current==='calendar'" class="wxwb-view">
          <article class="wxwb-card"><div class="wxwb-card-head"><h3>本机发布排期</h3><small>计划仅在当前页面会话有效</small></div>
            <form class="wxwb-form wxwb-plan-form" @submit.prevent="addPlan"><label>文章标题<input v-model="planTitle" required maxlength="80" placeholder="待发布文章"/></label><label>计划时间<input v-model="planDate" required type="datetime-local"/></label><button type="submit" class="wxwb-cta">＋ 添加本机计划</button></form>
          </article>
          <article class="wxwb-card"><div class="wxwb-card-head"><h3>排期清单</h3><small>{{plans.length}} 项 · 未发布</small></div><p v-if="plans.length===0" class="wxwb-muted">尚无本机排期。这里不会执行微信定时发布。</p>
            <div v-for="(item,i) in plans" :key="i" class="wxwb-list-item"><div><strong>{{item.title}}</strong><p>{{item.date.replace('T',' ')}} · 未发送</p></div><button type="button" @click="plans.splice(i,1)">移除</button></div>
          </article>
        </div>
        <div v-else-if="current==='viral'" class="wxwb-view">
          <article class="wxwb-card"><div class="wxwb-card-head"><h3>标题结构拆解</h3><small>本地规则初筛，非阅读量预测</small></div><label class="wxwb-form">输入待研究的标题<textarea v-model="viralTitle" rows="3" maxlength="300" placeholder="粘贴标题，分析数字、问题与冲突等结构…"/></label>
            <div class="wxwb-signals" v-if="viralTitle.trim()"><strong>标题特征</strong><span v-for="signal in titleSignals" :key="signal">{{signal}}</span></div>
            <p class="wxwb-muted">建议结合真实来源、行业比较、利益结构与反例审视，不根据标题虚构数据。</p>
          </article><button type="button" class="wxwb-cta" :disabled="!viralTitle.trim()" @click="toDraft(viralTitle)">使用标题创建草稿 →</button>
        </div>
        <div v-else-if="current==='analytics'||current==='messages'" class="wxwb-view"><article class="wxwb-card wxwb-unavailable"><span aria-hidden="true">◈</span><h3>{{current==='analytics'?'数据复盘':'留言与私信'}}</h3><p>{{current==='analytics'?'官方阅读量、完读率、粉丝和分享接口尚未接入，不能展示示例为真实数据。':'公众号留言、评论和私信管理 API 尚未接入，不提供自动回复或群发。'}}</p><button type="button" class="wxwb-cta" @click="go('draft')">使用已接入的草稿功能 →</button></article></div>
        <div v-show="current==='draft'||current==='layout'||current==='assets'" class="wxwb-view wxwb-editor-view">
          <div class="wxwb-card-head"><div><h3>{{current==='draft'?'图文草稿编辑':current==='layout'?'排版与封面':current==='assets'?'公众号素材库':'图文草稿编辑'}}</h3><p>微信官方素材/草稿 API 可用；AI 自动成稿与自动排版尚未接入。</p></div><span class="wxwb-token">真实 API · 人工触发</span></div>
          <div class="wxwb-editor-grid"><div class="wxwb-card"><WechatDraftPanel :suggested-title="draftTitle"/></div><aside class="wxwb-preview"><h3>微信图文预览</h3><div class="wxwb-phone"><strong>公众号草稿</strong><small>请在微信公众号后台核对真实预览</small><div class="wxwb-phone-cover">▧<span>请上传真实封面</span></div><p>此处不生成模拟文章封面，也不把本地预览视为微信后台验收。</p></div><h4>安全边界</h4><p>上传封面 / 正文图 → 插入正文 → 创建草稿 → 验证详情。不会自动发布或群发。</p></aside></div>
        </div>
        <div class="wxwb-footer">● AI 助手就绪 · 真实接口返回才标记成功 <span>不自动发布 / 群发 / 定时发送</span></div>
      </main>
    </div>
  </section>
</template>

<style src="./wechat-workbench.css" scoped></style>