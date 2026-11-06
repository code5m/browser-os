<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { bridge, type WechatWorkingDraft } from "../../bridge";
import { open as openNativeDialog } from "@tauri-apps/plugin-dialog";

// AppSecret is an ephemeral form value and is never included in the working-draft DTO.
const appId = ref("");
const appSecret = ref("");
const configured = ref("未配置");
const props = defineProps<{ suggestedTitle?: string }>();

const title = ref("");
const author = ref("");
const digest = ref("");
const contentHtml = ref("");
const coverPath = ref("");
const bodyImagePath = ref("");
const coverMediaId = ref("");
const bodyImageUrl = ref("");
const draftMediaId = ref("");
const verified = ref(false);
const outcomeUnknown = ref(false);

const busy = ref(false);
const loadingDraft = ref(true);
const draftStorageAvailable = ref(false);
const storageNotice = ref("正在从系统密钥库恢复工作草稿…");
const lastSavedAt = ref(0);
const notice = ref("");

let hydrating = true;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let saveQueue: Promise<void> = Promise.resolve();

const draftNotSentErrors = new Set([
  "WECHAT_DRAFT_INPUT_INVALID",
  "WECHAT_ACCOUNT_NOT_CONFIGURED",
  "WECHAT_ACCOUNT_INVALID",
  "WECHAT_HTTP_CONFIG_FAILED",
  "WECHAT_TOKEN_NETWORK_FAILED",
  "WECHAT_TOKEN_MISSING",
]);

function statusError(error: unknown): string {
  // Never echo a secret, provider response body, access token or request URL.
  return String(error).match(/WECHAT_API_-?\d+|WECHAT_[A-Z0-9_]+/)?.[0] ?? "WECHAT_REQUEST_FAILED";
}

function snapshot(): WechatWorkingDraft {
  return {
    version: 1,
    title: title.value,
    author: author.value,
    digest: digest.value,
    content_html: contentHtml.value,
    cover_path: coverPath.value,
    body_image_path: bodyImagePath.value,
    cover_media_id: coverMediaId.value,
    body_image_url: bodyImageUrl.value,
    draft_media_id: draftMediaId.value,
    verified: verified.value,
    outcome_unknown: outcomeUnknown.value,
    updated_at_ms: lastSavedAt.value,
  };
}

function restoreDraft(draft: WechatWorkingDraft) {
  title.value = draft.title ?? "";
  author.value = draft.author ?? "";
  digest.value = draft.digest ?? "";
  contentHtml.value = draft.content_html ?? "";
  coverPath.value = draft.cover_path ?? "";
  bodyImagePath.value = draft.body_image_path ?? "";
  coverMediaId.value = draft.cover_media_id ?? "";
  bodyImageUrl.value = draft.body_image_url ?? "";
  draftMediaId.value = draft.draft_media_id ?? "";
  verified.value = draft.verified === true;
  outcomeUnknown.value = draft.outcome_unknown === true;
  lastSavedAt.value = draft.updated_at_ms ?? 0;
}

async function loadWorkingDraft() {
  loadingDraft.value = true;
  hydrating = true;
  try {
    const saved = await bridge.wechatWorkingDraftLoad();
    if (saved) {
      restoreDraft(saved);
      storageNotice.value = "已从系统密钥库恢复未完成草稿。";
    } else {
      if (props.suggestedTitle?.trim()) title.value = props.suggestedTitle.slice(0, 32);
      storageNotice.value = "尚无本地工作草稿；后续编辑会自动保存到系统密钥库。";
    }
    draftStorageAvailable.value = true;
  } catch (error) {
    draftStorageAvailable.value = false;
    storageNotice.value = "系统密钥库不可用：" + statusError(error) + "。在恢复成功前，禁止创建新草稿。";
  } finally {
    hydrating = false;
    loadingDraft.value = false;
  }
}

