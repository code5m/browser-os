<script setup lang="ts">
import { computed, ref } from "vue"
import type { CapabilityRecord } from "../../capability/runtime"
import { CAPABILITY_CATALOG, CAPABILITY_DEFINITIONS } from "../../capability/platform/catalog"
import { contributionRegistry } from "../../capability/contribution/registry"
import { isCapabilityToggleSafe, transitionCapability, type CapabilityManagerAction } from "../../capability/platform/manager"
import { peekCapabilityRuntime } from "../../capability/runtimeSingleton"
import { loadCapabilityConfig, saveCapabilityConfig } from "../../capability/platform/persistence"

const runtime = peekCapabilityRuntime()
const message = ref("")
const refresh = ref(0)
const busy = ref(false)
const stored = loadCapabilityConfig() ?? { enabled: {} }
function isSuspendable(record: CapabilityRecord): boolean {
  return record.definition.lifecycle.supported.includes("SUSPENDED")
    && record.definition.resources?.suspendable === true
}

function hasEnabledDependent(id: string): boolean {
  return runtime?.inspect().some((entry) => entry.id !== id && entry.enabled
    && runtime.get(entry.id)?.definition.dependsOn.includes(id)) ?? false
}

function isFrameworkResident(id: string): boolean {
  return CAPABILITY_DEFINITIONS[id]?.lifecycle.resident === true
}

function stateLabel(manifest: typeof CAPABILITY_CATALOG[string], record: CapabilityRecord | undefined): string {
  if (!record) return isFrameworkResident(manifest.id) ? "FRAMEWORK" : "UNAVAILABLE"
  return record.enabled ? record.state : "DISABLED"
}

function actionLabel(manifest: typeof CAPABILITY_CATALOG[string], record: CapabilityRecord | undefined): string {
  if (!record) return isFrameworkResident(manifest.id) ? "常驻" : "未装配"
  if (!record.enabled) return "启用"
  if (record.state === "ACTIVE") return "暂停"
  if (record.state === "SUSPENDED") return "停用"
  return "启用"
}

function nextAction(record: CapabilityRecord | undefined): CapabilityManagerAction {
  if (!record?.enabled) return "enable"
  if (record.state === "ACTIVE") return "pause"
  if (record.state === "SUSPENDED") return "disable"
  return "enable"
}

function actionDisabled(manifest: typeof CAPABILITY_CATALOG[string], record: CapabilityRecord | undefined, action = nextAction(record)): boolean {
  if (busy.value || !record || manifest.kind === "core" || record.definition.lifecycle.resident || record.definition.governanceStatus !== "GOVERNED") return true
  if ((action === "pause" || action === "disable") && hasEnabledDependent(manifest.id)) return true
  if (!isCapabilityToggleSafe(manifest)) return true
  return record?.enabled === true && record.state === "ACTIVE" && !isSuspendable(record)
}

function unavailableReason(manifest: typeof CAPABILITY_CATALOG[string], record: CapabilityRecord | undefined): string {
  if (!record && isFrameworkResident(manifest.id)) return "框架常驻服务，不参与运行时装卸"
  if (!record) return "当前启动配置未装配此能力"
  if (manifest.kind === "core") return "CORE 不可停用"
  if (record.definition.lifecycle.resident || record.definition.governanceStatus !== "GOVERNED") return "常驻或尚未治理的能力，不支持运行时切换"
  if (hasEnabledDependent(manifest.id)) return "请先停用依赖此能力的模块"
  if (Number(manifest.maturity.slice(1)) < 3) return "尚未达到 C3 组合成熟度，暂不支持安全停用"
  if (!manifest.hotPlug.enable || !manifest.hotPlug.disable) return "未声明完整的运行时启用/停用契约"
  if (record?.enabled === true && record.state === "ACTIVE" && !isSuspendable(record)) return "该能力未声明可暂停，不能安全停用"
  return ""
}

