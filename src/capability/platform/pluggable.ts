// Manifest + Registry + Dependency + Contribution + Lifecycle 的统一运行入口。

import type { CapabilityDefinition } from "../types"
import type { CapabilityRuntime } from "../runtime"
import { createCapabilityRuntime } from "../runtime"
import type { ContributionRegistry } from "../contribution/registry"
import { assemble, type AssemblyResult } from "./assembly"
import type { CapabilityManifestV1 } from "./contract"
import type { CapabilityEnabledConfig } from "./config"
import { resolveCapabilityConfig } from "./config"
import { validateManifestV1 } from "./contract"
import { inspectCapabilities, type CapabilitySnapshot } from "./inspector"
import { createCapabilityEventBus, type CapabilityEventBus } from './events'

export interface PluggableRuntimeOptions {
  catalog: Record<string, CapabilityManifestV1>
  definitions: Record<string, CapabilityDefinition>
  config: CapabilityEnabledConfig
  contributions?: ContributionRegistry
}

export interface PluggableRuntime {
  runtime: CapabilityRuntime
  assembly: AssemblyResult
  activate(): void
  deactivate(id: string): void
  inspectConfig(id: string): { values: Record<string, unknown>; errors: string[] }
  inspect(): CapabilitySnapshot[]
  events: CapabilityEventBus
}

export function createPluggableRuntime(options: PluggableRuntimeOptions): PluggableRuntime {
  for (const manifest of Object.values(options.catalog)) {
    const violations = validateManifestV1(manifest)
    if (violations.length > 0) throw new Error(`${manifest.id} manifest 无效: ${violations.map((v) => v.message).join('; ')}`)
  }
  const assembly = assemble(options.catalog, { enabled: options.config.enabled })
  if (!assembly.ok) {
    throw new Error(`Capability 配置无法装配: ${assembly.rejections.map((r) => r.message).join("; ")}`)
  }

  const runtime = createCapabilityRuntime()
  const events = createCapabilityEventBus()
  const registry = options.contributions
  let active = false
  const resolvedConfigs = new Map<string, { values: Record<string, any>; errors: string[] }>()
  for (const [id, manifest] of Object.entries(options.catalog)) {
    const resolved = resolveCapabilityConfig(manifest, options.config.config?.[id])
    if (resolved.errors.length > 0 && options.config.enabled[id] !== false) throw new Error(`Capability 配置无效: ${resolved.errors.join('; ')}`)
    resolvedConfigs.set(id, resolved)
  }

  function activate(): void {
    if (active) return
    const started: string[] = []
    try {
      for (const id of assembly.activationOrder) {
        const definition = options.definitions[id]
        if (!definition) throw new Error(`Capability 缺少运行时定义: ${id}`)
        events.emit({ type: 'capability.registered', capabilityId: id, at: Date.now() })
        runtime.register(definition)
        runtime.resolve(id)
        events.emit({ type: 'capability.activating', capabilityId: id, at: Date.now() })
        try { runtime.activate(id) } catch (error) {
          events.emit({ type: 'capability.failed', capabilityId: id, error: error instanceof Error ? error.message : String(error), at: Date.now() })
          throw error
        }
        if (registry) {
          for (const contribution of options.catalog[id]?.contributions ?? []) {
            // Lifecycle registrations contain the actual component and UI metadata.
            if (!registry.getBySlot(contribution.slot).some((entry) => entry.id === contribution.id)) {
              registry.registerContribution({ ...contribution, capabilityId: id, type: contribution.type as 'surface' | 'navigation' })
            }
          }
        }
        events.emit({ type: 'capability.activated', capabilityId: id, at: Date.now() })
        started.push(id)
      }
      active = true
    } catch (error) {
      for (const id of started.reverse()) {
        try { deactivate(id) } catch { /* rollback is best effort; original error is authoritative */ }
      }
      throw error
    }
  }

  function deactivate(id: string): void {
    const record = runtime.get(id)
    if (!record) return
    if (!record.enabled) {
      registry?.unregisterCapability(id)
      return
    }
    events.emit({ type: 'capability.deactivating', capabilityId: id, at: Date.now() })
    if (record.state === "ACTIVE") runtime.suspend(id)
    if (record.state === "SUSPENDED") {
      runtime.disable(id)
      record.definition.lifecycle.onDeactivate?.()
    }
    registry?.unregisterCapability(id)
    events.emit({ type: 'capability.deactivated', capabilityId: id, at: Date.now() })
  }

  function inspectConfig(id: string) {
    return resolvedConfigs.get(id) ?? { values: {}, errors: [`未知 capability: ${id}`] }
  }

  function inspect() {
    return inspectCapabilities(options.catalog, runtime, registry, inspectConfig)
  }

  return { runtime, assembly, activate, deactivate, inspectConfig, inspect, events }
}
