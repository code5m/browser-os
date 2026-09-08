# A1 · M5-W18-R3B Corrected Browser-First Workbench（终稿，消费 A2-A9 R3B 修正）

> Lane: A1（RESEARCH_AND_PROTOTYPE，R3B correction wave）
> BASE = `origin/master` `d96b9b5`（docs(M5-W18-R3): integrate UX research and dispatch corrections）
> Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3` @ branch `codex/m5-w18-a1`
> 依据：`M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A1；`A0-M5-W18-R3-acceptance-audit-20260908.md`（A0 rulings）
> 消费：A2(bfe8e86) · A3(8d8719a) · A4(5d9e447) · A5(3ff1f8b) · A6(2bce4e3) · A7(b96cd51) · A8(ba92110) · A9(d17091c)
> 边界：只读研究 + 自包含合成 HTML 原型；零产品代码改动；不 push。

---

## 0. 输出头

```text
LANE=A1
DISPATCH=M5-W18-R3B
STATUS=FINAL_READY_FOR_A10_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=d96b9b5
HEAD=<pending commit>
CONSUMED_PEERS=A2(bfe8e86), A3(8d8719a), A4(5d9e447), A5(3ff1f8b), A6(2bce4e3), A7(b96cd51), A8(ba92110), A9(d17091c)
FILES=logs/research/M5-W18/A1-R3B-browser-workbench.md, logs/research/M5-W18/A1-R3B-prototype.html, logs/research/M5-W18/A1-R3B-checkpoint.md
CORRECTIONS=R3B-01 Git 14 units visible; R3B-02 donor branding removed from UI; R3B-03 collapse semantics aligned with A0; R3B-04 ARIA/role/keyboard added; R3B-05 geometry SSOT per A0 formula; R3B-06 Ctrl+K/Ctrl+Shift+P frozen distinct; R3B-07 Rebased pin cee14e9 + 4/10 totals; R3B-08 900x600 added, 800x600 dropped; R3B-09 native NOT_RUN
VERIFY=HTML prototype browser-openable; A2 measure.py PASS (6 sizes); A6 registry-audit.mjs PASS; A3 58 assertions PASS; A4 29/29 pure tests PASS; A5 78 checks PASS; no build or product test run
PROPOSED_SLICES=none (R3B is prototype research, not coding authorization)
OPEN_DECISIONS=A0 裁定 F05(体积余量)/F12(WebView owner)/F01(license); W19 实现时 A6 注册表 + A7 Git 后端为前置
NEXT=A10 独立复核；A11 验收打包
NO_PRODUCT_CODE=true
NO_PUSH=true
```

---

## 1. R3 验收审计与 R3B 修正波次

### 1.1 A0 审计裁定（`A0-M5-W18-R3-acceptance-audit-20260908.md`）

Verdict: `REVISE_TARGETED`。9 项阻塞发现：

| # | 发现 | A0 ruling |
|---|---|---|
| R3B-01 | Git 14 单元未完整覆盖 | worktree/amend/reset-revert/conflict/patch/command-log 必须可见 |
| R3B-02 | 原型 UI 含 donor 品牌 | donor 名仅限报告，不出现在产品原型 UI |
| R3B-03 | Collapse All 与 pin 语义分歧 | Collapse All 隐藏所有（含 pinned），记录一个可恢复快照 |
| R3B-04 | 无完整 role/aria/keyboard | 每个交互元素须有 role、aria、focus、disabled reason |
| R3B-05 | 几何口径不统一 | SSOT: 60px top, 24px status, 28px strip; 公式见下 |
| R3B-06 | Ctrl+K 与 Ctrl+Shift+P 冲突 | Ctrl+K=地址/搜索, Ctrl+Shift+P=命令面板 |
| R3B-07 | Rebased 版本与分类不一致 | SSOT=v1.1.15@cee14e9; COPY=0/ADAPT_PRODUCT=4/REIMPLEMENT=10 |
| R3B-08 | 900x600 未覆盖, 800x600 不应出现 | 六尺寸: 1920/1440/1366/1200/1024/900×600 |
| R3B-09 | 原生 WebView 未验证 | 记 NOT_RUN，归 A9/A11 |

### 1.2 R3B 任务（`M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A1）

> "Revise the existing A1 report and HTML, consuming the corrected A2-A9 outputs. Keep two 30px browser rows, the 28px activity strip and the large content surface. Demonstrate all six target sizes, collapsed/restore/focus behavior, keyboard focus, context menus and failure states. The Git mode must visibly locate all 14 workflow units, including worktrees, amend, reset/revert, conflicts, patch and command log; advanced actions belong in menus/palette rather than permanent buttons. Add semantic roles and ARIA. Remove donor names from UI."

