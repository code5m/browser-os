# A10 · M5-W17 Desktop Client Completeness and Home Recovery 安全复审（SECURITY REVIEW）

> Lane: A10（M5 独立安全复审） · Wave: **M5-W17 Desktop Client Completeness and Home Recovery Dispatch**（board §M5-W17；A0 加于 W16 收尾后；base `052b18a`）
> A10 W17 任务（board L1194）：**Review W17 patch boundaries against the locked runtime-authority list and desktop startup safety. Findings first; no product-code modifications.**
> 复审时间：2026-09-08 ~12:00 CST · BASE=`052b18a`（`feat(M5): close W15 release readiness`，已 push 且 A0 接受）
> 交付物：本复审（仅文档，**零产品代码改动、零补丁、未 push**）。Lane 默认不编辑产品码；当前无 W17 具体违规，故不加 policy 夹具（A4 拥有 `check-home-client-policy.py` 新建、A9 拥有 `check-home-ui-logic.mjs` 新建，A10 不越权）。

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=PROVISIONAL_PASS（准备期安全复审：当前树安全、无 W17 运行时扩张；A2-A7 W17 产品码/补丁未交付，终审 BLOCK/PASS 待其 patch 到后按 §5 闸判定）
BASE=052b18a
HEAD=logs/assist/A10-M5-W17-desktop-client-security-review-20260908-1200.md
FILES=logs/assist/A10-M5-W17-desktop-client-security-review-20260908-1200.md
VERIFY=实证（§3）：git pull --ff-only @052b18a（W11-W15 已集成推送）；W17 产品码未到本树（run-gui.sh/useHomeStore.ts/home组件/ActivityBar/MainArea/StatusBar/App.vue 均 git diff HEAD 空=基线未改；check-home-*.mjs/check-home-client-policy.py ABSENT=未建）；6 策略门全绿（agent-skill ACTIVE=3/PENDING=10、mcp --expect-current-gaps PASS[含 gated stdio 骨架]、core ACTIVE=7、plugin ACTIVE=7/PENDING=5、graph ACTIVE=8、agent-memory ACTIVE=5）；tauri::command 基线计数=137（W17 须保持，禁新增）；ACL 末条仍为 list_artifact_images（无新 ACL 项）；W17 各 lane scope 经 §2 比对全部落在锁定权限清单内（L1170）
CHECKPOINT=logs/assist/A10-M5-W17-desktop-client-security-review-20260908-1200.md
MERGE_NOTES=A10 W17 安全复审：① 锁定运行时权限清单(L1170)为唯一闸——禁命令执行/插件调用/动态加载/远程下载监听/daemon/模型调用/Agent·Skill 执行/MCP live 运行时/图谱写导出/后台 worker；无新依赖、无新 Tauri 命令/bridge/ACL/FS 权限/网络特权；② 当前树(052b18a)连续性确认——W15 收口+W16 文档-only，无 W17 运行时扩张，6 策略门全绿，tauri command 137 未变；③ W17 各 lane scope(A2 启动助手/A3 home store/A5 home UI/A6 ActivityBar/A7 外壳 fallback)经逐项比对均不越权（§2 表）；④ 桌面启动安全闸(§5 GATE-S)——run-gui.sh 须 ownership-safe（仅启停自有 Vite、退出清自己、无特权升、无孤儿进程）、main.rs 仅 debug/release 资源选择、无新命令/bridge/ACL/FS/网络特权；⑤ 共享验收 #4(无敏感 URL/query/凭据/本地路径泄露)与 #5(构建指标不绕过，超预算即停报 delta)纳入 GATE-UI/GATE-M；⑥ 现状=PROVISIONAL_PASS，终审待 A2-A7 W17 patch 到后按 §5 GATE-A~M 判定；⑦ 与 A4(隐私静态审查)/A9(UI 逻辑 DOM 检查)/A11(验证矩阵)分工互补，A10 交叉确认不冲突
NEXT=A2-A7 W17 交付后由 A10 套 §5 闸终审：GATE-A(无新 Tauri command，137 不变)/GATE-B(无新 ACL/bridge/FS/网络特权)/GATE-C(无命令执行/插件调用/动态加载/远程下载监听/daemon/模型调用/Agent·Skill 执行/MCP live 运行时/图谱写导出/后台 worker)/GATE-D(无新依赖)/GATE-S(启动助手 ownership-safe)/GATE-UI(无敏感披露)/GATE-M(指标≤25.2% 且不绕过)。全过=PASS；任一违=BLOCK 交 A0。附 A4/A9/A11 协审结论
```

## 1. 复审范围与方法

W17 是**真实产品码 wave**，但刻意 bounded 到既有本地客户端能力（board L1168）。A10 方法（实证，findings-first，不修改产品码）：
- 对当前集成树（HEAD=`052b18a`，W15 收口已 push）重跑 6 策略门 + 权限基线计数，确认 W17 前连续性。
- **逐一比对 W17 各 lane scope（L1185-1195）vs 锁定运行时权限清单（L1170）**，确认 dispatch 自身不授权任何禁止能力。
- 复审桌面启动安全（共享验收 #1 + A2 scope）。
- 定义 A2-A7 **W17 patch 到后** A10 必须 BLOCK/PASS 的精确闸（GATE-A~M）。
- 范围隔离：A10 不编辑产品码 / 不新建 `scripts/`（A4 拥有 `check-home-client-policy.py`、A9 拥有 `check-home-ui-logic.mjs`、A3 拥有 `check-home-store-logic.mjs`、A6 拥有 `check-client-navigation-logic.mjs`）。当前无具体违规 → 不加夹具。

## 2. W17 各 Lane Scope vs 锁定权限清单（L1170）对齐

锁定清单（L1170 禁止项）：命令执行 / 插件调用 / 动态加载 / 远程下载·监听 / daemon / 模型调用 / Agent·Skill 执行 / MCP live 运行时 / 图谱写·导出 / 后台 worker；**无新依赖**；**无新 Tauri command / bridge 能力 / ACL 项 / 文件系统权限 / 网络特权**。

| Lane | W17 允许 scope（L1185-1195） | 是否越权 | A10 预审 |
|---|---|---|---|
| A2 | `run-gui.sh` + `scripts/` 启动助手/测试 + 仅必要时 `src-tauri/src/main.rs`(debug/release 资源选择) | 否 | ✅ 启动助手 ownership-safe（§5 GATE-S）；不改命令/bridge/ACL/生产运行时权限 |
| A3 | `src/stores/useHomeStore.ts`,`src/utils/homeUi.ts`,`scripts/check-home-store-logic.mjs` | 否 | ✅ 有界校验 home 状态 + 畸形数据迁移安全；无后端/无组件样式 |
| A4 | `scripts/check-home-client-policy.py`(新) + 复审 | 否 | ✅ 仅静态隐私/安全复审（A10 交叉其脱敏闸，不重复造轮） |
| A5 | `src/components/home/` 仅 | 否 | ✅ 重建 home 表面；无新依赖/无后端 |
| A6 | `src/components/layout/ActivityBar.vue`,`src/stores/useLayoutStore.ts`,`scripts/check-client-navigation-logic.mjs` | 否 | ✅ 导航可发现/键盘可达；不动 home/MainArea/后端 |
| A7 | `src/components/layout/MainArea.vue`,`StatusBar.vue`,`src/App.vue` 仅 | 否 | ✅ 外壳 fallback/boot/error；无新运行时行为 |
| A8 | 手动 QA（运行原生客户端） | 否 | ✅ 验收清单，不编代码 |
| A9 | `scripts/check-home-ui-logic.mjs`(新) + 仅必要时 `pre-merge.sh` | 否 | ✅ DOM/源码级检查；不改产品组件 |
| A10 | 复审（本文件） | 否 | ✅ |
| A11 | 验证矩阵（测试/`npm run build`/指标/Rust 测试） | 否 | ✅ 不源码编辑（除自含验证脚本） |

**结论：W17 dispatch 各 lane scope 全部落在锁定权限清单内**，未授权任何禁止能力。A10 不预判具体 patch 违规，但终审以 §5 闸实测为准。

## 3. 连续性实证（当前树 HEAD=`052b18a`，W17 前安全）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git pull --ff-only   # @052b18a（W11-W15 已集成推送；W16 文档-only 未 push 但本树仅 dirty 文档）
# W17 产品码未到本树：
git diff --name-only HEAD -- run-gui.sh src/stores/useHomeStore.ts src/components/home/ \
  src/components/layout/ActivityBar.vue src/components/layout/MainArea.vue \
  src/components/layout/StatusBar.vue src/App.vue   # → 空（基线文件未改）
ls scripts/check-home-store-logic.mjs scripts/check-home-client-policy.py \
   scripts/check-home-ui-logic.mjs scripts/check-client-navigation-logic.mjs 2>&1  # → ABSENT（未建）
# 6 策略门（W15 收口基线仍绿）：
python3 scripts/check-mcp-policy.py --expect-current-gaps  # PASS（含 gated stdio 骨架）
python3 scripts/check-core-boundary.py --self-test          # CORE_POLICY_SELF_TEST=PASS(ACTIVE=7)
python3 scripts/check-plugin-policy.py --self-test         # ACTIVE=7 PENDING=5
python3 scripts/check-agent-skill-policy.py --self-test    # ACTIVE=3 PENDING=10
python3 scripts/check-graph-policy.py --self-test          # GRAPH_POLICY_SELF_TEST=PASS(ACTIVE=8)
python3 scripts/check-agent-memory-policy.py --self-test   # AGENT_KV_POLICY_SELF_TEST=PASS(ACTIVE=5)
# 权限基线：
grep -rnE "tauri::command|#\[command\]" src-tauri/src/ | wc -l   # 137（W17 须保持，禁新增）
grep -nE "list_artifact_images" default-commands.toml | tail -1   # 末条仍 list_artifact_images（无新 ACL 项）
```

