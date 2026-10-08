import { defineAsyncComponent, h, type Component } from "vue"
import type { CapabilityDefinition } from "../types"
import { contributionRegistry } from "../contribution/registry"
import type { Contribution } from "../contribution/types"

type LazyContribution = Omit<Contribution, "capabilityId" | "component"> & {
  load: () => Promise<{ default: Component }>
  panelState?: boolean
}

type LifecycleAction = () => void | Promise<void>

const panelLoading = {
  render: () => h("div", { class: "modview panel-state", role: "status", "aria-live": "polite" }, "面板加载中…"),
}
const panelError = {
  render: () => h("div", { class: "modview panel-state panel-error", role: "alert" }, "该面板暂时无法显示"),
}

export function createLazyContributionRegistrar(
  manifest: CapabilityDefinition,
  items: LazyContribution[],
): () => void {
  const contributions = items.map(({ load, panelState, ...item }) => ({
    ...item,
    capabilityId: manifest.id,
    component: panelState
      ? defineAsyncComponent({
          loader: load,
          loadingComponent: panelLoading,
          errorComponent: panelError,
          delay: 80,
          timeout: 10000,
        })
      : defineAsyncComponent(load),
  }))
  return () => contributions.forEach((item) => contributionRegistry.registerContribution(item))
}

export function withLazyContributions(
  manifest: CapabilityDefinition,
  items: LazyContribution[],
): CapabilityDefinition {
  const registerContributions = createLazyContributionRegistrar(manifest, items)
  return {
    ...manifest,
    lifecycle: {
      ...manifest.lifecycle,
      onActivate: registerContributions,
    },
  }
}

export function createLifecycleContributionCapability(
  manifest: CapabilityDefinition,
  items: LazyContribution[],
  activate: () => void,
  suspend: LifecycleAction,
  deactivate: LifecycleAction = suspend,
) {
  const register = createLazyContributionRegistrar(manifest, items)
  return {
    register,
    capability: {
      ...manifest,
      lifecycle: {
        ...manifest.lifecycle,
        onActivate: () => {
          activate()
          register()
        },
        onSuspend: suspend,
        onDeactivate: deactivate,
      },
    } satisfies CapabilityDefinition,
  }
}
