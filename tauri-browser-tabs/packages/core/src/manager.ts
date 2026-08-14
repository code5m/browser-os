import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  BrowserTabEvent,
  BrowserTabManagerOptions,
  CreateTabOptions,
  LogicalRect,
  TabId,
  TabState,
} from './types';
import { domRectToLogical, isValidRect, rectsEqual } from './geometry';

/**
 * BrowserTabManager coordinates between the DOM and the Tauri backend.
 *
 * It handles:
 * - Creating and closing tabs
 * - Syncing DOM element rects to child webviews
 * - Debouncing resize events
 * - Listening to backend events
 */
type ResolvedOptions = Omit<Required<BrowserTabManagerOptions>, 'container'>;

export class BrowserTabManager {
  private container: HTMLElement;
  private options: ResolvedOptions;
  private tabs = new Map<TabId, TabState>();
  private resizeObserver?: ResizeObserver;
  private unlistenEvent?: UnlistenFn;
  private unlistenResize?: UnlistenFn;
  private debounceTimer?: ReturnType<typeof setTimeout>;
  private destroyed = false;

  constructor(options: BrowserTabManagerOptions) {
    this.container = options.container;
    this.options = {
      debounceMs: options.debounceMs ?? 16,
      autoSyncOnContainerResize: options.autoSyncOnContainerResize ?? true,
      autoSyncOnWindowResize: options.autoSyncOnWindowResize ?? true,
      onTabCreated: options.onTabCreated ?? (() => {}),
      onTabClosed: options.onTabClosed ?? (() => {}),
      onTabChanged: options.onTabChanged ?? (() => {}),
      onError: options.onError ?? ((e) => console.error('[BrowserTabManager]', e)),
    };
  }

  /**
   * Initialize the manager. Must be called before using other methods.
   */
  async init(): Promise<void> {
    if (this.destroyed) {
      throw new Error('BrowserTabManager has been destroyed');
    }

    // Listen to backend events.
    this.unlistenEvent = await listen<BrowserTabEvent>(
      'browser-tabs://event',
      (event) => this.handleBackendEvent(event.payload),
    );

    // Listen to main window resize events from the backend.
    this.unlistenResize = await listen('browser-tabs://window-resized', () => {
      if (this.options.autoSyncOnWindowResize) {
        this.syncAllRects();
      }
    });

    // Observe container resize.
    if (this.options.autoSyncOnContainerResize && typeof ResizeObserver !== 'undefined') {
      this.resizeObserver = new ResizeObserver(() => {
        this.syncAllRects();
      });
      this.resizeObserver.observe(this.container);
    }
  }

  /**
   * Create a new browser tab.
   */
  async createTab(options: Omit<CreateTabOptions, 'rect'> & { rect?: Partial<LogicalRect> }): Promise<void> {
    const containerRect = this.getContainerRect();
    if (!isValidRect(containerRect)) {
      throw new Error('Container has zero size');
    }

    const rect: LogicalRect = {
      x: options.rect?.x ?? containerRect.x,
      y: options.rect?.y ?? containerRect.y,
      width: options.rect?.width ?? containerRect.width,
      height: options.rect?.height ?? containerRect.height,
    };

    if (!isValidRect(rect)) {
      throw new Error(`Invalid rect for tab ${options.id}: ${JSON.stringify(rect)}`);
    }

    const fullOptions: CreateTabOptions = {
      id: options.id,
      url: options.url,
      rect,
      visible: options.visible ?? true,
      autoResize: options.autoResize ?? true,
      userAgent: options.userAgent,
      transparent: options.transparent ?? false,
    };

    await invoke('plugin:browser-tabs|create_tab', { options: fullOptions });

    const state: TabState = {
      id: options.id,
      url: options.url,
      title: '',
      visible: fullOptions.visible ?? true,
      rect,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };

    this.tabs.set(options.id, state);
    this.options.onTabCreated(state);
  }

  /**
   * Close a tab.
   */
  async closeTab(id: TabId): Promise<void> {
    await invoke('plugin:browser-tabs|close_tab', { id });
    const tab = this.tabs.get(id);
    if (tab) {
      this.tabs.delete(id);
      this.options.onTabClosed(tab);
    }
  }

