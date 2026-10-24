# BrowserOS 浏览器外壳、数据库与 Git 细节复核 v2

日期：2026-10-24（Asia/Shanghai）。模型：GPT-6。
范围：修正用户截图中的实际应用问题；原型文件保留。

本记录取代旧审核中“只显示变更 / 显示全部、编辑器、Merge/初始提交全部通过”的笼统结论。旧检查证明了入口和源码存在，没有充分证明最终界面和组合行为。

## 事实与原因

- 数据库截图明确报 `DB_CONNECT_FAILED: IoError { server disconnected }`，因此 `表(0)` 不代表数据库为空。此前 `SELECT 1` 的单次结果不足以验证表列表。
- 用已保存的数据源及系统密钥库，经原生驱动、真实查询安全闸门执行对象树 SQL，修改前后各连续建立六次连接，每次返回 102 个对象；断线暂未再次复现，不能据此虚构固定根因。驱动的 `server disconnected` 对应 BrokenPipe。现在对握手阶段的 BrokenPipe/EOF/reset/aborted 只重试一次，TLS 参数保持相同；查询和事务不重放。
- 收藏栏原来直接遍历全部书签，丢失 category 文件夹层级，并以横向滚动处理溢出；没有显隐偏好。
- 数据库检查器引用了未定义的 `config`，导致引擎和权限等元数据错误兜底。
- Git 初始提交提示的 `v-else-if` 原来绑定“父提交数量大于一”的条件，普通提交也落入“初始提交”。
- 历史 Diff 原来过滤掉全部上下文，既没有折叠占位，也没有展开真实补丁间隙；每行独立横向滚动。它不是 IDEA 的折叠 Diff。
- 最后复核修正暂存 Diff 的内容来源：HEAD→暂存区使用 index 快照，暂存区→工作区才使用 worktree，避免折叠上下文混入尚未暂存的内容。

## 参照证据

Rebased 固定源码版本：`db13477117db68585f8f9f814255926ca779ee1b`。截图参照为本轮用户提供的 Chrome/Edge 及 BrowserOS 实际截图。以下属于源码/文档确认，不声称启动了完整 Rebased 或做到像素一致。

