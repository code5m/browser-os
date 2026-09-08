# A10 — M5-W18-R3 独立复核：参考身份/许可边界 + 原型可实现性

- Lane：A10（独立 UX 与参考复核）
- 模式：`RESEARCH_AND_PROTOTYPE`（研究/复核，**零产品代码改动、零依赖/ACL/capability 改动、不 push**）
- 工作树：`/home/ainfinit/.codex/worktrees/m5-w18-a10/mvp-browser-os-v3`
- 分支：`codex/m5-w18-a10`（已 `git rebase origin/master`，本次复核起点 = `200f0f1`）
- 时间：2026-09-08
- 卡片来源：`M5-W18-R3-UX-TASKS-20260908.md`（A10 卡）、`M5-W18-PROTOTYPE-REVIEW-20260908.md`（用户裁决）、`WORKBENCH_BLUEPRINT-20260908.md`
- 本波产物：本报告 + `A10-R3-viewport-budget.py` + `A10-checkpoint-R3-20260908.md`

---

## 0. 结论摘要

1. **参照身份可确认、许可不可简化**：`DetachHead/rebased` 确为 `JetBrains/intellij-community` 的 fork（GitHub API `fork=true`，`parent/source=JetBrains/intellij-community`），HEAD 修订 `2896562e69ff2cac3c90eb3aae4bcce0d4aa9a99`（2026-09-06）。但其 `LICENSE.txt` 是 **JetBrains Open-Source Build Terms v1.3（2026-06-15 生效）**，不是 Apache-2.0；仓库 SPDX 为 `NOASSERTION`。"内含开源软件适用 Apache-2.0" 只是其中一句，外层条款是**附加义务**（个人数据处理/遥测、Feedback 授权、出口管制、捷克法+仲裁、权利保留）。**因此 JetBrains 作者的任何文件，默认分类只能是 `REIMPLEMENT_FROM_BEHAVIOR`，`COPY` 在拿到 A0/法务对 Build Terms v1.3 的书面接受前不予通过。**
2. **自研增量不是独立模块，而是"上游平台源码里的小改"**：抽查 11 个 rebased 自有提交，改动全部落在 `platform/vcs-log/**`、`platform/tasks-platform-impl/**`、`platform/platform-resources/**`、`platform/build-scripts/**`、`platform/core-api/**` 与 `.github/**` 上。R3 最想抄的行为（"Git log 作为中央编辑器内容 / 底部工具窗口"）正落在 `platform/vcs-log/impl/**` —— **属于继承的 JetBrains 平台代码，源码复用判 REJECT，只能按行为重实现**。
3. **被否决原型的"更小更挤"可以量化**：`A1-wireframe-prototype-R2B.html` 的 1440 帧为 top-bar 40 + activity 48 + left 260 + right 240 + bottom 240 + status 28，中央内容仅 **892px 宽（61.9%）/ 592px 高（65.8%）**；1024 帧为 **756px（73.8%）/ 472px（65.6%）**。R3 的"折叠态 ≥92% 宽 / ≥85% 高"正是针对这个数字。
4. **R3 的 Git 全流程不是依赖问题，是"命令面 + 预算 + 状态机"问题**：`git2 = 0.19` 已是现有依赖，且已暴露 rebase/cherry-pick/merge/stash/blame/worktree 等 API（无需新依赖、无需 `git` CLI 子进程）。但当前只落地 `git_status/git_diff/git_branch_list` + 写闸门（Stage/Unstage/Discard/Commit/CreateBranch/CheckoutBranch/Push，**均为路径/分支粒度**）；R3 要求的 hunk 暂存、log/graph、worktree、stash、merge、交互式 rebase、cherry-pick、冲突解决、blame、patch、命令日志**全部为新增能力**。
5. **体积是硬约束**：基线 `4f0e8ab`=612,943B，上限 25.2% ⇒ 767,405B；`logs/m0-build-metrics/build-metrics-052b18a.json` 记录 795,517B（脏树采集，29.8%，会 FAIL），干净 052b18a 约 767,059B ⇒ **余量约 346B（0.06pp）**。蓝图 §7 禁止靠抬阈值过关。**任何"一屏 Git 工作台"原型都不应被当成可在单片内实现。**
6. **A2–A9 的 R3 报告在本次复核时均未出现在 `master`（已核查，无 R3 产物）**。故本报告先交付"**基线事实 + 可机械执行的复核闸门 G1–G8 + 冲突预告**"，待 A2–A9 落地后由 A10 出终审（卡片序列第 2 步）。

