# Lane A6 · M5 BUG-HUNT 剪贴板明文落盘修复（B11-1 · P0）

- 时间：2026-09-07 22:30 CST
- 基线/分支：`master` @ `052b18a`（`git pull --ff-only` → 已经是最新的）；`ls -t` 确认本波 BUG-HUNT 派发表已发布（board §BUG-HUNT Follow-up Dispatch L1231-1253）
- 派发表原文：`A6 | START CODE | src/stores/useSystemStore.ts, src/components/system/ClipboardPanel.vue, focused UI test | Stop persistent plaintext clipboard history by default; keep the in-session UX bounded and explicit. No credential display or browser storage of clipboard bodies.`
- 根因（来自 `logs/bug-hunt/BUG-HUNT-SUMMARY.md`）：B11-1（P0 敏感泄露）`useSystemStore.ts:18/27-31/86` 把剪贴板历史（最多 50 条）`JSON.stringify` 明文落 `localStorage` 跨重启留存；`ClipboardPanel.vue:31/34` 明文渲染；复制的密码/token 任何人可读文件获取。

```text
LANE: A6
STATUS: PASS
SCOPE: src/stores/useSystemStore.ts（改）、src/components/system/ClipboardPanel.vue（改）、scripts/check-clipboard-persistence-logic.mjs（新增）
DELIVERED: 剪贴板历史默认不再持久化（仅会话内存、上限 30 条）；历史条目展示走 redactSecrets 脱敏；面板明确"仅本次会话、不写入磁盘"
VERIFY: node scripts/check-clipboard-persistence-logic.mjs → CLIPBOARD_PERSIST_RESULT=PASS (16/16)
        node scripts/check-client-navigation-logic.mjs → CLIENT_NAV_RESULT=PASS (60/60)（W17 文件无回归）
        npm run build → exit 0；git diff --check → 干净
        隔离 worktree 测本波构建增量 = -126B（净减，远低于 25.2% 上限）
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A6-M5-BUG-HUNT-clipboard-20260907-2230.patch
RISKS: ① 历史现仅会话内保留，重启后清空（这是 B11-1 修复的既定行为，非缺陷）；② 文本框（clipText）仍明文可编辑——属用户当前活动剪贴板的编辑区，不属"历史"，未脱敏（与 board"keep the in-session UX"一致）；③ 纯密码（无 token/URL 形态）可能不被 redactSecrets 命中，沿用应用既有脱敏策略
NO_PUSH: confirmed
```

## 1. 做了什么

### 1.1 `src/stores/useSystemStore.ts`
- 删除 `CLIP_KEY` 常量及 `loadClipHistory`/`saveClipHistory` 中的 `localStorage.getItem/setItem` 调用（B11-1 明文落盘根因）。
- `loadClipHistory()` 改为无副作用空函数：保留签名供 `src/App.vue:80` 现有调用点不变（**未改 App.vue**，避免越出本 Lane 允许文件）。
- `saveClipHistory()` 改为只做内存上限裁剪：`if (clipHistory.length > CLIP_CAP) clipHistory.splice(CLIP_CAP)`；新增 `const CLIP_CAP = 30`，使剪贴板历史"有界"。
- `clipReadSilent` / `clipCopy` / `copyClipItem` / `clearClipHistory` 内部仍调用 `saveClipHistory()`，行为不变（不再落盘）。

### 1.2 `src/components/system/ClipboardPanel.vue`
- 引入 `redactSecrets`（`src/utils/redact.ts`，既有脱敏实现）。
- 历史条目 `:title` 与可见文本均由 `item.text` 改为 `redactSecrets(item.text)`（B11-1 明文展示修复）。
- 提示文案改为：`已保存 {{ system.clipHistory.length }} 条历史（仅本次会话保留，关闭应用后清空，不写入磁盘）` —— 满足"explicit / by default 不持久化"。

