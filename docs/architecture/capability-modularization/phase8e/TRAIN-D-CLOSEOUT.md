# TRAIN D — Terminal Capability Isolation: CLOSEOUT

> 阶段：Capability Modularization / Train D（Phase 8E）
> 日期：2026-09-20
> 结论：**PASS，Terminal = C3（OPTIONAL / 可组合）**；`CURRENTLY_COMPOSABLE` 3 → **4**。
> tag：`capability-phase8e-terminal-composable-code-pass`
> 未 push（硬约束）；`SYSTEM_INSTALL_MODIFIED: NO`；`USER_DATA_MODIFIED: NO`。

---

## 1. 交付范围

| 类别 | 内容 |
|---|---|
| 审计 | `phase8e/TRAIN-D-TERMINAL-AUDIT.md`（Terminal 全量资产盘点、边界裁决、历史债务三分法） |
| SCR | `docs/architecture/semantic-changes/SCR-20260920-terminal-owner-extraction.md` |
| 代码 | 新增 `src/capabilities/terminal/{manifest,public,index}.ts`、`state/useTerminalStore.ts`、`ui/{TerminalPane,TerminalView,TerminalDockPanel}.vue`、`ui/useTerminalResize.ts`、`src/composables/terminalNav.ts`；`useSystemStore.ts` 收敛为 Clipboard+Apps |
| 登记 | `states.yaml` / `owners.yaml` / `intents.yaml` / `capabilities.yaml` / `dependencies.yaml` |
| 门禁 | 新增 `scripts/check-terminal-owners.mjs`（20 断言）；`check-capability-composition.mjs` 22→33；`check-terminal-policy.py` 29→30 变异；`check-terminal-ui-logic.mjs` 25→32 断言；`check-capability-pilot.mjs` PLT-04/05 重基线 |
| 接线 | `package.json` `check` + `scripts/pre-merge.sh` Phase 03 |

**新增贡献契约槽**：`workbench-main-resident`（常驻主视图，显隐由能力自管）——为了让终端
`v-show` 的「切走不卸载 xterm」行为原样保留；Shell 只按槽渲染，槽为空即不渲染。

---

## 2. 验收项逐条（TERM-01 … TERM-13）

| id | 要求 | 结论 | 证据 |
|---|---|---|---|
| TERM-01 | Terminal state 单 owner | PASS | owner = `useTerminalStore`（`src/capabilities/terminal/state/`）；`check-terminal-owners.mjs` TERM-01（11 状态全声明）+ TERM-02c（旧 owner 零 terminal 状态） |
| TERM-02 | 无第二 truth | PASS | TERM-02（全仓仅 owner 声明 terminal 状态）+ TERM-02b（无直写 termPanes/activeTermId/terminalOpen）+ TERM-02d/02e（per-pane 历史有界、零落盘）；Semantic Registry R8 无新增 |
| TERM-03 | Shell 不 import Terminal internals | PASS | TERM-03a（适配器零状态/零 native）+ TERM-03b（layout/home 零内部 import）+ composition `C5-TERM-{MAINAREA,ACTIVITYBAR,STATUSBAR,UNIFIEDTABBAR,HOMELAUNCHERS,APP}` 6/6 |
| TERM-04 | absent → Shell boots | PASS | `C6-TERMINAL-ABSENT`（槽为空）+ TERM-04a（absent 对照槽为空）+ TERM-04b（真实 bootstrap：bookmark/workspace/browser/terminal 全 ACTIVE）+ `vite build` PASS |
| TERM-05 | absent → no PTY | PASS | TERM-05a（**PTY 出生点唯一** = `ui/TerminalView.vue`）+ TERM-06（加载/实例化 store 零 spawn）+ `C6-TERMINAL-NO-SHELL-PTY`（Shell 无 spawnTerm/ensureTerm/addTermPane 调用点） |
| TERM-06 | absent → no child process | PASS | 同上（`term_spawn_channel` 唯一调用方在能力 UI；absent → 组件不挂载 → 零 invoke） |
| TERM-07 | present → PTY/process 正常 | PASS | TERM-07a（ensureTerm → 1 PTY，termPanes/activeTermId 一致 INV-4-1/4-2）+ 07b（幂等）+ 07c（追加不夺焦）+ 07d（贡献落槽）；`check-terminal-ui-logic.mjs` 32 断言（多面板、per-pane 隔离、回放、零落盘）+ `check-terminal-policy.py` 30 变异（后端管道/进程组/退避/隐私红线） |
| TERM-08 | destroy → cleanup | PASS | TERM-08a（killTerm 发出终止请求 + 注册表移除 + activeTermId 重置）+ 08b（被杀 pane 回放为空）+ 08c（全关后 activeTermId 归零）；Rust 侧进程组回收 `killpg/SIGTERM/SIGKILL` 仍由 `check-terminal-policy.py` 守卫 |
| TERM-09 | Semantic Registry PASS | PASS | `--self-test ALL_PASS`（R1..R9 + RI CASE A–E）；真实扫描 `fail=0 warn=6 info=72`（与 Train D 前**逐位一致**） |
| TERM-10 | Writer Enforcement PASS | PASS | R9 零新增失败；`check-semantic-closure-logic.mjs` 27/27；TERM-02b 组件直写为 fail 级 |
| TERM-11 | Capability Boundary PASS | PASS | `CAPABILITY_BOUNDARIES_RESULT=PASS (fail=0 warn=1)`（warn 与 Train D 前同源）；`check-capability-registry` PASS (0/0) |
| TERM-12 | Build PASS | PASS | `npm run build` exit 0（`TerminalView`/`TerminalPane` 独立 chunk，xterm 仍懒加载） |
| TERM-13 | Independent Review PASS | PASS | 本表逐条自证 + §4「诚实性审查」；无 skip / 无删 checker / 无放宽规则 / 无大 allow-list |

