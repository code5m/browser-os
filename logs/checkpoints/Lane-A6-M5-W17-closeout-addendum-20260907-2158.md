# Lane A6 · W17 收口补录：五方 HOLD 确认 + 自检缺陷 SF-1 + 待派发占位

- 时间：2026-09-07 21:58 CST
- 分支：`master` @ `052b18a`（`git pull --ff-only` → 已经是最新的，HEAD 未变）
- 前置产物：`logs/checkpoints/Lane-A6-M5-W17-client-navigation-20260907-1658.{patch,md}`、`Lane-A6-M5-W17-closeout-20260907-2135.{patch,md}`

```text
LANE: A6
STATUS: PASS_WITH_DEBT（W17 收口完成；自检发现 1 条自身缺陷 SF-1，未在本波改动，待 BUG-HUNT 派发）
SCOPE: 本条补录零产品代码改动；仅复核 + 自检（src/stores/useLayoutStore.ts、src/components/layout/ActivityBar.vue 与已交付补丁逐字节一致）
DELIVERED: 五方 HOLD 确认对账；SF-1（Esc 无法收起"最近/常用"行）定位 + 两版修复方案；SF-2 咨询项；下波待命状态
VERIFY: node scripts/check-client-navigation-logic.mjs → CLIENT_NAV_RESULT=PASS (60/60)
        node scripts/check-ui-a11y-logic.mjs → UI_A11Y_RESULT=PASS
        node scripts/check-home-ui-logic.mjs → HOME_UI_RESULT=PASS
        node scripts/check-home-store-logic.mjs → 通过 105 / 失败 0
        diff 两文件 vs 16:58 交付备份 → 无差异；git diff --check → 干净
METRICS: 同 2135 checkpoint（A6 独占 +3,666B；F-1 基线污染待 A0 处置）
PATCH: 仍为 logs/checkpoints/Lane-A6-M5-W17-closeout-20260907-2135.patch（未变）
RISKS: SF-1 未修（Esc 收起 omni 行后立即被 @focus 重开）；F-1/F-2 仍归 A0；BUG-HUNT 派发表未发布，A6 不自派
NO_PUSH: confirmed
```

## 1. 五方对账：A6 HOLD 已被确认，无 reopen

| 来源 | 对 A6 的结论 |
|---|---|
| A11 `A11-M5-W17-closeout-matrix-20260907-2139.md` | 第 59 行 `客户端导航逻辑（A6）… CLIENT_NAV_RESULT=PASS (60/60) ✅`；第 89 行「无具体阻塞 → 维持 HOLD」；第 121 行把 16:52 的「⏸ 缺失」改判为 `✅ PASS 60/60`（我 2135 checkpoint 的 F-3 已被闭环） |
| A1 `A1-M5-W17-acceptance-closeout-20260908-1230.md` | §3 表：A6 = `PASS_WITH_DEBT（HOLD，无 reopen 项）`；四条跨 lane 发现（F1/F2/F3/O1）登记为**挂账项**而非阻塞项；§102 引用 A6 独占增量 +3,666B |
| A10 `A10-M5-W17-closeout-security-review-20260908-1230.md` | 「A6/A7 完整性波次改动均不新增命令/bridge/ACL/FS/网络特权——无具体安全阻塞」 |
| A8 `A8-M5-W17-acceptance-closeout-20260907-1745.md` | §61-66「**A6 HOLD**… 既有导航（门禁）绿；A8 未重复开发、未重开」 |
| A4 `A4-M5-W17-acceptance-closeout-20260908-1145.md` | D-5 行：未见需要重开 A5/A6/A7 的具体阻塞 |

→ W17 对 A6 已闭环；F-3 已由 A11 自行修正，剩余 **F-1（体积基线脏树采集）、F-2（W17 四个新门禁未接进 pre-merge.sh）、O-1（最近目录路径持久化）** 三项归 A0 决策。

## 2. 自检缺陷 SF-1（Medium-High，功能性）：Esc 无法收起"最近/常用"行

**这是我本波自己引入的回归**，非验收发现，但确为真实缺陷，必须如实登记。

- 位置：`src/components/layout/ActivityBar.vue`
  - L101-107 `onEscape()`：收起扩展行后，把焦点还给 `[data-nav-toggle="<open>"]`
  - L294 地址栏 `<input>` 上带 `data-nav-toggle="omni"`
  - L299 同一 input 上带 `@focus="layout.navSection = 'omni'"`
- 成因链：聚焦地址栏 → 打开 omni 行 → 按 Esc → `closeNavSection()` → `nextTick` 把焦点还给该 input → **focus 事件再次触发 L299 → `navSection = 'omni'` → 行立刻重开**。
- 用户可见后果：键盘/鼠标用户无法用 Esc 关闭"最近网址/最近常用目录"行（其余两个扩展行 grid/more 的触发按钮是 `<button>`，无 focus 副作用，Esc 正常）。
- 为什么门禁没抓到：`check-client-navigation-logic.mjs` 是源码级 + 纯函数级断言，不做 DOM 事件仿真，因此 60/60 仍绿 —— **这是门禁覆盖面的真实缺口，不是"代码没问题"的证据**。