async function persistWorkingDraft(): Promise<void> {
  if (hydrating) return;
  const payload = snapshot();
  const task = saveQueue.then(async () => {
    const savedAt = await bridge.wechatWorkingDraftSave(payload);
    lastSavedAt.value = savedAt;
    draftStorageAvailable.value = true;
    storageNotice.value = "工作草稿已加密保存到系统密钥库。";
  });
  saveQueue = task.then(() => undefined, () => undefined);
  try {
    await task;
  } catch (error) {
    draftStorageAvailable.value = false;
    storageNotice.value = "工作草稿保存失败：" + statusError(error) + "。请先恢复密钥库保存，再创建公众号草稿。";
    throw error;
  }
}

function schedulePersist() {
  if (hydrating || loadingDraft.value) return;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void persistWorkingDraft().catch(() => {});
  }, 650);
}

watch(
  [title, author, digest, contentHtml, coverPath, bodyImagePath, coverMediaId, bodyImageUrl, draftMediaId, verified, outcomeUnknown],
  schedulePersist,
  { flush: "post" },
);

// Once an existing draft has been verified, changing its content starts a new
// local working version and drops only the old verification reference.
watch(
  [title, author, digest, contentHtml, coverPath, bodyImagePath, coverMediaId, bodyImageUrl],
  () => {
    if (hydrating || !verified.value || outcomeUnknown.value) return;
    draftMediaId.value = "";
    verified.value = false;
  },
  { flush: "sync" },
);

watch(
  () => props.suggestedTitle,
  (value) => {
    if (!value?.trim() || title.value.trim()) return;
    title.value = value.slice(0, 32);
  },
);

watch(coverPath, () => {
  coverMediaId.value = "";
}, { flush: "sync" });
watch(bodyImagePath, () => {
  bodyImageUrl.value = "";
}, { flush: "sync" });

async function run(action: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true;
  notice.value = "";
  try {
    await action();
  } catch (error) {
    notice.value = "操作失败：" + statusError(error);
  } finally {
    busy.value = false;
  }
}

function refresh() {
  void run(async () => {
    configured.value = await bridge.wechatAccountStatus();
    notice.value = "已连接本机系统凭据：" + configured.value;
  });
}

function saveAccount() {
  void run(async () => {
    try {
      configured.value = await bridge.wechatAccountSave(appId.value.trim(), appSecret.value);
      coverMediaId.value = "";
      bodyImageUrl.value = "";
      await persistWorkingDraft();
      notice.value = "账户配置已存入系统密钥库；AppSecret 未写入工作草稿。";
    } finally {
      appSecret.value = "";
    }
  });
}

function checkPermission() {
  void run(async () => {
    const result = await bridge.wechatDraftPermissionCheck();
    notice.value = "官方草稿读取权限检查通过；账号草稿数：" + (result.draft_count ?? "未知") + "。写入权限仍须通过真实草稿核对。";
  });
}

function chooseCover() {
  void run(async () => {
    const selected = await openNativeDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "封面 JPG", extensions: ["jpg", "jpeg"] }],
    });
    if (typeof selected === "string") coverPath.value = selected;
  });
}

function chooseBodyImage() {
  void run(async () => {
    const selected = await openNativeDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "正文图片", extensions: ["jpg", "jpeg", "png"] }],
    });
    if (typeof selected === "string") bodyImagePath.value = selected;
  });
}

function uploadCover() {
  void run(async () => {
    const path = coverPath.value.trim();
    const result = await bridge.wechatUploadCover(path);
    if (coverPath.value.trim() !== path) {
      notice.value = "封面路径已变更，请重新上传封面。";
      return;
    }
    coverMediaId.value = result.media_id;
    await persistWorkingDraft();
    notice.value = "封面素材上传成功，media_id 已保存到加密工作草稿。";
  });
}

function uploadBodyImage() {
  void run(async () => {
    const path = bodyImagePath.value.trim();
    const result = await bridge.wechatUploadBodyImage(path);
    if (bodyImagePath.value.trim() !== path) {
      notice.value = "正文图片路径已变更，请重新上传图片。";
      return;
    }
    bodyImageUrl.value = result.url;
    await persistWorkingDraft();
    notice.value = "正文图片已上传，官方返回的图片地址已保存到加密工作草稿。";
  });
}

