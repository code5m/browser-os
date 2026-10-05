import { defineAsyncComponent, type Component } from "vue"
import type { CapabilityDefinition } from "../types"
import { contributionRegistry } from "../contribution/registry"
import type { Contribution } from "../contribution/types"

type LazyContribution = Omit<Contribution, "capabilityId" | "component"> & {
  load: () => Promise<{ default: Component }>
}

export function withLazyContributions(
  manifest: CapabilityDefinition,
  items: LazyContribution[],
): CapabilityDefinition {
  const contributions = items.map(({ load, ...item }) => ({
    ...item,
    capabilityId: manifest.id,
    component: defineAsyncComponent(load),
  }))
  return {
    ...manifest,
    lifecycle: {
      ...manifest.lifecycle,
      onActivate: () => contributions.forEach((item) => contributionRegistry.registerContribution(item)),
    },
  }
}
