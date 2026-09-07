# A1 M5-W17 Acceptance Closeout checkpoint（DOCS ONLY · 2026-09-08 12:30 CST）

> Lane A1（M5-W17 Acceptance Closeout · 记录 W17 证据 + 唯一真实 home 存储债 + 原生客户端手动验收边界）
> 派发来源：`PARALLEL_COMMAND_BOARD.md` L1203-1229（**M5-W17 Acceptance Closeout Dispatch**）
> A1 任务行（board L1209）：*Record W17 evidence, the one real home-storage debt, and the native-client manual-acceptance boundary. No product code.*
> 派发证据（board L1205 原文）：*"Current evidence: targeted W17 checks PASS, `npm run build` PASS, and `bash scripts/pre-merge.sh` is `ALL_PASS`. Do not repeat completed UI work. The remaining product debt is `HOME_NO_SECRET_PERSIST`: app execution command bodies are still stored in browser storage by `useHomeStore.ts`. Native desktop visual acceptance is still user-side evidence only."*
> 配套交付：A1 W17 用户可见验收清单 `logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`
> 立场：**DOCS ONLY · 零产品代码 · 不 commit · 不 push**
> 集成顺序（board L1223）：`A3 -> A4/A9 -> A2/A8/A10 -> A1 -> A11 -> A0`

---

## §0 一句话

A1 在 W17 收口波只做三件记录：**(1)** W17 证据（已跑门禁实测 + 各方 lane 结论）；**(2)** 唯一真实 home 存储债 `HOME_NO_SECRET_PERSIST`（**实测已闭环**，但 board 派发时的表述仍为"未修"，须显式更正对账）；**(3)** 原生客户端手动验收边界（headless 环境 BLOCKED，视觉证据仍在用户侧，**未伪造截图**）。另按 HOLD 规则复核 A5/A6/A7：**均无具体阻塞，不 reopen、不重复开发**。

---

## §1 启动门禁与实测（A1 自跑 · 只读，不改产品代码）