function insertBodyImage() {
  if (!bodyImageUrl.value) return;
  const safeUrl = bodyImageUrl.value.replaceAll("&", "&amp;").replaceAll('"', "&quot;");
  contentHtml.value += "\n<p><img src=\"" + safeUrl + "\" alt=\"正文图片\" /></p>";
}

async function saveBeforeDraftAdd() {
  if (saveTimer) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  await persistWorkingDraft();
}

function createDraft() {
  void run(async () => {
    if (loadingDraft.value || !draftStorageAvailable.value) {
      notice.value = "系统密钥库工作草稿尚未就绪，不能创建草稿。";
      return;
    }
    if (outcomeUnknown.value) {
      notice.value = "上一次创建结果不明或尚未验证。请先到微信公众号后台确认，禁止盲目重试。";
      return;
    }

    // Persist the no-blind-retry latch before making the irreversible draft/add request.
    outcomeUnknown.value = true;
    draftMediaId.value = "";
    verified.value = false;
    await saveBeforeDraftAdd();

    let result: { media_id: string; verified: boolean; status: string };
    try {
      result = await bridge.wechatCreateDraft(
        title.value,
        author.value,
        digest.value,
        contentHtml.value,
        coverMediaId.value,
      );
    } catch (error) {
      if (draftNotSentErrors.has(statusError(error))) {
        outcomeUnknown.value = false;
        await saveBeforeDraftAdd();
      }
      throw error;
    }

    draftMediaId.value = result.media_id;
    verified.value = result.verified;
    // A returned media_id blocks duplicate creation until details have been verified.
    outcomeUnknown.value = !result.verified;
    await saveBeforeDraftAdd();

    if (!result.verified) {
      notice.value = "微信接口已返回 media_id，但详情核验未完成。请点击「验证草稿详情」，不要重复创建。";
      return;
    }
    notice.value = "微信草稿箱已返回并验证草稿；未发布、群发或定时发送。";
  });
}

function verifyDraft() {
  void run(async () => {
    const mediaId = draftMediaId.value.trim();
    if (!mediaId) {
      notice.value = "没有可验证的 media_id。";
      return;
    }
    const result = await bridge.wechatGetDraftStatus(mediaId);
    verified.value = result.verified;
    if (!result.verified) {
      outcomeUnknown.value = true;
      await saveBeforeDraftAdd();
      notice.value = "草稿详情尚未验证，请勿重复创建。";
      return;
    }
    outcomeUnknown.value = false;
    await saveBeforeDraftAdd();
    notice.value = "草稿详情接口确认存在。请继续在真实公众号草稿箱检查图片、标题和正文。";
  });
}

function confirmResolvedUnknown() {
  if (!confirm("你已在微信公众号后台检查，并确认上次不明结果没有创建草稿？")) return;
  void run(async () => {
    outcomeUnknown.value = false;
    await saveBeforeDraftAdd();
    notice.value = "已记录人工核实；允许重新创建一次草稿。";
  });
}

function retryStorage() {
  void run(async () => {
    if (!draftStorageAvailable.value) {
      // Never overwrite an unknown, existing keyring draft after a transient
      // read failure. Recover it first; do not discard unsaved local content.
      if (title.value || author.value || digest.value || contentHtml.value ||
          coverPath.value || bodyImagePath.value || draftMediaId.value) {
        notice.value = "当前页面存在尚未保存的内容。请先手动备份文本；不能在未核对原草稿时覆盖系统密钥库。";
        return;
      }
      await loadWorkingDraft();
      if (!draftStorageAvailable.value) throw new Error("WECHAT_DRAFT_STORE_UNAVAILABLE");
      notice.value = "密钥库已重新连接，原工作草稿已恢复。";
      return;
    }
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    await persistWorkingDraft();
    notice.value = "当前工作草稿已保存到系统密钥库。";
  });
}

