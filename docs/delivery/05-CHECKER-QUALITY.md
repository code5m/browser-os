# 05 · Checker 质量与误报记录

## 1. 规则清单（R1–R9）

| 规则 | 名称 | 检测 |
|-|-|-|
| R1 | Duplicate State | 同名受治理状态重复声明 |
| R2 | Unregistered State | 业务代码 `ref/reactive/computed` 但 Registry 未登记 |
| R3 | Owner Violation | 非 owner 文件声明受治理状态 |
| R4 | Duplicate Intent | 同义意图入口（intents.yaml duplicate_names）|
| R5 | Side Effect Unknown | 调用未声明副作用认知的函数 |
| R6 | Derived State Stored | 派生态被存为独立真源 |
| R7 | Credential Sensitive Input | 凭据敏感输入泄露（密码回前端）|
| R8 | Semantic State Multi Owner | 受治理状态在 owner 文件之外被声明（第二真源）|
| R9 | Semantic State Writer Violation | 写入点越权（非 owner 直写 / owner 内非 canonical 函数）|

## 2. 自测质量

每个 Checker 自带 `--self-test` 夹具，覆盖三类：

- **Positive**：合法写法不报错（如 `useBrowserStore.buildGrid` 写 `gridSession`）。
- **Negative**：违规写法必被捕获（跨 store 直写、组件直写、owner 内非 canonical 函数写入）。
- **False-Positive**：读取不误报（如 `const x = gridSession.value`、`if (gridSession.value === 0)`）。

```text
SELF_TEST_RESULT=ALL_PASS（R1..R9 全部夹具通过）
```

## 3. 误报风险（已记录，不夸大）

| 风险 | 说明 | 处置 |
|-|-|-|
| R9 读取 vs 写入 | `.value` 后接 `===` / `=>` 是读取/比较，非赋值 | 正则显式排除 `===` / `=>`，夹具验证不误报 |
| **Debt-6B-1** | R9 brace 配对不识别"无参 parenless 箭头"(`const f = x => {}`) 的 enclosing | 仅影响极少数写法；真实 writer 均为 `function NAME()`，未触发误报 |
| **Debt-6A-3** | R8 仅识别 `const X = ref/reactive`，解构/动态声明盲区 | 与 R2 同源；登记于 Known-Debt，不静默消失 |

## 4. 设计纪律（防误报 / 防滥用）

- **不硬编码 allow-list**：R9 完全读 `states.yaml` 的 `canonical_writer` / `forbidden_writers` / `owner`，
  脚本零硬编码状态名/函数名（违反任务"为过 Checker 加 allow-list"禁令）。
- **不扩大范围**：R9 仅作用于 `single_owner_required === true` 的 7 个状态，Terminal/Bookmark/credential 自动跳过。
- **不修改正确派生态**：R9 仅处理 `derived !== true` 的存储态。

> Checker 通过"真源在 Registry + 自带负向/误报夹具"保证质量，而非靠放宽断言刷绿。
