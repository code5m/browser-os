import { onBeforeUnmount, onMounted, ref, shallowRef, type Ref } from 'vue';
import {
  BrowserTabManager,
  type BrowserTabManagerOptions,
  type CreateTabOptions,
  type LogicalRect,
  type TabId,
  type TabState,
} from '@tauri-browser-tabs/core';

export interface UseBrowserTabsOptions extends Omit<BrowserTabManagerOptions, 'container'> {
  /**
   * The container element ref. If not provided, you must call `setContainer` manually.
   */
  container?: Ref<HTMLElement | undefined>;
}

export interface UseBrowserTabsReturn {
  /**
   * All managed tabs.
   */
  tabs: Ref<TabState[]>;

  /**
   * The currently active tab ID.
   */
  activeTabId: Ref<TabId | null>;

  /**
   * The active tab state.
   */
  activeTab: Ref<TabState | null>;

  /**
   * Whether the manager is initialized.
   */
  isReady: Ref<boolean>;

  /**
   * Set the container element manually.
   */
  setContainer: (element: HTMLElement) => Promise<void>;

  /**
   * Create a new tab.
   */
  createTab: (options: Omit<CreateTabOptions, 'rect'> & { rect?: Partial<LogicalRect> }) => Promise<void>;

  /**
   * Close a tab.
   */
  closeTab: (id: TabId) => Promise<void>;

  /**
   * Set the active tab.
   */
  setActiveTab: (id: TabId) => Promise<void>;

  /**
   * Navigate the active tab to a URL.
   */
  navigate: (url: string) => Promise<void>;

  /**
   * Navigate a specific tab to a URL.
   */
  navigateTab: (id: TabId, url: string) => Promise<void>;

  /**
   * Sync all tab rects immediately.
   */
  syncRects: () => Promise<void>;

  /**
   * Get a tab by ID.
   */
  getTab: (id: TabId) => TabState | undefined;

  /**
   * The underlying manager instance.
   */
  manager: Readonly<Ref<BrowserTabManager | null>>;
}

/**
 * Vue 3 composable for managing browser tabs.
 */
export function useBrowserTabs(options: UseBrowserTabsOptions = {}): UseBrowserTabsReturn {
  const tabs = ref<TabState[]>([]);
  const activeTabId = ref<TabId | null>(null);
  const isReady = ref(false);
  const manager = shallowRef<BrowserTabManager | null>(null);

  const activeTab = ref<TabState | null>(null);

  let containerElement: HTMLElement | null = null;

  const updateTabs = () => {
    if (!manager.value) return;
    tabs.value = manager.value.getTabs();
    activeTab.value = activeTabId.value
      ? manager.value.getTab(activeTabId.value) ?? null
      : null;
  };

  const initManager = async (element: HTMLElement) => {
    if (manager.value) {
      await manager.value.destroy();
    }

    const mgr = new BrowserTabManager({
      ...options,
      container: element,
      onTabCreated: (tab) => {
        options.onTabCreated?.(tab);
        if (!activeTabId.value) {
          activeTabId.value = tab.id;
        }
        updateTabs();
      },
      onTabClosed: (tab) => {
        options.onTabClosed?.(tab);
        if (activeTabId.value === tab.id) {
          const remaining = tabs.value.filter((t) => t.id !== tab.id);
          activeTabId.value = remaining[0]?.id ?? null;
        }
        updateTabs();
      },
      onTabChanged: (tab) => {
        options.onTabChanged?.(tab);
        updateTabs();
      },
      onError: (error) => {
        options.onError?.(error);
      },
    });

    await mgr.init();
    manager.value = mgr;
    containerElement = element;
    isReady.value = true;
    updateTabs();
  };

  const setContainer = async (element: HTMLElement) => {
    await initManager(element);
  };

  const createTab = async (
    tabOptions: Omit<CreateTabOptions, 'rect'> & { rect?: Partial<LogicalRect> },
  ) => {
    if (!manager.value) {
      throw new Error('Manager not initialized. Call setContainer first.');
    }
    await manager.value.createTab(tabOptions);
    updateTabs();
  };

  const closeTab = async (id: TabId) => {
    if (!manager.value) return;
    await manager.value.closeTab(id);
    updateTabs();
  };

  const setActiveTab = async (id: TabId) => {
    if (!manager.value) return;

    const currentActive = activeTabId.value;
    if (currentActive === id) return;

    // Hide previous active tab.
    if (currentActive) {
      await manager.value.setVisible(currentActive, false).catch(() => {});
    }

    // Show new active tab.
    await manager.value.setVisible(id, true);
    activeTabId.value = id;
    updateTabs();
  };

  const navigate = async (url: string) => {
    if (!activeTabId.value) return;
    await navigateTab(activeTabId.value, url);
  };

  const navigateTab = async (id: TabId, url: string) => {
    if (!manager.value) return;
    await manager.value.navigate(id, url);
  };

  const syncRects = async () => {
    if (!manager.value) return;
    await manager.value.syncAllRectsNow();
  };

  const getTab = (id: TabId) => {
    return manager.value?.getTab(id);
  };

  onMounted(() => {
    if (options.container?.value) {
      initManager(options.container.value);
    }
  });

  onBeforeUnmount(async () => {
    if (manager.value) {
      await manager.value.destroy();
      manager.value = null;
    }
  });

  return {
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
  };
}