---

## 3. 成熟度定级（严禁高报）

| 等级 | 判据 | 是否达成 |
|---|---|---|
| **C3 OPTIONAL** | absent 时不加载/不创建资源；present 时功能与冻结语义不变 | **达成**（TERM-04/05/06 + 07） |
| C4 RUNTIME_CONTROLLABLE | runtime 可对能力做生命周期可控（activate/suspend/destroy 实际生效） | **未达**：Terminal 的 `suspend` 无实现；manifest `lifecycle.supported` 仍只有 ACTIVE/SUSPENDED 声明，**无物理 suspend** |
| C5 RESOURCE_RELEASABLE | 真实证明资源可释放（进程/PTY 回收可验证） | **未达**：本次只证明「destroy 请求 + 注册表清理 + 历史清空」（逻辑层），**未做真机进程数/PTY 数的释放实测** |

结论：**Terminal = C3**。C4/C5 需 Train F（Resource Governor + 真实资源测量）在真机取数后才能主张，
本 Train 不推测、不高报。

---

## 4. 诚实性审查（禁止项逐条自查）

| 禁止项 | 自查结果 |
|---|---|
| 只是搬目录？ | 否。owner 符号迁移（SCR）+ Contribution 化 + Shell 解耦 + 新增 4 类门禁 + 4 个旧门禁重基线 |
| God Store 仍存在？ | Terminal 已摆脱混居 owner；**保留** Debt-8E-1（Clipboard≡Apps 共居 useSystemStore），显式登记未静默 |
| God Runtime？ | 否。Runtime 仍只持 id/state/enabled；PTY 出生点在能力 UI，不经 Runtime |
| Contribution registry 变 service locator？ | 否。Shell 只按 slot/view 取值渲染；新增的 `workbench-main-resident` 是通用槽（非终端专用） |
| 第二状态真源？ | 否。TERM-02/02b/02c 三条静态断言 + R8 |
| Shell 知道 capability internals？ | 否。TERM-03b + 6 条 `C5-TERM-*`；Shell 仅经 `capabilities/terminal/public` 只读展示（与 Browser 同口径） |
| absent 真的不加载？ | 是。槽为空 → 组件不挂载 → 零 spawn；另有 TERM-06 运行期对照 |
| heavy resource 真的不创建？ | PTY/子进程：是（零 invoke 可证）。xterm（334KB chunk）也只在能力 UI 挂载时加载 |
| destroy 真的 release？ | **仅逻辑层为真**（终止请求 + 注册表 + 历史）；真机资源释放未测 → 故不上 C4/C5 |
| Registry 漂移？ | 无。`check-capability-registry` + `check-semantic-registry` RI + PLT-04 的 bootstrap 声明清单一致性断言 |
| Checker 失守？ | 无。新增 1 个、加固 3 个、修 2 个既有失效（分类见 §5） |
| 放宽规则？ | 无。历史清空检测由「文本计数 ≥3」改为「`clearTermHistory(id)` + `termHistories.delete(id)` 同在」并追加行为级证明；resize 检测由「含子串」改为「含调用」 |
| shared 是否垃圾桶？ | `terminalNav.ts` 为单函数窄缝（与 browserNav/recentsNav 同族），非垃圾桶 |
| facade 永久化？ | 无 facade；`public.ts` 是纯再导出（不持状态） |
| 误报 C4/C5？ | 无（明确未主张） |

