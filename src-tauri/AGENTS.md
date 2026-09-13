# src-tauri/AGENTS.md

## 原生代码规则

该目录涉及 Tauri、Rust、GTK/WebKitGTK 和 Linux 窗口系统。

修改前必须：

1. 阅读根目录 `AGENTS.md`
2. 阅读 `PROJECT-RULES.md`
3. 确认当前任务明确授权修改原生代码
4. 说明涉及的 Tauri Command 和调用方
5. 说明失败及回滚行为

禁止：

- 在多个位置重复进行 scale factor 转换
- 根据前端 CSS 猜测物理像素
- 仅凭编译成功宣布窗口行为正常
- 在没有 revision 或时序说明的情况下增加异步场景同步
- 修改窗口拖动路径而不运行拖动回归检查