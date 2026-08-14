/**
 * Logical rectangle in CSS pixels (DPI-independent).
 */
export interface LogicalRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Unique identifier for a browser tab.
 */
export type TabId = string;

/**
 * Options for creating a new browser tab.
 */
export interface CreateTabOptions {
  id: TabId;
  url: string;
  rect: LogicalRect;
  visible?: boolean;
  autoResize?: boolean;
  userAgent?: string;
  transparent?: boolean;
}

/**
 * Events emitted by the browser tabs plugin.
 */
export type BrowserTabEvent =
  | { type: 'navigationStarted'; id: TabId; url: string }
  | { type: 'navigationFinished'; id: TabId; url: string }
  | { type: 'titleChanged'; id: TabId; title: string }
  | { type: 'faviconChanged'; id: TabId; favicon: string }
  | { type: 'closed'; id: TabId };

/**
 * Tab metadata maintained by the manager.
 */
export interface TabState {
  id: TabId;
  url: string;
  title: string;
  favicon?: string;
  visible: boolean;
  rect: LogicalRect;
  createdAt: number;
  updatedAt: number;
}

/**
 * Options for the BrowserTabManager.
 */
export interface BrowserTabManagerOptions {
  /**
   * The HTML element that hosts the browser tabs.
   * The manager will observe this element for size changes.
   */
  container: HTMLElement;

  /**
   * Debounce interval in milliseconds for rect updates.
   * @default 16 (about one frame)
   */
  debounceMs?: number;

  /**
   * Whether to automatically sync tab rects when the container resizes.
   * @default true
   */
  autoSyncOnContainerResize?: boolean;

  /**
   * Whether to automatically sync tab rects when the main window resizes.
   * @default true
   */
  autoSyncOnWindowResize?: boolean;

  /**
   * Called when a tab is created.
   */
  onTabCreated?: (tab: TabState) => void;

  /**
   * Called when a tab is closed.
   */
  onTabClosed?: (tab: TabState) => void;

  /**
   * Called when a tab's state changes.
   */
  onTabChanged?: (tab: TabState) => void;

  /**
   * Called when an error occurs.
   */
  onError?: (error: Error) => void;
}
