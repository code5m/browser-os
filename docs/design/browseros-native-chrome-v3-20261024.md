# BrowserOS 顶部与原生收藏菜单 v3

日期：2026-10-24，Asia/Shanghai。范围：真实客户端顶部与收藏夹，不修改产品原型、数据库或 Git 编辑器。本记录独立于 v2，v2 的参与布局式收藏夹不再代表本阶段方案。

## 产品行为

顶部从左到右为后退、前进、刷新、主页、圆角地址栏、贡献入口和更多操作。标签区内部滚动，窗口控制保留在固定右侧。无边框窗口原有 mousedown 拖动路径不变。

地址栏展示当前网页、目录路径或 browseros 模块地址。聚焦不会把旧网页地址带到数据库等模块。Esc 恢复当前地址，Enter 执行实际导航；在未改动的模块地址上 Enter 不访问外部搜索引擎。

原生网页及宫格中，地址栏聚焦和输入不再插入建议布局行。方向键或“最近地址”按钮调用原生菜单；HTML 工作台保留受控的地址建议。原生菜单打开后不能继续像 Chrome 自动补全一样持续输入，这是本方案明确保留的差异，不以 HTML 浮层绕过。

收藏栏在标准顶部的各类页面均可显示，偏好保持。文件夹先于散落书签展示，组内保留现有顺序；旧数据没有混合排序元数据，因此不是恢复原始 Chrome 精确顺序。超出的条目进入“更多收藏”，不显示横向滚动轨道。“所有收藏”打开完整原生目录树，“收藏夹选项”提供显隐、管理及导入入口。

文件夹点击使用原生多级菜单，书签条目默认在当前标签页打开。菜单末尾的“在新标签页打开…”提供相同目录树；收藏栏书签右键提供当前/新标签页打开。原生子菜单内不支持 Chrome 式逐条右键菜单及中键行为，不能声称交互完全一致。

## 实现与权限

`src/composables/nativeMenu.ts` 封装 Tauri Menu.new、LogicalPosition、popup 和 close。原生适配器不放入无原生依赖的 shared/ui。组件不直接 invoke，不新增应用命令、不新增窗口、不改变 WebView 生命周期。现有 default capability 的 core:default 已包含 core:menu:default，本轮不修改权限文件，也不扩大远程网页权限。

菜单锚点为按钮 getBoundingClientRect 的 left/bottom，传逻辑像素，不乘 devicePixelRatio。GTK 负责菜单抓取、子菜单、边缘约束及长列表滚动。外观由系统主题决定，不承诺 Chrome 像素级外观。

弹出期间保留菜单资源；选择或取消后释放。重复打开被拦截，所属组件卸载后不再执行导航回调。创建、弹出及导航失败只进入现有状态栏，不恢复 HTML 替代菜单。

源码依据：本地 Tauri 2.11.5 menu/plugin.rs::popup、menu/menu.rs::ContextMenuBase::popup_inner；muda 0.19.3 gtk/mod.rs::show_context_menu 使用 popup_at_rect 并等待 selection_done/cancel。公开 API：[Tauri Menu](https://v2.tauri.app/reference/javascript/api/namespacemenu/)。这是现成原生菜单接入，不是修改 GTK 实现。

## 验收记录

- NM-1 原生适配器：`node scripts/check-native-shell-menus.mjs` 通过。验证逻辑坐标、子菜单回调、标签文字转义、非桌面错误、创建/弹出失败、释放、重复调用和卸载后禁止导航。此为传输 fixture，不是原生绘制证明。
- BM-1 实际 Vue 组件：`scripts/check-browser-ui-details.cjs` 通过。验证嵌套目录菜单数据及新标签页回调；原生菜单传输被明确替换为 fixture，不虚构 DOM 菜单。
- BM-2 显隐：隐藏、刷新、通过收藏夹选项恢复及 Shell 焦点 Ctrl+Shift+B 通过。原生网页焦点快捷键转发仍未增加。
- CH-1 地址布局：实际输入并 Enter 导航、聚焦前后收藏栏位置和顶部高度不变、切换数据库显示 browseros://database 通过。数据库只作为模块切换测试，不修改其业务功能。
- CH-2 标签与宽度：900/1200/1920px，36 个标签下窗口按钮仍可见，无页面横向溢出。原生窗口最大化、真实鼠标拖动另行验收。
- CH-3 事件与边界：窗口拖动、原生 WebView 遮挡、adaptive shell、UI product quality、Workspace 启动和图片懒加载检查通过。适配器独立 TypeScript 检查通过。
- NM-2 真实 X11：以新 deb 解包二进制、独立 D-Bus、Xvfb :97、隔离 XDG 数据启动，不触碰用户运行窗口。真实网页菜单覆盖、多级键盘展开、Enter 导航、Esc 关闭、60 条菜单滚动至末尾、1400px 屏幕右边缘自动向左避让均已观察；截图位于 `/tmp/browseros-native-menu-GtbZXK/`。这不是已安装入口的用户验收。
- NM-2b 最终包导航：默认选择嵌套书签时日志仍为 tab-1；选择“在新标签页打开…”才创建 tab-2。曾在实测中发现旧 openBrowser 入口会新建标签，已改用 navigateCurrent 并重新打包、验证。
- BM-3 原生显隐与同尺寸截图：最终包在 1854×1048 客户区中经原生选项隐藏/恢复收藏栏；`bar-hidden.png` 与 `same-viewport-menu.png` 分别保存结果。用户截图扣除左侧系统 Dock 66px 和顶部系统栏 32px 后同为 1854×1048；不复制系统外壳。顶部正常高度为标签 42px、导航 46px、收藏栏约 35px；原生菜单展开不增加这些高度。与 Chrome 仍有地址建议方式、系统菜单主题、图标和标签细节差异。
- NM-3 2 倍缩放：Xvfb :98 + GDK_SCALE=2 的菜单锚点为物理 (20,238)，对应逻辑 (10,119)，子菜单正常；但网页内容下方出现额外空白偏移，不能标记整个高 DPI 页面通过。未改锁定的原生定位代码，待独立分析。
- PACKAGE 安装与桌面：release deb 已生成并解包检查，desktop 入口正确。已安装二进制仍不同，sudo 非交互检查要求密码，用户正在运行的旧实例未关闭。因此安装版验收保持 GUI_PENDING；不得将 Xvfb 结果称为已安装版本 GUI_PASS。
- GATE 完整 pre-merge 失败：构建体积对比超限、既有 tauri-browser-tabs/AGENTS.md 危险区修改。未修改 baseline、阈值或未知用户文件。

最终包 SHA256：`e4527eb7983993436c66b12c6283bb6e12c4467279389f55005d54da0f5bdc52`；解包程序 SHA256：`97b95bbf2c51ba723460dd1452139310cc67aed47a0f5a1dc636506a56ced99a`。代码与自动检查结果分项见上，整体仍因门禁、高 DPI 与安装版验证存在阻断，不标记 ALL_PASS。

## 未完成差异

1. Chrome 连续输入式自动补全、精确图标/圆角/全部状态样式尚未等价；本阶段不引入自绘原生窗口。
2. favicon、标签拖拽排序不在本阶段新增实现中。
3. 原生菜单主题不同于 Chrome；菜单内部右键及中键动作不完全相同。
4. Wayland 实际桌面、高 DPI、屏幕边缘、长列表及键盘交互必须按平台证据分别结论，不能从组件测试推定。
5. 数据库与 IDEA/DBX 的布局差异仍待独立任务，不属于本阶段的已完成项。
