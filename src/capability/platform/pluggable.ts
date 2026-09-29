// Manifest + Registry + Dependency + Contribution + Lifecycle 的统一运行入口。

import type { CapabilityDefinition } from "../types"
import type { CapabilityRuntime } from "../runtime"
import { createCapabilityRuntime } from "../runtime"
import type { ContributionRegistry } from "../contribution/registry"
import { CONTRIBUTION_SLOTS } from "../contribution/types"
import { assemble, type AssemblyResult } from "./assembly"
import type { CapabilityManifestV1 } from "./contract"
import type { CapabilityEnabledConfig } from "./config"

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
}

const slots = Object.values(CONTRIBUTION_SLOTS)

function removeContributions(registry: ContributionRegistry, id: string): void {
  for (const slot of slots) {
    for (const contribution of registry.getBySlot(slot)) {
      if (contribution.capabilityId === id) registry.unregisterContribution(contribution.id)
    }
  }
}

export function createPluggableRuntime(options: PluggableRuntimeOptions): PluggableRuntime {
  const assembly = assemble(options.catalog, { enabled: options.config.enabled })
  if (!assembly.ok) {
    throw new Error(`Capability 配置无法装配: ${assembly.rejections.map((r) => r.message).join("; ")}`)
  }

  const runtime = createCapabilityRuntime()
  const registry = options.contributions
  let active = false

  function activate(): void {
    if (active) return
    for (const id of assembly.activationOrder) {
      const definition = options.definitions[id]
      if (!definition) throw new Error(`Capability 缺少运行时定义: ${id}`)
      runtime.register(definition)
      runtime.resolve(id)
      runtime.activate(id)
    }
    active = true
  }

  function deactivate(id: string): void {
    const record = runtime.get(id)
    if (!record) return
    if (record.state === "ACTIVE") runtime.suspend(id)
    if (record.state === "SUSPENDED") {
      runtime.disable(id)
      record.definition.lifecycle.onDeactivate?.()
    }
    if (registry) removeContributions(registry, id)
  }

  return { runtime, assembly, activate, deactivate }
}