---

## 1. 方法、证据与边界

- 参考核验走 GitHub REST API + raw 抓取（`api.github.com/repos/...`、`raw.githubusercontent.com/...`），**未克隆仓库**：`size`≈5,294,926 KB（约 5.3GB），克隆需 A0 单独批准（如需请用 blobless + sparse）。
- 产品事实走本工作树源码检索（`src/**`、`src-tauri/**`、`scripts/**`、`logs/m0-build-metrics/**`），**没有跑 GUI，没有跑构建**：凡像素数字均标注 `source-derived`（file:line）或 `estimate`，**不冒充实测**（遵守蓝图 §7：`NOT_RUN` 不能写 `PASS`）。
- 复核范围严格限定：未修改任何产品代码、依赖、ACL、capability，未触碰他 lane 文件。

证据复现命令（A0/A11 可原样重跑）：

```bash
python3 logs/research/M5-W18/A10-R3-viewport-budget.py
curl -s https://api.github.com/repos/DetachHead/rebased | python3 -c "import sys,json;d=json.load(sys.stdin);print(d['fork'],d['parent']['full_name'],d['default_branch'],d['license'])"
curl -s https://raw.githubusercontent.com/DetachHead/rebased/master/LICENSE.txt | head -3
curl -s https://raw.githubusercontent.com/DetachHead/rebased/master/NOTICE.txt
```

---

## 2. Rebased 参照独立核验

### 2.1 身份与修订（已确认）

| 项 | 值 | 证据 |
|---|---|---|
| 仓库 | `DetachHead/rebased` | API `full_name` |
| fork 身份 | `fork=true`，`parent=source=JetBrains/intellij-community` | API |
| 默认分支 | `master` | API |
| **建议固定修订** | **`2896562e69ff2cac3c90eb3aae4bcce0d4aa9a99`（2026-09-06T09:15:36Z，message: `document the upstream merging process (#347)`）** | API `/commits/master` |
| 上游 HEAD（同时间） | `61c385761745149a60b699d90f4bdac4aa736308`（2026-08-31） | API `/repos/JetBrains/intellij-community/commits/master` |
| 体量 | 5,294,926 KB；star 5,436 | API `size` |
| 自身许可元数据 | `license.key=other`、`spdx_id=NOASSERTION`（上游同为 NOASSERTION） | API |

**注意（F15）**：跨仓 compare `JetBrains/intellij-community:master...DetachHead/rebased:master` 返回 **404**，因此"fork 领先/落后多少"本次**无法计算**。固定引用必须**按 SHA**，不得按日期；任何 lane 若写"fork 落后 N 个提交"而无本地克隆证据，判为 unverified。

### 2.2 继承 vs 自研边界（已确认，且结论对 R3 不利）

顶层目录为完整 IntelliJ 社区版（`platform/ plugins/ java/ python/ xml/ jps/ build/ lib/ native/ ...`），说明绝大多数是**继承代码**。

抽查 rebased 自有提交（作者 DetachHead）的改动文件：

