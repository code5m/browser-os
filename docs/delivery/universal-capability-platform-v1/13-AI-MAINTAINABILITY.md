# 13 — AI Maintainability

目标：一个**普通 AI Agent** 改某个能力时，只需要读这个能力自己的包 + public contract + 门禁，
不需要理解其它能力的内部实现。

## 改动面（以 bookmark 为例）

需要读：

```
src/capabilities/bookmark/manifest.ts     # 依赖/贡献/资源/hot-plug 都在这里
src/capabilities/bookmark/public.ts       # 对外契约（别人只能用它）
src/capabilities/bookmark/state/*.ts      # 该能力的业务真源
src/capabilities/bookmark/ui/*.vue        # 该能力的 UI（只在包内被引用）
src/capabilities/bookmark/index.ts        # 贡献注册入口
```

**不需要读**：browser / terminal / workspace 的任何内部文件或 store。

## 边界

- 允许：`capability A → capability B 的 public contract`。
- 禁止：跨能力 import internal store / component / composable；Shell import 能力内部实现。
- 由这些门禁机械验证：`check-capability-boundaries` / `check-capability-composition` /
  `check-developer-owners` / `check-terminal-owners` / `check-capability-platform`。

## 验证命令（改完必须跑）

```bash
npm run check          # 含全部既有门禁 + 平台门禁
npm run build
node scripts/capability-demo.mjs inspect bookmark
```

## 一句话判据

改 bookmark 时如果你被迫去读 terminal 的内部实现，那就是设计失败，应视为平台缺陷上报。
