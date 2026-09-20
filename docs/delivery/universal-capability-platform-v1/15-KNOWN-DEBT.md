# 15 — Known Debt（诚实登记，逐项可追溯）

| ID | 债务 | 位置 | 为什么现在不还 | 分类 |
|---|---|---|---|---|
| D-1 | `MainArea.vue` 仍直接 import 约 10 个面板：`HomePanel/VaultPanel/ClipboardPanel/AppPanel/SettingsPanel/ToolBox` + 懒加载 `TaskPanel/DatabasePanel/SkillManagerPanel/AgentManagerPanel/GraphPanel/PluginManager` | `src/components/layout/MainArea.vue` | 迁移需连同其 owner store 一并物理搬迁，风险 > 本夜收益 | PRE_EXISTING + 平台缺口 |
| D-2 | `git` / `database` 仅有 owner 物理分离（C1），未进 catalog | `src/capabilities/../stores` | 需要 manifest + contribution 化 | 未完成项 |
| D-3 | `useBrowserHost` 属 browser 能力却位于 `src/composables/`（全局） | `src/composables/useBrowserHost.ts` | 物理归位待 browser 包二次收拢 | 物理边界债 |
| D-4 | 各 `utils/*Ui.ts`（dbUi/taskUi/graphUi/pluginUi/agentSkillUi/snippetUi/scriptUi）为能力业务逻辑但不在能力包内 | `src/utils/` | 同上 | 物理边界债 |
| D-5 | `notes` 能力前端无 owner，仅残留 backend 命令 `save_note`（在既有 known drift 列表） | Rust / collect.js | 既有 drift，按 KNOWN 规则保留，不擅自删除命令 | PRE_EXISTING |
| D-6 | `skill_*` / `agent_*` 命令存在 main invoke 与 backend 注册不一致 | 既有 known drift | 属于既有登记 drift（GATE 判定 only known drift） | PRE_EXISTING |
| D-7 | 进程级资源测量（WebKit 子进程数 / PTY 数）今晚无实例 | `scripts/measure-resources.mjs` | 需要运行中的应用实例；headless 下如实记 UNKNOWN | 测量口径 |
| D-8 | 能力的「业务动作级」停用拦截（store 层拒绝调用）尚未全面接入 | 各能力 store | Bookmark 无后台行为故不受影响；带资源能力需逐个补 | 未完成项 |
| D-9 | 物理多包（npm workspace）未迁移 | 仓库根 | 见 `07-MULTI-MODULE-ARCHITECTURE.md` | DEFERRED |

## 分类口径（§42）
- `PRE_EXISTING`：本系列之前就存在。
- `NEW_REGRESSION = 0`：今晚新增回归为 **0**（`npm run check` 全门通过）。
- 未出现「为通过而放宽/删除 checker / 加 blanket allow-list」的情况。