**实测**：W17 产品码 0 改动（baseline 未动）；4 个 W17 check 脚本 ABSENT；6 策略门全绿；tauri command=137；ACL 末条 `list_artifact_images` 未变。**→ 当前树零 W17 运行时扩张，符合锁定权限清单。**

## 4. 桌面启动安全（共享验收 #1 + A2 scope）

验收 #1（board L1174）：开发者启动桌面 debug 客户端时不得落到 `localhost:1421` connection refused——helper 须 detect/start Vite dev server、wait、launch 桌面 app、退出时**仅清理自己拥有的 server**；release 行为仍用 bundled assets。

A10 启动安全闸（待 A2 交付 `run-gui.sh` + 必要时 `main.rs` 改后套用）：
- **S-1 ownership**：脚本只启停**自己 spawn 的** Vite 进程；退出清理仅匹配自有 PID/端口；不得 kill 任意 `localhost:1421`/vite 进程（避免误杀他人/孤儿）。
- **S-2 无特权升**：不得 `sudo`/setuid/提权；不得改系统级文件或网络配置。
- **S-3 无孤儿**：异常退出路径也清理自有 server（trap EXIT）；不得留后台 daemon。
- **S-4 资源选择仅 debug/release**：`main.rs` 改动仅做 bundled vs dev-asset 选择，不引入新命令/bridge/ACL/FS 权限/网络特权。
- **S-5 无敏感披露**：脚本日志/错误不得回显 token/密码/secret/内部 URL。

