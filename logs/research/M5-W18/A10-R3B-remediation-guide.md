# A10 — M5-W18-R3B 逐 Lane 修复指引（机器生成）

- 来源：`A10-R3B-closure-audit.py --json`（strict_sizes=False）
- 门禁：`GATE=FAIL`，HIGH=18 / MEDIUM=30
- 探针失败：9/9

> 本指引按「谁负责修」聚合发现。**A10 不自改他 lane 资产**（W18-R 硬约束）；
> 各 lane 修复后由 A0/本 lane 复跑 `bash A10-R3B-rerun.sh` 确认 `GATE: PASS`。

## Lane A1（24 项）

### P1 [HIGH×3]

**裁定修复**：原型 UI 文本/芯片不得出现 donor 品牌名或字体名（Rebased / IntelliJ / JetBrains / JetBrains Mono 等）；报告里可作署名引用，但 UI 须用中性表述（如「日志/图谱作中央文档区」）。

- `HIGH` A1-R3-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (2 hit(s)) / L413: <h2>6a. Git Mode — Central Log — 1440×900（Rebased 风格：log 作中央文档）</h2> / L414: <div class="frame-meta">Git log/graph 占中央文档区 | 可切换到底部工具窗口（见 6b）| 不复制 Rebased 品牌/资产</div>`
- `HIGH` A3-R3-toolwindow-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (2 hit(s)) / L164: <span class="chip">🌐 设计参考 · Rebased</span> / L186: <h2>活动文档：设计参考 · Rebased</h2>`
- `HIGH` A5-R3-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (1 hit(s)) / L11: --mono:ui-monospace,"JetBrains Mono","Cascadia Mono",Consolas,monospace;`

### P2 [HIGH×6]

**裁定修复**：几何口径统一为 A0 冻结值：顶部 chrome = 2×30px = 60px、状态栏 24px、折叠竖条 28px。删除一切 68/65/89/94/108px 等旧 chrome 数字。

- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L71 — `合计 68px ✅ ≤80px`
- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L119 — `- 顶部 chrome 68px + 边缘图标条 28px + 内容区 + 状态栏 24px`
- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L273 — `| A2 | 8 控件顶栏 (19→8), AFTER 帧 §7.3/§7.4, 实测 90.1% H 平静态, chrome 89px | §2.1 顶栏, §4 viewport |`
- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L308 — `- [x] 顶部 chrome ≤2 行 ≤80px（实测 68px）`
- `HIGH` A2-R3-shell-density-audit.md still states a superseded chrome number (A0 froze 60px top + 24px status) L20 — `| 1 | 当前**平静态**并不拥挤：1440x900 下总 chrome 89px（活动条 32 + 页签条 31 + 状态栏 26），内容占 **90.1% 高 / 100.0% 宽**，已满足 R3"≥85% 高 ≥92% 宽" |`
- `HIGH` A2-R3-shell-density-audit.md still states a superseded chrome number (A0 froze 60px top + 24px status) L228 — `### 7.1 BEFORE · 1440x900 · 平静态（chrome 89px）`

### P4 [HIGH×1]

**裁定修复**：A1 终稿原型须以可见 mock 面板/列表呈现 14 个 Git 单元 + amend + reset/revert；当前缺 diff/worktrees/conflicts/patch/命令日志/amend/reset|revert。

- `HIGH` A1-R3-prototype.html does not visibly locate 7 workflow unit(s) — `diff; worktrees; conflicts; patch; command log; amend; reset / revert`

### P5 [HIGH×1 + MED×3]

**裁定修复**：无障碍达 R3B 地板：A1 至少 8 个 role= + 8 个 aria-* + 1 个 tabindex + :focus-visible；其余原型至少各 1 个 role= + 1 个 aria-*。

- `HIGH` A1-R3-prototype.html accessibility below the R3B floor — `role=0/8 aria-*=0/8 tabindex=0/1`
- `MEDIUM` A1-R3-prototype.html has no :focus-visible styling
- `MEDIUM` A2-R3-density-replica.html accessibility below the R3B floor — `role=0/1 aria-*=0/1 tabindex=0/0`
- `MEDIUM` A8-R3-visual-density-frames.html accessibility below the R3B floor — `role=0/1 aria-*=0/1 tabindex=0/0`