| 提交 | 主题 | 改动路径 |
|---|---|---|
| `c0909096` | editor log tab 无法关闭 | `platform/vcs-log/impl/src/com/intellij/vcs/log/ui/CanCloseVirtualLogFile.kt` |
| `dd9be09b` | 关闭"Show the log in the editor window"后崩溃 | `platform/vcs-log/impl/src/com/intellij/vcs/log/impl/IdeVcsLogManager.kt` |
| `73e6f746` | checkout 分支崩溃 | `platform/tasks-platform-impl/.../BranchContextTracker.java`、`ConfigureBranchContextDialog.java` |
| `8cb56172` | 移除工具栏 Build 动作 | `platform/platform-resources/src/META-INF/PlatformExtensions.xml` |
| `2630967f` | 回退 `build.txt` 版本校验 | `platform/build-scripts/src/org/jetbrains/intellij/build/impl/BuildContextImpl.kt` |
| `4b86f650` | 支持调试面向 Rebased 的插件 | `community-resources/resources/META-INF/plugin.xml`（rename）、`platform/core-api/.../ApplicationNamesInfo.java`、`README.md` |
| `1f8708d9`/`5fe9b155`/`0475ae0c`/`c089d649`/`2896562e` | 工程与发布 | `.gitignore`、`.github/workflows/**`、`build/src/OpenSourceCommunityInstallersBuildTarget.kt`、`README.md`、`CONTRIBUTING.md` |

**判读**：rebased 的差异化价值（"Git log 可作为编辑器标签页/底部工具窗口"、UI 微调、构建裁剪）不是独立可移植模块，而是**散落在 JetBrains 平台源码里的少量修改 + 构建配置**。

### 2.3 插件裁剪机制（部分确认，仍有未决）

- `plugins/` 目录：rebased 95 个、上游 97 个；rebased 缺失：`extended-toolwindows-ui`、`lsp.client.playground`、`minimap`、`tips-of-the-day`、`ui.webview`；rebased 独有：`agent-workbench`、`evaluation-plugin`、`junit6_rt_tests`（**归属未核实**，可能只是上游已删除/改名的时间差）。`plugins/git4idea` **存在**（Git 集成保留）。
- 真正表达"裁剪"的是**生成的模块集构建文件**：`module-set-plugins/generated/intellij.moduleSet.plugin.main/{BUILD.bazel,.iml}`，与上游存在可验证差异（rebased 移除 `//platform/vcs/plugin`、`//platform/problemsView/plugin`、`//platform/tasks-platform-impl/plugin`，加入 `//libraries/misc/plugin`）。
- `intellij.yaml` 与上游**逐字节相同（480B）**，不是裁剪点；`.bazelignore` 亦非裁剪点。
- **未决（交 A7）**：完整"保留/移除插件清单"尚未定位到单一文件。方法：比对 fork 与上游的 `module-set-plugins/generated/**` 与 `build/` 下产品定义（`CommunityRepositoryModules` 类及同类），逐项记录并标注 license。

### 2.4 LICENSE.txt / NOTICE.txt（已确认）

- `LICENSE.txt`（91 行）标题 **JETBRAINS OPEN-SOURCE BUILD TERMS, Version 1.3, effective as of June 15, 2026**。关键条款：
  - L9：open-source build **consists of** OSS subject to Apache-2.0（只是构成说明）
  - L21-43：个人数据处理与匿名数据、可选遥测（含 JetBrains 账号口令一类字段的收集叙述）
  - L47-51：**Feedback 授予 JetBrains 非独占、全球、免费、可再许可、可转让的许可**（含修改/公开表演/分发）
  - L53-56：第三方软件按各自条款
  - L58-65：AS IS、免责与责任排除
  - L67-69：出口管制合规义务
  - L74-87：**捷克法管辖 + 仲裁（捷克商会仲裁院，三仲裁员，英语）**
  - L71-73：JetBrains 保留随时变更/停止提供的权利；L88+：声明已充分审阅，"格式合同"规则不适用
- `NOTICE.txt`（3 行）：“This software includes code from IntelliJ IDEA / Copyright (C) JetBrains s.r.o. / https://www.jetbrains.com/idea/”
- 本产品自身许可：`LICENSE` = **木兰宽松许可证 第 2 版（MulanPSL-2.0）**；`package.json` / `src-tauri/Cargo.toml` **未声明 license 字段**（F02b，低危但需补）。

