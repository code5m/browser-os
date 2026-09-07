# Lane A9 — M5-W7 插件命令面审查 Checkpoint（SUPPORT/REVIEW ONLY）

> 生成：2026-09-07 09:00 CST · Lane A9（M5-W7 · SUPPORT/REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W7 → A9 交付 Plugin review note（行 161）。
> BASE：`a26fbaf`（HEAD：`docs(M5): dispatch W7 read-only command bridge lanes`；`5f92ece` 已集成 W6 插件策略切片）

```
LANE=A9
STATUS=PASS (含一项 W6 回归修复)
BASE=a26fbaf
HEAD=logs/checkpoints/Lane-A9-M5-W7-plugin-surface-review-20260907-0900.patch
FILES=src-tauri/src/plugin.rs (W6 回归最小修复),
      logs/assist/A9-M5-W7-plugin-surface-review-20260907-0900.md,
      logs/checkpoints/Lane-A9-M5-W7-plugin-surface-review-20260907-0900.md
VERIFY=见 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W7-plugin-surface-review-20260907-0900.md
MERGE_NOTES=见下
NEXT=W8 运行时 lane (A18/A19)
```

## 审查结论（插件命令面）

| 维度 | 结果 | 证据 |
|---|---|---|
| ACL 插件命令 | 零条目 | `default-commands.toml` 搜 `plugin` → 0 命中 |
| 命令注册 | 无插件系统命令 | `main.rs` 仅 `mod plugin;` + `tauri_plugin_*` 框架依赖 |
| 桥接命令 | 无插件桥接 | `bridge.rs` 20 命中全为 `tauri_plugin_browser_tabs` |
| 安装/启用/停用/卸载运行时 | 无 | 全仓搜 `plugins_dir`/`plugin_install/enable/disable/uninstall` 仅文档注释 |
| 纯策略切片 | 是 | `plugin.rs` 仅 validate/verify/state-machine/permission-preview；无 `std::process`/`std::fs`/`#[tauri::command]` |
| 能力单一真源 | 是 | `PLUGIN_CAPABILITY_V1` 唯一定义于 security_policy.rs；`PLUGIN_CAP_SINGLE_DEF` OK |
| 策略门禁 | PASS | `check-plugin-policy.py` self-test ALL_PASS / default PASS（1 ACTIVE+5 PENDING） |

## 🔴 W6 集成回归（Critical，已最小修复）

- **现象**：`cargo test --manifest-path src-tauri/Cargo.toml plugin` 测试构建 E0422（`PluginCapability` 未找到，`src/plugin.rs:432/440`）。
- **根因**：集成提交 `5f92ece` 后的 `plugin.rs` 第 19 行 `use crate::domain::{...}` 遗漏 `PluginCapability`（W6 终版本含该导入）→ W6「tests PASS」门禁在 mainline 实际被破坏。
- **修复**：补回 `use crate::domain::{AclLevel, PermissionPreview, PluginCapability, PluginManifest, PluginSignature, PluginState};`（最小单行，rustfmt 展开 3 行）。
- **复绿验证**：`cargo test plugin` → **11 passed**；`cargo fmt --check` 干净；`cargo check --locked` 仍 3 条既有告警（非本切片新增）。

## Verify（只读 + 回归修复）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
grep -niE "plugin" src-tauri/permissions/default-commands.toml      # 期望：0 命中
grep -rniE "plugins_dir|plugin_install|plugin_enable|plugin_disable|plugin_uninstall" src-tauri/src/   # 期望：仅文档注释
python3 scripts/check-plugin-policy.py --self-test                 # 期望：PLUGIN_SELF_TEST=ALL_PASS
python3 scripts/check-plugin-policy.py                             # 期望：PLUGIN_POLICY=PASS
cargo test --manifest-path src-tauri/Cargo.toml plugin 2>&1 | tail -3   # 期望：test result: ok. 11 passed
cargo fmt --manifest-path src-tauri/Cargo.toml --check             # 期望：OK
```

## Merge Notes

- **冲突**：无。唯一产品代码改动是 `plugin.rs` 最小单行导入修复（W6 回归），属 A9 自有模块维护，不触 A3/A5 的 W7 桥接代码。
- **强烈建议 A0 套用该单行修复**：否则 `cargo test`/`pre-merge.sh` 将在插件模块失败，违反 W6「tests PASS」承诺。修复已在本 patch 内含。
- **能力真源**：`PLUGIN_CAPABILITY_V1` 落 security_policy.rs（与 SKILL/AGENT 同文件），MCP 仍在 domain.rs（W4/W6 碎片化待收口项，完整 `capability.rs` 收口交 A0 裁决）。
- 未 push（board Merge Rule：仅 A0 推送）。

## FORBID 遵守（W7）

- 仅新增审查笔记 + checkpoint + 一项最小回归修复；**未新增任何 W7 功能代码/命令/ACL**。
- 插件命令面保持零命令、只读；无安装/卸载运行时、无下载/执行、无 secret 持久化、无新依赖。
- 未移动 NEXT；未 push。
