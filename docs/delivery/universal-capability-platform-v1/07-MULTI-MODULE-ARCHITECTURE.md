# 07 — Multi-Module 架构裁决（不得写模糊结论）

## 结论

```
MULTI_MODULE_STATUS: PARTIAL
MULTI_MODULE_PILOT:  DEFERRED
```

## 现状（PARTIAL 的依据 = 已做到的部分）

已建立**严格的物理模块边界 v1**：

- 能力必须自带包：`src/capabilities/<id>/{manifest,index,public,contracts,intents,lifecycle,resource,state,ui}`。
- 4 个能力已真正住进自己的包（`bookmark` 还含 `contracts/intents/lifecycle/resource`）。
- 边界由确定性 checker 守护：`check-capability-boundaries` / `check-capability-composition` /
  `check-developer-owners` / `check-terminal-owners` / `check-capability-platform`。
- **依赖只能是 id + public contract**，跨包 internal import 直接 FAIL。

## 为什么 pilot 是 DEFERRED（不是不想做，是今晚不该做）

物理多包（`packages/capability-bookmark` 等 npm workspace）需要同时满足：

1. Vite alias / root 调整为多入口，且 Tauri 构建链（`tauri.conf.json` frontendDist、`beforeDevCommand`）随之变更；
2. 每个包独立依赖声明与版本边界；
3. 现有 12 个 checker 全部按包路径重写；
4. TypeScript path mapping、IDE 与 CI 同步调整。

**判断**：收益（目录更清晰）< 风险（构建链与门禁同时失效，可能掩盖真实回归）。
且本夜目标是「能力是否真的成了积木」，而非「目录是否像 Maven」。故采用 §5 允许的
**PHYSICAL MODULE BOUNDARY V1**：先有严格边界与检查器，再逐个包化。

## 迁移计划（下一步）

1. 先把 D-1 的 10 个面板契约化（解除 Shell 直连）；
2. 再以 **bookmark** 做单包试点：抽出 `packages/capability-bookmark`，验证 Vite+Tauri 构建链；
3. 成功后按相同配方推广，逐个 likely 失败点都可单独回滚。