**A10 裁决**：Build Terms v1.3 是**附加限制**，与"纯 Apache-2.0"不等价，与 MulanPSL-2.0 的兼容组合需法务/ A0 明确。因此：
- 对 JetBrains 作者文件：`REJECT`（源码复用）/ `REIMPLEMENT_FROM_BEHAVIOR`（行为复制）
- 若将来确需复制个别文件，前置条件：① A0 书面接受 Build Terms v1.3；② 文件级 provenance（仓库 + SHA + 路径 + 许可）；③ 产品侧 NOTICE 增加 IntelliJ IDEA/JetBrains s.r.o. 归属声明；④ 不得带入 branding/资源（`community-resources/resources/rebased.svg`、`screenshot.png`、产品名 "Rebased"/"IntelliJ IDEA"）。

### 2.5 对"每一个复用分类"的裁决规则（A10 复核口径）

| 分类 | 允许的证据门槛 |
|---|---|
| `COPY` | 仅允许**非 JetBrains** 来源且有明确 Apache-2.0/MIT 许可文件；给出仓库+SHA+路径+许可文件行号；**JetBrains 来源一律不得 COPY** |
| `ADAPT` | 同上来源要求，且需说明改造点与为何不是 COPY |
| `REIMPLEMENT_FROM_BEHAVIOR` | 需给出可观察行为描述 + 本产品现有能力落点（文件:行）；这是 Rebased 的**默认分类** |
| `REJECT` | 需一句话理由（branding/平台耦合/GPL/裁剪无关/不可验证） |

---

## 3. 被否决原型的量化复核（"为什么更小更挤"）

来源：`logs/research/M5-W18/A1-wireframe-prototype-R2B.html`（用户裁决的输入 1，source-derived，非实测）。

| 帧 | top-bar | activity | left | right | bottom | status | 中央内容宽 | 宽占比 | 中央内容高* | 高占比 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1440×900 | 40 | 48 | 260 | 240 | 240 | 28 | 892 | **61.9%** | 592 | **65.8%** |
| 1024×720 | 40 | 48 | 220 | — | 180 | 28 | 756 | **73.8%** | 472 | **65.6%** |
| 800×600 | 36 | 40 | — | — | — | 24 | 760 | 95.0% | 540 | 90.0% |

\* 高按"帧高 − top − bottom − status"粗算（未扣 WM 标题栏），用于相对比较。

另有 **三套标签行并存**（`doc-tabs` + `panel-tabs` + 底部 `bp-tabs`），与用户"别更挤"的裁决直接冲突。

**给 A1（R3 替代原型）的硬要求**：同一份表格必须在 R3 原型里重算并写进文档；**默认态**不得出现"left 260 + right 240 + bottom 240 同时常驻"的组合；文档标签只保留**一条**主标签行，浏览器页签与工具窗口页签需明确合并策略。

---

## 4. 当前产品事实基线（用于判断"能否实现"）

### 4.1 外壳与窗口

- 结构：`App.vue` = `ActivityBar`（`v-show=!layout.compactMode`）+ `body`（`AINavPanel` + `MainArea`）+ `StatusBar`（`height: 26px`）。
- `ActivityBar`：主行控件 `height: 24px`（`.tbtn`），扩展行 `padding: 4px 8px`、按钮 `padding: 3px 9px`；**`.omni-row { max-height: 74px }`**；主行无固定高度（估算 ≈34px，**estimate，未实测**）。
- 窗口：`src-tauri/src/main.rs:1225-1226` → `inner_size(1200, 800)`、`min_inner_size(900, 600)`。`tauri.conf.json` 的 `app.windows` 为 `[]`（主窗程序化创建）。
- **命令注册表/命令面板：不存在**（`src` 全量检索 `palette|quickopen|cmdK|command palette` → 0 命中）。

### 4.2 Git 能力现状

