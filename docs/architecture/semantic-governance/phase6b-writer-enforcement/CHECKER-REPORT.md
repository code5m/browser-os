# Phase 6B — CHECKER REPORT（R9）

> 新增规则：**R9 `SEMANTIC_STATE_WRITER_VIOLATION`**，位于 `scripts/check-semantic-registry.mjs`。

## 1. 触发条件（registry 驱动）

仅对 `states.yaml` 中满足以下全部条件的状态强制：

```yaml
someState:
  single_owner_required: true      # 显式要求唯一 owner（Phase 6B 范围信号）
  derived: false                   # 仅存储态；派生态由 R6 守护
  canonical_writer:                # 声明了合法 writer 函数（否则跳过，避免误报）
    - useBrowserStore.buildGrid
  owner: useBrowserStore           # 可映射为已知 store 文件
  forbidden_writers:               # 越权面（仅文档/可选，R9 主判据为 canonical_writer）
    - components/**
    - useLayoutStore
```

本阶段实际生效的 7 个状态：`aiNavOpen` / `gridSession`（owner=useBrowserStore）、
`sidebarOpen` / `clipOpen` / `fileEditorOpen` / `browserDockOpen` / `browserDockTab`
（owner=useLayoutStore）。

## 2. 检测逻辑

- **owner 文件内**：用 brace 配对提取函数体区间，定位每个写入点的「最内层 enclosing 函数」。
  - 写入点无 enclosing 函数（顶层）→ FAIL（应位于 canonical_writer 函数内）。
  - enclosing 函数 ∉ `canonical_writer` → FAIL。
- **非 owner 文件**（任何非 owner 的 .ts/.vue，含组件 / 其它 store / composable）→ 任何
  `.value=` 直写 → FAIL。

## 3. 读取 vs 写入（防误报，核心）

写入正则：`\b<NAME>\.value\s*(?:\+=|=(?![=>]))`

- 仅匹配 `.value +=` 与 `.value =`（其后非 `=` / `>`）。
- 显式排除 `===` / `===` / `=>`（比较 / 箭头），避免把比较或箭头参数误判为赋值。
- 读取（`const x = gridSession.value`、`if (gridSession.value === 0)`）不触发。

## 4. 不误报 / 不扩大范围

- 真源在 YAML，脚本零硬编码状态名/函数名 → 不违反 §6「不为了通过 Checker 加 allow-list」。
- 仅作用于 `single_owner_required === true` 的 7 个状态；Terminal/Bookmark/credential 等无此标志，
  自动跳过 → 不违反 §13 不处理域。
- `derived === true` 状态跳过（R6 已守护）→ 不修改正确派生状态。

## 5. 自检结果（--self-test）

| 夹具类型 | 内容 | 期望 | 实际 |
|-|-|-|-|
| Positive | owner 文件内 `buildGrid`/`forceGridRelayout` 写 `gridSession` | 0 fail/warn | ✓ 0 |
| Negative R9a | `useLayoutStore.ts`（`strayGridSession`）跨域直写 `gridSession.value` | fail | ✓ 检出 |
| Negative R9b | 组件 `BadGrid.vue` 直写 `browser.gridSession.value += 1` | fail | ✓ 检出 |
| Negative R9c | owner 文件内非 canonical 函数 `rogueWriter` 写 `gridSession.value` | fail | ✓ 检出 |
| False-Positive | `const x = gridSession.value` / `if (gridSession.value === 0)` | 0 fail/warn | ✓ 0 |

```text
SELF_TEST_RESULT=ALL_PASS
```

## 6. 真实扫描结果

```text
files scanned: 104 | rules: 9
fail=0  warn=6（pre-existing R5 副作用认知，非本阶段）  info=72（observed_not_governed）
SEMANTIC_REGISTRY_RESULT=PASS
```