---

## 5. 历史债务处理（三分法，附证据）

| 项 | 分类 | 处置 |
|---|---|---|
| `check-terminal-ui-logic.mjs` 崩溃（`system.startShell is not a function`） | PRE_EXISTING CHECKER DEBT（引入于 `2a96cb1`，checker 自 `0dd4cf4` 后未更新） | **修检测器**：对接 per-pane 真实 API；断言 25 → **32**（只增不删） |
| `check-terminal-policy.py` `TERM_HISTORY_CLEAR_MISSING`（FAIL） | PRE_EXISTING CHECKER DEBT（同一提交引入） | **修检测器 + 收紧**：要求 `clearTermHistory(id)` 与 `termHistories.delete(id)` 同在；自检变异 29 → **30**；新增行为级证明（TERM-08b/08c） |
| `check-terminal-policy.py` 变异 #7 锚点失效 | PRE_EXISTING CHECKER DEBT（resize 调用点重构后未同步） | 校正锚点（仍检出 `TERM_RESIZE_STUB`） |
| `TERM_PANE_RESIZE_MISSING` 被 probe 标签字符串掩盖 | PRE_EXISTING CHECKER 精度缺陷 | 检测改为调用形态 `termResize(` |
| `check-capability-pilot.mjs` PLT-04（“注册项应为 1”） | PRE_EXISTING（引入于 Train B 装配第 2 个能力） | **重基线为真不变量**（bootstrap 声明清单一致性 + bookmark 无强依赖），非放宽 |
| `check-capability-pilot.mjs` PLT-05（`src/components/home` 被改） | PRE_EXISTING（引入于 Train C `d6a2134`，改的是 Shell 导航胶水 HomeLaunchers.vue） | 路径集合改为 bookmark 的当前规范业务面（locator 驱动），基线仍锁 `semantic-governance-v1`；并把 HomeLaunchers 纳入 `C3-HOMELAUNCHERS` **补回**覆盖 |
| 终端物理路径迁移引发的潜在新 FAIL | 本 Train 引入风险 | 已同步 `check-terminal-policy.py` / `check-semantic-registry.mjs` 锚点 / `check-clipboard-persistence-logic.mjs`；**修复前后逐项复跑，零新增 FAIL** |

新增债务（显式，未静默）：Debt-8E-1（Clipboard≡Apps 共居）、Debt-8E-2（Terminal 无 `adapters/` 层）、
Debt-8E-3（`terminal-auto-confirm-cli` 仍写 localStorage，UI 偏好非敏感）、Debt-8E-4（历史清空检测器
重基线化记录）。Debt-7A-2（Terminal↔Clipboard 无法拆分）**CLOSED**。

---

## 6. 门禁实数（Train D 后）

```
check-semantic-registry            PASS   fail=0 warn=6 info=72   （--self-test ALL_PASS）
check-semantic-closure-logic       PASS   27/27
check-capability-boundaries        PASS   fail=0 warn=1           （--self-test 12/12）
check-capability-composition       PASS   33/33                   （--self-test 3/3）
check-capability-registry          PASS   fail=0 warn=0           （--self-test 13/13）
check-capability-runtime           PASS   15/15
check-capability-pilot             PASS   8/8
check-workspace-owners             PASS
check-terminal-owners              PASS   20/20                   （--self-test 7/7）
check-clipboard-persistence-logic  PASS   16/16
check-terminal-ui-logic            PASS   32 assertions
check-terminal-policy.py           PASS   （--self-test 30 变异全检出）
npm run check                      PASS
npm run build                      PASS
git diff --check                   clean
```

CURRENTLY_COMPOSABLE = **4**（Bookmark C3 + Workspace C3 + Browser C3 + **Terminal C3**）。
历史遗留的非本阶段 RED（`pre-merge` 中 build metrics / grid / 文档尾随空白等）仍存在，
未在本 Train 掩盖或放宽；Train G 统一复核。

NEXT：**Train E — Developer Capability Family（Database / Git，credential 只经 reference）**。
