# DESIGN TOKEN AUDIT — Phase UI-0C

> **本阶段只识别，不全局替换。** 一次性替换大量 CSS 会造成视觉漂移，明令禁止。

---

## 0. 核心结论

| 指标 | 结果 |
|---|---|
| CSS 自定义属性（`:root { --* }`） | **0** |
| Design Token 层 | **不存在** |
| 全部硬编码于 `src/styles/global.css`（401 行）+ 49 个 `.vue` 的 `<style>` 块 |
| 重复/近重复色值 | **高**（见 §2） |
| 冲突值 | **有**（`.app-name` 字体大小冲突、两套 modal 遮罩） |

**分类口径**：`CANONICAL` / `CANDIDATE` / `CAPABILITY_SPECIFIC` / `LEGACY` / `UNKNOWN`

---

## 1. 全局色值频次（`global.css`，硬编码 TOP 24）

| 值 | 次数 | 语义推测 | 分类 |
|---|---:|---|---|
| `#fff` | 45 | 表面白 | **CANONICAL**（主表面） |
| `#2b6cb0` | 28 | 主色蓝（primary / active） | **CANONICAL**（primary） |
| `#e5e6eb` | 22 | 边框灰 | **CANONICAL**（border） |
| `#2b6` | 16 | 成功/强调绿 | **CANONICAL**（success） |
| `#ccc` | 13 | 输入边框 | **CANDIDATE**（与 `#e5e6eb` 近重复 → 需裁决） |
| `#4e5969` | 12 | 次级文字 | **CANONICAL**（text-secondary） |
| `#bbb` | 11 | 弱化文字 / 空态 | **CANDIDATE**（与 `#999`/`#86909c`/`#9aa5b5` 冲突） |
| `#c33` | 10 | 危险红 | **CANONICAL**（danger） |
| `#999` | 10 | 弱化文字 | **CANDIDATE** |
| `#666` | 9 | 辅助文字 | **CANDIDATE** |
| `#eef3ff` | 6 | hover 浅蓝 | **CANONICAL**（hover-bg） |
| `#eee` | 5 | 分隔线 | **CANDIDATE** |
| `#ddd` | 5 | 边框（ctx-menu） | **CANDIDATE** |
| `#f4f4f4` | 4 | 分隔线 | **CANDIDATE** |
| `#f0f1f3` | 4 | 分隔线（Git） | **CANDIDATE**（CAPABILITY_SPECIFIC 嫌疑） |
| `#f0f0f0` | 4 | hover 灰 | **CANDIDATE** |
| `#eef0f3` | 4 | 边框（成果/Git） | **CANDIDATE** |
| `#86909c` | 4 | 弱化文字 | **CANDIDATE** |
| `#555` | 4 | 文字 | **CANDIDATE** |
| `#2b3a55` | 4 | 深色标题 | **CANONICAL**（text-strong / 与宫格标题栏同系） |
| `#fff4e5` | 3 | 警告底 | **CANONICAL**（warning-bg） |
| `#ffeaea` | 3 | 危险底 | **CANONICAL**（danger-bg） |
| `#fafbfc` | 3 | 次级表面 | **CANDIDATE** |
| `#f6f6f6` | 3 | 代码块底 | **CANDIDATE** |

---

## 2. 近重复值簇（NEAR-DUPLICATE CLUSTERS）— 需裁决为单一 token

### 2.1 弱化灰（muted text）— 4 个值并存
`#999` / `#bbb` / `#86909c` / `#9aa5b5`
> 分布在空态、meta、图标等处。**裁决建议**：收敛为 1 个 `--text-muted`（但**本阶段不动**）。

### 2.2 分隔线灰 — 5 个值并存
`#eee` / `#f0f0f0` / `#f4f4f4` / `#f6f6f6` / `#f0f1f3` / `#eef0f3`
> **裁决建议**：收敛为 1 个 `--border-subtle`。注意 `#f0f1f3`/`#eef0f3` 集中在 Git/成果面板 → 标记 `CAPABILITY_SPECIFIC` 嫌疑。

### 2.3 边框灰 — 2 个值并存
`#ccc`（输入框）vs `#e5e6eb`（面板边框）
> 可能是**有意区分**（输入控件 vs 容器），**不得盲目合并** → 标记 `UNKNOWN`，需人工确认。

### 2.4 危险/警告底
`#fff4e5`（warn-bg）/ `#ffeaea`（danger-bg）/ `#e6f7ee`（success-bg）/ `#eaf0ff`（info-bg）→ 语义清晰，**CANONICAL**。

