# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/lang/zh-CN/).

## [Unreleased]

## [0.1.0] - 2026-08-15

首个公开版本。源自 mvp-browser-os 项目中多页签子 webview 的真实踩坑与修复。

### Added

- `tauri-plugin-browser-tabs`：Rust 插件
  - 6 个 command：`create_tab` / `update_rect` / `set_visible` / `close_tab` / `navigate` / `list_tabs`
  - 全链路 Logical（CSS 像素）坐标，消除 Physical/Logical 混用
  - Linux WebKitGTK `size_allocate` 强制修复（解决子 webview 卡初始尺寸问题）
  - 懒绑定宿主窗口：兼容在应用 `.setup()` 中手动创建窗口的模式
  - `window.open` / `target="_blank"` 统一拦截为 `newWindowRequested` 事件
  - `initialization_script` 注入支持
  - 宿主窗口 resize 事件转发（`browser-tabs://window-resized`）
- `@tauri-browser-tabs/core`：框架无关 TypeScript 库
  - `BrowserTabManager`：ResizeObserver + 16ms 防抖 + rect 相等跳过
  - 低层 command 封装（`createTab` / `updateRect` / ...）
  - 几何工具（`domRectToLogical` / `rectsEqual` / ...）
- `@tauri-browser-tabs/vue`：Vue 3 适配
  - `useBrowserTabs()` composable
  - `<BrowserHost>` 组件
- `examples/basic`：最小可运行示例
- 文档：README / ARCHITECTURE / MIGRATION / TROUBLESHOOTING / CONTRIBUTING

### Notes

- 依赖 Tauri v2 `unstable` feature（`add_child` / `WebviewBuilder` / `with_webview`）。
- 仅支持桌面端（Windows / macOS / Linux）。
