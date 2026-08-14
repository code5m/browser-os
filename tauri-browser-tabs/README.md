# tauri-browser-tabs

A Tauri v2 plugin for embedding multiple browser tabs as child webviews.

## Why

Tauri v2 introduced `Window::add_child` for embedding child webviews, but getting the sizing and positioning right is surprisingly hard:

- **Coordinate system confusion**: `LogicalPosition` vs `PhysicalPosition`, and when to use which.
- **Linux WebKitGTK resize bug**: child webviews can get "stuck" at an initial size and ignore subsequent `set_size` calls.
- **Async timing issues**: DOM layout, IPC, and native window events don't line up naturally.
- **Platform differences**: macOS, Windows, and Linux all behave slightly differently.

This plugin solves these problems by:

1. **Using logical coordinates (CSS pixels) exclusively** — no more Physical/Logical mixing.
2. **Providing platform-specific fixes** — especially for Linux WebKitGTK's `size_allocate` caching behavior.
3. **Managing debouncing and event sync** — so your frontend doesn't flood the backend with resize commands.
4. **Offering both low-level commands and a high-level manager** — use what's right for your app.

## Packages

| Package | Description |
|---------|-------------|
| `tauri-plugin-browser-tabs` | Rust Tauri plugin |
| `@tauri-browser-tabs/core` | Framework-agnostic TypeScript manager |
| `@tauri-browser-tabs/vue` | Vue 3 composables and components |

## Installation

### Rust

```toml
[dependencies]
tauri-plugin-browser-tabs = "0.1"
```

### Frontend

```bash
npm install @tauri-browser-tabs/core @tauri-browser-tabs/vue
```

## Quick Start

### 1. Register the plugin

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_browser_tabs::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 2. Add permissions

```json
// src-tauri/capabilities/default.json
{
  "permissions": [
    "browser-tabs:default"
  ]
}
```

### 3. Use in Vue

```vue
<script setup lang="ts">
import { BrowserHost } from '@tauri-browser-tabs/vue';
</script>

<template>
  <div class="app">
    <div class="toolbar">
      <button @click="createTab">New Tab</button>
    </div>
    <BrowserHost
      ref="host"
      class="browser-area"
      @tab-created="onTabCreated"
    />
  </div>
</template>

<script lang="ts">
import { ref } from 'vue';

const host = ref<InstanceType<typeof BrowserHost>>();

let counter = 0;

function createTab() {
  host.value?.createTab({
    id: `tab-${++counter}`,
    url: 'https://example.com',
  });
}

function onTabCreated(tab: TabState) {
  console.log('created', tab);
}
</script>

<style>
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
}
.toolbar {
  height: 40px;
  flex-shrink: 0;
}
.browser-area {
  flex: 1;
  min-height: 0;
}
</style>
```

### 4. Or use the low-level API

```ts
import { createTab, updateRect, closeTab } from '@tauri-browser-tabs/core';

await createTab({
  id: 'tab-1',
  url: 'https://example.com',
  rect: { x: 0, y: 40, width: 1200, height: 760 },
});

await updateRect('tab-1', { x: 0, y: 40, width: 800, height: 600 });
await closeTab('tab-1');
```

## API Reference

### Rust Plugin Commands

All commands are prefixed with `plugin:browser-tabs|`.

| Command | Arguments | Description |
|---------|-----------|-------------|
| `create_tab` | `options: CreateTabOptions` | Create a new child webview |
| `update_rect` | `id: TabId, rect: LogicalRect` | Update position and size |
| `set_visible` | `id: TabId, visible: bool` | Show or hide |
| `close_tab` | `id: TabId` | Close and destroy |
| `navigate` | `id: TabId, url: String` | Navigate to URL |
| `list_tabs` | — | List all tab IDs |

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `browser-tabs://event` | `BrowserTabEvent` | Navigation / `newWindowRequested` (window.open interception) |
| `browser-tabs://window-resized` | `()` | Host window was resized |

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — 设计决策：坐标系统一、平台隔离、懒绑定、线程模型
- [Migration Guide](docs/MIGRATION.md) — 从手写 `add_child` 迁移到本插件（真实项目案例）
- [Troubleshooting](docs/TROUBLESHOOTING.md) — 卡 400px、Wayland 崩溃、DPR 错位等症状索引
- [Contributing](CONTRIBUTING.md)

### Types

```ts
interface LogicalRect {
  x: number;      // CSS pixels
  y: number;      // CSS pixels
  width: number;  // CSS pixels
  height: number; // CSS pixels
}

interface CreateTabOptions {
  id: string;
  url: string;
  rect: LogicalRect;
  visible?: boolean;      // default: true
  autoResize?: boolean;   // default: true
  userAgent?: string;
  transparent?: boolean;  // default: false
  initializationScript?: string; // JS injected before any page script
}

interface TabState {
  id: string;
  url: string;
  title: string;
  favicon?: string;
  visible: boolean;
  rect: LogicalRect;
  createdAt: number;
  updatedAt: number;
}
```

## How It Works

### Coordinate System

This plugin uses **logical pixels (CSS pixels)** exclusively. The frontend sends `getBoundingClientRect()` values directly, and the Rust backend uses `LogicalPosition` / `LogicalSize` when calling Tauri APIs. This avoids the confusion of mixing physical and logical coordinates.

### Linux WebKitGTK Fix

On Linux, WebKitGTK sometimes caches the initial `size_allocate` of a `WebKitWebView` and ignores later resize requests. The plugin works around this by:

1. Creating child webviews with the correct initial size (not `1x1` then resize).
2. Forcing a `size_allocate` on the underlying GTK widget when the allocation doesn't match the expected size.

### Debouncing

The frontend manager debounces rect updates using `requestAnimationFrame`-aligned timeouts (default 16ms). This prevents IPC flooding during continuous resize operations.

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux | ✅ | Full support with WebKitGTK fixes |
| macOS | ✅ | Standard Tauri behavior |
| Windows | ✅ | Standard Tauri behavior |
| Android | 🚧 | Planned |
| iOS | 🚧 | Planned |

## Development

```bash
# Install dependencies
npm install

# Build all packages
npm run build

# Run example
npm run example:dev
```

## Contributing

Issues and PRs are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT OR Apache-2.0