### P6 [HIGH×3]

**裁定修复**：快捷键定版：Ctrl+Shift+P = 命令面板，Ctrl+K = 地址/搜索，二者零冲突。A6 注册表须显式冻结这两条；A1 原型不得把 Ctrl+K 绑到命令面板。

- `HIGH` A6-R3-context-menu-command-registry.md does not freeze the palette shortcut (Ctrl+Shift+P)
- `HIGH` A6-R3-context-menu-command-registry.md does not freeze the address/search shortcut (Ctrl+K)
- `HIGH` A1-R3-prototype.html binds Ctrl+K to the command palette (A0 froze Ctrl+Shift+P) L169 — `<div class="cmd-btn" title="command.palette (PROPOSED_NEW, A6)">🔍 (Ctrl+K)</div>`

### P3 [HIGH×0 + MED×7]

**裁定修复**：验收证据须覆盖 A0 四强制尺寸（1920×1080/1440×900/1366×768/1024×720）；若 A0 改判六尺寸全强制（--strict-sizes），还需补 1200×800 与 900×600 帧。800×600 仅保留「剔除」说明，不得作为可达帧。

- `MEDIUM` A1 evidence omits product-edge size(s) (1200x800 default / 900x600 min inner) — reported as hint — `1200x800  [group: A1-R3-prototype.html]`
- `MEDIUM` A1-R3-prototype.html still references the dropped 800x600 size — `A1-R3-prototype.html L152: <p class="desc">替代壳层原型（终稿，消费 A2-A10）。保留 Chrome 式紧凑标签+地址栏+大内容区；IDEA 式渐进披露（可折叠工具窗口、右键菜单、命令面板）。合成数据，不接真实 IPC。覆盖 1920×1080 /`
- `MEDIUM` A1-R3-prototype.html still references the dropped 800x600 size — `A1-R3-prototype.html L787: <tr><td style="padding:3px 8px;border:1px solid var(--border);">尺寸覆盖</td><td style="padding:3px 8px;border:1px solid var`
- `MEDIUM` A1-R3-prototype.html still references the dropped 800x600 size — `A1-R3-prototype.html L809: <tr><td style="padding:3px 8px;border:1px solid var(--border);">A10</td><td style="padding:3px 8px;border:1px solid var(`
- `MEDIUM` A1-R3-browser-workbench.md still references the dropped 800x600 size — `A1-R3-browser-workbench.md L210: | ~~800×600~~ | **剔除**：产品 `min_inner_size(900,600)` 不可达 | A10 F13 |`
- `MEDIUM` A1-R3-browser-workbench.md still references the dropped 800x600 size — `A1-R3-browser-workbench.md L281: | A10 | F08(260px FAIL), F09(omni=扩展态), F13(800×600 剔除), G4(PROPOSED_NEW), G5(source-derived), G6(viewport budget PASS) `
- `MEDIUM` A1-R3-browser-workbench.md still references the dropped 800x600 size — `A1-R3-browser-workbench.md L321: - 7 种模式 × 4 种尺寸 × 4 种状态全覆盖（800×600 剔除 per A10 F13）`

## Lane A2（21 项）

### P2 [HIGH×6]

**裁定修复**：几何口径统一为 A0 冻结值：顶部 chrome = 2×30px = 60px、状态栏 24px、折叠竖条 28px。删除一切 68/65/89/94/108px 等旧 chrome 数字。

- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L71 — `合计 68px ✅ ≤80px`
- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L119 — `- 顶部 chrome 68px + 边缘图标条 28px + 内容区 + 状态栏 24px`
- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L273 — `| A2 | 8 控件顶栏 (19→8), AFTER 帧 §7.3/§7.4, 实测 90.1% H 平静态, chrome 89px | §2.1 顶栏, §4 viewport |`
- `HIGH` A1-R3-browser-workbench.md still states a superseded chrome number (A0 froze 60px top + 24px status) L308 — `- [x] 顶部 chrome ≤2 行 ≤80px（实测 68px）`
- `HIGH` A2-R3-shell-density-audit.md still states a superseded chrome number (A0 froze 60px top + 24px status) L20 — `| 1 | 当前**平静态**并不拥挤：1440x900 下总 chrome 89px（活动条 32 + 页签条 31 + 状态栏 26），内容占 **90.1% 高 / 100.0% 宽**，已满足 R3"≥85% 高 ≥92% 宽" |`
- `HIGH` A2-R3-shell-density-audit.md still states a superseded chrome number (A0 froze 60px top + 24px status) L228 — `### 7.1 BEFORE · 1440x900 · 平静态（chrome 89px）`