function clearWorkingDraft() {
  const warning = outcomeUnknown.value
    ? "上次草稿创建结果尚未明确。请先在公众号后台核实是否已创建，再清除本机恢复草稿。确定继续清除？"
    : "这会清除本机保存的工作草稿、素材路径和 media_id，不会删除微信公众号后台已有草稿。确定继续？";
  if (!confirm(warning)) return;
  void run(async () => {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    // Drain previously queued writes BEFORE deleting the keyring item, then
    // suppress watchers while clearing reactive fields to prevent resurrection.
    hydrating = true;
    try {
      await saveQueue;
      await bridge.wechatWorkingDraftClear();
      title.value = "";
      author.value = "";
      digest.value = "";
      contentHtml.value = "";
      coverPath.value = "";
      bodyImagePath.value = "";
      coverMediaId.value = "";
      bodyImageUrl.value = "";
      draftMediaId.value = "";
      verified.value = false;
      outcomeUnknown.value = false;
      await nextTick();
      draftStorageAvailable.value = true;
      lastSavedAt.value = 0;
      storageNotice.value = "本机工作草稿已从系统密钥库清除。";
      notice.value = "已清除本机工作草稿；微信公众号后台内容不受影响。";
    } finally {
      hydrating = false;
    }
  });
}

onMounted(() => {
  void loadWorkingDraft();
  bridge.wechatAccountStatus()
    .then((value) => { configured.value = value; })
    .catch(() => { configured.value = "未配置"; });
});

onBeforeUnmount(() => {
  if (saveTimer) clearTimeout(saveTimer);
  if (!hydrating && draftStorageAvailable.value) {
    void persistWorkingDraft().catch(() => {});
  }
});
</script>

<template>
  <section class="wechat-draft" aria-label="微信公众号草稿箱">
    <header>
      <strong>微信公众号 · 素材与草稿箱</strong>
      <small>仅人工触发上传与新建草稿；不会发布、群发或定时发送</small>
    </header>
    <p>本机凭据：{{ configured }}。要求公众号开通草稿及素材 API 权限，微信后台 IP 白名单应包含本机出口 IP。</p>
    <p class="storage-status" role="status" aria-live="polite">
      {{loadingDraft ? "正在恢复本机工作草稿…" : storageNotice}}
      <span v-if="lastSavedAt"> · 最近保存 {{new Date(lastSavedAt).toLocaleString("zh-CN",{hour12:false})}}</span>
    </p>
    <div v-if="!draftStorageAvailable && !loadingDraft" class="storage-warning" role="alert">
      系统密钥库不可用，当前草稿不能安全持久化，因此禁止新建公众号草稿。
      <button type="button" :disabled="busy" @click="retryStorage">重试恢复密钥库</button>
    </div>

    <form @submit.prevent="saveAccount" class="form-row">
      <label>AppID <input v-model.trim="appId" required maxlength="64" autocomplete="off" placeholder="wx..." /></label>
      <label>AppSecret <input v-model="appSecret" type="password" required maxlength="512" autocomplete="new-password" placeholder="仅写入系统密钥库" /></label>
      <button type="submit" :disabled="busy || loadingDraft">保存账户</button>
      <button type="button" :disabled="busy" @click="refresh">刷新配置</button>
      <button type="button" :disabled="busy" @click="checkPermission">验证草稿读取权限</button>
    </form>

    <div class="form-row">
      <label>封面 JPG（不超过 64 KB）<input v-model="coverPath" :disabled="loadingDraft" placeholder="/home/.../cover.jpg" aria-label="封面绝对路径" /></label>
      <button type="button" :disabled="busy || loadingDraft" @click="chooseCover">选择封面文件</button>
      <button type="button" :disabled="busy || loadingDraft || !coverPath.trim()" @click="uploadCover">上传永久封面</button>
      <span>封面 media_id：{{ coverMediaId || "未上传" }}</span>
    </div>

    <div class="form-row">
      <label>正文 JPG/PNG（不超过 1 MB）<input v-model="bodyImagePath" :disabled="loadingDraft" placeholder="/home/.../body.png" aria-label="正文绝对路径" /></label>
      <button type="button" :disabled="busy || loadingDraft" @click="chooseBodyImage">选择正文图片</button>
      <button type="button" :disabled="busy || loadingDraft || !bodyImagePath.trim()" @click="uploadBodyImage">上传正文图片</button>
      <button type="button" :disabled="!bodyImageUrl || loadingDraft" @click="insertBodyImage">插入正文</button>
    </div>

    <form class="draft-form" @submit.prevent="createDraft">
      <div class="form-row">
        <label>标题 <input v-model="title" :disabled="loadingDraft" required maxlength="32" /></label>
        <label>作者 <input v-model="author" :disabled="loadingDraft" maxlength="16" /></label>
        <label>摘要 <input v-model="digest" :disabled="loadingDraft" maxlength="120" /></label>
      </div>
      <label>草稿 HTML 正文<textarea v-model="contentHtml" :disabled="loadingDraft" required rows="7" maxlength="19999" spellcheck="false" placeholder="仅发送用户明确填写的 HTML 内容" /></label>
      <div class="form-row">
        <button type="submit" :disabled="busy || loadingDraft || !draftStorageAvailable || outcomeUnknown || !title.trim() || !contentHtml.trim() || !coverMediaId">
          创建到草稿箱（不发布）
        </button>
        <span>提交前会先将「禁止盲目重试」状态保存到系统密钥库。</span>
      </div>
    </form>

    <div v-if="draftMediaId" class="form-row">
      <span>草稿 media_id：{{ draftMediaId }} · {{ verified ? "API 验证成功" : "待验证" }}</span>
      <button type="button" :disabled="busy" @click="verifyDraft">验证草稿详情</button>
    </div>
    <div v-else-if="outcomeUnknown" class="form-row">
      <strong>上次提交结果不明，请先到公众号后台核实。</strong>
      <button type="button" :disabled="busy" @click="confirmResolvedUnknown">人工核实无重复草稿后解除禁止重试</button>
    </div>

    <div class="form-row">
      <button type="button" class="clear-draft" :disabled="busy || loadingDraft || !draftStorageAvailable" @click="clearWorkingDraft">清除本机工作草稿</button>
      <button type="button" class="clear-draft" :disabled="busy || loadingDraft || !draftStorageAvailable" @click="retryStorage">立即保存当前草稿</button>
    </div>
    <p role="status" aria-live="polite">{{ busy ? "请求中…" : notice }}</p>
    <p>请在微信公众号管理后台草稿箱复核封面、正文图、权限及素材。自动保存只保存在系统密钥库，不会保存 AppSecret 到草稿记录。</p>
  </section>
