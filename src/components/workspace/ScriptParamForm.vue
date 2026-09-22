<script setup lang="ts">
import { EmptyState } from "../../shared/ui";
import type { ScriptParam, ParamType } from "../../types";

const props = defineProps<{ params: ScriptParam[] }>();
const emit = defineEmits<{ "update:params": [ScriptParam[]] }>();

const PARAM_TYPES: ParamType[] = ["string", "int", "bool", "enum", "path"];

function update(idx: number, patch: Partial<ScriptParam>) {
  const next = props.params.map((p, i) => (i === idx ? { ...p, ...patch } : p));
  emit("update:params", next);
}
function addParam() {
  emit("update:params", [
    ...props.params,
    { name: "", label: "", param_type: "string", required: false, default: null, options: [], raw: false, secret: false },
  ]);
}
function removeParam(idx: number) {
  emit("update:params", props.params.filter((_, i) => i !== idx));
}
function setOptions(idx: number, text: string) {
  const opts = text
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);
  update(idx, { options: opts });
}
</script>

<template>
  <div class="param-form">
    <div class="param-head">
      <span>参数（{{ params.length }}）</span>
      <button type="button" @click="addParam">+ 添加参数</button>
    </div>
    <!-- secret 默认值仅以 password 输入框回填，绝不明文插值显示 -->
    <div v-for="(p, idx) in params" :key="idx" class="param-row">
      <div class="param-grid">
        <label>占位符名
          <input :value="p.name" @input="update(idx, { name: ($event.target as HTMLInputElement).value })" placeholder="如 env" />
        </label>
        <label>显示名
          <input :value="p.label" @input="update(idx, { label: ($event.target as HTMLInputElement).value })" placeholder="如 环境" />
        </label>
        <label>类型
          <select :value="p.param_type" @change="update(idx, { param_type: (($event.target as HTMLSelectElement).value) as ParamType })">
            <option v-for="t in PARAM_TYPES" :key="t" :value="t">{{ t }}</option>
          </select>
        </label>
        <label>默认值
          <input
            v-if="!p.secret"
            :value="p.default ?? ''"
            @input="update(idx, { default: ($event.target as HTMLInputElement).value || null })"
            placeholder="可选"
          />
          <input
            v-else
            type="password"
            :value="p.default ?? ''"
            @input="update(idx, { default: ($event.target as HTMLInputElement).value || null })"
            placeholder="敏感值不回显"
            autocomplete="new-password"
          />
        </label>
      </div>
      <div class="param-flags">
        <label><input type="checkbox" :checked="p.required" @change="update(idx, { required: ($event.target as HTMLInputElement).checked })" /> 必填</label>
        <label><input type="checkbox" :checked="p.secret" @change="update(idx, { secret: ($event.target as HTMLInputElement).checked })" /> 敏感(secret)</label>
      </div>
      <label v-if="p.param_type === 'enum'" class="options-label">选项（每行一个）
        <textarea :value="p.options.join('\n')" @input="setOptions(idx, ($event.target as HTMLTextAreaElement).value)" rows="3" placeholder="prod&#10;test"></textarea>
      </label>
      <button type="button" class="del" @click="removeParam(idx)">删除该参数</button>
    </div>
    <EmptyState v-if="!params.length" as="p" text="暂无参数" />
  </div>
</template>

<style scoped>
.param-form { border: 1px solid #e3e7f5; border-radius: 6px; padding: 8px; margin-top: 8px; }
.param-head { display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: #4e5969; margin-bottom: 6px; }
.param-head button { border: 1px solid #2b6cb0; background: #fff; color: #2b6cb0; border-radius: 4px; padding: 2px 8px; cursor: pointer; font-size: 11px; }
.param-row { border-top: 1px dashed #e3e7f5; padding-top: 6px; margin-top: 6px; }
.param-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 8px; }
.param-grid label { font-size: 11px; color: #4e5969; display: flex; flex-direction: column; }
.param-grid input, .param-grid select { border: 1px solid #d5dbe7; border-radius: 4px; padding: 3px 5px; font-size: 11px; }
.param-flags { display: flex; gap: 12px; margin: 4px 0; font-size: 11px; color: #4e5969; }
.options-label { display: block; font-size: 11px; color: #4e5969; margin-top: 4px; }
.options-label textarea { width: 100%; box-sizing: border-box; border: 1px solid #d5dbe7; border-radius: 4px; font-size: 11px; }
.del { border: none; background: transparent; color: #c33; cursor: pointer; font-size: 11px; margin-top: 4px; }
.empty { color: #86909c; font-size: 11px; }
</style>
