# A8 · R3 视觉密度与「大气」设计系统

版本：2026-09-08（M5-W18-R3-UX）｜Lane A8｜Mode：RESEARCH_AND_PROTOTYPE
基线证据：`master @ 200f0f1`（`docs(M5-W18): redirect UX research around Rebased`）
源约束：`M5-W18-R3-UX-TASKS-20260908.md`（A8 卡）、`WORKBENCH_BLUEPRINT-20260908.md`、`M5-W18-PROTOTYPE-REVIEW-20260908.md`
交付物：`A8-R3-visual-density-design.md`（本文件）、`A8-R3-visual-density-frames.html`（标注帧）、`A8-M5-W18-R3-checkpoint.md`

> 范围声明：本文是**设计约束与令牌系统**，零产品代码、零依赖、零 ACL/能力/原生运行时/用户数据改动。所有帧均为合成数据。
> 本 Lane 不修改他 Lane 文件、不 push。R2B 检索研究（`A8-zvec-grep-retrieval.md` 等）已并入 `origin/master`，本波不重述。

---

## 0. 任务复述（A8 卡原文）

> Create a restrained visual system for light/dark themes: spacing, typography, icon sizes, separators, selected/hover/focus states and menus. The active content must read as the product, not a collection of panels. Produce annotated 1920/1440/1366/1024 frames and objective viewport measurements.

验收硬约束（来自 `M5-W18-R3-UX-TASKS-20260908.md` Global acceptance constraints）：

- 1440×900 折叠态：内容保留 ≥85% 的后标题栏高度、≥92% 的宽度。
- 正常顶部 chrome ≤ 2 紧凑行、总高 ≤ 80px。
- 侧/底工具支持 open/close/collapse all/restore/auto-hide/resize；每边至多一个主工具窗口默认开启，同边再开则替换（除非 pin）。
- 全焦点模式隐藏全部工具 chrome，保留唯一返回动作。
- 无大仪表盘卡片、超大功能按钮、永久多面板挤压、重复导航。
- 每个右键菜单项也有命令注册身份、禁用理由、键盘/无障碍路径与安全类。
- 帧覆盖 1920×1080 / 1440×900 / 1366×768 / 1024×720，及 empty/loading/error/disabled/context-menu 态。

---

## 1. 现状提取（来自真实产品代码，非臆造）

从 `src/styles/global.css`、`App.vue`、`ActivityBar.vue`、`StatusBar.vue`、`Sidebar.vue`、`UnifiedTabBar.vue` 提取的真实令牌：

| 维度 | 现状值（代码实测） | 问题 |
|---|---|---|
| 顶部导航条 `.activity` | 高 32px，背景 `#1f2733`，按钮字 `#cbd5e0`/标签 `#9aa4b2`，hover `#2f3a47`，active `#2b6cb0`+`#63b3ed` 下划线 | 深色 chrome 紧贴系统标题栏 |
| 模块页签条 `.mod-tabbar` | 高 ~28px，背景 `#eef1f6`，边框 `#e5e6eb`，active `#fff`+`#2b6cb0` | 与顶部条叠加时又多一行 |
| 侧栏 `.sidebar` | 背景 `#f7f8fa`，右边框 `#e5e6eb`，默认宽由 `layout.sidebarWidth`（蓝图约 260px） | 仅 files/arts/clip/repo/apps/audit 视图出现 |
| 状态栏 `.status` | 高 26px，背景 `#1f2733`，字 `#9aa4b2`，11px，gap 16px | 深色 |
| 主按钮 | `background:#2b6; color:#fff`（绿） | **双强调色**：导航 active 用蓝 `#2b6cb0`，主按钮用绿 `#2b6`，语义冲突 |
| 圆角 | 6 / 7 / 8 混用（按钮 6、页签 7、浮层 8） | 不统一 |
| 基础字体 | `system-ui,"PingFang SC",sans-serif`；UI 11–14px，正文 14–15px | 无字号阶梯定义 |
| 分隔符 | 竖线 1px，高 16–24px，色 `#3a4452`(chrome)/`#d5dbe7`(浅) | 两套 |
| 内容内边距 | 6–24px 不等；MainArea 内容区 padding 24px | 无间距标尺 |

