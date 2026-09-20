# AI Change Surface（普通 Agent 的改动面）

> 目标：改一个能力时，不需要理解其它能力的内部实现。

## 改 Bookmark 需要读的文件

```
src/capabilities/bookmark/manifest.ts     契约：依赖/贡献/资源/hot-plug
src/capabilities/bookmark/public.ts       对外契约
src/capabilities/bookmark/index.ts        注册入口（registerBookmarkContributions）
src/capabilities/bookmark/state/*.ts      业务真源
src/capabilities/bookmark/ui/*.vue        UI（仅包内可见）
```

**不需要读**：browser / terminal / workspace / git / database 的任何内部实现或 store。

## Allowed Dependencies

- 本能力包内的一切。
- 其它能力的 **`public.ts`（public contract）**。
- `src/capability/**`（框架底座：runtime / registry / contract / assembly）。
- Tauri 命令经 `bridge.ts`（基础设施）。

## Forbidden Boundaries（违反即 FAIL）

- import 其它能力的 `state/*`、`ui/*`、`index.ts` 内部符号。
- import 其它能力的 internal composable / store。
- Shell（`App.vue`、`components/layout/**`）import 能力内部实现。
- 复制别人的 state truth（第二真源）。

## Verification Commands

```bash
npm run check
npm run build
node scripts/check-capability-platform.mjs
```

（rename/移动文件后还需跑：`node scripts/check-capability-pilot.mjs`、语义自扫描系列，
因 Semantic Registry 用 locator 解析实现路径。）
