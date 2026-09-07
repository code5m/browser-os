# A10 · M5-W17 Acceptance Closeout 安全复审（SECURITY REVIEW）

> Lane: A10（M5 独立安全复审） · Wave: **M5-W17 Acceptance Closeout Dispatch**（board §M5-W17 Closeout L1203-1229；base `052b18a`）
> A10 Closeout 任务（board L1218）：**Review the final W17 diff for raw sensitive persistence, shell/privilege expansion, and startup ownership. Findings first; no product-code edits.**
> 复审时间：2026-09-08 ~12:30 CST · BASE=`052b18a`（`feat(M5): close W15 release readiness`，已 push）
> 集成顺序（L1223）：`A3 -> A4/A9 -> A2/A8/A10 -> A1 -> A11 -> A0`。A5/A6/A7 = **HOLD**（L1213-1215：无新工作，仅对具体验收发现重开）。
> 交付物：本复审 + checkpoint（仅文档，**零产品代码改动、零补丁、未 push**）。Lane 默认不编辑产品码；当前无具体违规，故不加 policy 夹具。

## 0. Lane Output Template（机器可读结论）

```text
LANE: A10
STATUS: PASS
SCOPE: logs/assist/A10-M5-W17-closeout-security-review-20260908-1230.md ; logs/checkpoints/A10-M5-W17-closeout-20260908-1230.md（仅复审文档 + checkpoint；零产品代码 / 零门禁脚本改动）
DELIVERED: 对 W17 最终 diff 的三轴安全复审——① raw sensitive persistence：HOME_NO_SECRET_PERSIST(app 命令体落浏览器存储) 已被 A3 closeout 修复闭环；② shell/privilege expansion：W17 范围内零扩张，startup 助手 ownership-safe；③ startup ownership：run-gui.sh/dev-server.sh/check-dev-startup.sh 仅回收自有 dev server、无孤儿。另标记两处越界 Rust 文件交 A0 确认（非阻塞）
VERIFY: python3 scripts/check-home-client-policy.py: home client policy all invariants hold(ACTIVE=3) PASS; python3 scripts/check-home-client-policy.py --expect-pending: HOME_CLIENT_POLICY_PENDING_RESULT=NONE(3 项 PENDING 全闭环: useHomeStore.ts:71/180 + HomePanel.vue:48); node scripts/check-home-store-logic.mjs: 通过 105 失败 0; node scripts/check-home-ui-logic.mjs: 通过 32 失败 0(HOME_UI_RESULT=PASS); node scripts/check-client-navigation-logic.mjs: CLIENT_NAV_RESULT=PASS(60/60); grep -c 'tauri::command|#[command]'=137(无新增); git diff --quiet ACL/bridge/main.rs=UNCHANGED; bash scripts/pre-merge.sh: PRE_MERGE_RESULT=ALL_PASS; 6 策略门全绿(mcp current-gaps PASS / core ACTIVE=7 / plugin ACTIVE=7 PENDING=5 / agent-skill ACTIVE=3 PENDING=10 / graph ACTIVE=8 / agent-memory ACTIVE=5); git diff --check: 干净; bash scripts/check-dev-startup.sh(A2): PASS(所有权语义 5 组全绿)
METRICS: N/A（A10 零前端/Rust 改动，不触碰 build metrics；指标超限红灯属 A11/A0 域，closeout 后 pre-merge=ALL_PASS 已确认解决）
PATCH: N/A（复审仅文档，无源文件 diff；不生成 binary patch）
RISKS: (1) script_runner.rs(加 terminate_group 孤儿进程组回收) 与 tauri-browser-tabs/linux.rs(去 gtk_webview.hide() 修主线程死锁) 均越出 W17 任何 lane scope(A2 仅 run-gui.sh，未改 main.rs)，属 bug 修复、非特权扩张；建议 A0 集成时确认是否故意纳入（非阻塞）；(2) board L1205 所述 HOME_NO_SECRET_PERSIST「残留债」在当前树已由 A3 toPersisted+isStorageSafe+迁移抹除闭环（--expect-pending=NONE），url/dir 绝对路径/URL 仍持久化为非敏感主页元数据(快捷方式需跨重启存活)，渲染面经 homeDisplayTarget 脱敏无披露——若 A0 后续要求连 dir 绝对路径也不落库，属增强而非 W17 阻断；(3) 构建指标 25.55%>25.2% 与 SCHEDUI_PANEL_LAZY 两红灯为完整性波次(A11 16:52 BLOCKED)，closeout 后 pre-merge=ALL_PASS 已解决，非安全轴；(4) A5/A6/A7 在 closeout 为 HOLD，A5 实测零改动、A6/A7 完整性波次改动均不新增命令/bridge/ACL/FS/网络特权——无具体安全阻塞
NO_PUSH: confirmed
```

