# 16 — Next Steps（按「真实隔离收益 / 风险」排序）

## 下一步该做什么（技术）

1. **Shell 彻底解耦**：把 D-1 的 10 个面板改造成——能力包 + manifest + contribution，让 `MainArea.vue` 零能力 import。
   收益：Shell 不再知道这些能力的存在（§11/§12 全满足）。
2. **Git / Database 升 C3**：二者已有独立 owner（本次告饶的是「未契约化」），是投入产出比最高的下一块积木。
3. **Browser / Terminal HP1**：实现 WebView / PTY 的 graceful 停用 + 释放，并用 `measure-resources.mjs` 在**真实实例**下取 before/after，届时才允许升级到 HP1/C4。
4. **补全业务动作级停用拦截**（D-8）：让「disabled」不仅意味着 UI 消失。
5. **物理多包试点**：先做 Bookmark 的 package 边界（最大的风险是 Vite alias / Tauri 构建链），成功后再推广。

## 下一步该做什么（流程）

- Human GUI Acceptance（未执行）：用 `npm run tauri dev` 手动验证
  framework-only / 加 terminal / 拔 browser / bookmark 热插拔四项。
- ff-only 合并到 master（用户侧执行；本夜未 merge、未 push）。

## 明确不做什么

- 不为了漂亮的 C./HP. 数字放松 checker。
- 不在没有动态加载器/沙箱的情况下宣称 HP3。
- 不在无真实实例时对资源数字做估算（一律 UNKNOWN/DECLARED）。
