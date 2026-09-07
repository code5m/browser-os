# Lane A9 — M5-W8 插件命令面策略审查 Checkpoint（POLICY REVIEW ONLY）

> 生成：2026-09-07 13:00 CST · Lane A9（M5-W8 · POLICY REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W8 Excluding-A3 Dispatch → A9 交付 Plugin review note（行 198）。W8 硬停：no plugin install/enable/delete/download。
> BASE：`6c1f30e`（HEAD：`feat(M5-W7,A3): read-only MCP registry/policy bridge commands`）

```
LANE=A9
STATUS=PASS (POLICY REVIEW ONLY；零产品代码改动)
BASE=6c1f30e
HEAD=logs/checkpoints/Lane-A9-M5-W8-plugin-surface-review-20260907-1300.patch
FILES=logs/assist/A9-M5-W8-plugin-surface-review-20260907-1300.md,
      logs/checkpoints/Lane-A9-M5-W8-plugin-surface-review-20260907-1300.md
VERIFY=见 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W8-plugin-surface-review-20260907-1300.md
MERGE_NOTES=见下
NEXT=W8+ 运行时 lane (A18/A19)
```

## 审查结论（插件命令面 · W8）

| W8 关注点 | 结果 | 证据 |
|---|---|---|
| ① 无 install/enable/disable/uninstall/delete/download/runtime 命令 | PASS | 全仓扫 `plugin_install/enable/disable/uninstall/delete/download/plugins_dir` 仅文档注释；`default-commands.toml` 0 命中；`plugin.rs` 0 危险构造（`std::process`/`Command::new`/`std::fs`/`unzip`/`#[tauri::command]`/网络） |
| ② 生命周期保持纯 | PASS | `can_transition` 纯 `matches!`；`transition` 无文件 I/O/进程 |
| ③ 能力裁决文本有界/脱敏 | PASS | `permission_preview_for_plugin` 仅输出 `gate`(AclLevel 枚举) + 白名单能力 ID（不含 `reason`/secret/metadata）；`PLUGIN_CAPABILITY_V1` 空集合 fail-closed |
| 策略门禁 | PASS | `check-plugin-policy.py` self-test ALL_PASS / default PASS（1 ACTIVE + 5 PENDING） |

## 非阻塞前向兼容建议（预存在 W6 缝隙，非 W8 回归，未实现）

- **F1（低）**：capability 字符串长度未单独边界（reject 路径 `UnknownCapability(id.clone())` 克隆原串）；当前无 manifest 来源不可达，未来接入未验证 manifest 前建议加 `≤ MAX_TEXT_FIELD_BYTES`。
- **F2（低）**：capability.reason 仅判非空未 `bound_text`；不进裁决、manifest 不持久化，无风险，建议未来展示/持久化路径前补齐。

## Verify（只读）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
grep -rniE "plugin_install|plugin_enable|plugin_disable|plugin_uninstall|plugin_delete|plugin_download|plugins_dir" src-tauri/src/ src/   # 期望：仅文档注释
grep -niE "plugin" src-tauri/permissions/default-commands.toml   # 期望：0 命中
python3 scripts/check-plugin-policy.py --self-test               # 期望：PLUGIN_SELF_TEST=ALL_PASS
python3 scripts/check-plugin-policy.py                           # 期望：PLUGIN_POLICY=PASS
grep -nE "std::process|Command::new|std::fs|unzip|tar::|zip::|#\[tauri::command\]" src-tauri/src/plugin.rs   # 期望：0 命中
cargo test --manifest-path src-tauri/Cargo.toml plugin 2>&1 | tail -3   # 期望：test result: ok. 11 passed
```

## Merge Notes

- **冲突**：无。A9 W8 零产品代码改动（review-only）；未触 W8 他 lane 文件。
- **依赖 A0 动作**：套用 W7 补丁 `logs/checkpoints/Lane-A9-M5-W7-plugin-surface-review-20260907-0900.patch` 以落 `plugin.rs` 的 `PluginCapability` 导入修复（W7 集成回归），否则 mainline `cargo test` 在插件模块失败。本 W8 未重复修复（越权改产品代码）。
- **能力真源**：`PLUGIN_CAPABILITY_V1` 落 security_policy.rs（与 SKILL/AGENT 同文件），MCP 仍在 domain.rs（W4/W6 碎片化待收口，完整 `capability.rs` 交 A0 裁决）。
- 未 push（board Merge Rule：仅 A0 推送）。

## FORBID 遵守（W8）

- 仅新增审查笔记 + checkpoint；**未改任何产品代码**（含 `plugin.rs`）。
- 插件命令面保持零命令、只读；无安装/卸载运行时、无下载/执行、无 secret 持久化、无新依赖。
- 未移动 NEXT；未 push。
