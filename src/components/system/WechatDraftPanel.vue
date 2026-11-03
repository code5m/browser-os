<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { bridge } from '../../bridge';

// Credential is an ephemeral form field; never persist it in browser storage.
const appId = ref('');
const appSecret = ref('');
const configured = ref('未配置');
const title = ref('');
const author = ref('');
const digest = ref('');
const contentHtml = ref('');
const coverPath = ref('');
const bodyImagePath = ref('');
const coverMediaId = ref('');
const bodyImageUrl = ref('');
const draftMediaId = ref('');
const verified = ref(false);
const busy = ref(false);
const notice = ref('');
const outcomeUnknown = ref(false);

function statusError(error: unknown): string {
  // Only stable native error codes enter UI; never echo a secret, response URL or raw provider body.
  return String(error).match(/WECHAT_[A-Z0-9_]+/)?.[0] || 'WECHAT_REQUEST_FAILED';
}
async function run(action: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true;
  notice.value = '';
  try { await action(); }
  catch (error) { notice.value = `操作失败：${statusError(error)}`; }
  finally { busy.value = false; }
}
function refresh() {
  void run(async () => {
    configured.value = await bridge.wechatAccountStatus();
    notice.value = `已连接本机系统凭据：${configured.value}`;
  });
}
function saveAccount() {
  void run(async () => {
    try {
      configured.value = await bridge.wechatAccountSave(appId.value.trim(), appSecret.value);
      notice.value = '配置已存入系统密钥库，未保存到浏览器存储。';
    } finally { appSecret.value = ''; }
  });
}
function checkPermission() {
  void run(async () => {
    const result = await bridge.wechatDraftPermissionCheck();
    notice.value = `官方草稿读取权限检查通过；账号草稿数：${result.draft_count ?? '未知'}。写入权限仍须通过真实草稿核对。`;
  });
}
function uploadCover() {
  void run(async () => {
    const result = await bridge.wechatUploadCover(coverPath.value.trim());
    coverMediaId.value = result.media_id;
    notice.value = '封面素材上传成功，已获得永久素材 media_id。';
  });
}
function uploadBodyImage() {
  void run(async () => {
    const result = await bridge.wechatUploadBodyImage(bodyImagePath.value.trim());
    bodyImageUrl.value = result.url;
    notice.value = '正文图片已上传；可将图片地址插入正文。';
  });
}
function insertBodyImage() {
  if (!bodyImageUrl.value) return;
  // This URL is returned by the fixed WeChat HTTPS endpoint, not arbitrary HTML.
  contentHtml.value += `\n<p><img src="${bodyImageUrl.value.replaceAll('&', '&amp;').replaceAll('"', '&quot;')}" alt="正文图片" /></p>`;
}
function createDraft() {
  void run(async () => {
    if (outcomeUnknown.value) {
      notice.value = '上一次创建结果不明。请先到微信公众号后台确认是否已生成，禁止盲目重试。';
      return;
    }
    // A new, user-initiated draft/add only. No publish or mass-send path exists.
    try {
      const result = await bridge.wechatCreateDraft(title.value, author.value, digest.value, contentHtml.value, coverMediaId.value);
      draftMediaId.value = result.media_id;
      verified.value = result.verified;
      notice.value = result.verified
        ? '微信草稿箱已返回并验证草稿，未发布或群发。'
        : '草稿接口已返回 media_id，但详情核验未完成；请点击验证草稿，勿重复创建。';
    } catch (error) {
      if (statusError(error) === 'WECHAT_DRAFT_OUTCOME_UNKNOWN_NO_AUTO_RETRY') outcomeUnknown.value = true;
      throw error;
    }
  });
}
function verifyDraft() {
  void run(async () => {
    const result = await bridge.wechatGetDraftStatus(draftMediaId.value.trim());
    verified.value = result.verified;
    notice.value = '草稿详情接口确认存在。请继续在真实公众平台草稿箱检查图片、标题和正文。';
  });
}
function confirmResolvedUnknown() {
  if (!confirm('你已在微信公众号后台检查，并确认上次不明结果没有创建草稿？')) return;
  outcomeUnknown.value = false;
  notice.value = '已记录人工核实；允许重新创建一次草稿。';
}
onMounted(() => {
  // Absence of an account is expected, not an error. Never test with fake credentials.
  bridge.wechatAccountStatus().then(v => { configured.value = v; }).catch(() => { configured.value = '未配置'; });
});
</script>