| 项 | 命令 | 实测结果 | 判定 |
|---|---|---|---|
| 同步 | `git fetch origin && git pull --ff-only` | `已经是最新的`；HEAD = `origin/master` = `052b18a` | ✅ |
| 构建 | `npm run build` | `✓ built in 3.51s` | ✅ PASS |
| 空白/冲突标记 | `git diff --check` | 干净（无输出） | ✅ CLEAN |
| home 隐私门禁（默认/ACTIVE） | `python3 scripts/check-home-client-policy.py` | `home client policy: all invariants hold（ACTIVE=3）` | ✅ PASS |
| home 隐私门禁（pending 码位） | `python3 scripts/check-home-client-policy.py --expect-pending` | **`HOME_CLIENT_POLICY_PENDING_RESULT=NONE`（3 个 pending 码位均未检出）`** | ✅ **全闭环** |
| home store 纯逻辑 | `node scripts/check-home-store-logic.mjs` | `通过 105，失败 0` | ✅ PASS |
| 体积指标（脏树全量口径） | `python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-4f0e8ab.json --skip-build` | `total_bytes_pct=29.77`，`exceeds_growth_limit=true`，`cargo_warnings=0` | ⚠️ 见 §2.3 |

**未复跑项（诚实声明）**：`bash scripts/pre-merge.sh` 与 full Rust 测试属 **A11/A0 验证职责**（board L1219），A1 未独立复跑；board L1205 记录的 `ALL_PASS` 为 **A0/A11 口径**，A1 据实引用并标注来源，不代填结果。

---

## §2 三项记录

### 2.1 W17 证据（board 任务第 1 项）

**A0/board 口径（L1205）**：定向 W17 检查 PASS · `npm run build` PASS · `bash scripts/pre-merge.sh` = `ALL_PASS`。

**A1 实测补充**：`npm run build` PASS（3.51s）· `git diff --check` CLEAN · home 策略 ACTIVE=3 hold 且 pending 全闭环 · home store 逻辑 105/0。

**各方 lane 结论汇总**：

| Lane | W17 结论 | 关键证据 | 来源 |
|---|---|---|---|
| A2 | 启动路径（`run-gui.sh` + `scripts/check-dev-startup.sh` + `scripts/dev-server.sh`） | 安全面 PASS（无 `sudo`/`chmod 777`/`curl\|bash`/`eval`/无差别 `pkill`）；A4 判 `HOME_STARTUP_SAFE` 零命中 | `A2-M5-W17-native-client-launch-20260907-2135.md` · `A4-...-1130.md` §2.3 |
| A3 | home store/UI 契约 + `HOME_NO_SECRET_PERSIST` 修复 | `check-home-store-logic.mjs` 105/0 | `A3-M5-W17-home-store-20260907-1653.md` |
| A4 | 隐私静态守门 | 新建 `check-home-client-policy.py`：3 ACTIVE + 3 PENDING；`--self-test` PASS（1 好样本零违规 + 6 坏样本全检） | `A4-M5-W17-home-client-privacy-review-20260908-1130.md` |
| A6 | 导航/HOLD 复核 | `check-client-navigation-logic.mjs` **60/60 PASS**；`check-ui-a11y-logic.mjs` PASS；`check-session-logic.mjs` ALL_PASS | `Lane-A6-M5-W17-closeout-20260907-2135.md` |
| A7 | 外壳 fallback（本波 Desktop Completeness 为 START CODE） | `npm run build` 207 modules exit 0；`total_bytes_pct` delta = 0.0%；`cargo_warnings` delta = 0；主区无空白死区（6 异步面板 loading/error 兜底 + 未知视图 `v-else` 兜底） | `A7-M5-W17-checkpoint-20260907-1654.md` |
| A8 | 手动 QA | **PASS_WITH_DEBT**；`check-graph-ui-logic.mjs` 113/0；`grep 'invoke('` W17 scope **0 命中**（无新增 bridge/ACL/网络特权） | `A8-M5-W17-20260907-1700.md` |
| A10 | 安全复审 | **PROVISIONAL_PASS**；tauri::command 基线 **137**（W17 须保持不变）；ACL 末条仍为 `list_artifact_images`；定义 GATE-A~M 终审闸 | `A10-M5-W17-desktop-client-security-review-20260908-1200.md` |
| A11 | 独立验证矩阵 | 6 策略 `--self-test` 全 PASS（core ACTIVE=7 / MCP ACTIVE=13 PENDING=0 / agent-memory 5 / agent-skill 3 / graph 8 / plugin ALL_PASS）；**构建指标 25.55% 超限 +0.35pp（未抬上限）**；`pre-merge` FAIL（3 条，修复前快照） | `A11-M5-W17-verification-20260907-1652.md` |

**运行时权限面（W17 硬边界，board L1170）**：全部 lane 均未新增 Tauri 命令（`137` 不变）、bridge capability、ACL 条目、文件系统权限、网络特权、新依赖；无命令执行 / plugin 调用 / 动态加载 / 远程下载·监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP live runtime / graph 写·导出 / 后台 worker。

### 2.2 唯一真实 home 存储债 `HOME_NO_SECRET_PERSIST`（board 任务第 2 项）—— **已闭环，需更正派发表述**

**债的来历**：`useHomeStore.ts` 把首页快捷方式 / 最近项的 `target`（对 app 类型条目而言即 **app execution 命令体**）整体写入浏览器 `localStorage`，构成敏感命令体持久化面。A4 静态夹具 `check-home-client-policy.py` 将其登记为 PENDING 码位（检出位置初为 `:71`，随 A3 编辑漂移到 `:72`）。

**修复方式（A3）**：`toPersisted()` 过滤 + `isStorageSafe` 过滤 + 只落库非敏感主页元数据；`useHomeStore.ts:109` 注释明示 *"只落库非敏感主页元数据，app 命令体**不写**浏览器存储"*，最近项侧同样 `filter(isStorageSafe)`（*最近访问同理：app 条目的 target 也是命令体，不还原、不落库*）。

**A1 实测（本次，决定性证据）**：

```
python3 scripts/check-home-client-policy.py --expect-pending
→ HOME_CLIENT_POLICY_PENDING_RESULT=NONE（3 个 pending 码位均未检出）
python3 scripts/check-home-client-policy.py
→ home client policy: all invariants hold（ACTIVE=3）
node scripts/check-home-store-logic.mjs
→ 通过 105，失败 0
```

**时间线冲突与更正（A1 必须记账）**：

| 时点 | 记录 | `HOME_NO_SECRET_PERSIST` 状态 |
|---|---|---|
| board L1205（A0 派发） | *"app execution command bodies are still stored in browser storage"* | 表述为**未修** |
| A5 closeout `20260907-2133` | `--expect-pending` 命中 1 项 `useHomeStore.ts:72` | 实测**仍存在**（修复前快照） |
| **A1 本次（2026-09-08 12:30）** | `--expect-pending` = **NONE** | **已闭环** |

→ **结论**：该债在 A5 closeout 之后由 A3 完成闭环；board L1205 与 A5 closeout 的表述均为**修复前快照**，A0 集成时应以**当前实测 NONE** 为准。A1 不修改 board，仅在本文档与 M5-13/14 更正记录。

**残留观察（非 W17 新增，交 A0 决策）**：A6 §4-O1 指出活动条仍持久化最近目录路径（本地路径持久化，非 W17 新增、不在 `HOME_NO_SECRET_PERSIST` 码位范围内）。

### 2.3 原生客户端手动验收边界（board 任务第 3 项）

**边界事实（A8，诚实记录）**：

- **GUI / 视觉桌面验收在 headless 环境 BLOCKED**：无 `DISPLAY`/`WAYLAND`、无预编译二进制 → 视觉布局、窄窗视觉回流、焦点顺序、错误呈现**无法在本机自动验收**。
- 须由 **A0 / 人工在 A2 启动助手落地后执行**原生客户端实跑。
- **未伪造任何截图**（board L1216：*"Screenshots may be attached only if genuinely observed"*）。
- A8 已验证的非视觉部分：`HomeLaunchers.vue` 17 个主工作区入口复用 `layout.openModule`（无第二路径）；`useLayoutStore.ts` 窄窗三档裁剪（`navDensityForWidth` / `navTopViewsForWidth` / `setWindowWidth`）；`ActivityBar.vue` 宽度上报 + 键盘漫游（←/→/Home/End）+ Esc 焦点归还 + 全 aria；`MainArea.vue` 未知视图兜底。
- A2 closeout 已给出**用户如何拉起原生客户端**的精确路径（`A2-M5-W17-native-client-launch-20260907-2135.md`），供人工验收执行。

**体积门禁（与手动验收并列的开放项）**：

| 口径 | 值 | 判定 |
|---|---|---|
| A11 矩阵（修复前） | `total_bytes_pct=25.55` > 25.2%，超限 **+0.35pp**；`cargo_warnings` delta=0 | ❌ 超限，**未抬上限**（符合 AC-5） |
| A6 独占增量 | +3,666B（vs `4f0e8ab` +0.598pp → 25.742%） | 单 lane 口径 |
| **A1 本次实测（脏树全量）** | `total_bytes_pct=29.77`，`exceeds_growth_limit=true`，`cargo_warnings=0` | ⚠️ **多 lane 脏树叠加**采集，非单 lane 增量 |
| A7 自测 | delta = 0.0%（未触及上限） | 单 lane 口径 |

→ **A1 立场**：当前 29.77% 为**所有 lane 未集成改动叠加**的共享树口径，**不能**直接判为 W17 超限；A6 §4-F1 亦指出"体积门禁基线被脏树采集污染"。**最终体积判定必须由 A11 在 A0 集成后以干净树复测**。W17 的 AC-5 要求不变：超预算即停并报精确 delta，**禁止抬 25.2% 上限**。

---

## §3 A5 / A6 / A7 HOLD 复核（用户指令 + board L1213-1215）

**HOLD 规则**：只检查是否存在**具体阻塞**；无阻塞则**不重复开发**、不 reopen。

| Lane | board 状态 | 复核结论 | 具体阻塞 | 处置 |
|---|---|---|---|---|
| **A5** | HOLD（`src/components/home/`）· *"No new work unless A3 changes the state contract and a focused UI adjustment is proven necessary"* | **PASS（HOLD，无新工作）** | **无** | A3 修复为 store 层（不改变对外 state 契约字段形状，仅收紧持久化内容）→ 不构成"A3 改变 state 契约"的 reopen 条件；A5 不重复开发 |
| **A6** | HOLD（`ActivityBar.vue` / `useLayoutStore.ts` / `check-client-navigation-logic.mjs`）· *"No new work; existing navigation checks are green"* | **PASS_WITH_DEBT（HOLD，无 reopen 项）** | **无**（门禁 60/60 复绿） | 4 条**跨 lane 发现**交 A0/A11/A1 处置，均非 A6 自身阻塞：① F1 体积门禁基线被脏树采集污染；② F2 `pre-merge ALL_PASS` 未覆盖 W17 三个新门禁（`check-home-client-policy.py` / `check-home-store-logic.mjs` / `check-home-ui-logic.mjs`）；③ F3 A11 矩阵第 27/130 行对 A6 已过期；④ O1 活动条仍持久化最近目录路径（非新增） |
| **A7** | HOLD（`MainArea.vue` / `StatusBar.vue` / `App.vue`）· *"No new work; existing shell fallback is covered"* | **PASS（HOLD，无新工作）** | **无** | 本波（Desktop Client Completeness）A7 为 START CODE 且已交付 PASS（未知视图 `v-else` 兜底 + 6 异步面板 loading/error 兜底 → 主区无空白死区）；closeout 波无具体验收发现指向 shell → 不 reopen |

**A1 处置**：三条 HOLD 结论已记入本文档，并同步至 M5-13（验证矩阵，标注 HOLD lane 不产生新验证项）与 M5-14（债务账，登记 A6 四条跨 lane 发现为**挂账项**而非阻塞项）。

---

## §4 A1 立场与不越界

- **DOCS ONLY**：零产品代码；未触 `src/`、`src-tauri/`、`scripts/`、`package.json`、ACL/Capability。
- **只读实测**：§1 全部命令为只读门禁/构建/纯逻辑测试，未产生产品代码改动。
- **不代跑验证职责**：`pre-merge.sh` 与 full Rust 测试归 A11/A0；A1 引用并标注来源。
- **不伪造证据**：手动验收 BLOCKED 即记 BLOCKED；未观测的截图一律不附。
- **不越界解冲突**：允许文件若被他 lane 改动则出 binary patch（本次纯文档，未出现冲突）。
- **不 commit、不 push**；整包留工作树交 A0 集成（顺序 `... -> A1 -> A11 -> A0`）。

---

## §5 整包交付

- ✅ `logs/checkpoints/A1-M5-W17-acceptance-closeout-20260908-1230.md`（本 checkpoint）
- ✅ M5-0 `[W17 acceptance closeout]` 段 / M5-13 `[W17 acceptance closeout]` 段 / M5-14 `[W17 acceptance closeout]` 段
- ✅ 3 主文档 A1 W17 closeout update 行
- ✅ `logs/checkpoints/Lane-A1-M5-W17-acceptance-closeout-20260908-1230.patch`（路径限定 A1 范围）
- ✅ 自检：`git diff --check` 干净；`git apply --check --reverse` 通过

**patch 生成（路径限定）**：

```bash
git add -N logs/checkpoints/A1-M5-W17-acceptance-closeout-20260908-1230.md
git diff --binary -- 详细设计与实施计划.md AI-模型切换与接手清单.md 后续需求TODO.md \
  logs/checkpoints/M5-20260906 \
  logs/checkpoints/A1-M5-W17-acceptance-closeout-20260908-1230.md \
  > logs/checkpoints/Lane-A1-M5-W17-acceptance-closeout-20260908-1230.patch