## 1. 复审范围与方法（三轴）

A10 Closeout 三轴（board L1218）：**raw sensitive persistence / shell/privilege expansion / startup ownership**。方法（实证，findings-first，不编辑产品码）：
- 对当前工作树（HEAD=`052b18a` + W17 在制）重跑 home 策略门 + 6 既有策略门 + `pre-merge.sh`，确认无回归。
- 逐文件读 `useHomeStore.ts` / `homeUi.ts` / `run-gui.sh` / `dev-server.sh` / `check-dev-startup.sh` 验证三轴。
- 交叉 A4（home 隐私静态审查）、A9（home UI 逻辑）、A2（启动助手冒烟）、A11（验证矩阵）、A5/A6/A7（HOLD 报告）。
- 范围隔离：A10 不编辑产品码 / 不新建脚本（A4 拥有 `check-home-client-policy.py`、A9 拥有 `check-home-ui-logic.mjs`、A3 拥有 `check-home-store-logic.mjs`、A6 拥有 `check-client-navigation-logic.mjs`、A2 拥有 `check-dev-startup.sh`）。当前无具体违规 → 不加夹具。

## 2. 轴 1 — Raw Sensitive Persistence：PASS（债务已闭环）

**A3 closeout 修复（board L1211）已解决 board L1205 所列 `HOME_NO_SECRET_PERSIST` 残留债**：
- `src/utils/homeUi.ts`：
  - `isStorageSafe(s) => s.type !== "app"`（L234-236）：app 条目（命令体）判定为不可安全落库。
  - `toPersisted(list) => list.filter(isStorageSafe)`（L239-241）：落库前剥除 app。
  - `HOME_PERSISTED_TYPES = ["url","dir"]`（L231）：app 明确排除。
  - `HOME_APP_SESSION_ONLY_NOTICE`（L227-228）：用户明示「应用启动命令不写入浏览器存储」。
- `src/stores/useHomeStore.ts`：
  - `save()`（L107-112）：`localStorage.setItem(STORAGE_KEY, JSON.stringify(toPersisted(shortcuts)))` —— 只落非 app 元数据。
  - `load()`（L82-98）：`all.filter(isStorageSafe)`；若安全项少于原始（旧数据含 app 命令体），**随即把安全项写回存储抹除旧 app 条目**（迁移期清理）。
  - `loadRecents()`（L99-106）：`normalizeRecents(...).filter(isStorageSafe)` —— app 最近项不还原、不落库。

**实测评测**：`python3 scripts/check-home-client-policy.py --expect-pending` → **`HOME_CLIENT_POLICY_PENDING_RESULT=NONE`（3 个 pending 码位全部未检出）**。即 A4 早前（L1211 评审）记的 F-1(`useHomeStore.ts:71` app 命令体落 localStorage)/F-2(`useHomeStore.ts:180` raw error 回显)/F-3(`HomePanel.vue:48` `:title="s.target"`) **全部闭环**。A5 HOLD 报告（其复核时仍命中 :72，但本复审当前实跑已 NONE）印证 A3 修复在其后落地。

**诚实残留（非阻断）**：`url`/`dir` 的 target（完整 URL 含 query、本地绝对路径）**仍持久化**——属「非敏感主页元数据」（快捷方式须跨重启存活），且渲染面经 `homeDisplayTarget`（L299-307）只给 host+path / 路径末段 / 命令首段，绝不回显完整路径或凭据（共享验收 #4）。A4 的 `HOME_NO_SECRET_PERSIST` 描述为「含凭据/敏感 query/本地绝对路径/app 命令体」，其**实现层**仅守 app 命令体（权威夹具 `--expect-pending=NONE` 已确认）；dir 绝对路径持久化是按 wave 边界的有意设计，非「raw sensitive persistence of credentials/command bodies」。若 A0 后续要求连 dir 绝对路径也不落库，属增强（建议改 `toPersisted` 投影为脱敏摘要），不阻断 W17。

## 3. 轴 2 — Shell / Privilege Expansion：PASS（W17 范围内零扩张）

