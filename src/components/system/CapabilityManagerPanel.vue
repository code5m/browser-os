<script setup lang="ts">
import { computed, ref } from "vue"
import { CAPABILITY_CATALOG } from "../../capability/platform/catalog"
import { peekCapabilityRuntime } from "../../capability/runtimeSingleton"
import { loadCapabilityConfig, saveCapabilityConfig } from "../../capability/platform/persistence"

const runtime = peekCapabilityRuntime()
const message = ref("")
const refresh = ref(0)
const stored = loadCapabilityConfig() ?? { enabled: {} }
const rows = computed(() => {
  void refresh.value
  const entries = new Map(runtime?.inspect().map((entry) => [entry.id, entry]) ?? [])
  return Object.values(CAPABILITY_CATALOG).sort((a, b) => a.id.localeCompare(b.id)).map((manifest) => ({
    manifest,
    record: entries.get(manifest.id),
    dependent: Object.values(CAPABILITY_CATALOG).filter((candidate) => candidate.dependencies.includes(manifest.id)).map((candidate) => candidate.id),
  }))
})

function toggle(id: string, enabled: boolean) {
  if (!runtime) { message.value = "Runtime 尚未初始化"; return }
  try {
    const record = runtime.get(id)
    if (!record) { message.value = `${id} 当前未装配，需重启后应用配置`; return }
    if (enabled) { runtime.enable(id); runtime.resolve(id); runtime.activate(id) }
    else { runtime.disable(id) }
    stored.enabled[id] = enabled
    saveCapabilityConfig(stored)
    message.value = `${id} 已${enabled ? "启用" : "停用"}`
    refresh.value++
  } catch (error) { message.value = error instanceof Error ? error.message : String(error) }
}
</script>

<template>
  <section class="capability-manager">
    <header class="manager-header"><div><h2>Capability Manager</h2><p>模块代码与数据分离，停用不会删除持久化数据。</p></div><span v-if="message" role="status">{{ message }}</span></header>
    <div class="capability-list">
      <article v-for="row in rows" :key="row.manifest.id" class="capability-row">
        <div class="capability-main"><strong>{{ row.manifest.displayName }}</strong><small>{{ row.manifest.id }} · v{{ row.manifest.version }} · {{ row.manifest.kind || "feature" }}</small></div>
        <span class="capability-state">{{ row.record?.state || "AVAILABLE" }}</span>
        <span class="capability-deps">依赖：{{ row.manifest.dependencies.join(", ") || "无" }}</span>
        <button :disabled="row.manifest.kind === 'core' || !!row.dependent.length" @click="toggle(row.manifest.id, !row.record?.enabled)">{{ row.record?.enabled ? "停用" : "启用" }}</button>
        <small v-if="row.manifest.kind === 'core'">CORE 不可停用</small><small v-else-if="row.dependent.length">被依赖：{{ row.dependent.join(", ") }}</small>
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
</style>
