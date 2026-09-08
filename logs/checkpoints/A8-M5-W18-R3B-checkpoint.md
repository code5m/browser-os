# Lane A8 · M5-W18-R3B Checkpoint

时间：2026-09-08（CST）
Lane：A8 — Visual & accessibility states（R3B canonical-geometry revision）
分支：`codex/m5-w18-a8` ｜ 工作树：`/home/ainfinit/.codex/worktrees/m5-w18-a8/mvp-browser-os-v3`
Dispatch：`M5-W18-R3B`（A0 裁定 REVISE_TARGETED；NEXT=M5-W18-R3B；W19=CLOSED）
依据：`WORKSPACE_IDENTITY.md`（NEXT=M5-W18-R3B，覆盖历史 R2B 派发）、`M5-W18-R3B-CORRECTION-TASKS-20260908.md`（A8 卡）、`logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`（R3B-02/04/05 裁定）、`WORKBENCH_BLUEPRINT-20260908.md`、`PARALLEL_COMMAND_BOARD.md`。

## 1. 启动核查（WORKSPACE_IDENTITY 硬停规则）
- `pwd` = `/home/ainfinit/.codex/worktrees/m5-w18-a8/mvp-browser-os-v3` ✅ 匹配 `m5-w18-aN`
- 分支 = `codex/m5-w18-a8` ✅ 匹配 `codex/m5-w18-aN`
- 本 Lane 工作树：仅修改 A8 自有 `logs/research/M5-W18/A8-*` 与新增本 checkpoint
- 未改产品代码（`src/**`/`src-tauri/**`/`scripts/**`）、未改他 Lane 文件、未改 ACL/能力/依赖/用户数据、未 push ✅

## 2. R3B 任务复述（A8 卡）
> Revise all frames to canonical geometry and six target sizes. Define restrained light/dark tokens, keyboard focus, selected/hover/disabled states, menus and high-density 900x600 behavior. Check text fit and ensure the active content remains visually dominant. Provide objective pixel and ratio evidence.

A0 裁定（audit）落地到本 Lane：
- R3B-02 prototype hygiene：捐助品牌不得出现在 prototype UI → 全文检索 A8 文件 = 0 处捐助名（JetBrains/IDEA/Rebased/Obsidian/dbx/zvec/VSCode/Phantom…），已满足。
- R3B-04 accessibility：prototype 须演示键盘焦点/禁用理由 → A8 拥有视觉令牌与状态规范（§4 + §4.1 像素证据）；role/aria 演示属 A1 整合原型（非 A8 范围）。
- R3B-05 geometry：A1/A2/A3/A8 旧测量口径不兼容 → 改按 A0 SSOT 重算（见 §3）。

## 3. 依据源
- `M5-W18-R3B-CORRECTION-TASKS-20260908.md`（A8 段）
- `logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`（R3B-01~09 + A0 rulings）
- `WORKBENCH_BLUEPRINT-20260908.md`（§3 布局约束、J4 统一检索、§7 验收）
- 真实产品代码提取（仅读数，未改）：`src/styles/global.css`、`ActivityBar.vue`、`StatusBar.vue`、`Sidebar.vue`、`UnifiedTabBar.vue`

## 4. 交付物（本 Lane 新增/修订，A8 自有路径）
| 文件 | 变更 |
|---|---|
| `logs/research/M5-W18/A8-R3-visual-density-design.md` | 修订：§3.7 结构令牌（60/30/24/28 几何真源）；§4.1 无障碍像素证据；§5 按 A0 SSOT 重算六尺寸几何（retract 旧 32/26/4 尺寸口径）；§6/§8/§9 同步 |
| `logs/research/M5-W18/A8-R3-visual-density-frames.html` | 重写：6 尺寸 × 浅/暗 × 折叠/正常 + 状态画廊 + 右键菜单 + 双强调色 + empty/loading/error + 900×600 高密度；canonical 几何（60 顶 chrome / 24 状态栏 / 28 活动条） |
| `logs/checkpoints/A8-M5-W18-R3B-checkpoint.md` | 本文件 |

> R2B 检索研究（A8 搜索指标修正）已在 prior wave 完成（`A8-R2B-retrieval-metrics-correction.md` 等，HEAD=1e71b3a，已在 master）；本 R3B 不重述、不重复提交。R2B A8 卡（统一搜索体验）已闭环；当前 NEXT 为 R3B，A8 当前任务为视觉/无障碍态修订。