---

## 2. 修正内容

### 2.1 R3B-01 Git 14 单元完整覆盖

原型现在在三个位置可见全部 14 单元：

1. **Git unit grid**（content 区）：14 个 `git-unit` 卡片，每个含 command ID + U编号 + 分类标签
2. **Context menu**（右键菜单）：20 个 Git 菜单项（14 单元 + amend/reset/revert/rebase-continue/abort 等子操作）
3. **Command palette**（Ctrl+Shift+P）：24 个 Git 命令条目

| # | 单元 | command ID | 分类 (A7) | 菜单 | 面板 |
|---|---|---|---|---|---|
| U01 | status | `git.status.refresh` | ADAPT_PRODUCT | ✅ | ✅ |
| U02 | hunk staging | `git.hunk.stage` | REIMPLEMENT | ✅ | ✅ |
| U03 | diff | `git.file.diff` | ADAPT_PRODUCT | ✅ | ✅ |
| U04 | log/graph | `git.log.graph` | REIMPLEMENT | ✅ | ✅ |
| U05 | branch | `git.branch.list` | ADAPT_PRODUCT | ✅ | ✅ |
| U06 | worktree | `git.worktree.add` | REIMPLEMENT | ✅ | ✅ |
| U07 | stash | `git.stash.push` | REIMPLEMENT | ✅ | ✅ |
| U08 | merge | `git.branch.merge` | ADAPT_PRODUCT | ✅ | ✅ |
| U09 | rebase | `git.rebase` | REIMPLEMENT | ✅ | ✅ |
| U10 | cherry-pick | `git.commit.cherrypick` | REIMPLEMENT | ✅ | ✅ |
| U11 | conflict | `git.conflict.resolve` | REIMPLEMENT | ✅ | ✅ |
| U12 | blame/history | `git.blame` | REIMPLEMENT | ✅ | ✅ |
| U13 | patch | `git.patch.apply` | REIMPLEMENT | ✅ | ✅ |
| U14 | command log | `git.console` | REIMPLEMENT | ✅ | ✅ |

**额外可见操作**（A0 要求）：amend (`git.commit.amend`), reset (`git.commit.reset`), revert (`git.commit.revert`), rebase continue/abort, push (disabled with reason "No staged changes").

分类总计：**COPY=0, ADAPT_PRODUCT=4, REIMPLEMENT_FROM_BEHAVIOR=10**（A0 ruling 43, A7 b96cd51 确认）。

### 2.2 R3B-02 donor 品牌移除

原型 UI 中已移除所有 donor 名称：
- "Rebased" → "Behavior SSOT: v1.1.15@cee14e9 (reference only)"
- "JetBrains" / "Obsidian" / "IntelliJ" / "DetachHead" → 0 出现

品牌引用仅出现在本报告（`A1-R3B-browser-workbench.md`）中，不在产品原型 UI。

### 2.3 R3B-03 Collapse 语义对齐 (A0/A3/A4)

采纳 A0 ruling：
- **Collapse All**：隐藏所有工具窗口（含 pinned）。记录一个可恢复快照。
- **Restore Layout**：从快照恢复 visibility、dimensions、pin states。确定性。
- **Pin**：仅影响普通替换和 auto-hide。Pinned 窗口在 Collapse All 时仍隐藏。
- 瞬态 hover/overlay 不持久化（A4 5d9e447）。

A3(8d8719a) 和 A4(5d9e447) 已对齐此语义。原型 `collapseAll()` 函数实现此行为。

### 2.4 R3B-04 无障碍 (A6/A8)

原型现在包含：
- **94 个 `role=` 属性**：toolbar, tablist, tab, search, main, menu, menuitem, dialog, tree, treeitem, status, button, complementary, group, option, listbox
- **127 个 `aria-*` 属性**：aria-label, aria-selected, aria-hidden, aria-disabled, aria-pressed, aria-expanded, aria-modal
- **78 个 `tabindex`**：键盘可遍历
- **`:focus-visible`**：所有交互元素有焦点环
- **`aria-disabled` + `title`**：禁用项显示原因（如 "No staged changes"）
- **菜单键盘**：↑/↓ 导航, Enter 执行, Esc 关闭并返回焦点

### 2.5 R3B-05 几何 SSOT (A0/A2)

采纳 A0 公式为硬常量：
```css
--top-chrome-h: 60px;  /* 2 × 30px */
--row-h: 30px;
--status-h: 24px;
--strip-w: 28px;
```

折叠态：
- active H = (innerH − 60 − 24) / innerH
- active W = (W − 28) / W