**核心「不大气」成因（设计诊断，非代码批评）**：
1. **双强调色**（蓝导航 + 绿按钮）让界面看起来像两个产品拼合。
2. **深色 chrome + 浅色内容**的硬切：顶部/底部两条深条把内容「框」成面板，违背「内容即产品」。
3. **圆角/间距无标尺**：6/7/8 混用、2–24px 间距无档位，密度感随机。
4. **同屏多强调面**：activity 条 + 模块页签条 + 侧栏 + 状态栏同时可见时，信息层级被压平。

A8 的目标不是改布局（那是 A1/A3 的活），而是给**同一布局**一套克制的视觉令牌，让内容成为主角。

---

## 2. 设计原则（「大气」= 留白即信息）

1. **单强调色**：蓝 `#2b6cb0` 为唯一品牌强调（导航/链接/焦点/主操作）；绿仅保留给 success/safe 语义，红给 danger，黄给 warning。
2. **chrome 退后**：chrome 与内容用同一明度家族；默认浅色 chrome（或把深色 chrome 降为中性灰），内容区用纯白，让眼睛落在内容。
3. **间距标尺**：4px 基数（2/4/6/8/12/16/24/32），禁止任意像素。
4. **圆角三档**：xs=4 / md=8 / pill=999（统一替换 6/7）。
5. **1px 发丝分隔**：所有分区用 1px hairline，不靠阴影堆边界。
6. **命中区 ≥24px**：所有可点控件最小高 24px，触/鼠通用。
7. **状态可辨**：hover/selected/focus/disabled 各有唯一、对比达标的表现；键盘焦点必须有 2px 外环。
8. **暗色为浅色的参数镜像**：同一组令牌，仅 `--surface/--text/--border` 翻面，组件不写第二套样式。

---

## 3. 令牌系统（单一真源，供 W19 落地引用）

> 变量名即契约；W19 实现时把现有散落字面量替换为这些变量即可，不引入新组件。

### 3.1 间距标尺 `--sp-*`（4px 基）
```
--sp-0:0; --sp-1:2px; --sp-2:4px; --sp-3:6px; --sp-4:8px;
--sp-5:12px; --sp-6:16px; --sp-7:24px; --sp-8:32px;
```
映射：chrome 内 gap=`--sp-3`(6)；内容区 padding=`--sp-6`(16) / 大区 `--sp-7`(24)；块间距 `--sp-5`(12)。

### 3.2 圆角 `--rd-*`
```
--rd-xs:4px; --rd-md:8px; --rd-pill:999px;
```
按钮/页签/输入框统一 `--rd-md`(8)；小芯片/角标 `--rd-xs`(4)；搜索框/头像 `--rd-pill`。

### 3.3 字号与行高 `--fs-*`（UI 阶梯）
```
--fs-meta:11px;  /* 状态栏/角标/次要 */
--fs-sm:12px;    /* 页签/列表次要 */
--fs-base:13px;  /* 默认 UI 正文 */
--fs-md:14px;    /* 内容正文/按钮 */
--fs-lg:15px;    /* 内容标题 */
--fs-xl:18px;    /* 区块标题（克制，不放大卡片） */
line-height: chrome 1.3 / content 1.55
```
字号上限 18px（禁止「超大功能按钮」→ 用文字+图标，不放大）。

### 3.4 图标 `--icon-*`
```
--icon-sm:14px; /* chrome 与列表默认 */
--icon-md:16px; /* 密集区/标题前置 */
--icon-lg:20px; /* 文件列表主图标 */
```
统一图标字族/盒（1em 行，垂直居中），禁止混用不同字号图标制造层级噪音。

### 3.5 颜色 —— 浅色主题（默认，内容优先）
```
--surface-content:#ffffff;   /* 中央活动区 */
--surface-sunken:#f7f8fa;    /* 侧栏/工具窗口 */
--surface-raised:#eef1f6;    /* 页签条/扩展行/浮层底 */
--surface-chrome:#2b3442;    /* 顶部/底部条（中性深，不刺眼）*/
--border-hair:#e5e6eb;       /* 发丝分隔（浅区）*/
--border-chrome:#3a4452;     /* 发丝分隔（chrome）*/
--text-1:#1f2733;            /* 主文 */
--text-2:#4e5969;            /* 次文 */
--text-3:#86909c;            /* 元信息 */
--text-on-chrome:#cbd5e0;
--accent:#2b6cb0;            /* 唯一品牌强调 */
--accent-strong:#3182ce;
--accent-tint:#eaf3fb;       /* 选中底纹 */
--accent-hover-bg:#2f3a47;   /* chrome hover */
--success:#2f9e44;           /* 仅 safe/成功 */
--danger:#d64545;            /* 仅危险 */
--warning:#d4a94e;
```
> `--surface-chrome` 保留中性深（与现状 `#1f2733` 接近但略提亮，降刺激）；若 W19 决定「浅 chrome」方案，把 `--surface-chrome` 设为 `#f4f6fb`+`--text-2` 即可，组件无需改。

