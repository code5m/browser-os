# Lane A8 · M5-W18-R3 Checkpoint

时间：2026-09-08（CST）
Lane：A8 — Visual density and 「大气」design
分支：`codex/m5-w18-a8` ｜ 工作树：`/home/ainfinit/.codex/worktrees/m5-w18-a8/mvp-browser-os-v3`
基线：`origin/master @ 200f0f1`（rebase 后本分支 == origin/master，4 个 R2B 提交已被 A0 并入，rebase 跳过重放）

## 1. 启动核查（WORKSPACE_IDENTITY 硬停规则）

- `pwd` = `/home/ainfinit/.codex/worktrees/m5-w18-a8/mvp-browser-os-v3` ✅ 匹配 `m5-w18-aN` 模式
- 分支 = `codex/m5-w18-a8` ✅ 匹配 `codex/m5-w18-aN`
- 工作树 clean（rebase 后无未提交改动）✅
- 未改产品代码、未改他 Lane 文件、未 push ✅

## 2. Rebase 记录

```
git fetch origin
git rebase origin/master
# → warning: skipped previously applied commit 4c750a4 / f4a4f3b / 1e71b3a / ed5c0c8
#   （4 个 A8 R2B 提交已存在于 origin/master，作为重复提交跳过）
# → Successfully rebased; HEAD = 200f0f1 == origin/master
```
结论：本分支与 `origin/master` 等价；A8 R2B 检索研究（`A8-zvec-grep-retrieval.md`、`A8-R2B-*.md`、`A8-*-benchmark-*.mjs`、`A8-M5-W18-R2B-checkpoint.md`）均已在 canonical master，本波不重述、不重复提交。

## 3. 依据源

- `WORKSPACE_IDENTITY.md` — W18 lane 隔离规则（研究/原型仅本 lane 工作树）
- `PARALLEL_COMMAND_BOARD.md` — Current Dispatch Entry：R3 仅交互研究/原型，禁改产品源/依赖/ACL/能力/原生/用户数据
- `M5-W18-R3-UX-TASKS-20260908.md` — **A8 卡**：克制视觉系统（浅/暗）、间距/字/图标/分隔/状态/菜单、内容即产品、4 尺寸标注帧 + 客观测量
- `WORKBENCH_BLUEPRINT-20260908.md` — 第一版布局约束（chrome 32/状态栏 26、侧栏 ~260、DPI/WebView 契约）
- `M5-W18-PROTOTYPE-REVIEW-20260908.md` — 用户裁决：保留浏览器式大内容视口，去掉常驻大功能按钮
- 真实产品代码提取：`src/styles/global.css`、`App.vue`、`ActivityBar.vue`、`StatusBar.vue`、`Sidebar.vue`、`UnifiedTabBar.vue`

## 4. 交付物（本 lane 新增，3 文件，均 A8 自有路径）

| 文件 | 内容 |
|---|---|
| `logs/research/M5-W18/A8-R3-visual-density-design.md` | 克制视觉系统：令牌（间距/圆角/字/图标/浅暗色）、状态规范、客观视口测量表（4 尺寸）、与他 Lane 接口、自检 |
| `logs/research/M5-W18/A8-R3-visual-density-frames.html` | 合成标注帧：4 尺寸×浅/暗×折叠/正常 + 状态画廊 + 右键菜单 + 双强调色修复对照 + empty/loading/error/disabled/context-menu 态 |
| `logs/checkpoints/A8-M5-W18-R3-checkpoint.md` | 本文件 |

## 5. 关键设计判断（供 A1/A3/A6 消费，不越权）

1. **双强调色修复**：导航 active 蓝 `#2b6cb0` 与主按钮绿 `#2b6` 冲突 → 统一蓝为唯一品牌强调，绿仅保留 success/safe 语义。
2. **现状布局已满足视口硬线**：折叠态四尺寸内容 H% = 94.5/93.3/92.1/91.6，均 >85%；W% 均 100% >92%。R3「大气」增益来自令牌统一而非再砍 chrome。
3. **令牌单一真源**：间距 4px 基、圆角三档(4/8/pill)、字号上限 18px（禁超大按钮）、命中区 ≥24px、1px 发丝分隔、键盘焦点 2px 外环。
4. **暗色=浅色参数镜像**：同组件仅 `--surface/--text/--border` 翻面，不写第二套样式。
5. **菜单项 `data-cmd` / `data-disabled-reason` / `data-safety` 约定**：与 A6 命令注册对齐。

## 6. 自检（对照 R3 Global acceptance constraints）

| 约束 | 结果 |
|---|---|
| 1440×900 折叠 ≥85%H / ≥92%W | ✅ 93.3%H / 100%W |
| 顶部 chrome ≤2 行 ≤80px | ✅ 折叠 32 / 扩展 65 |
| 浅/暗主题 | ✅ §3.5/§3.6 |
| 无大卡片/超大按钮/多面板挤压 | ✅ 字号上限 18px + 单强调 |
| 右键项有命令身份/禁用理由/键盘路径/安全类 | ✅ 菜单 `data-*` |
| 帧覆盖 4 尺寸 + empty/loading/error/disabled/ctx-menu | ✅ HTML §1/§7 |
| 侧/底工具 open/close/collapse/restore/auto-hide/resize | ➖ 行为属 A3（令牌已备） |
| 每边至多一主工具窗口默认开 | ➖ 行为属 A1/A3 |
| 全焦点模式隐藏 chrome+唯一返回 | ➖ 行为属 A1/A3（隐藏态底色已定义） |

## 7. 边界与禁令遵守

- 零产品代码改动（`src/**`、`src-tauri/**` 未触碰）。
- 零他 Lane 文件改动（仅新增 `logs/research/M5-W18/A8-*` 与 `logs/checkpoints/A8-*`）。
- 零 push。
- 零依赖/ACL/能力/原生运行时/用户数据改动。

## 8. 提交

```
git add logs/research/M5-W18/A8-R3-visual-density-design.md \
        logs/research/M5-W18/A8-R3-visual-density-frames.html \
        logs/checkpoints/A8-M5-W18-R3-checkpoint.md
git commit -m "docs(M5-W18-R3): A8 restrained visual-density & 大气 design system"
```

STATUS：PASS_WITH_NOTE（视觉令牌完整、测量达标；行为/布局契约交 A1/A3/A6，未越权）。

## 9. 后续（交 A0 集成）

- A1 整合壳层原型时引用本令牌系统作为视觉基线。
- A3 渐进披露行为引用 §4 状态与菜单规范。
- A6 命令注册引用 §4 菜单 `data-*` 身份约定。
- W19 落地时将现有散落字面量替换为 §3 令牌变量（不引入新组件）。
