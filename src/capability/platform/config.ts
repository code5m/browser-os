// Pluggable Capability Runtime 配置入口。
// 配置只描述启用意图；依赖补齐、冲突和生命周期仍由 Assembly/Runtime 负责。

import type { CapabilityConfigValue, CapabilityManifestV1 } from './contract'

export interface CapabilityEnabledConfig {
  enabled: Record<string, boolean>
  config?: Record<string, Record<string, CapabilityConfigValue>>
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
  if (!override) return { enabled: { ...base.enabled }, config: { ...(base.config ?? {}) } }
  return {
    enabled: { ...base.enabled, ...override.enabled },
    config: { ...(base.config ?? {}), ...(override.config ?? {}) },
  }
}

export interface ResolvedCapabilityConfig {
  values: Record<string, CapabilityConfigValue>
  errors: string[]
}

export function resolveCapabilityConfig(
  manifest: CapabilityManifestV1,
  input: Record<string, CapabilityConfigValue> = {},
): ResolvedCapabilityConfig {
  const schema = manifest.config
  if (!schema) return { values: { ...input }, errors: [] }
  const values = { ...schema.defaults, ...input }
  const errors: string[] = []
  for (const [key, value] of Object.entries(values)) {
    const property = schema.properties[key]
    if (!property) {
      if (!schema.additionalProperties) errors.push(`${manifest.id}: 未知配置项 ${key}`)
      continue
    }
    if (property.type === 'string' && typeof value !== 'string') errors.push(`${manifest.id}.${key}: 必须是 string`)
    if (property.type === 'number' && typeof value !== 'number') errors.push(`${manifest.id}.${key}: 必须是 number`)
    if (property.type === 'boolean' && typeof value !== 'boolean') errors.push(`${manifest.id}.${key}: 必须是 boolean`)
    if (property.enum && !property.enum.includes(value)) errors.push(`${manifest.id}.${key}: 不在允许值内`)
  }
  for (const [key, property] of Object.entries(schema.properties)) {
    if (property.required && values[key] === undefined) errors.push(`${manifest.id}.${key}: 缺少必填配置`)
  }
  return { values, errors }
}
