# tauri-browser-tabs/AGENTS.md

该目录属于高级风险原生区域。

普通 UI、文案、业务状态任务不得修改本目录。

涉及以下行为必须进行真实桌面验收：

- WebView create/destroy
- show/hide
- move/resize
- size_allocate
- 主窗口与子窗口同步
- 弹窗期间 WebView 隐藏与恢复
- Wayland/X11