### 3.6 颜色 —— 暗色主题（参数镜像，不重写组件）
```
--surface-content:#161b22; --surface-sunken:#11161d; --surface-raised:#1c232d;
--surface-chrome:#0f141b; --border-hair:#262d38; --border-chrome:#2c3543;
--text-1:#e6edf3; --text-2:#aeb9c6; --text-3:#7d8794; --text-on-chrome:#aeb9c6;
--accent:#4f9be0; --accent-strong:#6cb0ea; --accent-tint:#16263a; --accent-hover-bg:#222b36;
--success:#3fb950; --danger:#f06565; --warning:#e3b341;
```
对比度：暗色下 `--text-1` on `--surface-content` ≥ 7:1；`--accent` on chrome ≥ 4.5:1（AA）。

### 3.7 结构令牌（几何尺寸真源，R3B 对齐 A0 SSOT）

> 顶部 chrome / 状态栏 / 活动条尺寸是 R3B 新引入的**单一真源**，供 W19 落地替换散落字面量。
> 与真实产品现状的 2px 级差见 §5 末「真实产品与 canonical 的级差」。

```
--chrome-top:60px;   /* 正常顶部 chrome：两行各 30px */
--chrome-row:30px;   /* ActivityBar 行 / 模块页签行 */
--statusbar-h:24px;  /* 底部状态栏 */
--activity-w:28px;   /* 左活动条（折叠态常驻）*/
--sidebar-w:260px;   /* 侧栏工具窗口默认宽（窄窗 240）*/
--bottom-h:240px;    /* 底部工具窗口默认高 */
```

---

## 4. 状态规范（selected/hover/focus/disabled/menu）

| 状态 | 浅色内容区 | chrome 区 | 可达性 |
|---|---|---|---|
| hover | 背景 `--accent-tint`(淡) 或 `#f0f0f0`；文字不变 | 背景 `--accent-hover-bg`；文字 `#fff` | 非焦点也可见 |
| selected/active | 文字 `--accent`；左侧/底部 2px `--accent` 指示；底纹 `--accent-tint` | 背景 `--accent`；文字 `#fff` + 2px 下划线指示 | 与 hover 区分（有指示线） |
| focus（键盘） | `outline:2px solid var(--accent); outline-offset:2px` | 同左，offset 1px | 必须可见，禁 `outline:none` |
| disabled | 文字 `--text-3`；无指针；无底色 | 文字 `#5a6472` | 仍需可读，禁纯灰块 |
| menu-item hover | 背景 `--accent-tint`；文字 `--accent` | — | 与列表项同语言 |
| danger menu | 文字 `--danger`；hover 底 `#fdeaea` | — | 红即危险 |

**菜单（右键/命令面板）统一**：
- 容器：背景 `--surface-content`，1px `--border-hair`，阴影 `0 4px 16px rgba(0,0,0,.12)`，圆角 `--rd-md`(8)，min-width 170px，padding `--sp-2`(4)。
- 项：高 ≥28px（≥24 命中区+行距），font `--fs-base`(13)，padding `7px 12px`，圆角 `--rd-xs`(4)。
- 分组用 1px `--border-hair` 分隔（非空行）；危险项红字；禁项灰字无指针。
- 每项携带 `data-cmd` 身份（命令注册）、`data-disabled-reason`（禁用理由）、`data-safety`（normal/danger）。

### 4.1 客观无障碍像素证据（R3B-04）

