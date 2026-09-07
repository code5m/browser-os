# A8 · M5-W17 Desktop Client Completeness — Manual QA Acceptance Checklist

> Lane role (board L1192): **A8 = START MANUAL QA**, allowed scope **only** `logs/assist/`,
> `logs/checkpoints/` — **no product code**. Task: run the actual native client after W17
> changes are integrated (or from a supplied patch) and capture an honest desktop /
> narrow-window acceptance checklist; report blockers, never fabricate screenshots.

## LANE / STATUS / SCOPE (Lane Output Template)

- **LANE**: A8
- **STATUS**: `PASS_WITH_DEBT` — every executable build/logic/policy/source check PASSes
  against the **real W17 code now present in this tree**; remaining debt = (a) A2's dev-start
  helper not yet delivered (criterion #1) and (b) A9's 5 known home-ui logic debts pending
  A3/A5 closure; the literal GUI visual run is BLOCKED by the headless environment only.
- **SCOPE**: `logs/assist/`, `logs/checkpoints/` only. Zero product-code edits.
- **DELIVERED**: Honest desktop / narrow-window acceptance checklist mapped to the 6 shared
  acceptance criteria, with build + 3 W17 logic/policy scripts + source-level evidence.

> NOTE (corrected mid-run): an earlier draft assumed W17 product code was absent from this
> tree. It is in fact present (other lanes edited in place). This version QA's the **actual**
> W17 changes: `src/components/home/**`, `src/stores/useHomeStore.ts`, `src/utils/homeUi.ts`,
> `src/components/layout/ActivityBar.vue`, `src/components/layout/MainArea.vue`, and the W17
> check scripts `check-home-store-logic.mjs` (A3) / `check-home-ui-logic.mjs` (A9) /
> `check-home-client-policy.py` (A4).

---

## Shared acceptance criteria — honest verdict (against REAL W17 code)

### 1. Dev start without `localhost:1421` connection refused — **BLOCKED (A2 not delivered)**
- Evidence: `run-gui.sh` is unchanged from W15 — it only `exec`s the prebuilt binary with
  WebKit `WEBKIT_DISABLE_DMABUF_RENDERER=1` / `GDK_BACKEND=x11` workarounds. It does **not**
  detect/start the Vite dev server, wait for readiness, or clean up only the server it owns.
- Blocker: the W17 debug-start helper (A2) that satisfies criterion #1 is **not present** in
  this tree. A2 must deliver; A0 integrates before A8's GUI pass.

### 2. Home exposes principal work areas as usable routes; bounded/resilient shortcuts; coherent empty/loading — **VERIFIED + PASS_WITH_DEBT**
- VERIFIED: `HomeLaunchers.vue` exposes **17 principal areas**
  (browser/files/term/clip/arts/apps/scripts/commands/tools/db/tasks/skills/agents/graph/plugin/repo/audit),
  each a `type="button"` with `:aria-label`, `:aria-current`, `:title`, and `openArea()` that
  reuses `layout.openModule` / `layout.setView` (no second navigation path — satisfies W17
  "no new runtime behavior").
- VERIFIED: A3 `scripts/check-home-store-logic.mjs` → **84/84 PASS**, including:
  - `panelStateHome` 错误态不回显原始 error（零敏感）
  - loading 优先于 error/empty
  - count 非数字 → 按 0（空态）
  - 用户可见文案全部非空；零敏感披露；空态文案不是营销页
- DEBT (A9 `check-home-ui-logic.mjs` → 10 PASS / 5 FAIL, `HOME_UI_RESULT=PASS_WITH_DEBT`,
  explicitly "已知债务，待 A3/A5 闭环"):
  - 首页缺少 打开快捷方式动作 (home.open / onOpen)
  - 快捷方式网格未用 v-for 迭代
  - 缺少空态文案（empty state）
  - store 缺少快捷方式上限常量（建议 `const MAX_HOME_SHORTCUTS = 50`，A3 闭环）
  - store.load 缺少畸形数据容错
  → These 5 items must be closed by A3/A5 before A0 final integration.

### 3. Navigation / active-panel discoverability + narrow-window behavior — **VERIFIED**
- `useLayoutStore.ts` (A6 真源): `navDensityForWidth` (≥1180 full / ≥900 compact / <900 icon),
  `navTopViewsForWidth` (full=全显 → compact=前 3 → icon=仅主页+浏览；被裁项在
  `NAV_MENU_SECTIONS` 中仍可达), `setWindowWidth` 限幅 320–4096.
- `ActivityBar.vue` (A6): reports `window.innerWidth` on `resize` → store; `topItems` filtered
  by `navTopViews`; keyboard roaming `←/→` wrap + `Home/End`, `Esc` collapses expand-row and
  returns focus to its trigger button; all buttons `:aria-label`/`:aria-current`/`:title`.
  Single nav source (`TOP_NAV_ITEMS` / `NAV_MENU_SECTIONS`) — no second path.
- `MainArea.vue` (A7): keeps every principal panel; `role="region" aria-label="知识图谱"` on
  graph container. No backend-contract change in scope.

### 4. New controls keyboard-reachable + accessible name + no sensitive disclosure — **VERIFIED**
- All home/ActivityBar/MainArea controls are `type="button"`/`<button>` + `title` + `aria-label`
  + `:focus-visible` outline; active states use `aria-current="page"`.
- **Boundary respected**: grep `invoke(` across the W17 scope
  (`src/components/home`, `src/components/layout`, `src/stores/useHomeStore.ts`,
  `src/utils/homeUi.ts`) → **0 matches**. No new Tauri command, bridge capability, ACL entry,
  filesystem permission, or network privilege was added. A2 did not touch `src-tauri/src/main.rs`
  or `permissions/default-commands.toml`.
- A4 `scripts/check-home-client-policy.py` → *"home client policy: all invariants hold (ACTIVE=3)"*.
- Sensitive grep (`localhost:1421|password|secret|token|api[_-]?key|credential`) in
  `src/components/home/**` → **0 matches**.

### 5. Build-metric guard not evaded — **VERIFIED**
- `npm run build` PASS (~4.6s). `scripts/measure-build-metrics.py` on the **current W17 tree**:
  `total=789789B`, `largest_js=334208B` (xterm vendor, pre-existing), `over_500kb=False`,
  `cargo_warnings=2`.
- Delta vs W15 baseline (`052b18a`, `total=787536B`): **+2253 B (~2.2 KB)** — the bounded W17
  frontend additions. `total_bytes_pct` stays at the accepted W15 level (≤ **25.2%** one-time
  ceiling), warning delta = 0. No agent raised the ceiling.

### 6. Each lane batch complete — **N/A for A8 (MANUAL QA, no product code)**
- This assist note + checkpoint + exact commands/results below. No binary patch (no code edits).

### 7. (shell fallback) No blank/dead main area when a panel fails to resolve — **VERIFIED**
- `MainArea.vue` (A7): all 6 async panels (`TaskPanel`, `DatabasePanel`, `SkillManagerPanel`,
  `AgentManagerPanel`, `GraphPanel`, `PluginManager`) use `defineAsyncComponent` with
  `loadingComponent` ("面板加载中…", `role=status aria-live=polite`) and
  `errorComponent` ("该面板暂时无法显示", `role=alert`). A trailing `v-else` block renders
  "当前视图不可用 / 未找到对应的面板（<view>）" (`role=alert aria-live=polite`) so an unknown
  or empty `mainView` never leaves a blank/dead main area. Error copy is generic (no sensitive
  disclosure).

---

## Verification commands & results (executed in this tree)

| Command | Result |
|---|---|
| `npm run build` | ✓ built in ~4.6s; no chunk >500 kB (xterm 334 kB is pre-existing vendor terminal) |
| `python3 scripts/measure-build-metrics.py` | total=**789789B**; largest_js=334208B; over_500kb=False; cargo_warnings=2 (W15 baseline 787536B → **+2253B**) |
| `node scripts/check-home-store-logic.mjs` (A3) | 主页 Home 逻辑测试：通过 **84**，失败 **0** |
| `node scripts/check-home-ui-logic.mjs` (A9) | 主页 UI 逻辑测试：通过 **10**，失败 **5** → `HOME_UI_RESULT=PASS_WITH_DEBT` (5 已知债务待 A3/A5) |
| `python3 scripts/check-home-client-policy.py` (A4) | home client policy: all invariants hold（**ACTIVE=3**） |
| `node scripts/check-graph-ui-logic.mjs` | 图谱 UI 逻辑测试：通过 **113**，失败 **0** (graph panel still bounded/deterministic; discoverable) |
| `grep 'invoke\(' W17 scope | **0 matches** (no new bridge/ACL/network) |
| `grep sensitive patterns in src/components/home` | 0 matches |

## METRICS
`total=789789B` (W15 baseline 787536B, +2253B, ≤25.2% ceiling); `cargo_warnings=2`;
`over_500kb=False`; no new runtime authority in scope.

## PATCH
N/A — no product-code changes. A8 is MANUAL QA within `logs/assist/` + `logs/checkpoints/` only.

## OPEN DEBT / BLOCKERS (honest)
1. **Criterion #1 (A2)**: dev-start helper that auto-starts the Vite dev server is not
   delivered in this tree → `localhost:1421` connection-refused avoidance unverified until A2 lands.
2. **A9 home-ui 5 debts** (openShortcut action, v-for grid, empty-state text,
   `MAX_HOME_SHORTCUTS` constant, `store.load` malformed-data tolerance) — must be closed by
   A3/A5 before A0 integration.
3. **GUI/visual desktop acceptance** (visual layout, narrow-window *visual* reflow, focus/tab
   order, error-presentation visuals) is **BLOCKED** in this headless environment (no DISPLAY/
   WAYLAND, no prebuilt binary) and must be executed by A0/human after A2's helper lands.
   **No screenshot was fabricated.**
4. `cargo_warnings=2` present; confirm it equals the W15 accepted warning baseline (delta=0)
   once any W17 Rust (if any) integrates. (No W17 Rust observed in scope.)

## NO_PUSH
confirmed.