</template>

<style scoped>
.wechat-draft{display:flex;flex-direction:column;gap:12px;width:100%;min-width:0;font-size:13px;line-height:1.5;color:var(--chrome-text,#3c4043)}
header{display:flex;gap:12px;align-items:baseline;flex-wrap:wrap}small{color:var(--chrome-muted,#70757a)}
.form-row{display:flex;gap:12px;align-items:flex-end;flex-wrap:wrap}
.form-row label{flex:1;min-width:180px}
label{display:flex;flex-direction:column;gap:5px}
input,textarea,button{font:inherit;padding:8px;border:1px solid var(--chrome-border,#dadce0);border-radius:6px;background:var(--chrome-address,#f1f3f4);color:inherit}
input,textarea{width:100%;box-sizing:border-box}textarea{resize:vertical;min-height:150px}button{cursor:pointer}button:hover:not(:disabled){border-color:var(--home-accent,#2b6cb0)}
button:focus-visible,input:focus-visible,textarea:focus-visible{outline:2px solid var(--home-accent,#2b6cb0);outline-offset:2px}
button:disabled{opacity:.65;cursor:default}
.draft-form{display:flex;flex-direction:column;gap:12px}p{margin:0;color:var(--text-secondary,#65768c)}
.storage-status{padding:8px 10px;border-radius:7px;background:var(--chrome-address,#f1f3f4);font-size:11px}
.storage-warning{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:10px;border:1px solid #b85e66;border-radius:7px;color:#8f3038;font-size:12px}
.clear-draft{background:transparent}
</style>