- ACL（`src-tauri/permissions/default-commands.toml:75-79`）：`git_status`、`git_diff`、`git_branch_list`、`request_git_write`、`confirm_git_write`（本机解析 allow 列表共 **135** 条，主窗全量放行；与 A9 命令集一致性脚本口径一致）。
- 实现：`src-tauri/src/bridge.rs:2109/2123/2140/2367/2480` 包装 `src-tauri/src/sync.rs`（注释明确"只允许 git2 只读 API"，写操作经闸门）。
- 写操作枚举（`src-tauri/src/domain.rs:234-264`）：`Stage / Unstage / Discard / Commit / CreateBranch / CheckoutBranch / Push` —— **全部路径或分支粒度**。
- 依赖：`git2 = "0.19"` 已在 `[dependencies]`；Rust 侧**无 `"git"` 子进程调用**（未发现该字面量），即**不存在第二条执行路径**。

### 4.3 体积

- `scripts/measure-build-metrics.py:39`：`TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2`。
- `logs/m0-build-metrics/build-metrics-4f0e8ab.json`：`dist.total_bytes = 612,943`。
- `logs/m0-build-metrics/build-metrics-052b18a.json`：`dist.total_bytes = 795,517`（**脏树采集，29.8%，会 FAIL**；干净 052b18a 约 767,059）。
- ⇒ 允许上限 767,405B；干净余量约 **346B（0.06pp）**。**当前 HEAD 尚无干净采集，A0 需重采**（沿用既有挂账）。

---

## 5. R3 Git 全流程可实现性矩阵

判定口径：**A**=已有能力；**B**=git2 0.19 可达（需新命令+UI，无新依赖）；**C**=git2 可达但需持久状态机/冲突 UI/凭据；**D**=git2 无高层 API，需自写或建议拒绝。

| R3 工作流项 | 现状 | git2 0.19 依据（docs.rs 方法清单） | 判定 | 备注/风险 |
|---|---|---|---|---|
| 仓库切换 | 工作区目录（`startDirs/enterDir`） | — | **A(部分)** | 缺"仓库身份"模型，归 A4 |
| status | `git_status` | `statuses` | **A** | — |
| 暂存/撤销（文件级） | 闸门内 `Stage/Unstage` | `index`/`Index` | **A** | — |
| **暂存（hunk/行级）** | 无 | `diff_blobs` + `Index` 手写 blob | **D** | 无高层 API；最高风险项 |
| diff（统一/并排） | `git_diff`（文本） | `Diff*`/`DiffHunk`/`DiffLine` | **B** | 并排为 UI；hunk 选择与 D 同源 |
| commit | 闸门内 `Commit` | — | **A** | — |
| branch 列表/新建/切换 | 已有 | `branches/find_branch` | **A/B** | 删除/重命名缺失（B） |
| **log / graph** | 无 | `revwalk`、`graph_ahead_behind`、`graph_descendant_of` | **B** | 图渲染需有界（树/图容量约束已存在） |
| **worktree** | 无 | `worktree/worktrees/find_worktree/open_from_worktree` | **B** | 与工作区/文档身份耦合（A4） |
| **stash** | 无 | `stash_save/save2/pop/apply/drop/foreach` | **B** | 写操作须过闸门（A9） |
| **merge** | 无 | `merge_analysis/merge_commits/merge_trees` | **C** | 冲突解决 UI 为净新增 |
| **rebase / 交互式 rebase** | 无 | `rebase/open_rebase`、`Rebase`、`RebaseOperationType` | **C** | 交互式=持久化 sequencer 状态机，跨重启（A4 + 关闭协调） |
| **cherry-pick** | 无 | `cherrypick/cherrypick_commit` | **B/C** | 冲突时转 C |
| **revert / reset** | 无 | `revert/revert_commit/reset` | **B** | 破坏性，A9 定级 |
| **冲突解决** | 无 | `IndexConflict(s)`、`checkout_index` | **C** | 三方合并编辑器为净新增，体量最大 |
| **blame / 文件历史** | 无 | `blame_file`、`Blame*` | **B** | — |
| **patch** | 无 | `apply`/`apply_to_tree`（**无 format-patch 高层 API**） | **D** | 导出需自写或判 REJECT |
| **tag** | 无 | `tag/tag_names/find_tag` | **B** | — |
| **remote fetch/pull** | 仅 `Push` | `Remote::fetch`（`find_remote/remote_anonymous`） | **C** | 凭据走 keyring，须 A6/A9 安全评审 |
| **命令日志** | 无 | 产品侧 `log_audit`（1000 FIFO） | **B** | 审计写放大，tick 内禁写（既有约束） |