### P5 [HIGH×1 + MED×3]

**裁定修复**：无障碍达 R3B 地板：A1 至少 8 个 role= + 8 个 aria-* + 1 个 tabindex + :focus-visible；其余原型至少各 1 个 role= + 1 个 aria-*。

- `HIGH` A1-R3-prototype.html accessibility below the R3B floor — `role=0/8 aria-*=0/8 tabindex=0/1`
- `MEDIUM` A1-R3-prototype.html has no :focus-visible styling
- `MEDIUM` A2-R3-density-replica.html accessibility below the R3B floor — `role=0/1 aria-*=0/1 tabindex=0/0`
- `MEDIUM` A8-R3-visual-density-frames.html accessibility below the R3B floor — `role=0/1 aria-*=0/1 tabindex=0/0`

### P3 [HIGH×0 + MED×11]

**裁定修复**：验收证据须覆盖 A0 四强制尺寸（1920×1080/1440×900/1366×768/1024×720）；若 A0 改判六尺寸全强制（--strict-sizes），还需补 1200×800 与 900×600 帧。800×600 仅保留「剔除」说明，不得作为可达帧。

- `MEDIUM` A2 evidence omits product-edge size(s) (1200x800 default / 900x600 min inner) — reported as hint — `1200x800  [group: A2-R3-shell-density-audit.md, A2-R3-measure.py, A2-R3-run-20260908.out]`
- `MEDIUM` A2-R3-density-replica.html still references the dropped 800x600 size — `A2-R3-density-replica.html L329: // 蓝图 WORKBENCH_BLUEPRINT §3 提到的窄窗 800x600；注意产品 min_inner_size=900x600`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L29: | 10 | 极窄窗已出现"功能不可用"而非"变窄"：900x600 三面同开内容只剩 **60px 宽**，800x600 为 **0px**；且 800x600 低于 `min_inner_size(900,600)`，与蓝图 §3 的`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L43: 蓝图提到的 900x600 / 800x600）。`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L46: ⚠️ `min_inner_size(900,600)` 意味着 **800x600 在现有产品里根本拉不出来**，与蓝图 §3 的"窄窗 1024x720、800x600"`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L115: ### 3.1 极窄窗（900x600 / 800x600）`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L117: | 状态 | 900x600 | 800x600 |`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L130: 2. 800x600 低于产品 `min_inner_size(900,600)`，真实产品拉不出这个尺寸；若要支持，必须先改窗口最小尺寸（属产品代码，A2 不做）。`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L309: - **A0/A1（尺寸口径）**：R3 卡写 4 个目标尺寸（1920x1080/1440x900/1366x768/1024x720），蓝图 §3 另提 800x600，`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L310: 但产品 `min_inner_size(900,600)`（`main.rs:1226`）使 800x600 不可达。请裁决：改最小窗口（产品代码，A2 不做）`
- `MEDIUM` A2-R3-shell-density-audit.md still references the dropped 800x600 size — `A2-R3-shell-density-audit.md L311: 还是把 800x600 从目标里去掉。A2 两个尺寸都测了（§3.1），不影响其它结论。`

## Lane A3（4 项）

### P1 [HIGH×3]

**裁定修复**：原型 UI 文本/芯片不得出现 donor 品牌名或字体名（Rebased / IntelliJ / JetBrains / JetBrains Mono 等）；报告里可作署名引用，但 UI 须用中性表述（如「日志/图谱作中央文档区」）。

