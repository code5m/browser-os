// src/capabilities/apps/state/useAppsStore.ts
// Apps（系统应用启动器）能力语义 owner（Capability Library Expansion v1，STAGE H）。
//
// 从 src/stores/useSystemStore.ts 拆出（解除 Debt-8E-1：Clipboard 与 Apps 曾共居一个 store）。
// 安全：应用启动经 bridge.launchApp → Rust security_policy::check_launch_target（白名单式解析，禁 sh -c）+ 审计。
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import type { AppEntry } from "../../../types";
import { appsLifecycleSnapshot } from "../lifecycle";

export const useAppsStore = defineStore("apps", () => {
  const layout = useLayoutStore();

  const apps = ref<AppEntry[]>([]);
  const appFilter = ref("");
  const brokenIcons = ref<Set<string>>(new Set());

  async function loadApps() {
    if (!appsLifecycleSnapshot().active) return;
    try {
      apps.value = await bridge.listApps();
      brokenIcons.value.clear();
    } catch (e: any) {
      layout.showToast("读取应用列表失败: " + (e?.message ?? e));
    }
  }
  const filteredApps = computed(() => {
    const f = appFilter.value.trim().toLowerCase();
    if (!f) return apps.value;
    return apps.value.filter((a) => a.name.toLowerCase().includes(f));
  });
  async function launchApp(app: AppEntry) {
    if (!appsLifecycleSnapshot().active) return;
    try {
      layout.showToast("正在启动: " + app.name);
      await bridge.launchApp(app.exec);
      layout.showToast("已启动: " + app.name);
    } catch (e: any) {
      layout.showToast("启动失败: " + app.name + " - " + (e?.message ?? e));
    }
  }
  function onAppImgError(exec: string) {
    brokenIcons.value.add(exec);
  }

  return {
    apps,
    appFilter,
    brokenIcons,
    filteredApps,
    loadApps,
    launchApp,
    onAppImgError,
  };
});