**总体结论**：R3 的 Git 全流程**在技术栈上可达**（git2 已就位、无新依赖、无需 `git` CLI、不产生第二条执行路径），但：
1. 需要**成批新命令**——按 `WORKSPACE_IDENTITY.md` 的原子交付定义，每条 = Rust 实现 + `check_invocation_source` + `generate_handler!` + ACL + `bridge.ts`/`types.ts` + 策略/测试；
2. 每条都撞**同一份体积预算**（余量 ≈346B）；
3. **hunk 暂存（D）**、**冲突解决/交互式 rebase（C）** 是真正的不可压缩工作量，且交互式 rebase 与 A4 持久化、关闭协调、崩溃恢复直接耦合。

**因此：任何把"完整 Git 工作台"画成一屏的原型，必须同时标注"分几片落地、每片多少命令、预算从哪来"，否则 A10 判 `NOT_IMPLEMENTABLE_AS_DRAWN`。**

---

## 6. 视口算术：R3 数字之间的冲突（已复算）

运行 `python3 logs/research/M5-W18/A10-R3-viewport-budget.py`（产物在该脚本输出中），结论：

1. **宽度**：折叠态 `(W − side)/W ≥ 92%` ⇒ `side ≤ 8%·W`
   - 1440 → **≤115.2px**；1366 → ≤109.3px；1024 → **≤81.9px**；1920 → ≤153.6px
   - ⇒ **蓝图 §3 的"左工具区默认约 260px"在折叠态下必然违规**；48px 图标栏通过；**1024 宽度下"左右各 48px 双栏"（96px）也 FAIL**。
2. **高度**：折叠态竖向 chrome ≤15%·Hpost
   - 1440×900（标题栏 32）→ ≤130.2px；1024×720 → ≤103.2px
   - ⇒ 240px 底部工具窗口在折叠态**必然违规**（除非"折叠态"定义为底部完全隐藏）。
3. **顶部行**：正常态 ≤2 行且 ≤80px。当前主行(≈34) + `.omni-row`(74) = **108px 超限**；A3/A8 必须明确哪些行算"正常"、omni 行如何压到 ≈46px 以内或归类为扩展态。
4. **窗口事实**：默认 1200×800、最小 900×600；1024×720 可达但非默认；**Linux 装饰高度由 WM 决定**，"post-titlebar height"必须实测或显式声明假设值。

**要求**：A2（密度审计）、A3（工具窗口）、A8（视觉密度）必须在各自报告里给出与上表同一口径的算术，并声明"折叠态"的精确含义（哪几块被隐藏）。数字不一致时，以本脚本口径仲裁，或由 A0 裁定改约束。

---

## 7. 所有权冲突与覆盖缺口（R3 卡集内部）

| # | 冲突/缺口 | 涉及 lane | A10 建议裁决 |
|---|---|---|---|
| 1 | "持久化"同时出现在 A3（工具窗口状态）与 A4（状态与持久化契约） | A3 / A4 | **A3 只拥有行为与状态迁移规则；A4 独占 DTO/store/schema_version/迁移/崩溃恢复** |
| 2 | "破坏性确认"同时在 A6（上下文菜单）与 A9（确认分级） | A6 / A9 | **A6 声明每个动作的 safety class；A9 定义分级策略与升级路径**，不得各自出一套分级 |
| 3 | 命令 ID 命名空间：A6 要"每个动作有命令注册表身份"，A7 出 Git 命令矩阵 | A6 / A7 / A1 | **A7 提出 Git 命令 ID；A6 拥有注册表结构与 disabled-reason；A1 原型不得发明 A7 未定义的 ID** |
| 4 | 分隔条最小尺寸（A3）vs 视觉间距（A8） | A3 / A8 | A3 的最小尺寸是**硬下限**，A8 的间距不得突破 |
| 5 | **覆盖缺口：原生子 WebView（浏览器页签）在工具窗口 resize/重父容器下的位置、焦点、遮挡** | 无 lane 认领 | 蓝图 §3 与 `WORKSPACE_IDENTITY.md` 均要求专项目设计+实机验证（历史上 `queue_resize/set_size_request/webview.hide()` 已失败过）。**请 A0 指派**（建议 A1 原型 + A9 威胁模型 + A0 native 验收） |
| 6 | **覆盖缺口：无障碍**——R3 要求每个动作有键盘/无障碍路径，但无 lane 明确认领 | A6 / A8 / A11 | 建议 A6 拥键盘路径、A8 拥焦点/对比度可见性、A11 验收 |
| 7 | R3 目标尺寸含 1366×768/1920×1080，但默认窗口 1200×800 | A2 / A11 | 原型须注明"1366 宽大于默认宽"，且不得把 1200×800 当作已验收尺寸 |

