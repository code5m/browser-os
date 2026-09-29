import type { CapabilityEnabledConfig } from './config'

const STORAGE_KEY = 'browser-os-capability-config'

export function loadCapabilityConfig(storage: Pick<Storage, 'getItem'> | undefined = typeof localStorage === 'undefined' ? undefined : localStorage): CapabilityEnabledConfig | null {
  if (!storage) return null
  try {
    const raw = storage.getItem(STORAGE_KEY)
    return raw ? JSON.parse(raw) as CapabilityEnabledConfig : null
  } catch {
    return null
  }
}

export function saveCapabilityConfig(config: CapabilityEnabledConfig, storage: Pick<Storage, 'setItem'> | undefined = typeof localStorage === 'undefined' ? undefined : localStorage): void {
  storage?.setItem(STORAGE_KEY, JSON.stringify(config))
}

export const CAPABILITY_CONFIG_STORAGE_KEY = STORAGE_KEY
