# Phase 6A — Checker Report

> 扩展既有语义门禁，固化 Phase 6A 的三条收敛结论，防止再生。

## 1. `check-semantic-registry.mjs` — 新增 R8

| 项 | 内容 |
|-|-|
| 规则号 | **R8** |
| 码 | `SEMANTIC_STATE_MULTI_OWNER` / `SEMANTIC_DERIVED_PANEL_STORED` |
| 严重级 | fail |
| 真源 | `states.yaml`（不硬编码语义，规则只实现"如何依据 registry 判定"） |

**判定逻辑**：

1. 对 `states.yaml` 中**显式标记 `single_owner_required: true`** 的存储态（`derived !== true`）且 owner 映射到已知 store 文件者：
   在 owner 文件之外若发现 `const <state> = ref/reactive/shallowRef(` 声明 → `SEMANTIC_STATE_MULTI_OWNER`（第二真源）。
   - owner→文件映射：`useBrowserStore/useLayoutStore/useWorkspaceStore/useBookmarkStore/useSystemStore`；`credential`（Rust 侧）跳过文件判定。
   - **只在 `/stores/` 文件内判定**；**只认存储声明形态（ref/shallowRef/reactive），不含 computed**。
2. `bmPanelOpen` 等**派生面板**若在 store 中被声明为存储态 → `SEMANTIC_DERIVED_PANEL_STORED`。

**为何用 `single_owner_required` 门控（防误报）**：`items` / `busy` / `error` 等 **generic 名**在不同域 store 中合法复用（useBookmarkStore.items ≠ usePluginStore.items），
若对所有治理态按名字强制唯一 owner 会大面积误报。故仅对**显式声明全局唯一**的状态（aiNavOpen / gridSession / 5 个面板开关）强制。
`single_owner_required` 是 registry 真源字段，新增唯一性要求须改 YAML（走 SCR），不在 checker 硬编码。

**覆盖**：
- aiNavOpen duplicate owner（策略 1）✅
- panel duplicate semantic（策略 1，仅对明确登记为 canonical 的状态生效）✅
- gridSession owner violation（策略 1，非 owner 文件声明即拦）✅

**fixtures（positive / negative / false-positive）**：

| 类型 | 内容 | 期望 |
|-|-|-|
| positive | 合法语义（gridOpen 在 owner、派生为 computed、治理域外变量） | 0 fail/warn |
| negative | `useLayoutStore` 声明 `aiNavOpen`；`__fx_bookmark_derived_bad.ts` 声明 `bmPanelOpen=ref`；`__fx_owner_bad.ts` 声明 `gridSession=ref` | 检出 R8 fail |
| false-positive | 派生字段名（对象字段）、注释、治理域外、授权调用 | 0 fail/warn |

自检结果：`SELF_TEST_RESULT=ALL_PASS`（9 组 fixture 全通过，含 R8）。

## 2. `check-semantic-closure-logic.mjs` — 新增功能测试

新增独立脚本（既有 `.mjs` checker 约定，非 vitest）：静态断言 + 真实 store（pinia）功能断言。

- 用法：`node scripts/check-semantic-closure-logic.mjs`
- 覆盖：见 `TEST-REPORT.md`。

## 3. pre-merge 接线

`scripts/pre-merge.sh` 的 Phase 03 checker gate 循环追加 `check-semantic-closure-logic`：

```
for c in ... check-semantic-registry check-semantic-closure-logic check-sensitive-side-effects doctor; do
```

（`check-semantic-registry.mjs` 原已在 gate 中，R8 自动随其生效。）

## 4. 未制造脆弱规则

- 不检测"写函数"级别（易误报），只检测**声明级唯一 owner**（稳定、与 registry 对齐）。
- 不新增 allow-list 绕过；R8 对 registry 驱动，语义变更须改 YAML（走 SCR）。
