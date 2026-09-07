# A1 · M5-W17 验收结论 + BUG-HUNT 状态与剩余风险（DOCS ONLY · 2026-09-08 13:30 CST）

> Lane A1（docs-only reconciliation）
> 依据：`PARALLEL_COMMAND_BOARD.md` L8（`NEXT=M5-BUG-HUNT`）+ L1231-1252（BUG-HUNT Follow-up Dispatch）+ `logs/bug-hunt/BUG-HUNT-SUMMARY.md` + `logs/bug-hunt/A11-B8-B10-B12-recheck-20260907-2150.md`
> 配套：A1 W17 验收收口 checkpoint `logs/checkpoints/A1-M5-W17-acceptance-closeout-20260908-1230.md`
> 立场：**DOCS ONLY · 零产品代码 · 不 commit · 不 push**
> 说明：BUG-HUNT Follow-up Dispatch（board L1235-1246）lane 表**仅含 A2-A11，无 A1 指派**；本文档为 A1 依用户指令所做的**只读整理与对账**，不执行任何 bug 修复、不改动任何产品文件。

---

## §1 一句话结论

**M5-W17 收口 = locally green（A0 口径），但存在 1 项由 W17 自身引入、已被 A11 结构性确认的 UI 回归（B10-b）** —— `MainArea.vue` 的 W17(A7) 兜底块在 `mainView !== 'editor'` 时**恒渲染**"当前视图不可用"告警。A1 此前记录的"A7 外壳兜底 PASS（主区无空白死区）"须**更正为 PASS_WITH_REGRESSION**。BUG-HUNT 侧：P0 中 **B9-1 / B3-1 已由 A0 修复并经 A1 diff 核实**；**B4-1 / B5-1 / B11-1 三项 P0 未闭环**，其中 **B5-1 被 board 明令"禁止一般 lane opportunistic 改写"**。

---

## §2 M5-W17 验收结论（A1 汇总，含本轮更正）

### 2.1 A0 口径与 A1 实测

| 维度 | 结论 | 来源 |
|---|---|---|
| 波次状态 | **locally green**（A0 认可）；`NEXT` 已转 `M5-BUG-HUNT` | board L8 |
| 定向 W17 检查 / `npm run build` / `pre-merge.sh` | PASS / PASS / **ALL_PASS** | board L1205（A0/A11 口径，A1 未复跑 full Rust，属 A11 职责） |
| A1 只读实测 | `npm run build` ✓ 3.51s · `git diff --check` CLEAN · `check-home-client-policy.py` 默认 ACTIVE=3 hold 且 `--expect-pending`=**NONE** · `check-home-store-logic.mjs` **105/0** | A1 实测 |
| 唯一真实产品债 `HOME_NO_SECRET_PERSIST` | **已闭环**（A3 以 `toPersisted()`/`isStorageSafe` 过滤；board L1205 与 A5 closeout 21:33 命中 `:72` 均为**修复前快照**） | A1 实测 + 代码核对 |
| 运行时权限面 | **零扩张**：tauri::command **137** 不变、ACL 末条仍 `list_artifact_images`、W17 scope 内 `grep 'invoke('` **0 命中**、无新依赖 | A8/A10 + A1 对账 |
| A5 / A6 / A7 HOLD 复核 | A5 PASS · A7 PASS · A6 PASS_WITH_DEBT（导航 60/60）→ **均无具体阻塞，不 reopen** | board L1213-1215 |

### 2.2 本轮更正（重要）

**A1 在 W17 收口 checkpoint 中记录的"A7 外壳兜底 PASS（主区无空白死区）"须更正为 `PASS_WITH_REGRESSION`。**

依据 A11 复跑（`A11-B8-B10-B12-recheck-20260907-2150.md` §3）与 A1 对当前代码核对（`src/components/layout/MainArea.vue`）：

```vue
<!-- :251 -->
<FileEditor v-if="layout.mainView === 'editor'" />
<!-- :253 —— W17(A7) 兜底：未知/空视图时主区不得空白或死区 -->
<div v-else class="modview panel-state" role="alert" aria-live="polite">
  <div class="pf-title">当前视图不可用</div>
```

- **结构性问题**：`:253` 的 `v-else` 与 `:251` 的 `v-if` 配对，而**不与主链（`:128`–`:241` 的 `v-if`/`v-else-if` 序列）相连** → 当 `mainView !== 'editor'` 时兜底块**恒渲染**。
- **用户可见后果**：正常视图下常驻"当前视图不可用"告警（模板结构推断，**A11 与 A1 均未真机验证**，需 A8/A0 实跑确认）。
- **性质**：A11 判 **✅ CONFIRMED（结构性）**，且应验 SUMMARY 中"疑似 A7 W17 引入"的怀疑 → **W17 自身引入的回归，非历史债**。
- **A1 处置**：更正 W17 验收结论（本 §2.2）→ 同步 M5-13/14 + 3 主文档；**A1 不修产品代码**（MainArea 属 A7 允许范围，应由 A7 或 A0 指派修复）。

