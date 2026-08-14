export { default as BrowserHost } from './BrowserHost.vue';
export { useBrowserTabs } from './useBrowserTabs';
export type { UseBrowserTabsOptions, UseBrowserTabsReturn } from './useBrowserTabs';

// Re-export core types for convenience.
export type {
  BrowserTabEvent,
  BrowserTabManagerOptions,
  CreateTabOptions,
  LogicalRect,
  TabId,
  TabState,
} from '@tauri-browser-tabs/core';
