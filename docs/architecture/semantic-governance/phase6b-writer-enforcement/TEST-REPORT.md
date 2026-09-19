# Phase 6B — TEST REPORT

## 1. 测试矩阵（任务 §12）

| 用例 | 类型 | 输入 | 期望 | 实际 |
|-|-|-|-|-|
| gridSession 合法写入 | Positive | `useBrowserStore.buildGrid` 写 `gridSession.value` | PASS | ✓ |
| aiNavOpen 合法写入 | Positive | `useBrowserStore.toggleAiNav`/`gotoAI` 写 `aiNavOpen.value` | PASS | ✓（real scan 0 fail）|
| 跨 store 直写 | Negative | fixture `useLayoutStore.ts` 写 `gridSession.value = 1` | FAIL | ✓ 检出 |
| 组件直写 | Negative | fixture `BadGrid.vue` 写 `browser.gridSession.value += 1` | FAIL | ✓ 检出 |
| owner 内非 canonical 写 | Negative | fixture `useBrowserStore.ts` 内 `rogueWriter()` 写 `gridSession.value` | FAIL | ✓ 检出 |
| 读取不误报 | False-Positive | `const x = gridSession.value` | 不报 | ✓ 0 fail |
| 比较不误报 | False-Positive | `if (gridSession.value === 0)` | 不报 | ✓ 0 fail |
| 函数级 writer 唯一 | Positive（runtime） | 实扫 `gridSession` 两处写入均在 buildGrid/forceGridRelayout 内 | PASS | ✓（27/27）|

## 2. 静态 + 功能（check-semantic-closure-logic.mjs）

```
[static] gridSession 唯一 owner
  ok  gridSession 在 useBrowserStore 中恰好声明 1 次
  ok  gridSession 仅由 buildGrid/forceGridRelayout 写入（2 处）
  ok  gridSession 两处写入均在 canonical_writer 函数（buildGrid / forceGridRelayout）内（Writer 唯一）
  ok  gridSession 不入 localStorage（非持久化）
[runtime] 加载真实 store 并验证行为
  ok  gridSession 初始为 0
  ok  forceGridRelayout 使 gridSession 自增 +1（缓存失效纪元）
  ok  gridSession 不落 localStorage（内存 runtime）
...
SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (27/27)
```

## 3. 门禁汇总

```text
node scripts/check-semantic-registry.mjs --self-test   → SELF_TEST_RESULT=ALL_PASS（R1..R9）
node scripts/check-semantic-registry.mjs                → SEMANTIC_REGISTRY_RESULT=PASS（fail=0）
node scripts/check-semantic-closure-logic.mjs           → SEMANTIC_CLOSURE_LOGIC_RESULT=PASS (27/27)
bash scripts/pre-merge.sh --self-test                   → Phase 03 gate 含 check-semantic-registry / check-semantic-closure-logic（未新增失败；terminal/check-terminal-ui-logic 既有债与本层无关）
```

## 4. 回归确认

- R1..R8 未被削弱（self-test 仍 ALL_PASS；real scan fail 数与 Phase 6A 一致 = 0）。
- 未修改任何业务代码（`src/` / `src-tauri/`）；仅扩展 checker 脚本与新增文档。
- `git diff --check` 干净；`node --check` 两脚本语法 OK。