### 2.3 W17 跨 lane 挂账（延续，未消解）

| # | 挂账 | 归属 |
|---|---|---|
| W17-D1 | 体积门禁须 **A0 集成后干净树**复测（A11 修复前 25.55%；A1 脏树全量 29.77% 为多 lane 叠加，不可作判定值） | A11 |
| W17-D2 | `pre-merge ALL_PASS` **未覆盖** W17 三个新门禁（`check-home-client-policy.py` / `check-home-store-logic.mjs` / `check-home-ui-logic.mjs`） | A11 |
| W17-D3 | A11 矩阵对 A6 的过期行 | A11 |
| W17-D4 | 活动条仍持久化最近目录路径（**非新增**） | A0 决策 |
| W17-D5 | 原生客户端**视觉**验收 headless BLOCKED（无 DISPLAY/WAYLAND、无预编译二进制）→ 须 A0/人工实跑；**未伪造截图** | A0/人工 |
| **W17-D6（本轮新增）** | **B10-b MainArea 兜底 `v-else` 配对错误**（W17/A7 引入，CONFIRMED）→ 正常视图常驻假告警 | A7 / A0 指派 |

---

## §3 B9-1 / B3-1 修复记录（A0 直接修复 · A1 diff 核实）

### B9-1 · 子 webview 死锁根因复燃（P0 卡死）

- **问题**：`tauri-browser-tabs/.../platform/linux.rs:83-84` 离屏分支调用 `gtk_webview.hide()`，违反 `PROJECT-RULES.md` 规则 3.5（对渲染中 WebKitGTK 子 webview 调 hide = 历史死锁根因）；切页签/布局守护 400ms 重放可触发主线程卡死。
- **修复**（A0，A1 核实 `git diff`：**-12 / +3**）：删除 `gtk_webview.hide()`，保留 `set_child_visible(false)`；注释改为 *"被移到屏幕外的 webview 只退出 GtkFixed 子布局；对正在渲染的 WebKitGTK 子控件禁止调用 hide()，否则会复燃历史主线程死锁"*；位置/尺寸/重绘仍由 `gtk_fixed_move` + 原尺寸 `size_allocate` + `queue_draw` 三步固定。
- **状态**：✅ 已在工作树落地（文件 `M`）。

### B3-1 · 脚本正常退出后 supervisor 永久挂起 + 并发槽泄漏（P0 挂起）

- **问题**：`src-tauri/src/script_runner.rs:657-667` 正常退出分支只 `join_output_readers` 不杀进程组；脚本内 `&` 后台化子进程继承 stdout fd → reader 永不 EOF → `finish` 永不执行 → 该 run 永远 `Running`；连续 8 次后 `MAX_CONCURRENT_RUNS` 触顶，脚本功能实质失效。
- **修复**（A0，A1 核实 `git diff`：**+4**）：在正常退出分支 `join_output_readers` **之前**插入 `terminate_group(pgid, HARD_GRACE_SECS)`，注释 *"A background child can inherit the output pipe after the leader exits. Close the process group before joining readers so the supervisor cannot remain Running forever waiting for EOF from an orphaned descendant."*
- **状态**：✅ 已在工作树落地（文件 `M`）。
- **关联**：同源的 **B3-2**（孤儿进程，条目已终态被 `kill_all_running` 跳过 → 后台子进程连应用退出都不回收）**未单独派发**，部分被 B3-1 修复覆盖，残留面待 A0 确认。

---

## §4 当前 BUG-HUNT 任务状态（A1 只读对账，2026-09-08 13:30）

### 4.1 已交付（有产物）

| Lane | 任务 | 产物 | 状态 |
|---|---|---|---|
| **A0** | B9-1 / B3-1 直接修复 | 工作树 `linux.rs` / `script_runner.rs` 改动 | ✅ 已落地（§3） |
| **A10** | B11-3 markdown 链接 XSS | `A10-M5-BUG-HUNT-markdown-xss-20260908-1305.md` + `scripts/check-markdown-xss-logic.mjs`（新）+ `src/utils/markdown.ts` 改 | ✅ **PASS**（`escapeAttr` 转义引号 + 外部链接 `rel="noopener noreferrer"`；headless 16/16 PASS；`npm run build` ✓ 3.26s） |
| **A11** | B8/B10/B12 截断三路复跑 | `logs/bug-hunt/A11-B8-B10-B12-recheck-20260907-2150.md` + patch | ✅ **PASS_WITH_DEBT**（3 CONFIRMED / 2 降级重分类 / 4 UNVERIFIED 待专项复跑） |