## 5. A10 终审 BLOCK/PASS 闸（A2-A7 W17 patch 到后套用）

> 原则：W17 允许「有界前端/启动体验产品码」，但**任一以下违例 = 立即 BLOCK**，交 A0 决策（L1170 硬边界）。

- **GATE-A（无新 Tauri command）**：`grep -c tauri::command/#[command]` 必须 = 137（基线），W17 patch 不得新增 Tauri 命令。
- **GATE-B（无新 ACL/bridge/FS/网络特权）**：`default-commands.toml` 末条仍 `list_artifact_images`（无新 ACL 项）；无新 bridge 能力；无新文件系统权限申请；无新网络特权（无新 `capability`/`.json` 权限项）。
- **GATE-C（无禁止运行时能力）**：W17 patch 不得含命令执行 / 插件调用 / 动态加载(`import()` 远程或 eval) / 远程下载·监听 / daemon / 模型调用 / Agent·Skill 执行 / MCP live 运行时 / 图谱写·导出 / 后台 worker。grep 关键词：`std::process::Command`/`Command::new`/`eval(`/`new Function`/`fetch(` 到非本地资源/`WebSocket` server/`setInterval` 长驻 worker/`graph.*write`/`export`/`plugin.*install`/`plugin.*enable`/`skill.*exec`/`agent.*exec`/`mcp.*serve`。
- **GATE-D（无新依赖）**：`package.json` / `Cargo.toml` 无新增依赖（A5「no new dependency」硬性）。
- **GATE-S（启动安全，§4）**：`run-gui.sh` + 必要时 `main.rs` 满足 S-1~S-5。
- **GATE-UI（无敏感披露，验收 #4）**：新增可见控件/错误不得泄露敏感 URL/query/凭据/本地绝对路径；空/加载/错误态文案经 `check-home-ui-logic.mjs`/`check-client-navigation-logic.mjs` 断言无 sensitive copy（A9 主审，A10 交叉）；home store 的 shortcut/recent 渲染不得拼接本地绝对路径或 token 到 DOM。
- **GATE-M（构建指标不绕过，验收 #5）**：W17 实现不得推高 `total_bytes_pct` 超过 W15 已批的 **25.2% one-time ceiling**；若超预算，**stop 并 report 精确 delta**，不得私自抬高 ceiling（A11 主审指标，A10 提示；board L6 现状 25.14 ≤ 25.2%）。
- **GATE-ROB（畸形数据弹性，验收 #2/#3）**：home store（A3）须对畸形本地数据 bounded + resilient（迁移安全、不崩、空/加载态 coherent）；A10 确认其不引入解析期代码执行（如 `JSON.parse` 后 `eval`、或把 unknown field 当代码）。
- **GATE-A11Y（键盘可达/可访问名，验收 #4）**：新控件键盘可达、有 accessible name、用既有视觉语言（A6/A5 主审，A10 交叉；不引入新运行时行为）。

