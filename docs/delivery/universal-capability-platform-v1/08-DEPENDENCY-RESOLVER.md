# 08 — Dependency Resolver

实现：`src/capability/platform/assembly.ts`（纯函数，deterministic）。

## 规则

| 情形 | 行为 |
|---|---|
| required 依赖在 catalog | 自动补入并排到前面（拓扑序） |
| required 依赖不存在 | **REJECT** `MISSING_REQUIRED_DEPENDENCY` |
| optional 依赖不存在/未装配 | **合法降级**，输出 warning（`... 以降级方式运行`） |
| 依赖成环 | **REJECT** `DEPENDENCY_CYCLE` 并给出环路径 |
| 两能力互相 `conflicts` | **REJECT** `CAPABILITY_CONFLICT` |
| 请求了不存在的 id | **REJECT** `UNKNOWN_CAPABILITY` |

关键：所有拒绝都发生在**启动之前**——不允许「跑起来以后才 undefined / crash」。

## Determinism

- 拓扑排序（Kahn）+ **id 字典序 tie-break**，因此 `resolve workspace,browser` 与 `resolve browser,workspace`
  输出完全一致（PLT2-03 逐字段比对）。
- 相同输入的 JSON 快照必须逐字节相同。

## 负例（都是真跑出来的）

```bash
node scripts/check-capability-platform.mjs     # PLT2-04 ~ PLT2-07
node scripts/capability-demo.mjs use invalid-missing-dependency
# → RESULT REJECTED / REJECTIONS UNKNOWN_CAPABILITY: 请求的能力不存在于 catalog: not-exist
```

## 依赖必须是显式声明

跨能力 direct import 由既有门禁（`check-capability-boundaries` 等）静态禁止，
resolver 只处理**显式写在 manifest 里的**依赖，杜绝 hidden runtime lookup / God Service Locator。