**建议修复（二选一，均未应用）**

```ts
// 方案 A（最小，1 行）：omni 行的触发控件本身已持有焦点，不必再还焦点
function onEscape() {
  const open = layout.navSection;
  if (!open) return;
  layout.closeNavSection();
  if (open === "omni") return; // 焦点仍在地址栏：不再 focus()，避免 @focus 重新打开
  nextTick(() => {
    document.querySelector<HTMLElement>(`[data-nav-toggle="${open}"]`)?.focus();
  });
}
```

```ts
// 方案 B（更稳，覆盖"焦点已移到 chip 上再按 Esc"的情形）：加一次性抑制标志
const omniSuppress = ref(false);
function onAddrFocus() {
  if (omniSuppress.value) return;
  layout.navSection = "omni";
}
function onEscape() {
  const open = layout.navSection;
  if (!open) return;
  layout.closeNavSection();
  if (open === "omni") omniSuppress.value = true;
  nextTick(() => {
    document.querySelector<HTMLElement>(`[data-nav-toggle="${open}"]`)?.focus();
    if (open === "omni") window.setTimeout(() => (omniSuppress.value = false), 0);
  });
}
// 模板：@focus="onAddrFocus()"
```

**本波未修的理由（严格遵守 HOLD）**：board §Closeout L1214 规定 A6「No new work … Re-open only for a concrete acceptance finding」；且我一旦改文件，工作树就不再与已交付的 `Lane-A6-M5-W17-closeout-20260907-2135.patch` 逐字节一致，会给 A0 集成制造歧义。故此处只登记 + 给方案。

**若 BUG-HUNT 把 SF-1 派给 A6**，我会在同文件内 1 行（方案 A）或 6 行（方案 B）修掉，并同步给 `check-client-navigation-logic.mjs` 补两条断言（"Esc 路径对 omni 不重新赋值 navSection"、"onAddrFocus 在抑制标志下不打开"），然后重出二进制补丁。

## 3. 咨询项 SF-2（Low，需 A9/A11 用 axe 口径确认）

地址栏 `<input>`（L294 附近）同时带 `aria-expanded` 与 `aria-controls`，但它是原生 `role=textbox`，不是 `role=combobox`；严格 ARIA 校验器（axe `aria-allowed-attr`）可能对 textbox 上的 `aria-expanded` 报警。可选项：① 维持现状（实践中广泛这样用，读屏可正确播报展开状态）；② 补 `role="combobox"` 使其合规（但那要求配套 listbox/activedescendant，当前扩展行不是 listbox，反而更不合规）。**A6 不自裁**，请 A9/A11 在集成前用既有 axe/lint 规则确认后给结论；如需改动，同属 A6 允许文件，可随 SF-1 一起修。

## 4. 下波（M5-BUG-HUNT）待命状态

- board L8：`Current NEXT: M5-BUG-HUNT confirmed high-risk bug closure and evidence refresh.` —— 但仓库内**尚未出现 BUG-HUNT 的 lane 派发表**（`grep -n "^## M5-BUG-HUNT"` 无命中，仅头部一行）。按 board 规则，A6 **不自派、不自行开新工作**，待 A0 发布派发表后按指派执行。
- 可立即承接的 A6 相关项（优先级建议给 A0）：
  1. **SF-1**（自引功能缺陷，1 行可修 + 2 条门禁断言）
  2. **F-2**（W17 四个新门禁未接进 `pre-merge.sh`：A6 的 `check-client-navigation-logic.mjs`、A3 的 `check-home-store-logic.mjs`、A9 的 `check-home-ui-logic.mjs`、A4 的 `check-home-client-policy.py`）—— 该文件在 W17 属 A9「integration required」范围，A6 可在被指派时提供接线片段（2135 checkpoint §已给）
  3. **F-1**（体积基线脏树采集）—— 基线文件不在 A6 允许范围，只能由 A0/A11 处置；A6 可提供干净 worktree 复测方法
- 当前 A6 允许文件状态：与 16:58 交付态逐字节一致（`diff` 无差异），补丁 `git apply --check --reverse` 通过，随时可集成。

## 5. 复跑证据（本条补录时点）

```
node scripts/check-client-navigation-logic.mjs → CLIENT_NAV_RESULT=PASS (60/60)
node scripts/check-ui-a11y-logic.mjs          → UI_A11Y_RESULT=PASS
node scripts/check-home-ui-logic.mjs          → HOME_UI_RESULT=PASS
node scripts/check-home-store-logic.mjs       → 通过 105 / 失败 0
diff /tmp/a6-w17-backup/*.{ts,vue} 与工作树     → 无差异
git diff --check                               → 干净
git log --oneline -1                           → 052b18a
```

NO_PUSH: confirmed。