---

## 8. A10 复核闸门（A1–A9 交付物的机械检查项）

**G1 复用证据**：任何 `COPY`/`ADAPT` 声明须含：仓库、固定 SHA、文件路径、许可文件及版本、NOTICE 义务；缺一即退回。
**G2 JetBrains 来源**：任何 JetBrains 作者文件标 `COPY`/`ADAPT` 一律退回（Build Terms v1.3 未获书面接受前）。
**G3 无品牌/资源**：原型与文档不得出现 rebased/IntelliJ 的 logo、svg、截图、产品名作为 UI 元素（作为引用出处可以）。
**G4 动作可落地**：原型里每个可见动作必须映射到 ①现有 ACL 命令 ID，或 ②显式标注 `PROPOSED_NEW` + 责任 lane + 预算归属；**不允许孤儿动作**。
**G5 数字可核实**：每个 px/% 要么 `source-derived`（file:line），要么 GUI 实测（记录运行方式）；估算须标 `estimate`；**不得出现无来源的百分比**。
**G6 视口算术**：须通过/显式偏离 `A10-R3-viewport-budget.py` 的推导上限。
**G7 状态唯一归属**：每个状态字段只能有一个 owner；两个 lane 都声称拥有即判冲突（见 §7）。
**G8 原型自包含**：不 import 产品源码、不改依赖、不改 ACL/capability、不调用原生命令、不使用真实用户数据（R3 卡已约束，A10 将逐项 grep 验证）。

**各 lane 交付核对清单**（A10 终审时逐条打勾）：A1 原型（含折叠/常规/数据库/知识/Git/右键/焦点七态 + 四尺寸）+ 重算后的视口表；A2 密度审计（每个可见按钮分类 + before/after 预算，口径对齐 §6）；A3 工具窗口与树行为（状态迁移表 + 分隔条下限 + 键盘焦点）；A4 状态与持久化（DTO/store/schema_version/未知视图回退 + 每工作区/全局边界）；A5 数据库（多文档/取消/截断/错误/右键）；A6 上下文菜单与命令注册表（对象域清单 + 禁用原因 + 安全分级 + 键盘/面板一致性）；A7 Git 参考映射（固定 SHA + 继承/自研边界 + 逐单元分类 + 命令矩阵 + license/NOTICE）；A8 视觉密度（四尺寸标注帧 + 客观视口测量）；A9 交互安全（威胁模型 + 确认分级 + 审计脱敏）。

---

## 9. 发现登记

