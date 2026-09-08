# A0 M5-W18 工作台集中实现 checkpoint

日期：2026-09-09
目录：/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
分支：master
起点：m5-w18-r3b-accepted / 58e50b1

## 范围

用户授权一次完成四组功能：浏览器式外壳与工具窗、宫格 AI 回复提取/复制/Markdown 归档、Obsidian Vault 浏览/链接/局部图谱、SQLite 与 Git 实际工作流补齐。

## 结果

- 外壳：统一命令入口、页签右键菜单、工具窗折叠/恢复、浏览器/模块导航和窄窗回归。
- 宫格：通过原生 UDS 请求读取固定站点注入脚本结果；每格独立失败/重试；成功项批量复制和逐文件原子保存；脚本和后端均在桥接前脱敏，回复/文件有界。
- Vault：允许根目录内有界 Markdown 读取、隐藏目录/符号链接跳过、搜索、Wiki 链接、反向链接和局部图谱。
- 数据库：SQLite 连接、对象树、独立查询页签、分页/排序/CSV、查询取消；写操作继续后端风险和确认闸门。
- Git：沿用现有写闸门，增加历史/commit diff 只读浏览，异步过期结果不覆盖当前仓库。

## 验证

cargo test 450/450 + 核心 2/2；release build 通过；npm run build 通过；工作台逻辑/UI 回归通过；隔离提取脚本通过；真实 Tauri smoke 17 项通过；bash scripts/pre-merge.sh 在最终提交后通过。

## 不宣称

不宣称 MySQL/PostgreSQL 已实现，不宣称真实登录第三方 AI 页面已完成，不宣称已运行用户本地的 Obsidian 或 rebased 源码。桌面视觉最后一步仍需用户在客户端窗口确认。
