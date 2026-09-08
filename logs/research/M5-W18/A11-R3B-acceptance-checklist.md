# A11 · M5-W18-R3B 原型验收清单（用户评审用）

> Lane A11 · `RESEARCH_AND_PROTOTYPE` · 基线 `origin/master` `d96b9b5`
> R3B = A0 退回的定向修订波（verdict `REVISE_TARGETED`，9 项阻塞 R3B-01..R3B-09）。
> 机器可读真源：`A11-R3B-integration-manifest.json`；复核命令：
> `python3 logs/research/M5-W18/A11-R3B-acceptance-matrix.py --manifest logs/research/M5-W18/A11-R3B-integration-manifest.json --report`
> 本清单不引用产品代码/依赖/ACL/capability/用户数据；原型均为合成数据。

## 0. 三句话

1. **当前状态 = `PROVISIONAL_PENDING_UPSTREAM`**。A11 在 **A1 R3B 修正原型** 与 **A10 R3B 独立复核** 落地后才能定稿；二者在本工作区尚未提交（A1 工作树仍停在 R3 终稿 `ee2b8fd`）。
2. **本包的核心改进（回应 A0 对 R3 的批语）**：Git 与无障碍探针**直接解析最终 A1 HTML 的 `.frame` 真实 UI 文本**，而不是在参考文档里找词。当前对 R3 原型实跑即暴露下方缺口。
3. **A0 退出门**：R3B 可交付用户评审仅当——A10 自包含检查器全绿、A1 原型 14 Git 单元全可见、A2 验完含 900×600 的六尺寸、A3/A4 折叠/恢复一致、无障碍可机检、A10 无 open high、A11 无未解释 gap/warn。

## 1. 怎么验

| 对象 | 看什么 |
|---|---|
| `A1-R3-prototype.html` | 替代壳层：六尺寸 × 模式 × 状态；Git 中央/底部放置；**必须含 14 单元 + amend/reset-revert** |
| `A11-R3B-acceptance-matrix.py` | 机器探针：尺寸 / Git 单元 / a11y / 折叠 / 卫生 / 快捷键 / 几何 / 钉版 / A10 检查器 |

判定口径（A0 裁定）：折叠态内容 ≥85% 高、≥92% 宽；**顶部 chrome 恰 60px（两行 30px）**；状态栏 24px；活动条 28px 在折叠态保留；`Ctrl+K`=地址/搜索，`Ctrl+Shift+P`=命令面板。

## 2. 六尺寸（S-01~S-06；R3B-08）

| ID | 尺寸 | 现状 |
|---|---|---|
| S-1920x1080 | 1920×1080 | ✅ 已渲染 |
| S-1440x900 | 1440×900（主） | ✅ 已渲染 |
| S-1366x768 | 1366×768 | ✅ 已渲染 |
| S-1200x800 | 1200×800 | ❌ 缺口（R3B-08） |
| S-1024x720 | 1024×720 | ✅ 已渲染 |
| S-900x600 | 900×600（产品最小） | ❌ 缺口（R3B-08；800×600 已剔除） |

## 3. Git 工作流（G-01~G-16；R3B-01）

参考 `DetachHead/rebased` v1.1.15 @ `cee14e9`，`COPY=0 / ADAPT_PRODUCT=4 / REIMPLEMENT=10`。A0 R3B-01 显式要求原型还须含 **amend / reset-revert**。

| ID | 单元 | 归类 | 原型可见（当前 R3 原型） |
|---|---|---|---|
| G-status | status | ADAPT | ✅ |
| G-hunk-staging | hunk 暂存 | REIMPLEMENT | ✅ |
| G-diff | diff | ADAPT | ✅ |
| G-log-graph | log/graph + 两放置 | REIMPLEMENT | ✅ |
| G-branches | 分支 | ADAPT | ✅ |
| G-worktrees | worktree | REIMPLEMENT | ❌ 缺口 |
| G-stash | stash | REIMPLEMENT | ✅ |
| G-merge | merge | ADAPT | ✅ |
| G-rebase | rebase | REIMPLEMENT | ✅ |
| G-cherry-pick | cherry-pick | REIMPLEMENT | ✅ |
| G-conflicts | 冲突解决 | REIMPLEMENT | ❌ 缺口 |
| G-history-blame | blame | REIMPLEMENT | ✅ |
| G-patch | patch | REIMPLEMENT | ❌ 缺口 |
| G-command-log | 命令日志 | REIMPLEMENT | ✅ |
| G-amend | amend | REIMPLEMENT | ❌ 缺口（R3B-01 显式要求） |
| G-reset-revert | reset/revert | REIMPLEMENT | ❌ 缺口（R3B-01 显式要求） |