## 6. 与历史安全终裁的承接

- **W10**（MCP stdio-prep）：A10 终审闸 GATE-D 要求 A3 翻转 `MCP_NO_RMCP_SERVER`→`MCP_SERVER_GATED`；W11 MCP dry-run hardening + W12 graph live query + W13 plugin manifest + W14 plugin UI + W15 a11y/2FA 均已集成推送，`mcp --expect-current-gaps` 含「gated stdio 骨架已落地」结论 → W10 闸已闭合。
- **W15**（A10-M5-W15-security-release-review）：PASS（可 push）。W17 系其之上的**受控桌面补全**，不新增运行时权限（L1170 硬边界）。
- 本 W17 复审在 W15 PASS 基础上加「桌面客户端/启动安全」专项闸（§4-§5），不重审已闭合项（MCP gated、graph 只读、plugin manifest lifecycle、Agent/Skill 锁、a11y/2FA）。

## 7. 残留债 / 交接（非 W17 阻断）

1. **GATE 依赖 A2-A7 W17 patch 交付**：A10 终审能否 PASS 直接取决于 A2（run-gui.sh）、A3（useHomeStore）、A5（home/）、A6（ActivityBar）、A7（MainArea/StatusBar/App.vue）是否按 scope 交付且过 §5 闸。未交付前仅 PROVISIONAL_PASS。
2. **W16 文档在途**（board/AI清单/详细设计/后续TODO 已 modified；A1/A2/A3/A4/A5/A6/A7/A8/A11 W16 文档+patch 在树未 push）：A10 不整合，留 A0。
3. **agent-skill PENDING=10**（较 W15 的 5 增）：W16 A5 加执行授权设计后 policy 脚本扩 PENDING 桶；属设计期守卫，不阻断 W17（W17 不碰 Agent/Skill 执行）。
4. **指标 ceiling 25.2%**：W17 若小幅超须 A11 报 delta + A0 决策，A10 不擅自认。

## 8. 声明

- 本交付为**文档复审**，**零产品代码改动、零补丁、未 push**（仅 A0 可 push）。
- 工作树现状：`PARALLEL_COMMAND_BOARD.md`（含 W17 段，+62）、W16 文档/补丁在途（见 §7-2）；**无 W17 产品码/补丁**。
- HEAD=`052b18a`，本地领先 origin 0（已 ff-only 同步）；**未 push**。
- **当前树 = PROVISIONAL_PASS（W17 前安全，无运行时扩张，6 策略门全绿，tauri command 137 未变，各 lane scope 不越权）**；A10 终审（BLOCK/PASS）待 A2-A7 W17 patch 到后按 GATE-A~M 实测判定。若任一 patch 含 GATE-C 禁止能力（尤其命令执行/插件调用/MCP live/图谱写导出）或 GATE-A 新增 Tauri 命令或 GATE-S 启动不安全，A10 将发布 BLOCK 并交 A0。
- 未改动 `scripts/`（A4/A9/A3/A6 分别拥有 W17 各 check 脚本新建职责；当前无具体违规，依 Lane 默认不加夹具）。