**W17 范围（A2/A3/A5/A6/A7 在制文件）**：
- `run-gui.sh` / `dev-server.sh` / `check-dev-startup.sh`：ownership-safe（详见轴 3）。无 `sudo`/`pkexec`/`chmod 777`/`curl|bash`/`eval $`、无差别 `pkill/killall node|vite|npm`（A4 `HOME_STARTUP_SAFE` 零命中）。
- `main.rs` / `default-commands.toml` / `bridge.rs`：**零改动**（`git diff --quiet` 实测 UNCHANGED；A2 经 `MVP_FORCE_DIST` 环境变量绕开 main.rs 编辑，check-dev-startup.sh §5 断言）。
- **无新 Tauri command**：`grep -c 'tauri::command|#[command]'` = **137**（与 W15 基线一致，W17 未增）。
- **无新 ACL/bridge/FS/网络特权**（A4 `HOME_NO_PRIVILEGE_EXPANSION` 零命中；check-dev-startup.sh §5 断言 ACL/bridge 未动）。
- A3/A5/A6/A7 改动均经对应策略脚本断言：无 `invoke(`/`Command::new`/`process spawn`/`TcpListener`/新依赖（check-home-client-policy / check-home-ui-logic / check-client-navigation-logic 全 PASS；6 既有策略门全绿）。

**越界 Rust 文件（非 W17 lane scope，FLAG 交 A0，非阻塞）**：
- `src-tauri/src/script_runner.rs`（supervise 末加 `terminate_group(pgid, HARD_GRACE_SECS)`）：**孤儿进程组回收**——关闭子进程组以免 supervisor 卡在 EOF 等孤儿。属既有 script_runner 监督逻辑的健壮性修复，**不新增命令执行能力、不扩张运行时权限**。
- `tauri-browser-tabs/crates/tauri-plugin-browser-tabs/src/platform/linux.rs`（屏外分支去 `gtk_webview.hide()`）：**修 GTK 主线程死锁**的纯 UI 渲染修复，无安全/特权影响。
- 两文件均**不在任何 W17 lane 的允许 scope**（A2 仅 `run-gui.sh`，未改 main.rs；无 lane 拥有 script_runner.rs / linux.rs）。既非特权扩张，也非 W17 授权改动 → A10 不 BLOCK，但标记 A0 集成时确认是否故意纳入 closeout（避免无意 drift）。

## 4. 轴 3 — Startup Ownership：PASS

`run-gui.sh` + `scripts/dev-server.sh` 实现 board 共享验收 #1（开发者启动客户端不落到 `localhost:1421` connection refused；helper 检测/启动/等待/退出仅清自有 server）：
- **私有 state dir**：`mktemp -d`，pidfile 只属本次运行（dev-server.sh L37 `DS_PIDFILE`，run-gui.sh L96）。
- **所有权判定**：`dev_server_ensure`（dev-server.sh L108-114）探测端口已服务 → 判定「他人所有」，不启动、cleanup 不杀；仅自己拉起时才写 pidfile。
- **仅回收自有**：`ds_cleanup`（L117-140）按 pidfile 的 pgid `kill -TERM "-${pgid}"` / `kill -KILL "-${pgid}"`，无 pidfile 则不动任何已有服务；幂等（`DS_CLEANED`）。
- **cleanup trap**：run-gui.sh L75-77 `trap 'cleanup' EXIT/INT/TERM`；**未用 `exec`**（L113 `"$BIN"` 而非 `exec "$BIN"`）保证客户端退出后 cleanup 运行、无孤儿。
- **无特权升**：仅 `cargo build`（缺二进制时）+ 导出 `WEBKIT_DISABLE_DMABUF_RENDERER=1`/`GDK_BACKEND=x11` 图形 workaround；无 sudo/setuid。

**实测**：`bash scripts/check-dev-startup.sh`（A2 冒烟）→ **PASS（5 组全绿：他人服务不接管不清理 / 自拉起并回收 / 超时失败无孤儿 / run-gui 静态契约 / main.rs 资产选择回归）**。

## 5. 集成门禁与策略连续性（独立复跑）

