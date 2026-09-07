<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { usePluginStore } from "../../stores/usePluginStore";
import type { PluginState } from "../../types";
import {
  PLUGIN_STATES,
  aclLevelClass,
  aclLevelLabel,
  isDangerousGate,
  pluginStateLabel,
  redactDetail,
} from "../../utils/pluginUi";

const store = usePluginStore();

// 选中插件详情 → 强制走脱敏投影（即使后端将来误带敏感字段，UI 也不渲染）。
const display = computed(() => (store.detail ? redactDetail(store.detail) : null));

const filterOptions = computed(() => [
  { value: null as PluginState | null, label: "全部状态" },
  ...PLUGIN_STATES.map((s) => ({ value: s, label: pluginStateLabel(s) })),
]);

// 受信任密钥表单（瞬时输入；pubkey 不回显、不持久化）
const keyForm = reactive({ keyId: "", pubkey: "", note: "" });

// 显式确认：状态变更前必须先确认（W14 Hard Stop：状态变更需显式确认）
type ConfirmKind = "install" | "enable" | "disable" | "addKey" | "removeKey";
const confirm = ref<{ kind: ConfirmKind; id?: string; keyId?: string } | null>(null);
const confirmText = computed(() => {
  switch (confirm.value?.kind) {
    case "install":
      return "确认安装该校验 manifest？仅做本地元数据校验，不下载、不执行。";
    case "enable":
      return "确认启用该插件？启用不等于运行，仅切换生命周期状态。";
    case "disable":
      return "确认禁用该插件？";
    case "uninstall":
      return "确认卸载该插件？";
    case "addKey":
      return "确认登记该受信任密钥指纹？";
    case "removeKey":
      return "确认移除该受信任密钥？";
    default:
      return "";
  }
});

function askInstall() {
  if (!store.manifestText.trim()) {
    store.error = "请粘贴 manifest JSON";
    return;
  }
  confirm.value = { kind: "install" };
}
function askEnable() {
  if (display.value) confirm.value = { kind: "enable", id: display.value.id };
}
function askDisable() {
  if (display.value) confirm.value = { kind: "disable", id: display.value.id };
}
function askAddKey() {
  if (!keyForm.keyId.trim() || !keyForm.pubkey.trim()) {
    store.error = "keyId 与 pubkey 均必填";
    return;
  }
  confirm.value = { kind: "addKey" };
}
function askRemoveKey(keyId: string) {
  confirm.value = { kind: "removeKey", keyId };
}

async function doConfirm() {
  const c = confirm.value;
  confirm.value = null;
  if (!c) return;
  switch (c.kind) {
    case "install":
      await store.install();
      break;
    case "enable":
      if (c.id) await store.enable(c.id);
      break;
    case "disable":
      if (c.id) await store.disable(c.id);
      break;
    case "addKey":
      await store.addKey(keyForm.keyId, keyForm.pubkey, keyForm.note);
      keyForm.keyId = "";
      keyForm.pubkey = "";
      keyForm.note = "";
      break;
    case "removeKey":
      if (c.keyId) await store.removeKey(c.keyId);
      break;
  }
}

onMounted(() => {
  void store.refreshList();
  void store.refreshKeys();
});
</script>