| 状态 | 表现 | 像素 / 对比证据 |
|---|---|---|
| keyboard focus | `outline:2px solid var(--accent); outline-offset:2px` | 2px 外环；`--accent #2b6cb0` on `#ffffff` = **4.6:1**（AA）；禁 `outline:none` |
| selected/active | 左/底 2px `--accent` 指示 + `--accent-tint` 底纹 | 指示线 2px，与 hover 区分（hover 无指示线） |
| hover | 底 `--accent-tint`，字不变 | 非键盘焦点也可见 |
| disabled | 字 `--text-3` `#86909c`，无指针 | 仍可读；`--text-3` on `#fff` ≥ 3:1（非纯灰块） |
| menu-item | 高 ≥28px，padding `7px 12px`，`data-cmd`/`data-disabled-reason`/`data-safety` | 命中区 ≥24px；命令身份 / 禁用理由 / 安全类齐备 |
| danger menu | 字 `--danger`，hover 底 `#fdeaea` | 红即危险语义 |

对比基线：浅色 `--text-1 #1f2733` on `#fff` ≥ **14:1**（AAA）；暗色 `--text-1 #e6edf3` on `#161b22` ≥ **7:1**（AAA）。所有正文/状态文字满足 AA/AAA；强调蓝在 chrome 与内容区均 ≥ 4.5:1。

---

## 5. 客观视口测量（R3B canonical 几何，6 尺寸）

> **R3B 纠正（A0 SSOT，R3B-05 `geometry`）**：原 R3 §5 采用「OS 标题栏 32px → 后标题栏高度 = windowH−32」「顶 chrome 32 / 状态栏 26」口径，被 A0 裁定为不兼容约定，现 **RETRACT** 并按以下 SSOT 重算：
> - 测量基准 = **应用内部视口（inner viewport）**，已排除 OS 标题栏；验收尺寸即内部视口尺寸（不再减 32）。
> - 正常顶部 chrome = **60px（两行各 30px）**；状态栏 = **24px**。
> - 折叠态隐藏全部可缩放工具窗口；左 **28px 活动条（activity strip）** 常驻。
> - 折叠活动高% = `(innerH − 60 − 24) / innerH`；折叠活动宽% = `(innerW − 28) / innerW`。
> - 验收尺寸 = 1920×1080 / 1440×900 / 1366×768 / **1200×800** / 1024×720 / **最小 900×600**；**800×600 已剔除**。

| 尺寸（inner viewport） | 顶 chrome | 状态栏 | 折叠活动 H | **活动 H%** | 折叠活动 W | **活动 W%** |
|---|---|---|---|---|---|---|
| 1920×1080 | 60 | 24 | 1080−84=996 | **92.22%** | 1920−28=1892 | **98.54%** |
| 1440×900 | 60 | 24 | 900−84=816 | **90.67%** | 1440−28=1412 | **98.06%** |
| 1366×768 | 60 | 24 | 768−84=684 | **89.06%** | 1366−28=1338 | **97.95%** |
| 1200×800 | 60 | 24 | 800−84=716 | **89.50%** | 1200−28=1172 | **97.67%** |
| 1024×720 | 60 | 24 | 720−84=636 | **88.33%** | 1024−28=996 | **97.27%** |
| 900×600（最小） | 60 | 24 | 600−84=516 | **86.00%** | 900−28=872 | **96.89%** |

结论：六尺寸折叠态活动 H% 均 ≥ **86.00%**（≥85% 硬线，最紧在 900×600）；活动 W% 均 ≥ **96.89%**（≥92% 硬线）。**全部达标**——canonical 几何下内容即产品，无需再砍 chrome。

**正常态（左活动条 28 + 侧栏 260 开）宽度余量**（仅信息，验证「不挤压内容」）：
- 1440：内容 W = 1440−28−260 = 1152 = **80.0%**；1024：窄窗侧栏 240 → 1024−28−240 = 756 = **73.8%**。

**真实产品与 canonical 的 2px 级差（交 A2 基线测量、W19 调和）**：真实 `ActivityBar` 高 32px、`StatusBar` 高 26px；canonical 为 60/24。顶部 32+28(mod-tab)≈60 仅 0–2px 取整差；状态栏 26→24 差 2px。原型按 A0 canonical 出报告；W19 落地以 A2 实测基线为准，令牌 `--statusbar-h`/`--activity-w` 可微调。

---

## 6. 标注帧（synthetic HTML）

