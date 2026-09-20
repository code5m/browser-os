# TRAIN G — Final Automated Acceptance + Red Team（Phase 8E）

> 日期：2026-09-20
> 结论：**PASS（能力平台 v1 代码验收通过）**。全能力门禁 + 自检全绿；红队逐项自查无 HARD STOP。
> 历史遗留 pre-merge RED（尾随空白 + grid-close checker）属 M4/M5 债务，非本系列 train 引入，未掩盖。
> FINAL tag：`capability-modularization-v1-code-pass`

---

## 1. 全门禁结果（Train G 终检）

```
check-semantic-registry        PASS  fail=0 warn=6 info=72   (--self-test ALL_PASS)
check-semantic-closure-logic   PASS  27/27
check-sensitive-side-effects   PASS
check-capability-registry       PASS  fail=0 warn=0          (--self-test 13/13)
check-capability-runtime        PASS  15/15
check-capability-boundaries     PASS  fail=0 warn=1          (--self-test 12/12)
check-capability-composition    PASS  33/33                  (--self-test 3/3)
check-workspace-owners          PASS
check-browser-runtime           PASS
check-native                    PASS
check-grid-close-logic          PASS (phase03 内单独 FAIL 见 §4，属历史债)
check-view-intent               PASS
check-task-boundary             PASS
check-terminal-owners           PASS  20/20
check-terminal-policy.py        PASS  (--self-test 30 变异全检出)
check-terminal-ui-logic         PASS  32 断言
check-developer-owners          PASS  11/11
check-composition-profiles      PASS  11/11
check-capability-pilot          PASS  8/8
npm run check                   PASS
npm run build                   PASS
git diff --check                clean
git fsck --full                 clean（仅 3 个悬空 blob，无损坏对象）
```

CURRENTLY_COMPOSABLE = **4**（Bookmark C3 + Workspace C3 + Browser C3 + Terminal C3）。
Database / Git = C1（边界固化，未强拆 C3，诚实不高报）。

## 2. Terminal 成熟度（Train D 结论，本 Train 复核）

Terminal = **C3 OPTIONAL**：
- absent → 0 PTY / 0 child process（PTY 出生点唯一 = ui/TerminalView.vue；槽空 → 组件不挂载 → 零 invoke）
- present → PTY/进程正常、session/resize 语义保持
- destroy → killTerm 发出终止请求 + 注册表清理 + 历史清空（逻辑层；真机资源释放未实测）
- C4（无物理 suspend）/ C5（真机资源释放实测）**未达，未谎报**

## 3. Red Team（§16 攻击项逐项）

| 攻击项 | 结论 |
|---|---|
| 只是搬目录？ | 否。owner 迁移走 AUDIT→SCR→REGISTRY→CODE→CHECKER→TEST；F 新增薄协调层 |
| God Store？ | 否。Terminal/Database/Git/Repo 各自独立 owner；无 DeveloperStore |
| God Runtime？ | 否。Runtime/Governor 只编排，不持业务 state |
| Contribution registry 变 service locator？ | 否。Shell 只按 slot/view 渲染 |
| 第二状态真源？ | 否（TERM-02/DEV-02c/workspace-owners） |
| Shell 知 capability internals？ | 否（C3/C5-* + DEV-05 + 各 public 边界） |
| absent 真不加载？ | 是（profile minimal 端到端零 PTY/零 WebView） |
| heavy resource 真不创建？ | 是（PTY/WebView 出生点唯一且随槽挂载） |
| destroy 真 release？ | 逻辑层是；真机资源释放未实测 → 不上 C5 |
| Registry 漂移？ | 无（registry + pilot PLT-04 声明清单一致性） |
| Checker 失守？ | 无。新增 3 门禁（terminal-owners/developer-owners/composition-profiles）；修复 2 个既有失效 |
| 放宽规则？ | 无 |
| shared 垃圾桶？ | 否（terminalNav/browserNav/recentsNav 均为窄缝） |
| facade 永久化？ | 无（public.ts 纯再导出） |
| 误报 C4/C5？ | 无 |

## 4. 已知 pre-merge RED（PRE_EXISTING，非本系列 train 引入，未掩盖）

`scripts/pre-merge.sh` 在 Train G 终检时仍 FAIL，两项均来自 M4/M5 历史债务：
1. `git diff --check (branch range)`：`docs/architecture/capability-platform/phase7e-resource-governance/Capability-Resource-Report.md:41` 尾随空白（历史文档，非本 train 改动）。
2. Phase 03 `check-grid-close.mjs` FAIL（Grid 关闭逻辑历史 checker，与能力隔离无关）。

二者与本系列 Train A–G 的改动无因果；按交接文档既定口径「历史非阻塞 RED，不掩盖/不放宽」，留待专项。

## 5. 交付物（Train A–G 累计）

- 能力：Bookmark / Workspace / Browser / Terminal = C3；Database / Git = C1（边界固化）
- 新增文件：`src/capabilities/terminal/**`、`src/composables/terminalNav.ts`、`src/capability/{profiles,resourceGovernor}.ts`
- 门禁：check-terminal-owners(20) / check-developer-owners(11) / check-composition-profiles(11) 新增；
  check-capability-composition 22→33；check-terminal-policy 默认 FAIL→PASS（30 变异）；
  check-terminal-ui-logic 崩溃→32；check-capability-pilot PLT-04/05 重基线 8/8
- 治理：states/owners/intents/capabilities/dependencies 同步；SCR-20260920-terminal-owner-extraction
- 文档：phase8e/{TRAIN-D-TERMINAL-AUDIT,TRAIN-D-CLOSEOUT,TRAIN-E-DEVELOPER-AUDIT,TRAIN-F-RESOURCE-GOVERNOR,TRAIN-G-FINAL-ACCEPTANCE}.md
- tag（均本地未 push）：
  - capability-phase8d-browser-composable-code-pass (Train C)
  - capability-phase8e-terminal-composable-code-pass (Train D)
  - capability-phase8e-developer-family-audit-pass (Train E)
  - capability-phase8e-resource-governor-profiles-pass (Train F)
  - **capability-modularization-v1-code-pass (Train G, FINAL)**

## 6. 后续（用户侧）

- Human GUI 验收仍由用户完成（Terminal xterm 交互 / Dock 终端 / 宫格 / 浏览器 webview / 各能力面板）。
- feature 分支 `feature/capability-platform-v1` 全部 train 完成后，由用户/A0 统一 ff-only merge 到 master。
- Debt-8E-1..6 显式登记，交后续 train（含 Database/Git 抽可选能力包达 C3）。