- [SimpleDiffViewer.java](https://github.com/DetachHead/rebased/blob/db13477117db68585f8f9f814255926ca779ee1b/platform/diff-impl/src/com/intellij/diff/tools/simple/SimpleDiffViewer.java)：`createToolbarActions`、`MyFoldingModel`、`MySyncScrollable`、`getDifferenceIterable` 分别确定展开/折叠、同步滚动、变更块导航；`createEditorPopupChangesActions` 区分替换和追加。只读历史不能冒充可写本地版本。
- [IDEA Diff Viewer](https://www.jetbrains.com/help/idea/differences-viewer.html)：双栏/统一、未变更段折叠、F7/Shift+F7、行对齐、同步滚动，以及接受/追加的不同含义。文字/字符级高亮与行级高亮是不同能力。
- [Chrome 收藏夹说明](https://support.google.com/chrome/answer/188842)：收藏栏显隐和收藏管理应有入口。用户截图补充了文件夹、单行排列、溢出按钮的视觉参照。

## 当前实现及验收记录

| ID | 预期 | 应用位置 | 证据及状态 |
| --- | --- | --- | --- |
| DB-1 | 加载失败不显示“空表”的确定结论 | `useDatabaseStore.refreshSchema`、`DatabasePanel` | 浏览器模拟首次握手失败，计数为 `—`，显示重试入口；重试后显示 102 行。UI fixture PASS；真实原生对象查询 PASS |
| DB-2 | 使用已保存凭据连接，安全策略照常执行 | `database.rs::mysql_auto_tls_readonly_e2e` | 实际读取 metadata+OS keyring，六次 `SELECT 1` 及 schema SQL PASS；未写业务数据 |
| DB-3 | 握手偶发断开可有限恢复 | `mysql_connect_handshake` | Rust 编译通过；重试仅发生在建立连接阶段，同模式一次。真实偶发故障未重新触发，不能宣称根因已证实 |
| BM-1 | 保留文件夹和子目录 | `bookmarkTree`、`BookmarkBar` | 测试“书签栏/测试环境/子目录”逐级打开到网页 PASS |
| BM-2 | 单行无横向轨道，溢出可访问 | `BookmarkBar.measure` | 900/1200/1920 三种宽度无页面横溢；更多收藏可打开。浏览器组件测试 PASS |
| BM-3 | 隐藏后可恢复，刷新保持偏好 | `useBookmarkStore.toggleBar`、地址栏按钮 | 隐藏→刷新→恢复 PASS；Ctrl+Shift+B 在主界面焦点时 PASS，原生网页焦点不在本次快捷键验收内 |
| CH-1 | 标签条、导航、圆角地址栏、收藏栏按 Chrome/Edge 风格 | `UnifiedTabBar`、`ActivityBar`、chrome tokens | 检查已渲染的真实 Vue 组件截图；36 标签/900px 下标签单独滚动，窗口按钮固定可见；构建及原有拖动事件检查 PASS。GTK 原生拖动未重新验收 |
| GT-1 | 两侧行对齐、前后版本标识、长行在面板内横向滚动 | `GitHistoryDiff` | 插入/删除行号单测；两栏变更行几何对齐、垂直滚动同步测试 PASS |
| GT-2 | 展开单个未变更段，展开全部真实内容，变更块导航 | `historyContextRows`、`GitHistoryDiff` | 65 行真实 fixture blob，展开后两栏合计 130 行；折叠占位展开与下一差异聚焦 PASS |
| GT-3 | 点击提交加载第一个文件；初始提交提示准确 | `selectCommit`、`selectCommitFile`、父提交提示条件 | 已修正源码；现有 Git SFC 状态/来源隔离测试 PASS。尚未用安装版逐项点击确认 |

组件测试为显式模拟 IPC，不能替代安装版、真实 MySQL 或真实 Git 写入验收。测试入口：`scripts/check-browser-ui-details.cjs`；截图输出 `/tmp/browseros-{chrome,database,git}-details.png`。原生只读测试不会记录密码或表内容。

## 确认仍有差距，继续保持未完成

1. **三方块级冲突算法和块接受/追加/忽略仍未达到 IDEA。** 当前后端 `studio.rs` 中名为 `hunks` 的数据实际上按相同行号比较 ours/theirs；一侧插入行后就可能错位。不能把它宣传为真实 diff3 hunk。后续要以共同祖先的区间为基准，构建稳定的三方区间与结果映射，再绑定小箭头、已解决状态、撤销/重做。
2. **历史版本只读与本地可编辑结果还没有完整的统一编辑器。** 当前历史 Diff 明确只读；工作区写入继续通过预览确认。尚不具备 IDEA 本地编辑器的光标、选区和即时重算 Diff。
3. **词/字符级高亮、空白忽略规则、代码语法高亮和中间连接图形尚缺。** 目前改善的是行级差异及可读性。
4. **二进制/图片比较与复杂重命名需专项验收。** 现有文本查看不能替代这些场景。
5. **收藏文件夹使用参与布局的展开区域，非原生弹出菜单。** 这是现有 Linux 子 WebView 约束下的实现；HTML 浮层不能可靠覆盖原生网页。若要与 Chrome 原生菜单同样弹出，需要独立的原生菜单接入及桌面验证，不使用 z-index 绕过。
6. **顶部标签的拖拽排序、真实网站 favicon、所有键盘/鼠标组合仍有差距。** 当前保留原有唯一窗口拖动路径，只改变视觉和入口排列。原生网页获得键盘焦点后，收藏栏快捷键还需要原生转发支持；现有按钮可直接显隐，因此不在按钮上宣传未验收的全局快捷键。
7. **真实安装版尚需冷启动与数据源点击验收。** 组件图、编译和原生只读测试不能标记 GUI_PASS；sudo 所需密码由用户在自己的终端输入。

后续优先顺序：安装版表树与收藏栏验证 → 真正的三方区间/冲突操作 → 本地编辑和撤销 → 细粒度高亮/空白规则 → 二进制与菜单/标签交互。

## 本轮源码指纹

以下 SHA-256 用于对应本轮测试源码，不表示已部署到已安装客户端。

```text
6fd16e3e1515e53fcbae6ae4acd3385f085d1f3efae1082e3c409aa3bba40c18  ActivityBar.vue
e89e3185f28eaf7bf09ea0eefc3d94aca7f2783c2faf7c9b93c0dd5f72e5ca39  UnifiedTabBar.vue
3273298e8623ad0c343338fabd3388cef3989ebf713a53f2a97991feecaa5d1e  BookmarkBar.vue
957ad4bb10971e0b1e3ea0013a45b45721b69e87aa70f28e29662bbd7cae8553  GitHistoryDiff.vue
dffdc96a1aa47d0477f3de2f7079f8aa0d6a594a12db769e2cd1617a410b969c  DatabasePanel.vue
db9146edd5634d9367c90753605f1a848db91077fe8c6cd60f1dd2dfccc3b0e8  database.rs
```
