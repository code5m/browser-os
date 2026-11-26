import { computed } from "vue";
import { useLayoutStore } from "../stores/useLayoutStore";
import { useBrowserStore } from "../capabilities/browser/public";
import { useFileStore } from "../capabilities/workspace/public";
import { useAppsStore } from "../capabilities/apps/public";
import { useTerminalStore } from "../capabilities/terminal/public";
import { projectTabResources } from "../application/tabResourceProjection.mjs";
import { createTabCoordinator } from "../application/tabCoordinator.mjs";

// App shortcuts and the visible tab strip share one command queue per Layout
// owner. All records and native lifecycles remain in their original owners.
const bindings = new WeakMap<ReturnType<typeof useLayoutStore>, ReturnType<typeof bindUnifiedTabs>>();

function bindUnifiedTabs(layout: ReturnType<typeof useLayoutStore>) {
  const browser = useBrowserStore();
  const tabOrder: string[] = [];
  const projectedTabs = computed(() => projectTabResources(
    browser.tabs, layout.modTabs, layout.mainView, browser.activeTabId, layout.activeModTab,
    tabOrder,
  ));

  const coordinator = createTabCoordinator({
    resources: () => projectedTabs.value,
    enabled: () => !useTerminalStore().m0Cfg?.driver,
    needsReactivation: (tab) => tab.kind === "web" && browser.rendererUnavailable(tab.id),
    activate: async (tab) => {
      if (tab.kind === "web") return browser.tabSwitch(tab.id);
      if (tab.kind !== "module") return false;
      const publish = layout.prepareModTabActivation(tab.id);
      if (!publish) return false;
      // Directory reading/discard may refuse too. Publish selection only after
      // that owner accepts, without bypassing or re-running the view guard.
      if (tab.path) {
        if (!await useFileStore().enterDir(tab.path, publish)) return false;
      } else if (!publish()) return false;
      if (tab.view === "apps") await useAppsStore().loadApps();
      return true;
    },
    close: async (tab) => {
      if (tab.kind === "web") return browser.tabClose(tab.id);
      if (tab.kind !== "module") return false;
      const prepared = layout.prepareModTabClose(tab.id);
      if (!prepared) return false;
      const { publish, next, wasVisible } = prepared;
      if (wasVisible && next?.path) {
        if (!await useFileStore().enterDir(next.path, publish)) return false;
      } else if (!publish()) return false;
      if (wasVisible && next?.path) browser.url = next.path;
      if (wasVisible && next?.view === "apps") await useAppsStore().loadApps();
      return true;
    },
  });

  async function report(operation: Promise<boolean>): Promise<boolean> {
    try { return await operation; }
    catch {
      layout.showToast("页签操作未完成，请重试");
      return false;
    }
  }
  return {
    projectedTabs,
    activate: (key: string) => report(coordinator.activate(key)),
    close: (key: string) => report(coordinator.close(key)),
    closeOthers: (key: string) => report(coordinator.closeOthers(key)),
    closeActive: () => report(coordinator.closeActive()),
    cycle: (delta: number) => report(coordinator.cycle(delta)),
  };
}

export function useUnifiedTabs() {
  const layout = useLayoutStore();
  let binding = bindings.get(layout);
  if (!binding) {
    binding = bindUnifiedTabs(layout);
    bindings.set(layout, binding);
  }
  return binding;
}
