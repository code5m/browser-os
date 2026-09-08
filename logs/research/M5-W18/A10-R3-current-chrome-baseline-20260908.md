# A10 — M5-W18-R3 复核补充（第 3 波）：当前外壳常驻动作基线 + 挂账核验

- Lane：A10；分支 `codex/m5-w18-a10`；起点 `200f0f1`；接续 `2a0722b` / `7d60cfc`
- 时间：2026-09-08
- 目的：建立 **A10 自己**的当前外壳基线，使 A2（密度审计）交付后 A10 能做**差分复核**而不是重新数一遍；同时核验一条历史挂账。
- 口径：全部 `source-derived`（file:line），**未跑 GUI**；不替代 A2 的审计，只作对照基线。

---

## 1. 当前外壳常驻动作清单（基线）

### 1.1 一级导航（`src/stores/useLayoutStore.ts:33-39` `TOP_NAV_ITEMS`）

| # | view | icon | label |
|---|---|---|---|
| 1 | `home` | 🏠 | 主页 |
| 2 | `browser` | 📁 | 浏览 |
| 3 | `term` | 💻 | 终端 |
| 4 | `clip` | 📋 | 剪贴板 |
| 5 | `arts` | 📚 | 知识库 |

密度阈值（`:75-83`）：`full` ≥1180px 显示 5 个；`compact` ≥900px 显示前 3 个；`icon` <900px 显示前 2 个（`:90-95`）。

### 1.2 ☰ 菜单（`NAV_MENU_SECTIONS`，`:41-74`）—— 共 **16** 项

- 工作区（3）：文件、剪贴板、知识库
- 工具（11）：终端、应用、脚本库、命令库、工具箱、数据库、定时任务、技能、智能体、图谱、插件
- 同步（2）：仓库、审计

注：`term/clip/arts` 同时存在于一级导航与 ☰ 菜单（源码注释说明为窄窗可达性有意保留）。

### 1.3 其他常驻 chrome

- `ActivityBar`：主行 + 最多三个扩展行（`grid`/`more`/`omni`），`.omni-row { max-height: 74px }`
- `UnifiedTabBar`（`MainArea.vue:125`）：`v-show=!layout.compactMode`
- `StatusBar`（`StatusBar.vue:136` `height: 26px`）：约 10 段信息（连接态、当前视图、页签数、终端就绪、仓库配置、审计计数、内存 3 项、grid 分组 RSS）
- 精简模式 `layout.compactMode`：`App.vue` 中 `ActivityBar` 与 `MainArea` 内 `UnifiedTabBar` 一并 `v-show=false`

### 1.4 与用户裁决的对照（"去掉常驻功能按钮、默认更简单更宽敞"）

- 默认（≥1180px）常驻 **5 个一级按钮 + 地址行 + 状态栏**；未达"大卡片/大按钮"，但一级入口与 ☰ 菜单存在 **3 项重复**（term/clip/arts）。
- 当前外壳是**顶部横向导航**，不是 IDEA 的左侧图标栏。R3 若引入左/右工具窗口，必须回答"与顶部 ActivityBar 如何并存"，否则触犯 R3 明令的**不得重复导航**。
- 1440 折叠态侧栏上限 115.2px（主报告 §6）⇒ 现有"顶部整行"仍可保留，但**新增侧栏默认宽度不得按 260px 设计**。

---

## 2. 挂账核验：D6「MainArea 兜底 `v-else` 恒渲染」—— **已修复**

历史挂账（M5-14 债务 D6 / B10-b）："`MainArea` 兜底 `v-else` 恒渲染，W17/A7 引入回归"。

**A10 核验结果（当前 `master`）：已修复，非恒渲染。**

- 现为**白名单守卫的 `v-if`**，`MainArea.vue:250-258`：

```vue
<!-- ===== W17(A7) 兜底：未知/空视图时主区不得空白或死区 ===== -->
<!-- This is intentionally independent from FileEditor; an adjacent v-else
     would bind to the editor v-if and render during every normal view. -->
<div
  v-if="![
    'home', 'browser', 'grid', 'files', 'arts', 'clip', 'repo', 'apps', 'audit',
    'scripts', 'commands', 'tools', 'db', 'tasks', 'plugin', 'skills', 'agents',
    'graph', 'settings', 'term', 'editor'
  ].includes(layout.mainView)"
```

- 引入提交：`61cff56 feat(M5): integrate bug-hunt closeout and scheduler safety`（2026-09-07，`git log -S` 定位到该防回归注释）。
- 代码中唯一的其他 `v-else` 是 `MainArea.vue:166` 的 dock 页签兜底（`files/net/session` 之外回落终端），属有意行为。

**建议**：请 A0/A11 据此**关闭 D6 挂账**（保留本文件作为证据）；若 W19 重排 `MainArea`，须保留该白名单守卫与注释，不得改回相邻 `v-else`。

---

## 3. 给 A2 的核对口径（A10 将据此做差分）

A2 的"当前外壳密度审计"交付后，A10 将核对：

1. 一级常驻按钮数是否 = 5，密度阈值是否 = 1180/900（对不上须说明依据）；
2. ☰ 菜单项数是否 = 16，重复项（term/clip/arts）是否被标记；
3. 每个可见控件是否被归入五分类之一（常驻 / 溢出 / 右键菜单 / 命令入口 / 可删重复）；
4. 是否给出与 `A10-R3-viewport-budget.py` 同口径的 px/% 数字；
5. 是否覆盖 `StatusBar`（26px，约 10 段信息）与 `UnifiedTabBar` —— 二者常被密度审计遗漏。

---

## 4. 本波范围声明

- 仅新增 A10 lane 自有研究文件；**零产品代码、零依赖/ACL/capability 改动；未 push**。
- 未修改已提交的 `2a0722b` / `7d60cfc`（历史保留，蓝图 §8 第 8 条）。
- 未触碰他 lane 文件。
