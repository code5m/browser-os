# 01 — Path Coupling Audit（Semantic Governance 与物理路径的耦合）

> Phase 8A.1　人工裁决：DOMAIN STATE OWNERSHIP != CAPABILITY COMPOSITION STATE。
> 物理迁移（Capability Modularization）会把 `src/stores/useXxxStore.ts` 等文件搬走。
> 本文列出：当前哪些治理规则**依赖物理路径**，以及解耦方案。

---

## 1. 现状：identity 被路径污染

`check-semantic-registry.mjs` 与 `check-semantic-closure-logic.mjs` 里，owner **符号**与
**物理路径**被写死绑定。文件一旦移动，下列规则会**静默失守**（不报错，但不再保护范围）。

### 1.1 registry checker（`check-semantic-registry.mjs`）

| 规则 | 行 | 路径耦合点 | 移动后后果 |
|---|---|---|---|
| R2 | `rule2` L169-173 | `governed_files` 精确匹配 `f.path === g` | 搬走的文件不再被 R2 扫描 → 新状态漏登记 |
| R3 | `rule3` L205 | `f.path.includes("useBrowserStore.ts")` 判定 owner | 搬家后 owner 判定失效，组件直调生命周期不再拦截 |
| R3 | `rule3` L226 | `f.path.includes("/stores/")`、`/composables/` 判定前端文件 | capability state 目录不在 `/stores/` → 误判 |
| R8 | `rule8` L367-373 | `OWNER_FILE` 硬编码 `src/stores/useXxxStore.ts` | 搬家后 `ownerFile` 解析不到 → 第二真源失守 |
| R8 | `rule8` L378 | `isStoreFile = p.includes("/stores/")` | capability 内 state 不算 store 文件 → 跳过 |
| R8 | `rule8` L380-388 | `DERIVED_ONLY = ["bmPanelOpen"]` 仅对 `/stores/` 文件生效 | 搬家后派生面板可偷偷存成态 |
| R9 | `rule9` L459-465 | `OWNER_FILE` 硬编码 + `f.path === ownerFile` | 搬家后写者越权失守 |

### 1.2 closure checker（`check-semantic-closure-logic.mjs`）

| 位置 | 行 | 路径耦合点 |
|---|---|---|
| readFileSync | L110-112 | `${ROOT}src/stores/useBrowserStore.ts` 等 3 处字面量 |
| 动态 import | L151-153 | `import(\`${ROOT}src/stores/useBookmarkStore.ts\`)` 等 3 处字面量 |
| MainArea.vue | L112 | `${ROOT}src/components/layout/MainArea.vue` 字面量（shell 文件，暂不迁移） |

> 闭包检查会**真实加载并运行 store**（L151-156），因此文件搬家会直接让 import 抛错 → 27/27 全红。

---

## 2. 解耦方案：Implementation Locator

**原则**：语义 identity（owner 符号，如 `useBookmarkStore`）恒定；
物理路径单独表达于 `states.owner_implementations`，由 resolver 取「首个磁盘存在的候选」。

```yaml
owner_implementations:
  useBookmarkStore:
    symbol: useBookmarkStore
    paths:
      - src/stores/useBookmarkStore.ts            # 当前真值
      - src/capabilities/bookmark/state/useBookmarkStore.ts  # 迁移后真值
```

Resolver 语义：

```text
resolved[owner] = 候选列表中「磁盘存在」的首项
none exists   → SEMANTIC_IMPLEMENTATION_UNRESOLVED   (FAIL)  ← 防静默失守核心
≥2 exist      → SEMANTIC_IMPLEMENTATION_DUPLICATE     (FAIL)  ← 防第二真源
```

### 2.1 各规则改造对照

| 规则 | 改造前 | 改造后 |
|---|---|---|
| R2 治理域 | `governed_files` 字面量 | `governed_files`(非 owner 固定) ∪ 各 owner 解析结果 |
| R3 owner 判定 | `includes("useBrowserStore.ts")` | `f.path === resolved.useBrowserStore` |
| R3 前端判定 | `includes("/stores/")` | 扩至 `/capabilities/*/(state\|services\|intents)` |
| R8 ownerFile | `OWNER_FILE` 字面量 | `resolved[owner]` |
| R8 isStoreFile | `includes("/stores/")` | `isStoreLikeFile`：`/stores/` 或 `/capabilities/*/(state\|services\|intents)/` |
| R9 ownerFile | `OWNER_FILE` 字面量 | `resolved[owner]` |
| closure | 字面量 readFileSync/import | `resolveOwnerFile(owner)` |

---

## 3. 防静默失守闸门（核心验收点）

新增 `check_semantic_registry` 规则 **RI**（Implementation Resolution）：

```text
RI-UNRESOLVED  owner 符号在 candidate paths 中均不存在           FAIL
RI-DUPLICATE    owner 符号解析到 ≥2 份实现                       FAIL
```

禁止：文件搬走 → checker 扫不到 → PASS。
禁止：把路径全部删掉让 identity 虚无化。**路径仍作为 evidence，但不再是 identity 本身。**

---

## 4. 迁移操作纪律（§17 强制）

任意受治理文件迁移必须：

```text
BEFORE:  semantic identity resolves
MOVE:     physical implementation
UPDATE:   owner_implementations（新路径加到候选首项）
AFTER:    semantic identity resolves
NEGATIVE: locator 未更新 → RI-UNRESOLVED 必须 FAIL
```

---

## 5. 影响评估

```text
无语义变更：所有 owner / canonical_writer / forbidden_writers / derived 语义保持不变。
仅变更「checker 如何找到实现文件」。
R2/R3/R8/R9 行为在「文件未移动」时与今日完全一致（候选首项即现路径）。
closure 27/27 在文件未移动时不变（resolver 命中现路径）。
真实移动（8B Bookmark）时：更新 locator → 所有规则继续覆盖新路径。
```
