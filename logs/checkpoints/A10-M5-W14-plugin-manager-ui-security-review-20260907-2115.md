# A10 · M5-W14 Plugin Manager UI — Security Review Checkpoint

> 检查点：2026-09-07 21:15 CST · Lane A10（SECURITY REVIEW）
> 基线 HEAD `a7eefbb`（ff-only 已最新，W13 已合），工作树含 A1 文档在改（非我任务，留 untouched），未提交未推送。

## VERIFY（启动门禁）
- `WORKSPACE_ID=BACKV3_MAIN` ✅ / 主目录 ✅ / `master` ✅ / `git pull --ff-only` 已最新 ✅
- 工作树 4 个 M 属 A1 W14 文档 reconcile，非 A10 范围，按 M4 模式留 untouched ✅
- 遵守「仅 A0 push」。

## SCOPE
board §M5-W14 Lane A10（row 1188）：review raw-invoke bypass / confirmation bypass / source-ACL drift / sensitive-rendering regressions after A6 output.

## 交付物
- `logs/assist/A10-M5-W14-plugin-manager-ui-security-review-20260907-2115.md`（完整复核）
- 本检查点
- 单补丁：`logs/checkpoints/Lane-A10-M5-W14-plugin-manager-ui-security-review-20260907-2115.patch`

## VERDICT = PASS_WITH_DEBT（后端/契约层 PASS + UI 层 PENDING）

### 已闭环（后端/契约层）
- **F-1 source/ACL 无漂移**：W13 合入 8 命令，每个函数体首行 `check_invocation_source`（bridge.rs:6735/6784/6799/6814/6826/6842/6880/6892）；ACL 8 条全登记于 `list_artifact_images` 前（default-commands.toml:130-137）；读命令不写审计、resource_path 仅取布尔不落盘（bridge.rs:6657-6658,6736-6737）。
- **F-2 契约层 raw-invoke 闭合**：`src/bridge.ts:718-740` 8 方法均经中央 `invoke()` 包装，无裸 `window.__TAURI__.invoke`，命令名/字段映射正确。
- **F-3 敏感渲染在类型层闭合**：`PluginSummary`/`PluginDetail`/`PluginSignatureView`/`TrustedKeyRecord`/`PluginResourceMeta`（types.ts:982-1037）展示面无 signature.value/公钥原文/资源路径，仅 hash_prefix/fingerprint/布尔。

### 未闭环（债 / 待办）
- **F-4（范围缺口·阻断复审）**：A6 W14 UI 产品代码（`src/components/plugin/**`、`usePluginStore.ts`、`check-plugin-ui-logic.mjs`、workspace 导航集成）当前**不在树中**（git status 仅 A1 文档；list_dir ENOENT；grep src/components,src/stores 零 plugin；无 A6 W14 文档/提交）。raw-invoke 旁路 / 确认旁路 / UI 敏感渲染 三项 UI 层判定须 A6 合入后 A10 复审。
- **F-5（前向门禁）**：供 A6 UI wave 的 5 条安全契约（仅经 bridge.ts / 状态变更显式确认 / 仅渲染 redacted 字段 / 命令 DTO 冻结 / UI 逻辑测试+构建 PASS 且不含 Agent/Skill 入口）。
- **F-6（INFO）**：`PluginManifest.signature.value` 与 `metadata?: unknown` 为 install 输入类型，须 UI 自律不渲染/持久化。

## 修复指派（均非 A10 权限）
- F-4：A0 确认 A6 本批补实现；若补，A10 复审（重点 F-5 三项 UI 层）。
- F-5/F-6：纳入 A6 UI wave 的 `check-plugin-ui-logic.mjs` 断言与组件实现要求。

## 红线对照（摘要）
| 硬停 | 判定 |
|---|---|
| 1171 禁裸 Tauri invoke | ✅ 契约层；⏳ UI 层 PENDING |
| 1172 敏感渲染 | ✅ 展示 DTO 已脱敏；⏳ UI 渲染 PENDING |
| 1173 命令/DTO 冻结 | ✅ 8 命令 + 视图类型，无新增 |
| 1172 状态变更显式确认 | ⏳ UI 层 PENDING |
| 仅 A0 push | ✅ |

## 声明
本批零产品代码、零策略脚本改动，未触碰 A1 的 4 个 dirty 文档。未实跑 cargo/npm（W14 无代码新增于本快照；后端 W13 已合且编译通过）。