A2(bfe8e86) 实测六尺寸全部 PASS，最紧 900×600 = 86.00% H（余量 1.00pp）。

### 2.6 R3B-06 快捷键冻结 (A0/A6)

| Chord | Command ID | Scope | 行为 |
|---|---|---|---|
| `Ctrl+K` | `browser.focusAddressSearch` | browser.page | 聚焦地址/搜索栏 |
| `Ctrl+Shift+P` | `app.openCommandPalette` | app | 打开命令面板 |

原型 JS 实现两个独立快捷键，无冲突。A6(2bce4e3) registry-audit.mjs 确认无重复 chord。

### 2.7 R3B-07 参考身份 (A7)

- Rebased behavior SSOT = `v1.1.15` @ `cee14e9`
- `2896562e` 仅作 later research snapshot
- 14 单元分类：COPY=0, ADAPT_PRODUCT=4, REIMPLEMENT_FROM_BEHAVIOR=10
- ADAPT_PRODUCT = 扩展本产品现有实现，非移植 donor 源码

### 2.8 R3B-08 尺寸集 (A0/A2)

六尺寸全覆盖：1920×1080 / 1440×900 / 1366×768 / 1200×800 / 1024×720 / 900×600

800×600 已剔除（产品 `min_inner_size(900,600)` 不可达）。

| 尺寸 | 折叠 H% | 折叠 W% | ≥85% H | ≥92% W |
|---|---|---|---|---|
| 1920×1080 | 92.22 | 98.54 | ✅ | ✅ |
| 1440×900 | 90.67 | 98.06 | ✅ | ✅ |
| 1366×768 | 89.06 | 97.95 | ✅ | ✅ |
| 1200×800 | 89.50 | 97.67 | ✅ | ✅ |
| 1024×720 | 88.33 | 97.27 | ✅ | ✅ |
| 900×600 | 86.00 | 96.89 | ✅ (余量1pp) | ✅ |

### 2.9 R3B-09 原生可行性

记 `NOT_RUN`。原生 WebView 焦点/遮挡/resize 验证归 A9/A11，不在本 lane 范围。

---

## 3. 模式覆盖

| 模式 | 描述 | A0 ruling |
|---|---|---|
| Normal | 60px top + 28px strip + content + 24px status | — |
| Collapsed | 所有工具窗口隐藏（含 pinned），28px strip 保留 | A0 ruling 38 |
| Focus | 所有 chrome 隐藏，一个返回操作（Esc） | A0 ruling 41 |
| Database | 左连接树 + 中SQL编辑 + 底结果（focus→满） | A5 3ff1f8b |
| Knowledge | 中笔记 + 右反链/局部图谱 | — |
| Git Central (D-R1a) | log/graph 在中央文档区 | A0 ruling 43, 可逆 |
| Git Bottom (D-R1b) | log/graph 在底部工具窗口 | A0 ruling 43, 可逆 |
| Context Menu | scope-aware, 分组, 禁用, 快捷键, safety class | A6 2bce4e3 |
| Command Palette | Ctrl+Shift+P, 模糊搜索, 分类 | A6 2bce4e3 |

---

## 4. 状态覆盖

| 状态 | 视觉 | 恢复路径 |
|---|---|---|
| empty | "empty — no content" | 新建/最近 |
| loading | "⟳ loading…" | 等待完成 |
| error | "⚠ error — [Retry]" | 重试按钮 |
| disabled | "disabled — no staged changes" | 满足条件后启用 |

---

## 5. 消费 A2-A9 R3B 报告

| Lane | SHA | 消费内容 | 应用于 |
|---|---|---|---|
| A2 | bfe8e86 | A0 公式六尺寸实测, 60/24/28 硬常量, 900×600 86.00% | §2.5 几何, §2.8 尺寸 |
| A3 | 8d8719a | Collapse All 含 pinned, Restore 快照, 58 断言 PASS | §2.3 collapse 语义 |
| A4 | 5d9e447 | 持久化契约, 快照生命周期, 瞬态不持久化, 29/29 pure tests | §2.3 collapse 语义 |
| A5 | 3ff1f8b | 数据库壳层对齐 canonical 几何, donor font 移除, 78 checks PASS | §3 database mode |
| A6 | 2bce4e3 | 107 命令注册表, Ctrl+K/Ctrl+Shift+P 冻结, 14 Git IDs, ARIA 契约, audit PASS | §2.4 无障碍, §2.6 快捷键, §2.1 Git units |
| A7 | b96cd51 | 14 单元 command-id 表, SSOT cee14e9, 4/10 分类, safety/tier 映射 | §2.1 Git units, §2.7 参考身份 |
| A8 | ba92110 | canonical 几何, light/dark 令牌, 键盘焦点, 900×600 高密度 | §2.4 无障碍, 视觉令牌 |
| A9 | d17091c | 14 Git 单元安全矩阵, 6 边界冻结, amend/reset/rebase 安全 | §2.1 safety class 标注 |

