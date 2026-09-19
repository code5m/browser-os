# Phase 6B — Semantic Writer Enforcement · ADRs

> 本文件记录 Phase 6B（在 Phase 6A Core Semantic Migration 基础上，把「Owner 唯一」提升为
> 「Owner 唯一 + Writer 唯一 + Checker 可证明」）的架构裁决。
> 真源：Semantic Registry（`docs/architecture/semantic-registry/states.yaml` 的 `canonical_writer` /
> `forbidden_writers` / `owner` / `derived` / `single_owner_required`）。
> 原则（任务 §6）：不为了通过 Checker 加 allow-list；不扩大规则导致误报；不把所有变量都强制 Registry；
> 不修改正确的派生状态。

---

## ADR-SEM-P6B-1 — 受治理状态唯一写者（R9）机器强制

- **状态**：ACCEPTED
- **背景**：Phase 6A 已把 `aiNavOpen` / `gridSession` / 5 个面板开关收敛为「唯一 owner」，但约束
  仍停留在「声明级唯一 owner」（R8）。函数级 writer 约束（Debt-6A-2）尚未机器化——即「谁可以写
  这个状态」仍靠人工审查。
- **决策**：新增 **R9 `SEMANTIC_STATE_WRITER_VIOLATION`**，registry 驱动，区分读取与写入：
  - 仅对 `states.yaml` 中 `single_owner_required === true` 且声明了 `canonical_writer` 的**存储态**强制。
  - **owner 文件内**：写入（`.value =` / `.value +=`）必须发生在 `canonical_writer` 列出的函数体内；
    函数外写入或写在非 canonical 函数 = FAIL。
  - **非 owner 文件**（含组件 / 其它 store / composable）：任何 `.value=` 直写 = FAIL。
  - 真源在 YAML（`canonical_writer` / `forbidden_writers` / `owner`），**不硬编码 allow-list**。
- **读取 vs 写入（防误报）**：写入 = `.value` 后接 `=` / `+=`；读取（`.value` 后非赋值，如 `===`
  比较、取值）不误报。正则 `\.value\s*(?:\+=|=(?![=>]))` 显式排除 `===` / `=>` 误判。
- **函数作用域分析**：用 brace 配对提取函数体区间，写入点取「最内层 enclosing 函数」判定是否属于
  canonical_writer。可证明「谁可以写」。
- **被否决方案**：
  - 方案 B（硬编码 allowed_writers 名单到脚本）：否决——违反 §6「不为了通过 Checker 加 allow-list」，
    真源应只在 YAML。
  - 方案 C（对全部 137 个 store 状态都强制 writer）：否决——扩大治理范围，违反 §13 不处理域，且会
    对 Terminal/Bookmark 等已冻结域产生误报。R9 仅作用于显式 `single_owner_required` 的 7 个状态。
- **后果**：`aiNavOpen` / `gridSession` / `sidebarOpen` / `clipOpen` / `fileEditorOpen` /
  `browserDockOpen` / `browserDockTab` 的 writer 被机器化守护；Debt-6A-2 收口。

---

## ADR-SEM-P6B-2 — Registry Writer Schema 评估（6B-C）

- **状态**：ACCEPTED（结论：无需新增 YAML 字段）
- **背景**：任务 §11 要求评估 `states.yaml` 是否需要新增 `owner` / `canonical_writer` /
  `allowed_writers` / `derived` / `single_owner_required` 字段。
- **评估**：逐项核对现有 `states.yaml`，上述字段**已全部存在且被 R8/R9 消费**：
  - `owner`：✅ 已有（R8 据此定位 owner 文件）。
  - `canonical_writer`：✅ 已有（R9 据此判定合法 writer 函数；`gridSession` 已登记
    `buildGrid` / `forceGridRelayout`）。
  - `forbidden_writers`：✅ 已有（R9 据此理解越权面；`gridSession` 已登记 `components/**` /
    `useLayoutStore` / `useBrowserHost`）。
  - `derived`：✅ 已有（R6 据此守护派生量不存储）。
  - `single_owner_required`：✅ 已有（R8/R9 据此圈定强制范围）。
- **决策**：**不新增 `allowed_writers` 字段**，也不直接扩 YAML。理由：
  - `allowed_writers` 与 `canonical_writer` 语义完全重叠，属冗余（allow-list 倾向，违反 §6）。
  - 既有字段已足以表达「谁能写」，R9 直接消费，符合「改语义改 YAML、不硬改脚本」原则。
  - 因此本阶段**无需走 SCR 修改 Registry schema**（SCR 是针对新增语义，而非既有字段复用）。
- **后果**：Registry schema 零改动；R9 完全由既有 YAML 驱动。`gridSession` 的 `canonical_writer`
  与 `forbidden_writers` 在 Phase 6A 已正确登记，无需补写。
