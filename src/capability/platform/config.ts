// Pluggable Capability Runtime 配置入口。
// 配置只描述启用意图；依赖补齐、冲突和生命周期仍由 Assembly/Runtime 负责。

export interface CapabilityEnabledConfig {
  enabled: Record<string, boolean>
}

function parseIds(raw: unknown): string[] {
  if (typeof raw !== 'string') return []
  return raw.split(',').map((id) => id.trim()).filter(Boolean)
}

export function configFromIds(ids: string[]): CapabilityEnabledConfig {
  return { enabled: Object.fromEntries([...new Set(ids)].sort().map((id) => [id, true])) }
}

export function configFromEnv(env: Record<string, unknown> = {}): CapabilityEnabledConfig | null {
  const enabled = parseIds(env.VITE_CAPABILITY_ENABLED)
  const disabled = new Set(parseIds(env.VITE_CAPABILITY_DISABLED))
  if (enabled.length === 0 && disabled.size === 0) return null
  const table: Record<string, boolean> = {}
  for (const id of enabled) table[id] = true
  for (const id of disabled) table[id] = false
  return { enabled: table }
}

export function mergeConfig(
  base: CapabilityEnabledConfig,
  override: CapabilityEnabledConfig | null,
): CapabilityEnabledConfig {
  if (!override) return { enabled: { ...base.enabled } }
  return { enabled: { ...base.enabled, ...override.enabled } }
}