---

## 6. 与 R3 终稿的差异

| 维度 | R3 (ee2b8fd, 被审计) | R3B (本修正) |
|---|---|---|
| Git 单元 | 部分可见（缺 worktree/amend/reset/revert/conflict/patch/command-log） | 全部 14 单元三处可见 |
| donor 品牌 | "Rebased" 出现在 UI | 0 donor 名称在 UI |
| Collapse 语义 | A3/A4 分歧 | 对齐 A0 ruling（含 pinned） |
| 无障碍 | role/aria = 0 | 94 role, 127 aria, 78 tabindex |
| 几何 | 多口径 | A0 SSOT: 60/24/28 硬常量 |
| 快捷键 | Ctrl+K 1次, Ctrl+Shift+P 8次 | 冻结独立，无冲突 |
| 尺寸 | 4 尺寸（缺 1200×800, 900×600） | 6 尺寸全覆盖 |
| 参考身份 | v1.1.15 + 2896562e 混用 | SSOT=cee14e9, 2896562e=snapshot only |
| 分类 | COPY=0/ADAPT=5/REIMPLEMENT=9 | COPY=0/ADAPT_PRODUCT=4/REIMPLEMENT=10 |

---

## 7. 合规声明

- [x] Rebase onto `origin/master`（`d96b9b5`，R3 提交已集成被跳过）
- [x] 仅产出 `logs/research/M5-W18/A1-R3B-*` 文件
- [x] 未修改产品代码、manifest、lockfile、脚本、capability、ACL、vault
- [x] 原型自包含合成数据，不接真实 IPC/库/模型/daemon
- [x] 无 donor 品牌在原型 UI（Rebased/JetBrains/Obsidian/IntelliJ = 0）
- [x] 覆盖 1920/1440/1366/1200/1024/900×600 六种尺寸
- [x] 覆盖 normal/collapsed/focus/database/knowledge/Git(central+bottom)/context-menu/command-palette 模式
- [x] 覆盖 empty/loading/error/disabled 四种状态
- [x] Git 14 单元全部可见（grid + menu + palette）
- [x] amend/reset/revert/conflict/patch/command-log 可见
- [x] role/aria/keyboard/disabled-reason 完整
- [x] Ctrl+K = 地址/搜索, Ctrl+Shift+P = 命令面板（独立冻结）
- [x] 几何 SSOT: 60px top, 24px status, 28px strip（A0 硬常量）
- [x] 折叠态 ≥85% H / ≥92% W（六尺寸全 PASS）
- [x] 未 push

---

## 8. 未解问题

| # | 问题 | 归属 | 证据类型 |
|---|---|---|---|
| U1 | A0 裁定 F05: 体积余量≈346B，Git 工作台分片须声明预算归属 | A0 | `INFERENCE` |
| U2 | A0 裁定 F12: 原生子 WebView 在工具窗口重排下的位置/焦点/遮挡无 lane 认领 | A0 | `INFERENCE` |
| U3 | A0 裁定 F01: rebased LICENSE = JetBrains Build Terms v1.3，COPY 需 A0 书面接受 | A0 | `INFERENCE` |
| U4 | W19 实现时 A6 命令注册表为前置（当前前端 0 匹配） | A6/W19 | `INFERENCE` |
| U5 | W19 实现时 A7 Git 后端 10 个 REIMPLEMENT 单元为前置 | A7/W19 | `INFERENCE` |
| U6 | R3B-09 原生 WebView 焦点/遮挡/resize 未验证（NOT_RUN） | A9/A11 | `NOT_RUN` |

---

## 9. 结论

`STATUS=FINAL_READY_FOR_A10_REVIEW`

A1 R3B 终稿已完成 browser-first workbench 修正原型，消费 A2-A9 R3B 全部修正输出：
- 修正 R3 验收审计的全部 9 项阻塞发现（R3B-01 ~ R3B-09）
- Git 14 单元三处可见（grid + context menu + command palette）
- donor 品牌从 UI 完全移除
- ARIA/role/keyboard/disabled 完整覆盖
- A0 几何 SSOT（60/24/28 硬常量）六尺寸全 PASS
- Ctrl+K / Ctrl+Shift+P 独立冻结
- Rebased SSOT=cee14e9, 分类 0/4/10

**待 A10 独立复核 + A11 验收打包。**
