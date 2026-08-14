export { BrowserTabManager } from './manager';
export type {
  BrowserTabEvent,
  BrowserTabManagerOptions,
  CreateTabOptions,
  LogicalRect,
  TabId,
  TabState,
} from './types';
export {
  clampRect,
  domRectToLogical,
  getRelativeRect,
  isValidRect,
  rectsEqual,
  scaleRect,
} from './geometry';

// Low-level API (direct wrappers around Tauri commands).
export async function createTab(options: import('./types').CreateTabOptions): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('plugin:browser-tabs|create_tab', { options });
}

export async function updateRect(id: string, rect: import('./types').LogicalRect): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('plugin:browser-tabs|update_rect', { id, rect });
}

export async function setVisible(id: string, visible: boolean): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('plugin:browser-tabs|set_visible', { id, visible });
}

export async function closeTab(id: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('plugin:browser-tabs|close_tab', { id });
}

export async function navigate(id: string, url: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('plugin:browser-tabs|navigate', { id, url });
}

export async function listTabs(): Promise<string[]> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke('plugin:browser-tabs|list_tabs');
}