### 4.2 未见产物（文件状态以工作树为准，谨慎表述）

| Lane | 派发任务 | 目标文件当前状态 | A1 观察 |
|---|---|---|---|
| A2 | `check_launch_target` 加固（env/interpreter/符号链接绕过）+ 回归测试 | `security_policy.rs` 已改 | 未见 A2 checkpoint/patch 产物 |
| A3 | workspace 破坏性写改原子写 + 损坏文件备份/错误处理 | `workspace.rs` 已改 | 未见 A3 产物；**B4-1 属 P0 数据丢失** |
| A4 | B5-1/B5-2 持久化顺序与共享锁边界**设计与测试**（未落地前不实现） | — | 未见产物；board 明令 B5-1 **禁 opportunistic 改写** |
| A5 | `tools.rs` 用户 HTML 有界读（防 OOM） | `tools.rs` **未动** | 未见产物 |
| A6 | 剪贴板历史默认不落盘（B11-1，P0 明文泄露） | `useSystemStore.ts` / `ClipboardPanel.vue` **未动** | 未见产物；**P0 未闭环** |
| A7 | DbValue/SkillDef/AgentDef 序列化 casing 漂移对齐（B8-1/B8-2） | `domain.rs` / `types.ts` 已改 | 未见 A7 产物（改动可能属他波或在制） |
| A8 | `closeGridAll` 修复（B9-4 浏览器区空白） | `useBrowserStore.ts` **未动** | 未见产物 |
| A9 | `open_tool` / collect.js 命令 ACL 与源一致性 + 最小静态门禁（B12-01/02） | `commands.rs` / `default-commands.toml` / `collect.js` **未动** | 未见产物；**B12-01 open_tool 缺 ACL = CONFIRMED** |

### 4.3 A11 复跑证据矩阵要点（决定"哪些算 bug"）

| 编号 | 原主张 | A11 判定 |
|---|---|---|
| B8-1 | `DbValue` TS PascalCase vs Rust snake_case → DB 面板渲染错误 | ✅ **CONFIRMED**（契约层确认；渲染后果未真机验证） |
| B8-2 | `SkillDef`/`AgentDef` 逐字段漂移 | ⚠ **PARTIALLY CONFIRMED**（仅 camelCase 声明子集漂移，非"逐字段"） |
| B10-a | `stats.main.rss_mb` 未守卫 → 白屏 | ❌ **UNSUPPORTED（降级 P3）**：`bridge.rs:4311` 的 `main` 为**非 Option**，常规路径不会缺失 |
| **B10-b** | MainArea 兜底 `v-else` 配对错误（疑似 A7 W17 引入） | ✅ **CONFIRMED（结构性）** → 本轮新增 W17-D6 |
| B10-c | `JSON.parse` 未守卫 | ❌ 抽查点 UNSUPPORTED + 其余 **UNVERIFIED** |
| B12-01 | `open_tool` 缺 ACL | ✅ **CONFIRMED**（与截断表 A−B 吻合） |
| B12-02 | 三源无自动化一致门禁 | ✅ **CONFIRMED（仍缺）** |
| B12-03 | collect.js 三命令未授权 | 🔄 **STALE / 重分类**：ACL 移除系**有意设计**（`remote-collect.toml` 注释 + 意图令牌补偿控制），原"未授权=bug"不成立；残留为"意图令牌通路可用性"，需真机/A9 复核 |
| C−A | bridge.ts skill/agent 11 条契约占位 | ✅ CONFIRMED 但**属有意设计**（运行时权限 LOCKED 下的预期态） |

---

## §5 剩余风险（A1 只读评级，不修不改）

### 5.1 P0 未闭环（最高优先）

| 编号 | 风险 | 状态 | 备注 |
|---|---|---|---|
| **B4-1/B4-2** | workspace 系非原子写 + `unwrap_or_default()` 静默清空 → **数据无痕蒸发**（audit 热路径最致命） | ❌ 未闭环 | A3 派发中，`workspace.rs` 已改但无产物；**崩溃中损坏 → 下次保存把空列表覆盖写回** |
| **B5-1** | 调度崩溃窗口 → 任务**重复执行**（`last_fired_at` 内存写晚于 spawn） | ❌ 未闭环（**禁 opportunistic 改写**） | board L1233 明令：需"先落盘判重再 spawn"的**测试化事务边界** + 崩溃窗口测试，由 A4 先出设计与测试 |
| **B11-1** | 系统剪贴板历史**明文落 localStorage** + 明文渲染（最多 50 条，跨重启留存） | ❌ 未闭环 | A6 派发中，目标文件**未动**；复制的密码/token 可被读文件获取 |
| **W17-D6** | MainArea 兜底 `v-else` 恒渲染（W17/A7 引入） | ❌ 未闭环 | 用户可见常驻假告警；A7/A0 指派 |

