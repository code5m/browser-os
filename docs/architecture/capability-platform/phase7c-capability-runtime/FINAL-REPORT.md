# Phase 7C — Minimal Capability Runtime（终稿）

> 日期：2026-09-19　分支：`feature/capability-platform-v1`
> 前置：`capability-phase7a-architecture-pass` / `capability-phase7b-contract-pass`

---

## 1. 交付物

```text
src/capability/runtime.ts                 最小 Runtime 实现（编排，非业务 Owner）
src/capability/types.ts                   最小 SDK 类型（7B 已交付）
scripts/check-capability-runtime.mjs      行为门禁 RT-01..RT-15（esbuild 转译真实 TS 后测试）
```

---

## 2. Runtime 能力（仅限编排）

| 操作 | 行为 | 拒绝条件 |
|---|---|---|
| `register(def)` | 登记 manifest，state=`DEFINED` | id 重复 → `DUPLICATE_ID`；定义非法 → `INVALID_DEFINITION` |
| `resolve(id)` | 校验强依赖齐全，state=`DEFINED→READY` | 强依赖未注册 → `MISSING_DEPENDENCY`（可选依赖缺失放行） |
| `activate(id)` | 调 `onActivate`，state=`ACTIVE` | 未 resolve / 已停用 → `INVALID_TRANSITION`；`status != COMPATIBILITY_WRAPPED` 或 `activatable != true` → `NOT_ACTIVATABLE` |
| `suspend(id)` | 调 `onSuspend`，state=`SUSPENDED` | 非 ACTIVE → `INVALID_TRANSITION`；未声明支持 suspend → `SUSPEND_NOT_SUPPORTED` |
| `enable(id)` / `disable(id)` | 切换 enabled | `disable` 仅允许"已治理 + 非常驻"能力，否则 `UNSAFE_OPERATION`；ACTIVE 态须先 suspend |
| `inspect()` | 返回编排元数据（id/name/category/state/enabled/governanceStatus/status/resourceClass/resident） | — |

**关键拒绝规则**：`activate` 要求 `status === 'COMPATIBILITY_WRAPPED'`。
这意味着 **NOT_INTEGRATED / TARGET_COMPOSABLE 的能力无法被激活** —— 从机制上杜绝"把目标态当成已实现"。

---

## 3. 边界确认（Runtime 不是 Owner）

```text
Runtime 只持有：id / definition / state / enabled   ← 编排元数据
Runtime 不持有：任何业务状态（tabs / termPanes / items / credential / gridSession ...）
```

由 RT-13 静态断言强制：Runtime 源码中不得出现 `useBrowserStore` / `useSystemStore` / `termPanes` / `gridSession` / `items.value` 等业务引用。

业务真源仍在 Semantic Registry 登记的 owner 手中（`useBrowserStore` / `useSystemStore` / `useWorkspaceStore` / `useBookmarkStore` / `KeyringStore` ...）。

**未引入**：DI 容器、反射加载、动态 `require`、God Runtime。

---

## 4. 测试证据

```text
CAPABILITY_RUNTIME_RESULT=PASS (15/15)
  RT-01 register → DEFINED              RT-09 suspend 未支持 → SUSPEND_NOT_SUPPORTED
  RT-02 重复注册 → DUPLICATE_ID         RT-10 disable 常驻 → UNSAFE_OPERATION
  RT-03 resolve → READY                 RT-11 disable ACTIVE → INVALID_TRANSITION
  RT-04 依赖缺失 → MISSING_DEPENDENCY   RT-12 inspect 仅元数据
  RT-05 activate → ACTIVE               RT-13 源码无业务状态引用
  RT-06 未声明可激活 → NOT_ACTIVATABLE  RT-14 未注册 → NOT_FOUND
  RT-07 未 resolve → INVALID_TRANSITION RT-15 SUSPENDED 后可再 activate
  RT-08 suspend → SUSPENDED
```

测试**加载真实 `src/capability/runtime.ts`**（经 esbuild 转译），不是复制逻辑另测。

---

## 5. 未做（诚实）

| 项 | 说明 |
|---|---|
| 物理启停 | Runtime **不做**真正的模块卸载/重载；它只驱动已声明的 lifecycle 钩子 |
| destroy / hibernate | 未实现（无能力声明支持，且涉及真实资源释放，风险高） |
| 依赖自动装配顺序 | 仅校验依赖存在，不做拓扑排序激活（当前无必要） |
| 事件总线 | 未引入；能力间不做直接互相调用（只能经 `provides` 声明） |

---

## 6. 已登记债务

| ID | 内容 |
|---|---|
| Debt-7C-1 | Runtime 不做物理卸载，`destroy`/`hibernate` 未实现（TARGET） |
| Debt-7C-2 | 无能力间通信机制（事件总线未引入），当前只能单向 `provides` 声明 |
| Debt-7C-3 | `inspect()` 未包含资源实测（依赖 Debt-7A-1 的解决） |

---

## 7. 结论

```text
PHASE_7C_RESULT: PASS（Runtime 最小实现 + 15/15 行为断言，零业务状态所有权）
```

下一步：**Phase 7D — Pilot Capability Integration（Bookmark）**。
