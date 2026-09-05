<script setup lang="ts">
// src/components/workspace/TaskEditDialog.vue
// M4-8 定时任务新增/编辑表单（纯前端态，保存走 store → bridge）。
//
// 契约落点：
//   - 默认「未启用」（A6 裁定 R-A6-1），启用是显式动作；
//   - cron 只接受 5 段；6 段（含秒）在**此处就显式拒绝并说明**，
//     避免用户从 cron 种子工具复制 6 位表达式后到后端才被拒（A6 O-A6-4）；
//   - 敏感参数输入框禁用：secret 参数值不落 tasks.json（A6 §3.3 R-3）；
//   - catch_up_limit 仅 CatchUp 策略下可编辑（A6 §3.2，防「看似可配实则 no-op」）。
import { computed } from "vue";
import { useTaskStore } from "../../stores/useTaskStore";
import {
  CRON_FIELD_COUNT,
  MAX_ATTEMPTS,
  MAX_CATCH_UP_LIMIT,
  MAX_INTERVAL_SECS,
  MAX_TIMEOUT_SECS,
  MIN_INTERVAL_SECS,
  isCatchUpLimitEditable,
  missedPolicyLabel,
} from "../../utils/taskUi";

const tasks = useTaskStore();

const targetsOfKind = computed(() => tasks.targets.filter((t) => t.kind === tasks.draft.kind));

const selectedTarget = computed(
  () => targetsOfKind.value.find((t) => t.id === tasks.draft.target_id) ?? null,
);

// 6 段（含秒）显式提示：与 cron 种子工具（M2-7）的 6 位模式不兼容。
const cronParts = computed(() => tasks.draft.cron_expr.trim().split(/\s+/).filter(Boolean));
const cronHasWrongFieldCount = computed(
  () => tasks.draft.trigger_kind === "cron" && cronParts.value.length !== CRON_FIELD_COUNT,
);

const catchUpEditable = computed(() => isCatchUpLimitEditable(tasks.draft.missed_run_policy));

function onKindChange() {
  // 换类型后原目标 id 对新类型无意义，必须清空，避免提交一个类型不匹配的目标。
  tasks.draft.target_id = "";
  tasks.draft.params = {};
}

function onTargetChange() {
  tasks.draft.params = {};
}
</script>