### 5.2 P1 高价值（未闭环，择要）

- **B2-1**：`check_launch_target` 仅 6 个 shell basename 黑名单 + 10 个元字符 → `/usr/bin/env bash -c`、符号链接、`python3 -c` 均可**绕过命令执行门禁**（A2 派发中）。
- **B2-2**：`check_html` 生产**零接入**（仅单测调用）→ 设计的 HTML 净化闸门从未生效。
- **B12-01**：`open_tool` 注册 + 前端调用但 **ACL 无条目** → 静默 not allowed（工具箱打开失效）。
- **B6-1**：`read_user_tool` 用 `read_to_string` 整文件读入用户可控 HTML，**无大小上限** → 单文件 OOM（A5 派发中，文件未动）。
- **B9-4**：`closeGridAll` 后活动页签坐标未复位 → 浏览器区空白（A8 派发中，文件未动）。
- **B8-1**：`DbValue` casing 漂移 → DB 面板单元格渲染错误（A7 派发中）。
- **B11-2**：10+ 处 `e?.message ?? e` / `String(e)` 未接 `redactSecrets` → 前端错误原文回显（后端已脱敏但前端未接力）。
- **B3-2**：孤儿进程（部分被 B3-1 覆盖，残留待确认）；**D25**：`on_channel_dead` 只 kill 不 wait → 僵尸 shell + 表项残留（历史挂账，仍未修）。
- **B5-2**：`tasks.json` 无共享锁 → 用户编辑可静默回滚。

### 5.3 已闭环 / 降级（本轮确认）

- ✅ **B9-1**（A0 修，已核实）· ✅ **B3-1**（A0 修，已核实）· ✅ **B11-3** markdown XSS（A10，16/16 PASS）
- 🔄 **B12-03** 重分类为**有意设计**（非 bug）· ❌ **B10-a** 降级 P3 防御建议 · ✅ **C−A** 占位属锁定权限下的预期态
- ✅ **F-2 / F-3**（home 隐私）已修且 W17 收口实测 `--expect-pending`=NONE

### 5.4 A1 视角的横向风险（跨波次）

1. **"locally green"不等于"无回归"**：W17 被 A0 判 locally green，但 B10-b 证明 W17 自身引入了结构性 UI 回归 → **A0 集成前须由 A8 真机确认**该告警是否实际可见。
2. **体积门禁仍是悬顶风险**：25.2% 一次性上限下 W17 已耗至 25.55%（A11），BUG-HUNT 各 lane 若再增 chunk 将直接触顶；AC-5 要求**报 delta 不抬上限**。
3. **BUG-HUNT 与 W17 文件重叠**：`MainArea.vue` / `ActivityBar.vue` / `StatusBar.vue` 同时是 W17（A6/A7）与 B10 复跑的对象 → A11 已声明"B10 结论基于在制代码、随时点漂移"；**A0 集成须按 board L1225 用 binary patch 保护，禁止 reset/clean**。
4. **B5-1 与 W17 的 `HOME_NO_SECRET_PERSIST` 易混淆**：前者 Rust spawn/持久化顺序（**未开工**），后者前端 localStorage 敏感命令体（**已闭环**），不同层，勿并案。

---

## §6 A1 立场与交付

- **DOCS ONLY**：零产品代码；未触任何 `src/`、`src-tauri/`、`scripts/`、ACL/Capability。本轮全部为只读（`git diff` / `grep` / `sed` 查看 + 既有文档读取）。
- **不越界**：BUG-HUNT 派发无 A1 指派 → A1 **不修 bug、不代做他 lane 任务**；仅做状态对账与风险记账。
- **不伪造证据**：凡"未真机验证"的结论（B10-b 用户可见后果、B8-1 渲染后果）均标注推断来源。
- **不 commit、不 push**：整包留工作树交 A0。
- **同步范围**：本 checkpoint + M5-0/13/14 相关段 + 3 主文档 A1 update 行 + 路径限定 patch。

---

## §7 关联指针

- BUG-HUNT 派发：`PARALLEL_COMMAND_BOARD.md` L1231-1252；`NEXT` 见 L8
- BUG-HUNT 汇总：`logs/bug-hunt/BUG-HUNT-SUMMARY.md`（P0×5 / P1×14 / P2-P3 十余项）
- A11 复跑矩阵：`logs/bug-hunt/A11-B8-B10-B12-recheck-20260907-2150.md`
- A10 markdown XSS：`logs/checkpoints/A10-M5-BUG-HUNT-markdown-xss-20260908-1305.md`
- A1 W17 验收收口：`logs/checkpoints/A1-M5-W17-acceptance-closeout-20260908-1230.md`
- A1 W17 用户可见验收清单：`logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`
