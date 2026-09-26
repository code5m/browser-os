<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { BrowserCredentialItem, AutofillResult } from "../../../types";
import { bridge } from "../../../bridge";
import { useBrowserStore } from "../../../capabilities/browser/public";
import { useLayoutStore } from "../../../stores/useLayoutStore";

// 已导入账号列表 + 用户主动触发的一次性填充。
//
// 安全红线（不可放宽）：
//   - 只展示 站点 / 用户名 / 密码状态；绝不显示、查看、复制、导出密码；
//   - 密码不进入本组件、不进入任何 Vue store、不进 localStorage/日志；
//   - 「填充」只传不透明 credential_id + 页签 id，由 Rust 完成 keyring 读取与注入；
//   - 只有与当前网页 **exact origin** 匹配的账号才出现「填充」；
//   - 不做自动填充、自动提交、自动登录。

const browser = useBrowserStore();
const layout = useLayoutStore();

const items = ref<BrowserCredentialItem[]>([]);
const loading = ref(false);
const error = ref("");
const fillingId = ref("");

// 当前网页 origin（与 Rust 的 credential_origin 同口径：scheme://host[:port]）
const pageOrigin = computed(() => {
  const u = browser.activeTab?.url || browser.url;
  if (!u) return "";
  try {
    return new URL(u).origin;
  } catch {
    return "";
  }
});

function matchesPage(item: BrowserCredentialItem): boolean {
  return !!item.origin && item.origin === pageOrigin.value;
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    items.value = await bridge.listBrowserCredentials();
  } catch (e) {
    // 只透出后端返回的安全原因（后端保证错误信息不含任何凭据内容）
    error.value = "读取已导入账号失败：" + String((e as Error)?.message || e);
    items.value = [];
  } finally {
    loading.value = false;
  }
}

const MESSAGES: Record<string, string> = {
  FILLED: "账号密码已填充，请确认后登录",
  ORIGIN_MISMATCH: "当前网页与该账号不同源，已拒绝填充",
  TAB_NOT_FOUND: "当前页签已关闭，未填充",
  WEBVIEW_NOT_FOUND: "当前网页不可用，未填充",
  NO_PASSWORD_FIELD: "未找到密码输入框，未填充",
  NO_USERNAME_FIELD: "无法确认用户名输入框，未填充",
  AMBIGUOUS_FORM: "页面存在多个密码框，未填充",
  CROSS_ORIGIN_IFRAME_UNSUPPORTED: "登录框位于跨域 iframe，暂不支持",
  KEYRING_READ_FAILED: "读取系统密钥库失败，未填充",
  CREDENTIAL_NOT_FOUND: "账号句柄已失效，请重新打开账号列表",
  FILL_FAILED: "填充失败，未填充",
};

async function fill(item: BrowserCredentialItem) {
  if (!browser.activeTabId) {
    layout.showToast("没有打开的页签，未填充");
    return;
  }
  if (!matchesPage(item)) {
    layout.showToast("当前网页与该账号不同源，未填充");
    return;
  }
  fillingId.value = item.credential_id;
  try {
    const code: AutofillResult = await bridge.fillBrowserCredential(
      item.credential_id,
      browser.activeTabId,
    );
    layout.showToast(MESSAGES[code] || "填充失败，未填充");
  } catch (e) {
    layout.showToast("填充失败：" + String((e as Error)?.message || e));
  } finally {
    fillingId.value = "";
  }
}

onMounted(load);

function hostOf(u: string): string {
  try {
    return new URL(u).hostname;
  } catch {
    return u;
  }
}

// 供父组件在导入成功后刷新列表
defineExpose({ load });
</script>

<template>
  <div class="cred-pane">
    <div class="cred-head">
      <span class="cred-title">🔑 已导入账号</span>
      <span class="cred-count">{{ items.length }}</span>
      <button title="刷新" @click="load">↻</button>
    </div>
    <div v-if="error" class="cred-error">{{ error }}</div>
    <div class="cred-list">
      <template v-if="items.length">
        <div v-for="item in items" :key="item.credential_id" class="cred-item">
          <div class="cred-row">
            <div class="cred-main">
              <div class="cred-host">{{ hostOf(item.url) }}</div>
              <div class="cred-user">{{ item.username }}</div>
            </div>
            <button
              v-if="matchesPage(item)"
              class="cred-fill"
              :disabled="fillingId === item.credential_id"
              title="把该账号填入当前网页的登录表单（不会自动登录）"
              @click="fill(item)"
            >
              {{ fillingId === item.credential_id ? "填充中…" : "填充" }}
            </button>
          </div>
          <div class="cred-state" :class="{ ok: item.has_password }">
            {{ item.has_password ? "✓ 已保存" : "— 未保存" }}
            <span v-if="!matchesPage(item)" class="cred-hint">· 与当前网页不同源</span>
          </div>
        </div>
      </template>
      <div v-else class="cred-empty">
        {{ loading ? "正在读取已导入账号…" : "暂无已导入账号" }}
      </div>
    </div>
    <div class="cred-foot">密码仅保存在系统密钥库，应用内不显示、不复制、不导出</div>
  </div>
</template>

<style scoped>
.cred-pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1;
}
.cred-head {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 6px;
}
.cred-title {
  font-size: 13px;
  font-weight: 600;
  color: #4e5969;
}
.cred-count {
  font-size: 11px;
  color: #86909c;
  margin-right: auto;
}
.cred-head button {
  background: none;
  border: none;
  padding: 4px 6px;
  cursor: pointer;
  font-size: 13px;
  color: #86909c;
}
.cred-head button:hover {
  color: #2b6cb0;
}
.cred-error {
  flex-shrink: 0;
  font-size: 11px;
  color: #c0392b;
  padding: 4px 8px;
  background: #fdecea;
  border-bottom: 1px solid #f5c6c2;
  word-break: break-all;
}
.cred-list {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 4px;
}
.cred-item {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 6px;
  border-radius: 5px;
}
.cred-item:hover {
  background: #eef2f7;
}
.cred-row {
  display: flex;
  align-items: center;
  gap: 4px;
}
.cred-main {
  flex: 1;
  min-width: 0;
}
.cred-host {
  font-size: 12px;
  color: #1d2129;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cred-user {
  font-size: 11px;
  color: #4e5969;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cred-fill {
  flex-shrink: 0;
  font-size: 11px;
  color: #fff;
  background: #2b6cb0;
  border: none;
  border-radius: 4px;
  padding: 3px 8px;
  cursor: pointer;
}
.cred-fill:hover {
  background: #245a94;
}
.cred-fill:disabled {
  background: #a9b6c6;
  cursor: default;
}
.cred-state {
  font-size: 10px;
  color: #86909c;
}
.cred-state.ok {
  color: #2f855a;
}
.cred-hint {
  color: #a9aeb8;
}
.cred-empty {
  font-size: 12px;
  color: #86909c;
  padding: 12px 8px;
  line-height: 1.6;
}
.cred-foot {
  flex-shrink: 0;
  font-size: 10px;
  color: #a9aeb8;
  padding: 6px 8px;
  border-top: 1px solid #eceef1;
  line-height: 1.5;
}
</style>