<template>
  <div v-if="tasks.editorOpen" class="modal-mask" @click.self="tasks.closeEditor()">
    <div class="modal task-editor">
      <h3>{{ tasks.draft.id ? "编辑定时任务" : "新建定时任务" }}</h3>

      <label class="field">
        <span>任务名</span>
        <input v-model="tasks.draft.name" placeholder="例如：每晚清理日志" />
        <em v-if="tasks.issueFor('name')" class="err">{{ tasks.issueFor("name") }}</em>
      </label>

      <div class="row">
        <label class="field">
          <span>类型</span>
          <select v-model="tasks.draft.kind" @change="onKindChange">
            <option value="script">脚本</option>
            <option value="command">命令片段</option>
          </select>
        </label>
        <label class="field grow">
          <span>执行目标</span>
          <select v-model="tasks.draft.target_id" @change="onTargetChange">
            <option value="">— 请选择 —</option>
            <option v-for="t in targetsOfKind" :key="t.id" :value="t.id">
              {{ t.name }}{{ t.dangerous ? "（高风险）" : "" }}
            </option>
          </select>
          <em v-if="tasks.issueFor('target_id')" class="err">{{ tasks.issueFor("target_id") }}</em>
        </label>
      </div>

      <div v-if="selectedTarget && selectedTarget.params.length > 0" class="params">
        <div class="params-title">参数值（敏感参数不接受保存）</div>
        <label v-for="p in selectedTarget.params" :key="p.name" class="field inline">
          <span>{{ p.label || p.name }}{{ p.required ? " *" : "" }}</span>
          <input
            v-if="!p.secret"
            v-model="tasks.draft.params[p.name]"
            :placeholder="p.default ?? ''"
          />
          <input v-else disabled value="（敏感参数，不在任务中保存）" />
        </label>
        <em v-if="tasks.issueFor('params')" class="err">{{ tasks.issueFor("params") }}</em>
      </div>

      <div class="row">
        <label class="field">
          <span>触发方式</span>
          <select v-model="tasks.draft.trigger_kind">
            <option value="cron">Cron 表达式</option>
            <option value="interval">固定间隔</option>
          </select>
        </label>
        <label v-if="tasks.draft.trigger_kind === 'cron'" class="field grow">
          <span>Cron（{{ CRON_FIELD_COUNT }} 段：分 时 日 月 周）</span>
          <input v-model="tasks.draft.cron_expr" placeholder="*/15 * * * *" />
          <em v-if="cronHasWrongFieldCount" class="warn">
            需要 {{ CRON_FIELD_COUNT }} 段，当前 {{ cronParts.length }} 段；含秒的 6 位表达式不被支持
          </em>
          <em v-if="tasks.issueFor('cron_expr')" class="err">{{ tasks.issueFor("cron_expr") }}</em>
        </label>
        <label v-else class="field grow">
          <span>间隔秒数（{{ MIN_INTERVAL_SECS }} ~ {{ MAX_INTERVAL_SECS }}）</span>
          <input v-model.number="tasks.draft.interval_secs" type="number" :min="MIN_INTERVAL_SECS" :max="MAX_INTERVAL_SECS" />
          <em v-if="tasks.issueFor('interval_secs')" class="err">{{ tasks.issueFor("interval_secs") }}</em>
        </label>
      </div>

      <div class="row">
        <label class="field">
          <span>错过执行策略</span>
          <select v-model="tasks.draft.missed_run_policy">
            <option value="skip">{{ missedPolicyLabel("skip") }}</option>
            <option value="run_once">{{ missedPolicyLabel("run_once") }}</option>
            <option value="catch_up">{{ missedPolicyLabel("catch_up") }}</option>
          </select>
        </label>
        <label class="field">
          <span>补跑上限（1~{{ MAX_CATCH_UP_LIMIT }}）</span>
          <input
            v-model.number="tasks.draft.catch_up_limit"
            type="number"
            :min="1"
            :max="MAX_CATCH_UP_LIMIT"
            :disabled="!catchUpEditable"
          />
          <em v-if="!catchUpEditable" class="hint">仅「按上限补跑」策略生效</em>
        </label>
      </div>

      <div class="row">
        <label class="field">
          <span>重试次数（1~{{ MAX_ATTEMPTS }}，1 = 不重试）</span>
          <input v-model.number="tasks.draft.retry_max_attempts" type="number" :min="1" :max="MAX_ATTEMPTS" />
        </label>
        <label class="field">
          <span>退避</span>
          <select v-model="tasks.draft.retry_backoff">
            <option value="fixed">固定间隔</option>
            <option value="exponential">指数退避</option>
          </select>
        </label>
        <label class="field">
          <span>基准延迟（秒）</span>
          <input v-model.number="tasks.draft.retry_base_delay_secs" type="number" :min="0" />
        </label>
        <label class="field">
          <span>最大延迟（秒）</span>
          <input v-model.number="tasks.draft.retry_max_delay_secs" type="number" :min="0" :max="MAX_TIMEOUT_SECS" />
        </label>
      </div>

      <div class="row">
        <label class="field">
          <span>超时（秒，0 = 后端默认 60，上限 {{ MAX_TIMEOUT_SECS }}）</span>
          <input v-model.number="tasks.draft.timeout_secs" type="number" :min="0" :max="MAX_TIMEOUT_SECS" />
        </label>
        <label class="field">
          <span>迟到宽限（秒）</span>
          <input v-model.number="tasks.draft.misfire_grace_secs" type="number" :min="0" />
        </label>
      </div>

      <label class="switch">
        <input v-model="tasks.draft.enabled" type="checkbox" />
        <span>启用（默认关闭：启用后才会无人值守自动执行）</span>
      </label>

      <div v-if="tasks.issues.length > 0" class="issues">
        <div v-for="(i, idx) in tasks.issues" :key="idx" class="err">· {{ i.message }}</div>
      </div>
      <div v-if="tasks.error" class="err">{{ tasks.error }}</div>

      <div class="actions">
        <button :disabled="tasks.loading" @click="tasks.closeEditor()">取消</button>
        <button class="primary" :disabled="tasks.loading" @click="tasks.saveDraft()">保存</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.task-editor {
  width: 720px;
  max-width: 92vw;
  max-height: 86vh;
  overflow: auto;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 8px 0;
}
.field.inline {
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
.field.inline span {
  min-width: 120px;
}
.row {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}
.grow {
  flex: 1;
  min-width: 220px;
}
.params {
  border: 1px solid var(--line, #2a2f3a);
  border-radius: 6px;
  padding: 8px 10px;
  margin: 8px 0;
}
.params-title {
  font-size: 12px;
  opacity: 0.75;
  margin-bottom: 6px;
}
.err {
  color: #ff6b6b;
  font-size: 12px;
}
.warn {
  color: #ffb454;
  font-size: 12px;
}
.hint {
  opacity: 0.6;
  font-size: 12px;
}
.switch {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 10px 0;
}
.issues {
  margin: 8px 0;
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
}
</style>