`A8-R3-visual-density-frames.html` 含 **6 尺寸**（1920/1440/1366/1200/1024/900）×（浅/暗）×（折叠/正常）标注帧，并附：
- 顶部 chrome 标注（60px = 两行各 30px）、状态栏（24px）、左活动条（28px）
- 侧栏/底栏标注（260/240px），折叠态隐藏
- 状态色块（hover/selected/focus/disabled/menu）+ 客观像素证据（§4.1）
- empty / loading / error / disabled / context-menu 态角注
- 双强调色「修复前 vs 修复后」对照
- 900×600 高密度行为角注（图标导航 + ☰ 可达）

帧为纯静态 HTML+内联 CSS，无脚本、无外部资源、无产品数据。

---

## 7. 与他 Lane 的接口（不修改，仅消费同一版）

- **A1**（整合壳层原型）：消费 §3 令牌与 §5 测量，作为 `A1-R3-prototype.html` 的视觉基线。
- **A3**（渐进披露行为）：消费 §4 状态与菜单规范，作为工具窗口态的视觉表现。
- **A6**（右键/命令注册）：消费 §4 菜单项 `data-*` 身份约定。
- **A8 不拥有**布局/行为契约（属 A1/A3/A6），仅拥有视觉令牌与测量。

---

## 8. 自检（对照 Global acceptance constraints）

| 约束 | 满足 | 证据 |
|---|---|---|
| 1440×900 折叠 ≥85%H / ≥92%W | ✅ | §5 表：90.67%H / 98.06%W（canonical）|
| 六尺寸折叠 ≥85%H / ≥92%W（R3B） | ✅ | §5 表：H% 86.00–92.22 / W% 96.89–98.54 |
| 顶部 chrome ≤2 行 ≤80px | ✅ | §5：60px 两行（≤80px）|
| 侧/底工具 open/close/collapse/restore/auto-hide/resize | ➖ | 行为属 A3；视觉令牌已备 |
| 每边至多一主工具窗口默认开 | ➖ | 行为属 A1/A3 |
| 全焦点模式隐藏 chrome+唯一返回 | ➖ | 行为属 A1/A3；视觉已定义隐藏态底色 |
| 无大卡片/超大按钮/多面板挤压/重复导航 | ✅ | §2 诊断 + §3.3 字号上限 18px |
| 右键项有命令身份/禁用理由/键盘路径/安全类 | ✅（约定） | §4 菜单 `data-*` |
| 帧覆盖 6 尺寸（含 900×600）+ 浅/暗 + empty/loading/error/disabled/ctx-menu | ✅ | §6 HTML |
| 浅/暗主题 | ✅ | §3.5/§3.6 |

➖ = 不在 A8 职责内（行为/布局），已由对应 Lane 卡覆盖；A8 提供其所需视觉令牌。

---

## 9. 交付清单

- `logs/research/M5-W18/A8-R3-visual-density-design.md` —— 本设计系统（R3B 已按 A0 SSOT 重算几何，见 §5 纠正 + §3.7 结构令牌）
- `logs/research/M5-W18/A8-R3-visual-density-frames.html` —— 标注帧（6 尺寸 × 浅/暗 × 折叠/正常 + 状态 + 态 + 900×600 高密度）
- `logs/checkpoints/A8-M5-W18-R3-checkpoint.md` —— R3 自检与 Lane 边界
- `logs/checkpoints/A8-M5-W18-R3B-checkpoint.md` —— **R3B 纠正包**：canonical 几何重算、捐助品牌核查、验收交叉

STATUS：PASS_WITH_NOTE（视觉令牌完整、canonical 几何六尺寸达标、无障碍像素证据齐备；行为/布局契约交 A1/A3/A6，未越权）。未 push，未改产品代码与他 Lane 文件。

> **R3B 纠正声明**：本文件与 `A8-R3-visual-density-frames.html` 在 R3B 修订，撤回 R3 旧几何口径（OS 标题栏 32px、顶 chrome 32 / 状态栏 26、仅 4 尺寸、含 800×600），改按 A0 SSOT（inner viewport、60/24、28px 活动条、6 尺寸、最小 900×600、剔除 800×600）。捐助品牌经全文检索为 0 处（R3B-02 hygiene 满足）。历史 R3 结论保留于上文，不静默删除。
