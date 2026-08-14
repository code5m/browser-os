<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useBrowserTabs, type UseBrowserTabsOptions } from './useBrowserTabs';
import type { CreateTabOptions, LogicalRect, TabId, TabState } from '@tauri-browser-tabs/core';

export interface BrowserHostProps {
  /**
   * Initial tabs to create on mount.
   */
  initialTabs?: Array<Omit<CreateTabOptions, 'rect'> & { rect?: Partial<LogicalRect> }>;

  /**
   * ID of the initially active tab.
   */
  initialActiveTab?: TabId;

  /**
   * Manager options.
   */
  managerOptions?: UseBrowserTabsOptions;
}

const props = withDefaults(defineProps<BrowserHostProps>(), {
  initialTabs: () => [],
  initialActiveTab: undefined,
  managerOptions: () => ({}),
});

const emit = defineEmits<{
  'tab-created': [tab: TabState];
  'tab-closed': [tab: TabState];
  'tab-changed': [tab: TabState];
  'active-tab-changed': [id: TabId | null];
  error: [error: Error];
}>();

const containerRef = ref<HTMLElement | null>(null);

const {
  tabs,
  activeTabId,
  activeTab,
  isReady,
  setContainer,
  createTab,
  closeTab,
  setActiveTab,
  navigate,
  navigateTab,
  syncRects,
  getTab,
  manager,
} = useBrowserTabs({
  ...props.managerOptions,
  onTabCreated: (tab) => {
    props.managerOptions?.onTabCreated?.(tab);
    emit('tab-created', tab);
  },
  onTabClosed: (tab) => {
    props.managerOptions?.onTabClosed?.(tab);
    emit('tab-closed', tab);
  },
  onTabChanged: (tab) => {
    props.managerOptions?.onTabChanged?.(tab);
    emit('tab-changed', tab);
  },
  onError: (error) => {
    props.managerOptions?.onError?.(error);
    emit('error', error);
  },
});

onMounted(async () => {
  if (!containerRef.value) return;

  await setContainer(containerRef.value);

  // Create initial tabs.
  for (const tab of props.initialTabs) {
    await createTab(tab);
  }

  // Set initial active tab.
  if (props.initialActiveTab) {
    await setActiveTab(props.initialActiveTab);
  } else if (props.initialTabs.length > 0) {
    await setActiveTab(props.initialTabs[0].id);
  }
});

defineExpose({
  tabs,
  activeTabId,
  activeTab,
  isReady,
  createTab,
  closeTab,
  setActiveTab,
  navigate,
  navigateTab,
  syncRects,
  getTab,
  manager,
});
</script>

<template>
  <div ref="container" class="tauri-browser-tabs-host">
    <slot
      :tabs="tabs"
      :active-tab-id="activeTabId"
      :active-tab="activeTab"
      :is-ready="isReady"
      :create-tab="createTab"
      :close-tab="closeTab"
      :set-active-tab="setActiveTab"
      :navigate="navigate"
      :navigate-tab="navigateTab"
      :sync-rects="syncRects"
    />
  </div>
</template>

<style scoped>
.tauri-browser-tabs-host {
  position: relative;
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>