const rows = computed(() => {
  void refresh.value
  return Object.values(CAPABILITY_CATALOG).sort((a, b) => a.id.localeCompare(b.id)).map((manifest) => ({
    manifest,
    record: runtime?.get(manifest.id),
    kindLabel: manifest.kind || (CAPABILITY_DEFINITIONS[manifest.id]?.category === "SERVICE" ? "service" : "feature"),
    dependent: Object.values(CAPABILITY_CATALOG).filter((candidate) => candidate.dependencies.includes(manifest.id)).map((candidate) => candidate.id),
  }))
})

async function transition(id: string, action: CapabilityManagerAction) {
  if (!runtime) { message.value = "Runtime 尚未初始化"; return }
  if (busy.value) return
  busy.value = true
  message.value = `${id} 操作进行中…`
  try {
    const result = await transitionCapability(runtime, contributionRegistry, id, action)
    let saved = true
    if (result.persistedEnabled !== undefined) {
      stored.enabled[id] = result.persistedEnabled
      saved = saveCapabilityConfig(stored)
    }
    message.value = result.message + (saved ? "" : "；配置保存失败，仅对本次会话生效")
  } catch (error) {
    message.value = `操作失败：${error instanceof Error ? error.message : String(error)}`
  } finally {
    refresh.value++
    busy.value = false
  }
}
</script>

<template>
  <section class="capability-manager" :aria-busy="busy">
    <header class="manager-header"><div><h2>Capability Manager</h2><p>模块代码与数据分离，停用不会删除持久化数据。</p></div><span v-if="message" role="status">{{ message }}</span></header>
    <div class="capability-list">
      <article v-for="row in rows" :key="row.manifest.id" class="capability-row" :data-capability-id="row.manifest.id">
        <div class="capability-main"><strong>{{ row.manifest.displayName }}</strong><small>{{ row.manifest.id }} · v{{ row.manifest.version }} · {{ row.kindLabel }}</small></div>
        <span class="capability-state">{{ stateLabel(row.manifest, row.record) }}</span>
        <span class="capability-deps">依赖：{{ row.manifest.dependencies.join(", ") || "无" }}</span>
        <div class="capability-actions">
          <button v-if="row.record?.enabled && row.record.state === 'SUSPENDED'" :disabled="actionDisabled(row.manifest, row.record, 'resume')" @click="transition(row.manifest.id, 'resume')">恢复</button>
          <button :disabled="actionDisabled(row.manifest, row.record)" :title="unavailableReason(row.manifest, row.record)" @click="transition(row.manifest.id, nextAction(row.record))">{{ actionLabel(row.manifest, row.record) }}</button>
        </div>
        <small v-if="unavailableReason(row.manifest, row.record)">{{ unavailableReason(row.manifest, row.record) }}</small><small v-else-if="row.dependent.length">被依赖：{{ row.dependent.join(", ") }}</small>
      </article>
    </div>
  </section>
</template>

<style scoped>
.capability-manager { padding: 24px; color: var(--ui-text, #222); max-width: 980px; }
.manager-header { display:flex; justify-content:space-between; gap:16px; align-items:flex-start; margin-bottom:20px; }
.manager-header h2 { margin:0 0 6px; }.manager-header p { margin:0; color:var(--ui-muted,#777); }
.capability-list { display:grid; gap:8px; }.capability-row { display:grid; grid-template-columns:minmax(180px,1.4fr) 110px minmax(120px,1fr) auto; gap:12px; align-items:center; padding:12px; border:1px solid var(--ui-border,#ddd); border-radius:6px; }
.capability-main { display:grid; gap:4px; }.capability-main small,.capability-deps,.capability-row > small { color:var(--ui-muted,#777); font-size:12px; }.capability-state { font-family:monospace; }.capability-row button { min-width:64px; }.capability-row button:disabled { opacity:.55; }
.capability-actions { display:flex; gap:8px; }
</style>
