import type { CapabilityEnabledConfig } from './config'
import type { CapabilityConfigValue } from './contract'

const STORAGE_KEY = 'browser-os-capability-config'
const RESERVED_KEYS = new Set(['__proto__', 'constructor', 'prototype'])

function browserStorage(): Storage | undefined {
  return typeof window === 'undefined' ? undefined : window.localStorage
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function normalizeConfig(value: unknown): CapabilityEnabledConfig | null {
  if (!isRecord(value)) return null
  const enabled: Record<string, boolean> = {}
  if (isRecord(value.enabled)) {
    for (const [id, state] of Object.entries(value.enabled)) {
      if (!RESERVED_KEYS.has(id) && typeof state === 'boolean') enabled[id] = state
    }
  }
  const config: Record<string, Record<string, CapabilityConfigValue>> = {}
  if (isRecord(value.config)) {
    for (const [id, rawValues] of Object.entries(value.config)) {
      if (RESERVED_KEYS.has(id) || !isRecord(rawValues)) continue
      const values: Record<string, CapabilityConfigValue> = {}
      for (const [key, rawValue] of Object.entries(rawValues)) {
        if (RESERVED_KEYS.has(key)) continue
        if (rawValue === null || ['string', 'number', 'boolean'].includes(typeof rawValue)) {
          values[key] = rawValue as CapabilityConfigValue
        }
      }
      config[id] = values
    }
  }
  return { enabled, ...(Object.keys(config).length > 0 ? { config } : {}) }
}

export function loadCapabilityConfig(storage?: Pick<Storage, 'getItem'>): CapabilityEnabledConfig | null {
  try {
    const target = storage ?? browserStorage()
    if (!target) return null
    const raw = target.getItem(STORAGE_KEY)
    return raw ? normalizeConfig(JSON.parse(raw)) : null
  } catch {
    return null
  }
}

export function saveCapabilityConfig(config: CapabilityEnabledConfig, storage?: Pick<Storage, 'setItem'>): boolean {
  try {
    const target = storage ?? browserStorage()
    if (!target) return false
    target.setItem(STORAGE_KEY, JSON.stringify(config))
    return true
  } catch {
    return false
  }
}

export const CAPABILITY_CONFIG_STORAGE_KEY = STORAGE_KEY