---

## 3. 冲突值（CONFLICTING VALUES）— 真实缺陷

| # | 冲突 | 位置 | 说明 |
|---|---|---|---|
| 1 | `.app-name` 字体大小 **13px vs 11px** | `global.css:281` vs `global.css:308` | 同名类重复定义，后者覆盖前者。**真实冲突**，需裁决 |
| 2 | `.path-bar` 重复定义 | `global.css:41` vs `global.css:291` | gap 6px vs 4px，padding 2px 8px vs 3px 8px。**重复选择器** |
| 3 | 两套 modal 遮罩 | `.home-modal-mask`（`rgba(0,0,0,.35)` / `z-index 100`）vs `.modal-mask`（`rgba(0,0,0,.4)` / `z-index 999`） | 透明度与层级均不同 → **视觉不一致的真实缺陷** |
| 4 | `.form` 重复定义 | `global.css:169` vs `global.css:269`（`.repo-panel .form`） | 后者是前者的作用域副本 |

---

## 4. 层级（z-index）盘点

| 值 | 出现 | 用途 | 分类 |
|---|---|---|---|
| `999` | 2 | `.ctx-menu`、`modal-mask` | **CANONICAL**（顶层浮层） |
| `100` | 1 | `.home-modal-mask` | **CANDIDATE**（与 999 冲突 → 见 §3-3） |
| `30` | 1 | `.compact-exit` | **CANONICAL**（精简模式退出钮） |
| `20` | 1 | `.grid-close-layer` | **CANONICAL**（宫格悬浮层，**不得改**） |

---

## 5. position: fixed 盘点（既有 architecture rule）

| 位置 | 元素 | 判定 |
|---|---|---|
| `global.css:78` | `.ctx-menu` | 既有用法，**允许保留**（右键菜单必须 fixed） |
| `global.css:137` | `.home-modal-mask` | 既有用法，允许保留 |
| `global.css:203` | `.modal-mask` | 既有用法，允许保留 |

> **UI-05 门禁**：禁止**新增** `position: fixed` browser overlay。以上 3 处登记为既有基线（baseline），门禁需显式允许，**不得静默洗绿**。

---

## 6. 尺寸 / 间距盘点（magic values）

| 类别 | 现有值 | 分类 |
|---|---|---|
| ActivityBar 高度 | `.activity` `32px` | **CANONICAL** |
| 地址栏高度 | `.addrbar` `34px` | **CANONICAL** |
| 状态栏高度 | 24px（StatusBar） | **CANONICAL** |
| Rail 宽度 | `28px`（WorkbenchRail） | **CANONICAL** |
| Dock 宽度 | `.browser-dock` `360px` | **CANONICAL** |
| 文件树宽度 | `.ftree` `260px` | **CANONICAL** |
| 宫格标题栏高 | `.grid-cell-bar` `26px` | **CANONICAL** |
| 圆角 | `4px` / `5px` / `6px` / `7px` / `8px` / `10px` / `12px` / `13px`(全圆) | **CANDIDATE**（8 种，需收敛） |
| 图标尺寸 | `:size="14"` / `16px` / `22px` / `34px` | **CANDIDATE** |
| 过渡时长 | `.15s` / `.2s` | 少量，暂无冲突 |

---

## 7. 字体盘点

- 字体族：`system-ui, "PingFang SC", sans-serif`（`global.css:3`）｜等宽：`monospace`（多处）
- 字号：10 / 11 / 12 / 13 / 14 / 15 / 16 / 18 / 19 / 22 px — **10 档**，其中 11/12/13 高频
- 字重：400 / 500 / 600 / 700

**分类**：字体族 **CANONICAL**；字号档位 **CANDIDATE**（待收敛，本阶段不动）。

---

## 8. 本阶段动作约束

- ✅ 只识别、只登记、只分类
- ❌ **禁止**全局替换色值
- ❌ **禁止**一次性重写 `global.css`
- ❌ **禁止**在 Pilot 提炼时顺手改色值/间距/圆角（§15 VISUAL_EQUIVALENCE）
- 后续 token 化必须在**独立批次**进行，并配视觉回归证据

## 9. 状态

- `TOKEN_AUDIT = CREATED`
- `DESIGN_TOKEN_DUPLICATION`：**高**（0 token 层，45+ 硬编码色值，5 组近重复簇，4 处真实冲突）
- `UNKNOWN` 项：2（`#ccc` vs `#e5e6eb` 是否有意区分；`.f0f1f3`/`.eef0f3` 是否 CAPABILITY_SPECIFIC）