- `HIGH` A1-R3-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (2 hit(s)) / L413: <h2>6a. Git Mode — Central Log — 1440×900（Rebased 风格：log 作中央文档）</h2> / L414: <div class="frame-meta">Git log/graph 占中央文档区 | 可切换到底部工具窗口（见 6b）| 不复制 Rebased 品牌/资产</div>`
- `HIGH` A3-R3-toolwindow-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (2 hit(s)) / L164: <span class="chip">🌐 设计参考 · Rebased</span> / L186: <h2>活动文档：设计参考 · Rebased</h2>`
- `HIGH` A5-R3-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (1 hit(s)) / L11: --mono:ui-monospace,"JetBrains Mono","Cascadia Mono",Consolas,monospace;`

### P3 [HIGH×0 + MED×1]

**裁定修复**：验收证据须覆盖 A0 四强制尺寸（1920×1080/1440×900/1366×768/1024×720）；若 A0 改判六尺寸全强制（--strict-sizes），还需补 1200×800 与 900×600 帧。800×600 仅保留「剔除」说明，不得作为可达帧。

- `MEDIUM` A3 evidence omits product-edge size(s) (1200x800 default / 900x600 min inner) — reported as hint — `1200x800, 900x600  [group: A3-R3-toolwindow-prototype.html, A3-R3-toolwindow-disclosure-20260908.md]`

## Lane A4（2 项）

### P9 [HIGH×2]

**裁定修复**：Collapse All 隐藏所有工具窗口（含 pinned）；不得保留 pinned。

- `HIGH` A4-R3-shell-state-persistence-contract.md keeps pinned tool windows during Collapse All (A0 ruled Collapse All hides all, pinned included) L268 — `- `collapseAll` sets every non-`pinned` window `state:"hidden"` (pinned survive, per global acceptance).`
- `HIGH` A4-R3-shell-state-persistence-contract.md keeps pinned tool windows during Collapse All (A0 ruled Collapse All hides all, pinned included) L290 — `| T14 | collapseAll | non-pinned hidden, pinned preserved |`

## Lane A5（4 项）

### P1 [HIGH×3]

**裁定修复**：原型 UI 文本/芯片不得出现 donor 品牌名或字体名（Rebased / IntelliJ / JetBrains / JetBrains Mono 等）；报告里可作署名引用，但 UI 须用中性表述（如「日志/图谱作中央文档区」）。

- `HIGH` A1-R3-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (2 hit(s)) / L413: <h2>6a. Git Mode — Central Log — 1440×900（Rebased 风格：log 作中央文档）</h2> / L414: <div class="frame-meta">Git log/graph 占中央文档区 | 可切换到底部工具窗口（见 6b）| 不复制 Rebased 品牌/资产</div>`
- `HIGH` A3-R3-toolwindow-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (2 hit(s)) / L164: <span class="chip">🌐 设计参考 · Rebased</span> / L186: <h2>活动文档：设计参考 · Rebased</h2>`
- `HIGH` A5-R3-prototype.html is not self-contained — `PROTO_NO_BRANDING: donor branding string (allowed only as a documented source citation) (1 hit(s)) / L11: --mono:ui-monospace,"JetBrains Mono","Cascadia Mono",Consolas,monospace;`

### P3 [HIGH×0 + MED×1]

**裁定修复**：验收证据须覆盖 A0 四强制尺寸（1920×1080/1440×900/1366×768/1024×720）；若 A0 改判六尺寸全强制（--strict-sizes），还需补 1200×800 与 900×600 帧。800×600 仅保留「剔除」说明，不得作为可达帧。

- `MEDIUM` A5 evidence omits product-edge size(s) (1200x800 default / 900x600 min inner) — reported as hint — `1200x800, 900x600  [group: A5-R3-prototype.html, A5-R3-database-shell.md]`

## Lane A6（3 项）

### P6 [HIGH×3]

**裁定修复**：快捷键定版：Ctrl+Shift+P = 命令面板，Ctrl+K = 地址/搜索，二者零冲突。A6 注册表须显式冻结这两条；A1 原型不得把 Ctrl+K 绑到命令面板。

- `HIGH` A6-R3-context-menu-command-registry.md does not freeze the palette shortcut (Ctrl+Shift+P)
- `HIGH` A6-R3-context-menu-command-registry.md does not freeze the address/search shortcut (Ctrl+K)
- `HIGH` A1-R3-prototype.html binds Ctrl+K to the command palette (A0 froze Ctrl+Shift+P) L169 — `<div class="cmd-btn" title="command.palette (PROPOSED_NEW, A6)">🔍 (Ctrl+K)</div>`

## Lane A7（7 项）

### P7 [HIGH×1 + MED×5]

**裁定修复**：参照钉版：cee14e9 为 SSOT；2896562e 仅作降级研究快照（引用处须标注「降级、非 SSOT、终裁前不得 W19 采用」）。COPY 分类维度删除（A0 裁定 COPY=0，并入 REJECT/forbidden）；tag 精确记为 refs/tags/1.1.15（无 v 前缀）。

- `HIGH` A7-R3-git-workflow-reference-map.md classifies a unit as COPY (A0 ruled COPY=0) L185 — `| COPY | 0 | — (forbidden: Kotlin/JVM Apache-2.0 → Rust/MulanPSL-2; A10 provenance gate required first) |`
- `MEDIUM` A7-R3-git-workflow-reference-map.md writes the tag as v1.1.15; the upstream ref is refs/tags/1.1.15 (no v prefix) -> record the exact ref name
- `MEDIUM` A10-R3-reference-and-feasibility-review-20260908.md cites 2896562e without marking it a demoted research snapshot L15 — `1. **参照身份可确认、许可不可简化**：`DetachHead/rebased` 确为 `JetBrains/intellij-community` 的 fork（GitHub API `fork=true`，`parent/sourc`
- `MEDIUM` A10-checkpoint-R3-20260908.md cites 2896562e without marking it a demoted research snapshot L19 — `1. `DetachHead/rebased` = `JetBrains/intellij-community` fork（已确认），**固定修订 `2896562e69ff2cac3c90eb3aae4bcce0d4aa9a99`（202`
- `MEDIUM` A11-R3-checkpoint.md cites 2896562e without marking it a demoted research snapshot L97 — `| F-A11-9 | medium | A7 钉 `cee14e9` 与 A10 钉 `2896562e` 不一致；A11 采纳 A10 权威钉版，A0/A10 终裁前不得 W19 采用任何 COPY/ADAPT 单元 |`
- `MEDIUM` A11-R3-progress.md cites 2896562e without marking it a demoted research snapshot L26 — `| Git 完整工作流 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | A7 实测：14 单元中 5 ADAPT（status/diff/branches/merge，其中 merg`

