# 观察记录：localhost:5173 pinia 导入报错

- 日期：2026-09-13
- 报告现象：浏览器打开 `http://localhost:5173/`，前端报
  `[plugin:vite:import-analysis] Failed to resolve import "pinia" from "src/main.ts"`

## 归因核实

- `ss -tlnp` 显示 5173 端口归属 pid 2021725
- `/proc/2021725/cwd` → `/home/ainfinit/Documents/Knowledge-Base/secondBrain/SQL语义工程/frontend`
- cmdline：`node .../SQL语义工程/frontend/node_modules/.bin/vite`
- 结论：5173 是**另一个独立项目「SQL语义工程」**的 dev server，与 `mvp-browser-os-v3` 没有任何关系

## 本项目状态

- `mvp-browser-os-v3` 前端跑在 `localhost:1421`（Tauri dev，窗口标题「浏览器OS融合」）
- `npm run build` / `npm run check` / pre-merge 全 PASS，无 pinia 解析错误
- 本仓库 `src/main.ts` 已 grep 确认**不依赖 pinia**（无 `import ... pinia`）

## 结论

该 pinia 报错**不属于**本轮 M6 / 本仓库缺陷，pre-merge 门禁无需覆盖，也不应计入 M6 验收。

## 后续观察

- 若 5173 项目持续报 pinia 错，是其自身 `node_modules` 缺 pinia 或 `main.ts` 引错，属另一项目依赖问题
- 不在本轮范围；如需修复需单独授权并明确项目路径
- 验收时务必区分窗口：本项目窗口标题为「浏览器OS融合」、端口 1421；5173 为 SQL语义工程