| ID | 严重度 | 发现 | 责任 | 要求动作 |
|---|---|---|---|---|
| F01 | **High** | rebased `LICENSE.txt` = JetBrains Open-Source Build Terms v1.3（附加义务：遥测/个人数据、Feedback 授权、出口管制、捷克法+仲裁），SPDX `NOASSERTION`，非纯 Apache-2.0 | A0 / A7 | COPY 需 A0 书面接受 v1.3；否则一律 REIMPLEMENT_FROM_BEHAVIOR |
| F02 | High | NOTICE 义务：复制即须在产品侧声明 IntelliJ IDEA / JetBrains s.r.o. | A0 / A11 | 若发生复制，追加 NOTICE 条目 |
| F02b | Low | `package.json`/`Cargo.toml` 无 `license` 字段（产品为 MulanPSL-2.0） | A0 | 补字段（非本波） |
| F03 | **High** | rebased 自研增量是"上游平台源码内的小改"（`platform/vcs-log/**` 等），无独立可移植模块 | A7 | Git log 双落位只能按行为重实现 |
| F04 | Medium | 插件裁剪实现于 `module-set-plugins/generated/**` 构建文件；完整清单未定位；`agent-workbench/evaluation-plugin/junit6_rt_tests` 归属未核实 | A7 | 用给定方法定位并逐项标注 license |
| F05 | **High** | 体积：上限 767,405B，干净 052b18a≈767,059B ⇒ 余量 ≈346B；且现有最新采集为脏树（795,517B/29.8%） | A0 / A6 | 重采干净基线；R3 任何分片须声明预算归属 |
| F06 | Medium(正面) | `git2 0.19` 已提供 rebase/cherry-pick/merge/stash/blame/worktree 等 API，且无 `git` CLI 子进程 | A7 / A0 | 明确"不引新依赖、不建第二条执行路径"为硬约束 |
| F07 | **High** | 现有 Git 仅 3 读 + 写闸门 7 操作（路径/分支粒度）；hunk 暂存、log/graph、worktree、stash、merge、交互式 rebase、cherry-pick、冲突解决、blame、patch、命令日志全为新增 | A7 / A1 / A0 | 原型须按 §5 矩阵标注 A/B/C/D 与分片 |
| F08 | **Medium** | 蓝图"左 260/底 240"与 R3"折叠 ≥92% 宽 / ≥85% 高"冲突（1440 侧栏上限 115.2px；1024 上限 81.9px，双 48px 栏亦 FAIL） | A2 / A3 / A8 | 明确定义"折叠态"并统一口径 |
| F09 | Medium | 主行(≈34)+omni 行(74)=108px > 80px 上限 | A3 / A8 | 分类"正常/扩展"行并给压缩方案 |
| F10 | Medium | 命令注册表/面板在前端**完全不存在**（grep 0 命中） | A6 / A1 | 明确为净新增基础设施，原型不得假设已有 |
| F11 | Medium | 卡集内部所有权重叠（A3↔A4 持久化；A6↔A9 确认分级；A6↔A7 命令 ID） | A0 | 按 §7 第 1–4 项裁决 |
| F12 | Medium | 原生子 WebView 在工具窗口重排下的位置/焦点/遮挡**无 lane 认领** | A0 | 指派 owner |
| F13 | Low | R3 尺寸集含 1366/1920，默认窗口仅 1200×800；Linux 装饰高度 WM 相关 | A2 / A11 | 原型注明假设 |
| F14 | Low | 无障碍无明确 owner | A6 / A8 / A11 | 按 §7 第 6 项分工 |
| F15 | Low | 跨仓 compare 404 ⇒ fork 与上游的领先/落后不可计算；仓库 ≈5.3GB 不建议克隆 | A0 / A7 | 按 SHA 固定；如需 diff 用 blobless+sparse 并获 A0 批准 |

---

## 10. 状态、债务与未决

- **STATUS = `PRE-REVIEW_COMPLETE`（等待 A2–A9 的 R3 产物后出终审）**。
- 未决依赖：
  1. A2–A9 的 R3 报告（本次复核时 `master` 上尚无 R3 产物，已核查）；
  2. A0 对 F05（预算重采与分片）与 F12（WebView owner）的裁决；
  3. A7 对 F03/F04（继承边界与插件清单）的补齐。
- 债务：D-A10-R3-1（未跑 GUI，所有像素为 source-derived/estimate）；D-A10-R3-2（fork↔上游提交级差异未计算，compare 404）；D-A10-R3-3（rebased 三个独有目录归属未核实）。
- **本波零产品代码改动、零依赖/ACL/capability 改动、未 push**；仅新增 A10 lane 自有研究文件。