```

---

## §5.1 波次流转记录（2026-09-08 13:00 CST · 补记）

**board L8 已更新**（A0 口径）：*`Current NEXT: M5-BUG-HUNT confirmed high-risk bug closure and evidence refresh. W17 desktop-client closeout is locally green; native visual acceptance remains user-side evidence. Only A0 pushes.`*

→ W17 收口 **locally green**（A0 认可）；原生视觉验收仍为用户侧证据（与本卡 §2.3 一致）。

**A1 在 M5-BUG-HUNT 波的指派状态**：BUG-HUNT Follow-up Dispatch（board L1235-1246）lane 表**仅含 A2/A3/A4/A5/A6/A7/A8/A9/A10/A11**，**无 A1 行**；BUG-HUNT 顺序 `A2/A3/A5/A6/A7/A8/A10/A9 -> A4 -> A11 -> A0` 亦不含 A1。→ **A1 无指派任务，不扩大范围、不代做他 lane 工作**。

**跨波次提醒（已同步 M5-0/13/14）**：BUG-HUNT 的 **B5-1**（persistence-before-spawn 事务边界，非常规补丁，禁 opportunistic 改写）与 W17 的 `HOME_NO_SECRET_PERSIST`（前端 localStorage 不落敏感命令体，**已闭环**）**主题相邻但不同层**（Rust spawn/持久化 vs 前端存储），A0 集成时勿混淆两者闭环状态。B9-1 / B3-1 已由 A0 直接修复。

**W17 挂账 W17-D1~D5 跨波次继续挂账**（不因 `NEXT` 变更自动消解）：干净树体积复测（A11）· `pre-merge` 覆盖 W17 三个新门禁（A11）· A11 矩阵过期行 · 活动条目录路径持久化（A0 决策）· 原生视觉验收（A0/人工实跑，未伪造截图）。

---

## §6 关联指针

- W17 Acceptance Closeout 派发：`PARALLEL_COMMAND_BOARD.md` L1203-1229
- A1 W17 用户可见验收清单：`logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`
- A1 W17 reconciliation：`logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md`
- A4 隐私审查（含 `HOME_NO_SECRET_PERSIST` 码位定义）：`logs/assist/A4-M5-W17-home-client-privacy-review-20260908-1130.md`
- A5 / A6 HOLD closeout：`logs/checkpoints/Lane-A5-M5-W17-closeout-20260907-2133.md` · `logs/checkpoints/Lane-A6-M5-W17-closeout-20260907-2135.md`
- A8 手动验收：`logs/checkpoints/A8-M5-W17-20260907-1700.md` · `logs/assist/A8-M5-W17-desktop-acceptance-20260907-1700.md`
- A2 原生客户端启动路径：`logs/assist/A2-M5-W17-native-client-launch-20260907-2135.md`
- A10 安全复审（GATE-A~M）：`logs/assist/A10-M5-W17-desktop-client-security-review-20260908-1200.md`
- A11 验证矩阵：`logs/checkpoints/A11-M5-W17-verification-20260907-1652.md`