### P8 [HIGH×1]

**裁定修复**：A7 分类计数须等于其 14 行表：ADAPT=4 / REIMPLEMENT=10 / COPY=0；散文与表保持一致。

- `HIGH` A7-R3-git-workflow-reference-map.md prose tally states REIMPLEMENT=9, table/A0 ruling says 10 L210 — `- ✅ Default to behavior reimplementation (REIMPLEMENT=9, COPY=0).`

## Lane A8（5 项）

### P5 [HIGH×1 + MED×3]

**裁定修复**：无障碍达 R3B 地板：A1 至少 8 个 role= + 8 个 aria-* + 1 个 tabindex + :focus-visible；其余原型至少各 1 个 role= + 1 个 aria-*。

- `HIGH` A1-R3-prototype.html accessibility below the R3B floor — `role=0/8 aria-*=0/8 tabindex=0/1`
- `MEDIUM` A1-R3-prototype.html has no :focus-visible styling
- `MEDIUM` A2-R3-density-replica.html accessibility below the R3B floor — `role=0/1 aria-*=0/1 tabindex=0/0`
- `MEDIUM` A8-R3-visual-density-frames.html accessibility below the R3B floor — `role=0/1 aria-*=0/1 tabindex=0/0`

### P3 [HIGH×0 + MED×1]

**裁定修复**：验收证据须覆盖 A0 四强制尺寸（1920×1080/1440×900/1366×768/1024×720）；若 A0 改判六尺寸全强制（--strict-sizes），还需补 1200×800 与 900×600 帧。800×600 仅保留「剔除」说明，不得作为可达帧。

- `MEDIUM` A8 evidence omits product-edge size(s) (1200x800 default / 900x600 min inner) — reported as hint — `1200x800, 900x600  [group: A8-R3-visual-density-frames.html, A8-R3-visual-density-design.md]`

## Lane A10（7 项）

### P7 [HIGH×1 + MED×6]

**裁定修复**：参照钉版：cee14e9 为 SSOT；2896562e 仅作降级研究快照（引用处须标注「降级、非 SSOT、终裁前不得 W19 采用」）。COPY 分类维度删除（A0 裁定 COPY=0，并入 REJECT/forbidden）；tag 精确记为 refs/tags/1.1.15（无 v 前缀）。