<template>
  <div class="pm">
    <!-- 左：列表 + 安装 + 密钥 -->
    <section class="pm-left">
      <header class="pm-head">
        <h3>插件管理器</h3>
        <button class="pm-btn" :disabled="store.busy" @click="store.refreshList()">刷新</button>
      </header>

      <label class="pm-filter">
        状态筛选：
        <select v-model="store.filterState" @change="store.refreshList()">
          <option v-for="o in filterOptions" :key="String(o.value)" :value="o.value">
            {{ o.label }}
          </option>
        </select>
      </label>

      <ul class="pm-list">
        <li
          v-for="p in store.list"
          :key="p.id"
          :class="{ active: store.detail?.id === p.id }"
          @click="store.selectFromList(p.id)"
        >
          <div class="pm-row1">
            <span class="pm-name">{{ p.display_name || p.id }}</span>
            <span class="pm-state">{{ pluginStateLabel(p.state) }}</span>
          </div>
          <div class="pm-row2">
            <span class="pm-id">{{ p.id }}</span>
            <span class="pm-hash">#{{ p.hash_prefix }}</span>
            <span class="pm-cap">能力×{{ p.capability_count }}</span>
          </div>
        </li>
        <li v-if="!store.list.length" class="pm-empty">暂无插件（点击"安装"校验一个 manifest）</li>
      </ul>

      <!-- 安装表单 -->
      <div class="pm-install">
        <div class="pm-sub">安装（本地 manifest 校验）</div>
        <textarea
          v-model="store.manifestText"
          placeholder="粘贴插件 manifest JSON（含 id/version/entry/capabilities/hash/signature）"
          rows="6"
        ></textarea>
        <input v-model="store.resourcePath" placeholder="资源包路径（可选）" />
        <button class="pm-btn primary" :disabled="store.busy" @click="askInstall">安装（校验）</button>
      </div>

      <!-- 受信任密钥 -->
      <div class="pm-keys">
        <div class="pm-sub">受信任密钥（仅 16-hex 指纹）</div>
        <ul>
          <li v-for="k in store.keys" :key="k.key_id">
            <span class="pm-keyid">{{ k.key_id }}</span>
            <span class="pm-fp">{{ k.fingerprint }}</span>
            <button class="pm-btn danger" @click="askRemoveKey(k.key_id)">移除</button>
          </li>
          <li v-if="!store.keys.length" class="pm-empty">暂无登记密钥</li>
        </ul>
        <input v-model="keyForm.keyId" placeholder="keyId" />
        <input v-model="keyForm.pubkey" placeholder="pubkey（仅用于登记，不回显）" />
        <input v-model="keyForm.note" placeholder="备注（可选）" />
        <button class="pm-btn" :disabled="store.busy" @click="askAddKey">登记密钥</button>
      </div>
    </section>

    <!-- 右：详情（仅脱敏字段） -->
    <section class="pm-right">
      <template v-if="display">
        <h3>{{ display.display_name }}</h3>
        <dl class="pm-dl">
          <dt>ID</dt><dd>{{ display.id }}</dd>
          <dt>版本</dt><dd>{{ display.version }}</dd>
          <dt>状态</dt><dd>{{ pluginStateLabel(display.state) }}</dd>
          <dt>最小应用版本</dt><dd>{{ display.min_app_version }}</dd>
          <dt>描述</dt><dd>{{ display.description }}</dd>
          <dt>Hash 前缀</dt><dd>{{ display.hash_prefix }}</dd>
          <dt>安装时间</dt><dd>{{ display.installed_at }}</dd>
          <dt>更新时间</dt><dd>{{ display.updated_at }}</dd>
        </dl>

        <div class="pm-sub">签名（无原文）</div>
        <ul class="pm-sig">
          <li>算法：{{ display.signature.algorithm }}</li>
          <li>keyId：{{ display.signature.key_id }}</li>
          <li>结构状态：{{ display.signature.status }}</li>
        </ul>

        <div class="pm-sub">资源包（无绝对路径）</div>
        <ul class="pm-sig">
          <li>声明 Hash：{{ display.resource.declared_hash }}</li>
          <li>已提供路径：{{ display.resource.path_provided ? "是" : "否" }}</li>
          <li>已校验：{{ display.resource.verified ? "是" : "否" }}</li>
        </ul>

        <div class="pm-sub">能力（逐项展示，禁折叠）</div>
        <ul class="pm-caps">
          <li v-for="(c, i) in display.capabilities" :key="i">
            <span class="pm-cap-name">{{ c.capability }}</span>
            <span class="pm-cap-reason">{{ c.reason }}</span>
            <span :class="['pm-acl', aclLevelClass(c.acl_level)]">
              {{ aclLevelLabel(c.acl_level) }}
              <template v-if="isDangerousGate(c.acl_level)">（白名单空，已阻止）</template>
            </span>
          </li>
          <li v-if="!display.capabilities.length" class="pm-empty">无声明能力（安全）</li>
        </ul>

        <div class="pm-actions">
          <button class="pm-btn" :disabled="!store.actionsFor.enable || store.busy" @click="askEnable">
            启用
          </button>
          <button class="pm-btn" :disabled="!store.actionsFor.disable || store.busy" @click="askDisable">
            禁用
          </button>
        </div>
      </template>
      <p v-else class="pm-hint">从左侧选择插件查看详情。</p>

      <p v-if="store.error" class="pm-error">{{ store.error }}</p>
    </section>

    <!-- 显式确认弹层 -->
    <div v-if="confirm" class="pm-modal" @click.self="confirm = null">
      <div class="pm-modal-box">
        <p>{{ confirmText }}</p>
        <div class="pm-modal-actions">
          <button class="pm-btn" @click="confirm = null">取消</button>
          <button class="pm-btn primary" @click="doConfirm">确认</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pm {
  display: flex;
  gap: 12px;
  height: 100%;
  padding: 10px;
  box-sizing: border-box;
}
.pm-left,
.pm-right {
  flex: 1 1 0;
  min-width: 0;
  overflow: auto;
  border: 1px solid #e3e7f5;
  border-radius: 8px;
  padding: 10px;
  background: #fff;
}
.pm-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.pm-sub {
  font-size: 12px;
  color: #86909c;
  margin: 10px 0 4px;
}
.pm-list,
.pm-keys ul,
.pm-sig,
.pm-caps {
  list-style: none;
  margin: 0;
  padding: 0;
}
.pm-list li {
  border: 1px solid #eef1f7;
  border-radius: 6px;
  padding: 6px 8px;
  margin-bottom: 6px;
  cursor: pointer;
}
.pm-list li.active {
  border-color: #2b6cb0;
  background: #f0f6ff;
}
.pm-row1 {
  display: flex;
  justify-content: space-between;
}
.pm-name {
  font-weight: 600;
}
.pm-state {
  font-size: 11px;
  color: #2b6cb0;
}
.pm-row2 {
  display: flex;
  gap: 8px;
  font-size: 11px;
  color: #86909c;
}
.pm-empty {
  color: #a9b0bd;
  font-size: 12px;
  padding: 6px 2px;
}
.pm-install textarea,
.pm-install input,
.pm-keys input {
  width: 100%;
  box-sizing: border-box;
  margin-bottom: 6px;
  border: 1px solid #d5dbe7;
  border-radius: 5px;
  padding: 6px;
  font-size: 12px;
  font-family: monospace;
}
.pm-dl {
  display: grid;
  grid-template-columns: 90px 1fr;
  gap: 4px 8px;
  font-size: 12px;
  margin: 0;
}
.pm-dl dt {
  color: #86909c;
}
.pm-dl dd {
  margin: 0;
  word-break: break-all;
}
.pm-sig li,
.pm-caps li {
  font-size: 12px;
  padding: 3px 0;
  border-bottom: 1px dashed #eef1f7;
}
.pm-caps li {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.pm-cap-name {
  font-weight: 600;
}
.pm-acl {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 4px;
}
.pm-acl.ok {
  background: #e8f7ee;
  color: #1a7f4b;
}
.pm-acl.warn {
  background: #fff5e6;
  color: #b9770e;
}
.pm-acl.danger {
  background: #fdecec;
  color: #c0392b;
}
.pm-actions {
  margin-top: 10px;
  display: flex;
  gap: 8px;
}
.pm-btn {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 5px;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
}
.pm-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.pm-btn.primary {
  background: #2b6cb0;
  border-color: #2b6cb0;
  color: #fff;
}
.pm-btn.danger {
  color: #c0392b;
  border-color: #f0c5c0;
}
.pm-error {
  color: #c0392b;
  font-size: 12px;
  margin-top: 8px;
}
.pm-hint {
  color: #a9b0bd;
}
.pm-modal {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.25);
  display: flex;
  align-items: center;
  justify-content: center;
}
.pm-modal-box {
  background: #fff;
  border-radius: 8px;
  padding: 16px;
  max-width: 360px;
}
.pm-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
</style>