### 1.3 `scripts/check-clipboard-persistence-logic.mjs`（新增，16 断言）
加载**真实** `useSystemStore.ts`（pinia 实例，`bridge` 以桩注入避免 Tauri 运行时）与**真实** `redactSecrets`，并对 `ClipboardPanel.vue` 做源码级断言。覆盖：
1. G1 通过公共动作写入历史不写 localStorage；
2. G2 `loadClipHistory` 不再从 localStorage 载入明文，且产品代码不再写 `browser-os-clipboard`；
3. G3 内存上限 = 30；
4. G4 `clearClipHistory` 不写 localStorage；
5. G5 `redactSecrets` 对 URL userinfo / 凭据查询参数 / GitHub token 前缀脱敏；
6. G6 模板不再以原始 `item.text` 渲染、走 `redactSecrets`、含"仅会话/不写磁盘"提示；
7. G7 store 源码无 `localStorage.setItem/getItem(CLIP_KEY …)`、存在 `CLIP_CAP = 30`。

## 2. 验证（命令 → 结果）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
node scripts/check-clipboard-persistence-logic.mjs
# CLIPBOARD_PERSIST_RESULT=PASS (16/16)          exit 0

node scripts/check-client-navigation-logic.mjs  # CLIENT_NAV_RESULT=PASS (60/60)  —— W17 文件无回归
npm run build                                    # ✓ built in 2.44s（exit 0）
git diff --check                                 # 干净
```

隔离 worktree 精确增量（排除 A4/A5/A8 并发改动污染）：
```bash
git worktree add --detach /tmp/a6-bh HEAD && ln -s <repo>/node_modules /tmp/a6-bh/node_modules
cd /tmp/a6-bh && npx vite build && du -sb dist            # 767,059（= 052b18a 干净基线）
cp <A6 两文件> ... && npx vite build && du -sb dist       # 766,933
git worktree remove --force /tmp/a6-bh
# 本波构建增量 DELTA = -126B（净减；测试脚本不进 bundle）
```

## 3. 边界遵守（对照 BUG-HUNT hard stops）
- 仅改本 Lane 允许文件（useSystemStore.ts / ClipboardPanel.vue）+ 新增测试；**未**触碰 `App.vue`、`src-tauri/`（Rust）、`bridge.ts`、其余 lane 文件。
- 未新增 Tauri 命令 / ACL / 文件权限 / 网络权限；未启用任何被锁运行时权限（plugin invoke / 动态加载 / 网络 / daemon / 模型 / Agent-Skill / MCP / graph 写 / 后台 worker）。
- 未改变既有剪贴板功能契约（`clipboardRead/clipboardWrite` 调用保持不变），仅去掉持久化与加展示脱敏。
- 未 push。

## 4. 设计取舍（诚实）
- 采用「默认不持久化 + 内存有界」而非「加密落盘」：board 明确 "No ... browser storage of clipboard bodies"，故不引入任何磁盘写入路径（加密落盘仍属 browser storage，违反红线）。
- 未新增"可选项开启持久化"开关：任何持久化开关都意味着存在明文/密文落盘路径，与 B11-1 修复目标冲突，故不加。
- `clipText` 文本区域仍明文：它是用户当前活动剪贴板的**编辑区**（用户点「复制当前」前在此改写），不属于"历史"，展示明文是功能所需；其敏感面仅存在于会话内存、不落盘、不进历史列表，故未脱敏。

## 5. 集成须知（A0）
- 补丁为 3 文件完整 diff（`git apply` 即可；新脚本已 `git add -N` 故包含在 diff）。
- 建议在 `scripts/pre-merge.sh` 的 UI 逻辑门禁区补一行（参照既有 W17 接线约定，A9/A0 权限）：
  ```
  (cd "$ROOT" && node "$SCRIPT_DIR/check-clipboard-persistence-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-clipboard-persistence-logic.mjs（BUG-HUNT B11-1 剪贴板不落盘）"
  ```
- 与 A4/A10 接缝：本修复消除 `HOME_NO_SECRET_PERSIST`/B11-1 这类"前端明文落盘"面；`redactSecrets` 为既有既有实现，无新增脱敏逻辑。
- 与 W17 A6 文件（ActivityBar.vue / useLayoutStore.ts）零重叠，无集成冲突；W17 门禁 60/60 仍绿。

NO_PUSH: confirmed。