<template>
  <section class="wechat-draft" aria-label="微信公众号草稿箱">
    <header><strong>微信公众号 · 素材与草稿箱</strong><small>仅人工触发上传与新建草稿；不会发布、群发或定时发送</small></header>
    <p>本机凭据：{{ configured }}。要求公众号开通草稿及素材 API 权限，微信后台 IP 白名单应包含本机出口 IP。</p>
    <form @submit.prevent="saveAccount" class="form-row">
      <label>AppID <input v-model.trim="appId" required maxlength="64" autocomplete="off" placeholder="wx..." /></label>
      <label>AppSecret <input v-model="appSecret" type="password" required maxlength="512" autocomplete="new-password" placeholder="仅写入系统密钥库" /></label>
      <button type="submit" :disabled="busy">保存账户</button>
      <button type="button" :disabled="busy" @click="refresh">刷新配置</button>
      <button type="button" :disabled="busy" @click="checkPermission">验证草稿读取权限</button>
    </form>
    <div class="form-row">
      <label>封面 JPG（不超过 64 KB）<input v-model="coverPath" placeholder="/home/.../cover.jpg" aria-label="封面绝对路径" /></label>
      <button :disabled="busy || !coverPath.trim()" @click="uploadCover">上传永久封面</button>
      <span>封面 media_id：{{ coverMediaId || '未上传' }}</span>
    </div>
    <div class="form-row">
      <label>正文 JPG/PNG（不超过 1 MB）<input v-model="bodyImagePath" placeholder="/home/.../body.png" aria-label="正文图片绝对路径" /></label>
      <button :disabled="busy || !bodyImagePath.trim()" @click="uploadBodyImage">上传正文图片</button>
      <button :disabled="!bodyImageUrl" @click="insertBodyImage">插入正文</button>
    </div>
    <form class="draft-form" @submit.prevent="createDraft">
      <div class="form-row">
        <label>标题 <input v-model="title" required maxlength="32" /></label>
        <label>作者 <input v-model="author" maxlength="16" /></label>
        <label>摘要 <input v-model="digest" maxlength="120" /></label>
      </div>
      <label>草稿 HTML 正文<textarea v-model="contentHtml" required rows="7" maxlength="19999" spellcheck="false" placeholder="仅发送用户明确填写的 HTML 内容" /></label>
      <div class="form-row">
        <button type="submit" :disabled="busy || outcomeUnknown || !title.trim() || !contentHtml.trim() || !coverMediaId">创建到草稿箱（不发布）</button>
        <span>创建后不会自动重试，以免出现重复草稿。</span>
      </div>
    </form>
    <div v-if="draftMediaId" class="form-row">
      <span>草稿 media_id：{{ draftMediaId }} · {{ verified ? 'API 验证成功' : '待验证' }}</span>
      <button :disabled="busy" @click="verifyDraft">验证草稿详情</button>
    </div>
    <div v-if="outcomeUnknown" class="form-row"><strong>提交超时：结果可能已成功。</strong><button :disabled="busy" @click="confirmResolvedUnknown">人工核实无重复草稿后解除禁止重试</button></div>
    <p role="status" aria-live="polite">{{ busy ? '请求中…' : notice }}</p>
    <p>请在微信公众号管理后台草稿箱复核封面、正文图、权限及素材。当前 UI 的模拟测试不代表账号真实验收。</p>
  </section>
</template>

<style scoped>
.wechat-draft{display:flex;flex-direction:column;gap:12px;font-size:13px;line-height:1.5;color:var(--text-primary,#26384a)}
header{display:flex;gap:12px;align-items:baseline;flex-wrap:wrap}small{color:var(--text-secondary,#65768c)}
.form-row{display:flex;gap:12px;align-items:flex-end;flex-wrap:wrap}
.form-row label{flex:1;min-width:180px}
label{display:flex;flex-direction:column;gap:5px}
input,textarea,button{font:inherit;padding:8px;border:1px solid #8295ac;border-radius:6px;background:var(--bg-secondary,#f5f7fa);color:inherit}
input,textarea{width:100%;box-sizing:border-box}textarea{resize:vertical;min-height:150px}button{cursor:pointer}button:disabled{opacity:.5;cursor:default}
.draft-form{display:flex;flex-direction:column;gap:12px}p{margin:0;color:var(--text-secondary,#65768c)}
</style>