## 4. 无障碍（A-01~A-02；R3B-04）

| ID | 检查项 | 现状 |
|---|---|---|
| A-aria | 原型 `role=`/`aria-*` 属性 | ❌ 缺口（当前 `.frame` 内零命中） |
| A-focus | `:focus-visible` 可见外环 | ⚠ 警告（CSS 无显式规则） |

## 5. 折叠/恢复（C-01~C-03；R3B-03）

| ID | 检查项 | 现状 |
|---|---|---|
| C-hide | 折叠隐藏所有工具窗口（含固定） | ✅ |
| C-strip | 折叠态保留 28px 活动条 | ❌ 缺口（当前 `.mode-collapsed .edge-strip{display:none}` 违反 A0） |
| C-restore | 恢复布局入口 | ✅ |

## 6. 卫生 / 快捷键 / 几何 / 钉版（R3B-02/05/06/07）

| ID | 检查项 | 现状 |
|---|---|---|
| H-brand | UI 文本无 donor 品牌 | ✅（品牌串仅出现在报告正文，A10 检查器对此更严，见下） |
| H-font | 中性字体栈（无 Cascadia/SF Mono/Segoe UI） | ❌ 缺口（R3B-02） |
| K-binding | `Ctrl+K`=地址，`Ctrl+Shift+P`=面板 | ❌ 缺口（当前 `Ctrl+K` 绑到 palette，R3B-06） |
| GEO | A0 折叠公式 ≥92%W/85%H（六尺寸） | ✅ 公式通过；⚠ 状态栏 26px≠24px（R3B-05） |
| M-pin | rebased 钉版 = `cee14e9` | ✅（A0 SSOT；`2896562e` 仅研究快照，R3B-07） |
| A10-gate | A10 自包含检查器 exit 0 | ❌ 缺口（exit=1：A1/A3/A5 原型含 donor 品牌/字体，R3B-02） |

> A10 检查器比本包更严：它把 h2/frame-meta 正文里的 "Rebased" 也判为品牌串（L413/L414）。R3B 修正版须把品牌串**连同报告正文**一并清除，并改用中性字体栈。

## 7. 原生可行性（R3B-09，留出）

子 WebView 焦点/遮挡/缩放未在真实 Tauri 客户端验证 → **保留给用户/A0 评审**，不计入 A11 门禁失败（A0 退出门允许）。

## 8. A11 记录的 9 项 R3B 发现（全部 OPEN，R3B-09 为原生预留）

| ID | 级 | 结论 | 归属 |
|---|---|---|---|
| R3B-01 | high | 14 Git 单元 + amend/reset-revert 未全在原型可见 | A1 |
| R3B-02 | high | donor 字体 + A10 检查器 FAIL | A1 字体栈 + A10 重跑 |
| R3B-03 | high | 折叠态隐藏活动条（应保留 28px） | A1 + A3/A4 |
| R3B-04 | high | 原型无 role/aria、无 focus 环 | A1 |
| R3B-05 | med | 几何口径未冻结（状态栏 26≠24） | A0 冻结 + A2 |
| R3B-06 | med | `Ctrl+K` 误绑 palette | A6 + A1 |
| R3B-07 | med | 参考钉版 cee14e9 vs 2896562e | A10 + A7 |
| R3B-08 | med | 缺 1200×800 / 900×600 | A1 + A2 |
| R3B-09 | med | 原生可行性未验证（预留） | A0/用户 |

## 9. 用户评审判定

- 全部 `通过` → A0 可开片交付用户评审。
- 任一 `不通过`/未关闭缺口 → 仅退回对应设计 lane，产品 UI 保持关闭，W19 仍 `CLOSED`。
- `R3B-09` 原生项不计入 A11 失败，但须由 A0/用户在真实客户端确认。
