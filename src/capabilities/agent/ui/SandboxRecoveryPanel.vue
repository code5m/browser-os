<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { bridge } from "../../../bridge";
import { sandboxRecoveryHint } from "../model/codingLsp.mjs";

type RecoveryRecord = { recordId: string | null; phase: string; code: string; retryable: boolean };
type RecoveryStatus = {
  state: string; canStart: boolean; retryAvailable: boolean; lastCode: string | null;
  lastReport: { recovered: number; active: number; pending: number; rejected: number };
  pendingCleanup: number; records: RecoveryRecord[]; truncated: boolean;
};
const props = defineProps<{ root: string }>();
const emit = defineEmits<{ availability: [available: boolean] }>();
const status = ref<RecoveryStatus | null>(null);
const pending = ref(false);
const failure = ref("");
let epoch = 0;
let disposed = false;
const visible = computed(() => !!props.root && (failure.value || (status.value && status.value.state !== "unsupported")));
const hint = computed(() => sandboxRecoveryHint(status.value));
const needsAttention = computed(() => !!failure.value || (status.value && !status.value.canStart));

async function refresh(retry = false) {
  if (!props.root || pending.value) return;
  const scope = epoch;
  const root = props.root;
  pending.value = true;
  failure.value = "";
  try {
    // A retry can only resume the already-installed native journal. Neither
    // paths nor record IDs are accepted as recovery/delete targets.
    const result = await bridge.codingService({
      root, action: retry ? "sandbox_recovery_retry" : "sandbox_recovery_status",
      ...(retry ? { approved: true } : {}),
    });
    if (disposed || scope !== epoch || root !== props.root) return;
    if (!result || typeof result.state !== "string" || typeof result.canStart !== "boolean"
        || typeof result.retryAvailable !== "boolean" || !Array.isArray(result.records)) {
      throw Error("SANDBOX_RECOVERY_STATUS_INVALID");
    }
    status.value = { ...result, records: result.records.slice(0, 64) };
    emit("availability", result.canStart);
  } catch {
    if (disposed || scope !== epoch) return;
    failure.value = "恢复状态暂时无法读取，请重新检查。";
    // Do not turn a failed read into a health approval.
    emit("availability", false);
  } finally {
    if (!disposed && scope === epoch) pending.value = false;
  }
}
watch(() => props.root, () => {
  ++epoch;
  pending.value = false;
  status.value = null;
  failure.value = "";
  emit("availability", false);
  void refresh();
}, { immediate: true });
onBeforeUnmount(() => { disposed = true; ++epoch; });
</script>

<template>
  <section v-if="visible" class="sandbox-recovery" :class="{ attention: needsAttention }" aria-label="隔离执行恢复状态">
    <div class="recovery-summary">
      <span role="status">{{ failure || hint }}</span>
      <button type="button" :disabled="pending" @click="refresh()">{{ pending ? "处理中…" : "重新检查" }}</button>
      <button v-if="status?.retryAvailable" type="button" :disabled="pending" @click="refresh(true)">重试安全恢复</button>
    </div>
    <details v-if="status && (status.records.length || status.lastCode || status.truncated)">
      <summary>查看恢复诊断</summary>
      <p v-if="status.state === 'review_required'">
        有记录尚不能确认资源归属。请保留下面的记录编号和原因码供维护者核对；
        重试仅处理能够证明归属的记录，不会自动删除身份不明的资源。
      </p>
      <p v-else>临时占用或未完成清理可稍后重试；重新检查只读取状态。</p>
      <p v-if="status.lastCode">状态码：<code>{{ status.lastCode }}</code></p>
      <ul>
        <li v-for="(record, index) in status.records" :key="record.recordId || index">
          <code>{{ record.recordId || "无法读取编号" }}</code> · {{ record.phase }} ·
          <code>{{ record.code }}</code> · {{ record.retryable ? "可重试" : "需核对" }}
        </li>
      </ul>
      <p v-if="status.truncated">本次诊断未完整读取所有记录；未列出的记录仍受原生恢复保护。</p>
    </details>
  </section>
</template>

<style scoped>
.sandbox-recovery{flex:0 0 auto;max-height:180px;overflow:auto;padding:7px 12px;border-bottom:1px solid #293143;background:#182332;font-size:12px}
.sandbox-recovery.attention{color:#f0cf9c;background:#30291f}
.recovery-summary{display:flex;align-items:center;gap:8px;flex-wrap:wrap}
.recovery-summary span{flex:1;min-width:180px}
button{font:inherit;padding:3px 7px;color:inherit;background:transparent;border:1px solid #596477;border-radius:4px}
button:disabled{opacity:.5}details{margin-top:6px}summary{cursor:pointer}p{margin:7px 0}ul{padding-left:20px}li{overflow-wrap:anywhere;margin:4px 0}code{font-size:11px}
</style>
