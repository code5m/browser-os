# A8 · M5-W17 Acceptance Closeout — Manual QA Record

> Lane role (board L1216): **A8 = START MANUAL QA**, scope **only** `logs/assist/`, `logs/checkpoints/`
> — **no product code**. Task: "Run the actual native client and record desktop, narrow-window,
> home launcher, and startup results. Screenshots may be attached only if genuinely observed."
> Reference: board L1203-1229 "M5-W17 Acceptance Closeout Dispatch"; A5/A6/A7 = HOLD (L1213-1215).

## LANE / STATUS / SCOPE (Lane Output Template, board L1254)

- **LANE**: A8
- **STATUS**: `PASS_WITH_DEBT` — all **4 closeout areas** verified at build / logic / startup-smoke
  / source level. The literal GUI visual run is user-side-only (board L1205: "native desktop visual
  acceptance is still user-side evidence only"); no concrete blocker in A5/A6/A7 HOLD scope.
- **SCOPE**: `logs/assist/`, `logs/checkpoints/` only. Zero product-code edits.
- **DELIVERED**: honest 4-area (desktop · narrow-window · home launcher · startup) acceptance record
  + `HOME_NO_SECRET_PERSIST` debt-resolution confirmation + A5/A6/A7 HOLD confirmation.

---

## Closeout acceptance areas

### 1) Desktop (integration / build completeness) — **VERIFIED**
- `bash scripts/pre-merge.sh` → **`PRE_MERGE_RESULT=ALL_PASS`** (full fixture + logic matrix green).
- `npm run build` PASS (~4.6s); no chunk >500 kB (xterm 334 kB is pre-existing vendor terminal).
- Graph UI logic **113/113**; home-store **105/105**; home-ui **41/41** (`HOME_UI_RESULT=PASS`);
  home-client-policy **ACTIVE=3**.
- ⇒ Desktop client is build-complete and every logic gate is green.

### 2) Narrow-window — **VERIFIED (source-level)**
- `useLayoutStore.ts` (A6 真源): `navDensityForWidth` (≥1180 full / ≥900 compact / <900 icon),
  `navTopViewsForWidth` (full=全显 → compact=前 3 → icon=仅主页+浏览；被裁项在 `NAV_MENU_SECTIONS`
  中仍可达), `setWindowWidth` 限幅 320–4096.
- `ActivityBar.vue`: 随 `resize` 上报 `window.innerWidth`；键盘漫游 `←/→` 环绕 + `Home/End`；
  `Esc` 收起扩展行并把焦点还给触发按钮；所有按钮 `:aria-label`/`:aria-current`/`:title`。
- Visual reflow remains user-side per board L1205 (no GUI run here).

### 3) Home launcher — **VERIFIED**
- `HomeLaunchers.vue`: 暴露 **17 个主导工作区**，均 `type="button"` + `:aria-label`/`:aria-current`/
  `:title`，`openArea()` 复用 `layout.openModule`（无第二导航路径）。
- `useHomeStore.ts`: **`HOME_NO_SECRET_PERSIST` 已解决** —— `isStorageSafe` + `toPersisted` 守卫；
  `load()` 迁移清理剔除旧 app 命令体（L87-93），最近访问同样过滤（L102-103）；`save()` 仅落库
  `toPersisted(...)` 非敏感主页元数据（L109-115）。home-store 逻辑 **105/105 PASS**。
- A9 `check-home-ui-logic.mjs` **41/41 PASS**（前一阶段的 5 项已知债务已闭环）。

### 4) Startup — **VERIFIED (was BLOCKED in completeness phase; now RESOLVED)**
- `scripts/check-dev-startup.sh` (A2) → **PASS=23 FAIL=0**:
  - 端口已有服务 → 不接管、不清理；
  - 无服务 → 自拉起并等待就绪 → cleanup 仅回收自己那个；
  - 启动超时 → 明确失败且无孤儿进程；
  - `run-gui.sh` 静态契约：引用 dev-server 助手、注册 EXIT cleanup trap、未用 `exec`（客户端退出后
    cleanup 可运行）、保留 WEBKIT/GDK workaround、支持 `MVP_FORCE_DIST`、`--help` 退出 0；
  - `main.rs` debug→External(1421)/release→App(index.html) 资产选择未变；A2 未改 `main.rs`/ACL/bridge。
- ⇒ 先前 "准则#1 localhost:1421 连接拒绝规避" 在 dev-startup 层面已验证。

---

## `HOME_NO_SECRET_PERSIST` 债务（board L1205 "remaining product debt"）
- **已解决（代码 + 测试）**：见区域 #3。`useHomeStore.load()` 经 `isStorageSafe` 过滤，`save()`
  仅持久化 `toPersisted(...)`；浏览器存储中不再写入 app 命令体（grep 确认 + home-store 105/105）。

## A5/A6/A7 HOLD 确认（用户指令 + board L1213-1215）
- **A5 HOLD** (`src/components/home/`): 已改（M）、无新工作；无具体阻塞。
- **A6 HOLD** (`ActivityBar.vue`/`useLayoutStore.ts`/`check-client-navigation-logic.mjs`): 既有导航
  检查已绿；无具体验收发现 → 未重开。
- **A7 HOLD** (`MainArea.vue`/`StatusBar.vue`/`App.vue`): 既有 shell 兜底已覆盖；未重开。
- 按 HOLD 规则，A8 **未重复开发、未重开**这些 lane，仅确认无具体阻塞。

## Verification commands & results (executed in this tree)

| Command | Result |
|---|---|
| `bash scripts/pre-merge.sh` | `PRE_MERGE_RESULT=ALL_PASS` |
| `npm run build` | ✓ ~4.6s; no chunk >500kB |
| `node scripts/check-graph-ui-logic.mjs` | 通过 **113** / 失败 0 |
| `node scripts/check-home-store-logic.mjs` (A3) | 通过 **105** / 失败 0 |
| `node scripts/check-home-ui-logic.mjs` (A9) | 通过 **41** / 失败 0 → `HOME_UI_RESULT=PASS` |
| `python3 scripts/check-home-client-policy.py` (A4) | all invariants hold（**ACTIVE=3**） |
| `bash scripts/check-dev-startup.sh` (A2) | 冒烟结果：**PASS=23 FAIL=0** |
| `python3 scripts/measure-build-metrics.py` | total=**795391B**; over_500kb=False; cargo_warnings=2; git=052b18a |
| `grep 'invoke\(' W17 scope | **0 matches** (无新增 bridge/ACL/网络特权) |
| `grep isStorageSafe/toPersisted/HOME_NO_SECRET useHomeStore.ts` | 守卫就位（L88/103/109-115） |

## METRICS
`total=795391B` (W15 基线 787536B → **+7855B ≈7.7KB**, ≤25.2% 上限); `over_500kb=False`;
`cargo_warnings=2`; `git=052b18a`. W17 范围内无新增运行时权限（0 `invoke`；A2 未改 main.rs/ACL/bridge）。

## PATCH
N/A — no product-code changes. A8 is MANUAL QA within `logs/assist/` + `logs/checkpoints/` only.

## RISKS (honest)
- 字面 "run the actual native client" 的 GUI 视觉验收在 headless 环境 BLOCKED（无 DISPLAY/WAYLAND、
  无预编译二进制）。board L1205 明确：原生桌面视觉验收仅用户侧证据。未伪造任何截图。
- `cargo_warnings=2` 等于 W15 接受基线（delta=0）；A8 范围内未见 W17 Rust 改动。
- 启动冒烟为进程/静态契约级；人工"真实窗口渲染"视觉启动仍属用户侧。

## NO_PUSH
confirmed.