## 5. 关键设计判断（供 A1/A3/A6 消费，不越权）
1. **canonical 几何重算**（R3B-05）：inner viewport 排除 OS 标题栏；顶 chrome 60（两行 30）、状态栏 24、活动条 28；折叠 H%=(H−84)/H、W%=(W−28)/W。六尺寸全部达标（H 86.00–92.22%，W 96.89–98.54%）。
2. **六尺寸 + 剔除 800×600**：新增 1200×800、900×600（最小）；800×600 非产品最小窗口，删除。
3. **双强调色修复**：导航蓝 `#2b6cb0` 唯一品牌强调，绿仅 success/safe。
4. **单令牌真源**：间距 4px 基、圆角三档、字号上限 18px、命中区 ≥24px、1px 发丝、键盘焦点 2px 外环；暗=浅参数镜像。
5. **菜单 `data-cmd`/`data-disabled-reason`/`data-safety`** 与 A6 命令注册对齐。
6. **真实产品 2px 级差**（交 A2 基线测量、W19 调和）：真实 ActivityBar 32 + mod-tab 28 ≈ 60；StatusBar 26 vs canonical 24 差 2px。

## 6. 客观几何与无障碍证据（VERIFY）
- 几何表见 design §5；公式与 A0 ruling 逐字一致：
  - 1920×1080 → H 92.22% / W 98.54%
  - 1440×900 → H 90.67% / W 98.06%
  - 1366×768 → H 89.06% / W 97.95%
  - 1200×800 → H 89.50% / W 97.67%
  - 1024×720 → H 88.33% / W 97.27%
  - 900×600 → H 86.00% / W 96.89%
  - 全部 ≥85%H / ≥92%W（R3B 硬线）。
- 无障碍像素：focus 2px 外环（accent on white 4.6:1 AA）；disabled `--text-3` on `#fff` ≥3:1；文字对比浅 ≥14:1 / 暗 ≥7:1（AAA）。均来自 design §4.1。
- 捐助品牌：grep 全 A8 文件 = 0（R3B-02 hygiene）。NOT_RUN：真实 GUI 像素测量（无桌面客户端通道，交 A0/A11 native 验收）。

## 7. 自检（对照 R3B A8 卡 + A0 rulings）
| 约束 | 结果 |
|---|---|
| 六尺寸 + canonical 几何 | ✅ §5 / HTML §1 |
| restrained 浅/暗令牌 | ✅ §3.5/§3.6 |
| keyboard focus / selected / hover / disabled / menu 态 | ✅ §4 + §4.1 / HTML §5 |
| 900×600 高密度行为 | ✅ HTML §4 |
| 客观像素/比值证据 | ✅ §5 表 + §4.1 |
| 捐助品牌 0 | ✅ grep |
| role/aria 演示 | ➖ 属 A1 整合原型（A8 仅令牌/态，非越权） |

## 8. 边界与禁令遵守
- 零产品代码改动；零他 Lane 文件改动；零 push；零依赖/ACL/能力/原生/用户数据改动。
- 研究原型仅合成数据，无脚本、无外部资源、无捐助品牌、无密钥。

## 9. 提交（本 Lane 分支，不 push）
```
git add logs/research/M5-W18/A8-R3-visual-density-design.md \
        logs/research/M5-W18/A8-R3-visual-density-frames.html \
        logs/checkpoints/A8-M5-W18-R3B-checkpoint.md
git commit -m "docs(M5-W18-R3B): A8 canonical-geometry & accessibility states revision"
# 之后 rebase origin/master（0db481a 已并入 master，rebase 自动 drop，R3B 提交落于 master 之上）
```

STATUS：PASS_WITH_NOTE（canonical 几何六尺寸达标、无障碍像素证据齐备、捐助品牌 0；role/aria 演示属 A1 整合原型，非 A8 越权）。未 push。

## 10. 后续依赖（交 A0 集成）
- A1：消费 §3 令牌与 §5 几何作为整合原型视觉基线；role/aria 演示在 A1 HTML。
- A2：以 A0 canonical 公式测量真实产品基线（ActivityBar/StatusBar 真实像素），W19 调和 2px 级差。
- A3/A6：消费 §4 状态与菜单 `data-*` 约定。
- A10：独立核对 R3B-05 几何公式与六尺寸比值、R3B-02 捐助品牌 0、R3B-04 无障碍像素。
- A11：收录本 Lane 提交与本 checkpoint 至 R3B manifest。
- W19 落地：将现有散落字面量替换为 §3 令牌变量（不引入新组件）。