| 检查 | 命令 | 结果 |
|---|---|---|
| home 客户端策略（默认） | `python3 scripts/check-home-client-policy.py` | all invariants hold（ACTIVE=3）✅ |
| home 客户端策略（pending） | `python3 scripts/check-home-client-policy.py --expect-pending` | **NONE（3 项全闭环）** ✅ |
| home store 逻辑 | `node scripts/check-home-store-logic.mjs` | 通过 105 失败 0 ✅ |
| home UI 逻辑 | `node scripts/check-home-ui-logic.mjs` | 通过 32，HOME_UI_RESULT=PASS ✅ |
| 客户端导航逻辑 | `node scripts/check-client-navigation-logic.mjs` | CLIENT_NAV_RESULT=PASS(60/60) ✅ |
| Tauri 命令数 | `grep -c 'tauri::command\|#[command]'` | **137**（无新增）✅ |
| ACL/bridge/main.rs | `git diff --quiet` | UNCHANGED ✅ |
| 启动冒烟 | `bash scripts/check-dev-startup.sh` | PASS（所有权语义 5 组）✅ |
| 集成门禁 | `bash scripts/pre-merge.sh` | **PRE_MERGE_RESULT=ALL_PASS** ✅ |
| MCP 当前相位 | `check-mcp-policy.py --expect-current-gaps` | PASS（gated stdio 骨架）✅ |
| core / plugin / agent-skill / graph / agent-memory | 各自 `--self-test` | 7 / 7(P5) / 3(P10) / 8 / 5 全 PASS ✅ |
| 空白检查 | `git diff --check` | 干净 ✅ |

→ 完整性波次 A11（16:52）报的两红灯（指标 25.55%>25.2%、SCHEDUI_PANEL_LAZY 夹具过时）在 closeout 已解决（pre-merge=ALL_PASS 实测）。两红灯均**非安全轴**：指标属 A11/A0 预算裁决（W17 AC5），SCHEDUI_PANEL_LAZY 属夹具锚点过时（非真实回归，MainArea.vue 用 `{loader}` 对象形式）。A10 仅确认其解决未引入安全回归。

## 6. A5/A6/A7 HOLD 处理（按 dispatch 规则）

- **A5（HOLD）**：checkpoint 声明 closeout **零文件改动**；`--expect-pending` 仅剩 `HOME_NO_SECRET_PERSIST` 归 A3（store 层，非 A5 范围），其余 2 项已在 A5 完整性波次重建中闭环；home 组件 `grep localStorage.setItem` 仅注释、零写入。→ **HOLD 尊重，无安全阻塞**。
- **A6（HOLD）**：完整性波次导航改动（`useLayoutStore.ts`/`ActivityBar.vue`/`check-client-navigation-logic.mjs`）经脚本断言**无新 Tauri 命令/bridge/ACL/FS/网络特权、无 invoke、复用既有 redactSecrets、零新依赖**；closeout 无新工作。→ **在锁定权限内**。
- **A7（HOLD）**：完整性波次 shell fallback（`MainArea.vue`/`StatusBar.vue`/`App.vue`）为启动/boot/error 呈现，无新运行时行为；pre-merge ALL_PASS + tauri 137 不变 + 导航/UI 脚本覆盖 → **在锁定权限内**。closeout 无新工作。

→ 三 HOLD lane 均无「具体安全阻塞发现」，无需重开。

## 7. 结论

A10 W17 Acceptance Closeout 安全复审 **STATUS=PASS**：
- **raw sensitive persistence**：HOME_NO_SECRET_PERSIST（app 命令体落浏览器存储）已由 A3 closeout 修复闭环（toPersisted+isStorageSafe+迁移抹除），`--expect-pending`=NONE 实证。
- **shell/privilege expansion**：W17 范围内零扩张（137 命令、ACL/bridge/main.rs 零改、startup 助手无 sudo/pkill）；两越界 Rust 文件为 bug 修复非特权扩张，FLAG 交 A0 确认（非阻塞）。
- **startup ownership**：run-gui.sh/dev-server.sh/check-dev-startup.sh ownership-safe，实测冒烟 PASS。
- 集成门禁 `pre-merge.sh`=ALL_PASS，6 策略门全绿，无回归。
- A5/A6/A7 HOLD 尊重，无安全阻塞。

**非阻塞交接**：① 两越界 Rust 文件（script_runner.rs / linux.rs）交 A0 确认是否故意纳入；② url/dir 绝对路径持久化为有意非敏感元数据，若后续要求增强脱敏属 A0 增强项；③ 指标/调度夹具红灯已在 closeout 解决（A11/A0 域）。

## 8. 声明

- 本交付为**文档复审**，**零产品代码改动、零补丁（无源文件 diff）、未 push**（仅 A0 可 push）。
- 工作树现状：board/W16 文档、A2-A11 W17 在制件（含 A10 早前 `A10-M5-W17-desktop-client-security-review-20260908-1200.md` 及本 closeout 复审）均在途；A10 不整合、不 reset、不改他 lane 文件。
- HEAD=`052b18a`，本地领先 origin 0（已 ff-only 同步）；**未 push**。
- 未改动 `scripts/`（A4/A9/A3/A6/A2 各拥 W17 脚本新建/维护职责；当前无具体违规，依 Lane 默认不加夹具）。