  /**
   * Navigate a tab to a new URL.
   */
  async navigate(id: TabId, url: string): Promise<void> {
    await invoke('plugin:browser-tabs|navigate', { id, url });
    const tab = this.tabs.get(id);
    if (tab) {
      tab.url = url;
      tab.updatedAt = Date.now();
      this.options.onTabChanged(tab);
    }
  }

  /**
   * Set tab visibility.
   */
  async setVisible(id: TabId, visible: boolean): Promise<void> {
    await invoke('plugin:browser-tabs|set_visible', { id, visible });
    const tab = this.tabs.get(id);
    if (tab) {
      tab.visible = visible;
      tab.updatedAt = Date.now();
      this.options.onTabChanged(tab);
    }
  }

  /**
   * Update a tab's rect manually.
   */
  async updateRect(id: TabId, rect: LogicalRect): Promise<void> {
    if (!isValidRect(rect)) {
      throw new Error(`Invalid rect for tab ${id}`);
    }

    const tab = this.tabs.get(id);
    if (tab && rectsEqual(tab.rect, rect)) {
      return; // No change needed.
    }

    await invoke('plugin:browser-tabs|update_rect', { id, rect });

    if (tab) {
      tab.rect = rect;
      tab.updatedAt = Date.now();
      this.options.onTabChanged(tab);
    }
  }

  /**
   * Sync all visible tabs to the container's current rect.
   * This is debounced to avoid excessive IPC calls.
   */
  syncAllRects(): void {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    this.debounceTimer = setTimeout(() => {
      this.debounceTimer = undefined;
      this.performSyncAllRects();
    }, this.options.debounceMs);
  }

  /**
   * Immediately sync all visible tabs without debouncing.
   */
  async syncAllRectsNow(): Promise<void> {
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = undefined;
    }
    await this.performSyncAllRects();
  }

  /**
   * Get all tabs.
   */
  getTabs(): TabState[] {
    return Array.from(this.tabs.values());
  }

  /**
   * Get a tab by ID.
   */
  getTab(id: TabId): TabState | undefined {
    return this.tabs.get(id);
  }

  /**
   * Destroy the manager and clean up resources.
   */
  async destroy(): Promise<void> {
    this.destroyed = true;

    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = undefined;
    }

    this.resizeObserver?.disconnect();
    this.resizeObserver = undefined;

    this.unlistenEvent?.();
    this.unlistenEvent = undefined;

    this.unlistenResize?.();
    this.unlistenResize = undefined;

    // Close all tabs.
    const ids = Array.from(this.tabs.keys());
    await Promise.all(ids.map((id) => this.closeTab(id).catch(() => {})));

    this.tabs.clear();
  }

  private getContainerRect(): LogicalRect {
    return domRectToLogical(this.container.getBoundingClientRect());
  }

  private async performSyncAllRects(): Promise<void> {
    if (this.destroyed) return;

    const containerRect = this.getContainerRect();
    if (!isValidRect(containerRect)) return;

    const updates: Promise<void>[] = [];

    for (const tab of this.tabs.values()) {
      if (!tab.visible) continue;
      if (rectsEqual(tab.rect, containerRect)) continue;

      updates.push(
        this.updateRect(tab.id, containerRect).catch((error) => {
          this.options.onError(error instanceof Error ? error : new Error(String(error)));
        }),
      );
    }

    await Promise.all(updates);
  }

  private handleBackendEvent(event: BrowserTabEvent): void {
    const tab = this.tabs.get(event.id);
    if (!tab) return;

    switch (event.type) {
      case 'navigationStarted':
      case 'navigationFinished':
        tab.url = event.url;
        break;
      case 'titleChanged':
        tab.title = event.title;
        break;
      case 'faviconChanged':
        tab.favicon = event.favicon;
        break;
      case 'closed':
        this.tabs.delete(event.id);
        this.options.onTabClosed(tab);
        return;
    }

    tab.updatedAt = Date.now();
    this.options.onTabChanged(tab);
  }
}
