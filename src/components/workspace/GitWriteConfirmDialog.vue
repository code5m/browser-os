<script setup lang="ts">
import { computed } from "vue";
import {
  useGitStore,
  GIT_OP_LABEL,
  GIT_RISK_LABEL,
  PUSH_NOTICE,
  DISCARD_NOTICE,
  GIT_PREVIEW_SHOWN_PATHS,
} from "../../stores/useGitStore";

// 写操作确认闸门（阶段二入口）。
// 只展示后端 request_git_write 返回的 preview（summary / affected_paths /
// path_count / dangerous / expires_at），再叠加前端派生的风险等级；
// 不展示、也不持有任何凭据。dangerous 操作未勾选二次确认时，
// 「确认执行」按钮禁用，且 store 侧还会再拦一次（双保险）。
const git = useGitStore();

const pv = computed(() => git.preview);
const risk = computed(() => git.previewRisk ?? "low");
const shownPaths = computed(() => pv.value?.affected_paths ?? []);
const hiddenCount = computed(() =>
  Math.max(0, (pv.value?.path_count ?? 0) - shownPaths.value.length)
);
const expiresText = computed(() => {
  const raw = pv.value?.expires_at;
  if (!raw) return "";
  const d = new Date(raw);
  return Number.isNaN(d.getTime()) ? raw : d.toLocaleTimeString();
});
const ackLabel = computed(() => {
  const op = pv.value?.op;
  if (op === "discard") return "我确认丢弃这些改动，且知道改动不可恢复";
  if (op === "push") return "我确认推送当前分支到 origin 同名分支（非 force）";
  return "我确认执行该操作";
});
</script>

<template>
  <div v-if="pv" class="modal-mask" @click.self="!git.busy && git.cancelWrite()">
    <div class="modal git-modal">
      <h3>确认执行「{{ GIT_OP_LABEL[pv.op] }}」？</h3>

      <p class="git-summary">{{ pv.summary }}</p>

      <div class="git-meta">
        <span class="gbadge" :class="risk">风险：{{ GIT_RISK_LABEL[risk] }}</span>
        <span v-if="pv.dangerous" class="gbadge high">需二次确认</span>
        <span class="git-hint">任务 {{ pv.job_id.slice(0, 8) }}… 有效期至 {{ expiresText }}</span>
      </div>

      <div v-if="pv.op === 'push'" class="notice">{{ PUSH_NOTICE }}</div>
      <div v-if="pv.op === 'discard'" class="notice danger">{{ DISCARD_NOTICE }}</div>

      <div v-if="shownPaths.length" class="git-paths-block">
        <div class="git-hint">
          影响 {{ pv.path_count }} 个路径<span v-if="hiddenCount">
            （仅显示前 {{ GIT_PREVIEW_SHOWN_PATHS }} 条，其余 {{ hiddenCount }} 条已省略）</span
          >：
        </div>
        <ul class="paths">
          <li v-for="p in shownPaths" :key="p">· {{ p }}</li>
        </ul>
      </div>
      <div v-else class="git-hint">该操作不影响具体文件路径。</div>

      <label v-if="pv.dangerous" class="ack">
        <input v-model="git.dangerousAck" type="checkbox" :disabled="git.busy" />
        <span>{{ ackLabel }}</span>
      </label>

      <div v-if="git.writeError" class="git-error">{{ git.writeError }}</div>
      <div v-if="git.busy" class="git-hint">执行中…完成后会自动刷新状态/diff/分支。</div>

      <div class="actions">
        <button :disabled="git.busy" @click="git.cancelWrite()">取消</button>
        <button class="primary" :disabled="git.busy || !git.canConfirm" @click="git.confirmWrite()">
          {{ git.busy ? "执行中…" : "确认执行" }}
        </button>
      </div>
    </div>
  </div>
</template>