- `HIGH` A7-R3-git-workflow-reference-map.md classifies a unit as COPY (A0 ruled COPY=0) L185 — `| COPY | 0 | — (forbidden: Kotlin/JVM Apache-2.0 → Rust/MulanPSL-2; A10 provenance gate required first) |`
- `MEDIUM` A7-R3-git-workflow-reference-map.md writes the tag as v1.1.15; the upstream ref is refs/tags/1.1.15 (no v prefix) -> record the exact ref name
- `MEDIUM` A10-R3-acceptance-preflight-20260908.md cites 2896562e without marking it a demoted research snapshot L14 — `| 参照身份/许可 | `A10-R3-reference-and-feasibility-review-20260908.md` | rebased SHA `2896562e…`；Build Terms v1.3；NOTICE 3 行 `
- `MEDIUM` A10-R3-reference-and-feasibility-review-20260908.md cites 2896562e without marking it a demoted research snapshot L15 — `1. **参照身份可确认、许可不可简化**：`DetachHead/rebased` 确为 `JetBrains/intellij-community` 的 fork（GitHub API `fork=true`，`parent/sourc`
- `MEDIUM` A10-checkpoint-R3-20260908.md cites 2896562e without marking it a demoted research snapshot L19 — `1. `DetachHead/rebased` = `JetBrains/intellij-community` fork（已确认），**固定修订 `2896562e69ff2cac3c90eb3aae4bcce0d4aa9a99`（202`
- `MEDIUM` A11-R3-checkpoint.md cites 2896562e without marking it a demoted research snapshot L97 — `| F-A11-9 | medium | A7 钉 `cee14e9` 与 A10 钉 `2896562e` 不一致；A11 采纳 A10 权威钉版，A0/A10 终裁前不得 W19 采用任何 COPY/ADAPT 单元 |`
- `MEDIUM` A11-R3-progress.md cites 2896562e without marking it a demoted research snapshot L26 — `| Git 完整工作流 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | A7 实测：14 单元中 5 ADAPT（status/diff/branches/merge，其中 merg`

## Lane A11（6 项）

### P7 [HIGH×1 + MED×5]

**裁定修复**：参照钉版：cee14e9 为 SSOT；2896562e 仅作降级研究快照（引用处须标注「降级、非 SSOT、终裁前不得 W19 采用」）。COPY 分类维度删除（A0 裁定 COPY=0，并入 REJECT/forbidden）；tag 精确记为 refs/tags/1.1.15（无 v 前缀）。

- `HIGH` A7-R3-git-workflow-reference-map.md classifies a unit as COPY (A0 ruled COPY=0) L185 — `| COPY | 0 | — (forbidden: Kotlin/JVM Apache-2.0 → Rust/MulanPSL-2; A10 provenance gate required first) |`
- `MEDIUM` A7-R3-git-workflow-reference-map.md writes the tag as v1.1.15; the upstream ref is refs/tags/1.1.15 (no v prefix) -> record the exact ref name
- `MEDIUM` A10-R3-reference-and-feasibility-review-20260908.md cites 2896562e without marking it a demoted research snapshot L15 — `1. **参照身份可确认、许可不可简化**：`DetachHead/rebased` 确为 `JetBrains/intellij-community` 的 fork（GitHub API `fork=true`，`parent/sourc`
- `MEDIUM` A10-checkpoint-R3-20260908.md cites 2896562e without marking it a demoted research snapshot L19 — `1. `DetachHead/rebased` = `JetBrains/intellij-community` fork（已确认），**固定修订 `2896562e69ff2cac3c90eb3aae4bcce0d4aa9a99`（202`
- `MEDIUM` A11-R3-checkpoint.md cites 2896562e without marking it a demoted research snapshot L97 — `| F-A11-9 | medium | A7 钉 `cee14e9` 与 A10 钉 `2896562e` 不一致；A11 采纳 A10 权威钉版，A0/A10 终裁前不得 W19 采用任何 COPY/ADAPT 单元 |`
- `MEDIUM` A11-R3-progress.md cites 2896562e without marking it a demoted research snapshot L26 — `| Git 完整工作流 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | A7 实测：14 单元中 5 ADAPT（status/diff/branches/merge，其中 merg`

