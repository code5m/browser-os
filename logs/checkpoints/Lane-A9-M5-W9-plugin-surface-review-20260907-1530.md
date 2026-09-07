# Lane A9 — M5-W9 插件命令面复核 Checkpoint（PLUGIN REVIEW ONLY）

> 生成：2026-09-07 15:30 CST · Lane A9（M5-W9 · PLUGIN REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W9 Runtime-Free Polish Dispatch → A9（行 199）。W9 硬停：no plugin install/enable/delete/download runtime。
> BASE：`ef87401`（HEAD：`docs(M5-W9,A3): MCP policy current-phase kept green + M5-2.b rmcp/server forward card`）

```
LANE=A9
STATUS=PASS (PLUGIN REVIEW ONLY；零产品代码改动；含下一步运行时阻塞点 B1~B9)
BASE=ef87401
HEAD=logs/checkpoints/Lane-A9-M5-W9-plugin-surface-review-20260907-1530.patch
FILES=logs/assist/A9-M5-W9-plugin-surface-review-20260907-1530.md,
      logs/checkpoints/Lane-A9-M5-W9-plugin-surface-review-20260907-1530.md
VERIFY=见 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W9-plugin-surface-review-20260907-1530.md
MERGE_NOTES=见下
NEXT=W9+ 运行时 lane (A18/A19)：须先就位 B1~B9
```

## 复核结论（插件命令面 · W9，W8 之后）

| W9 关注点 | 结果 | 证据 |
|---|---|---|
| ① manifest/lifecycle 保持纯 | PASS | `validate_plugin_manifest`/`can_transition`/`transition` 纯函数；W8 集成 `4d7be97` 未改 plugin 逻辑 |
| ② 真实 install/enable/delete/download 命令仍缺位 | PASS | 全仓扫仅文档注释；`default-commands.toml` 0 命中；`plugin.rs` 0 危险构造 |
| 策略门禁 | PASS | `check-plugin-policy.py` self-test ALL_PASS / default PASS（1 ACTIVE + 5 PENDING） |
| 隐私（CredentialLeak 脱敏） | PASS | `security_policy.rs:118-120` Display `<redacted>`；`cargo test plugin` 11 passed（W7 修复已集成 `4d7be97`） |

## 下一步运行时阻塞点（exact next runtime blockers）

- **B1** 18 只读+启用命令原子包（ACL 末条前/bridge.rs/types.ts/policy/单测）
- **B2** `plugins_dir` 解包沙箱 + 真 Ed25519 验签（当前仅结构）
- **B3** `c.capability` 长度边界（F1，防未验证 manifest 克隆超长串）
- **B4** `reason` 边界（F2，若未来进裁决/持久化/审计）
- **B5** 能力真源收口（PLUGIN_CAPABILITY_V1 vs MCP 在 domain.rs → 统一 `capability.rs`）
- **B6** 错误序列化用 Display 非 Debug（防 `CredentialLeak` 内存载荷泄露）
- **B7** M5-12 UI 消费随 B1 原子交付
- **B8** pre-merge 增 `PLUGIN_RUNTIME_GUARD`/`PLUGIN_MANIFEST_HASH_PINNED`
- **B9** runtime wave 须 A0 显式新 dispatch 开启（W9 硬停仍禁）

## Verify（只读）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
cargo test --manifest-path src-tauri/Cargo.toml plugin 2>&1 | tail -3   # 期望：test result: ok. 11 passed
grep -rniE "plugin_install|plugin_enable|plugin_disable|plugin_uninstall|plugin_delete|plugin_download|plugins_dir" src-tauri/src/ src/   # 期望：仅文档注释
grep -niE "plugin" src-tauri/permissions/default-commands.toml   # 期望：0 命中
python3 scripts/check-plugin-policy.py --self-test               # 期望：PLUGIN_SELF_TEST=ALL_PASS
python3 scripts/check-plugin-policy.py                           # 期望：PLUGIN_POLICY=PASS
grep -nE "std::process|Command::new|std::fs|unzip|tar::|zip::|#\[tauri::command\]" src-tauri/src/plugin.rs   # 期望：0 命中
grep -n "<redacted>" src-tauri/src/security_policy.rs           # 期望：命中 CredentialLeak Display
```

## Merge Notes

- **冲突**：无。A9 W9 零产品代码改动（review-only）；未触 W9 他 lane 文件。
- **W7 修复状态**：已由 A0 在 `4d7be97` 集成，mainline 已绿；本 wave 无需补丁。
- **能力真源**：`PLUGIN_CAPABILITY_V1` 落 security_policy.rs，MCP 在 domain.rs（W4/W6 碎片化待收口，完整 `capability.rs` 交 A0 裁决）。
- 未 push（board Merge Rule：仅 A0 推送）。

## FORBID 遵守（W9）

- 仅新增审查笔记 + checkpoint；**未改任何产品代码**（含 `plugin.rs`）。
- 插件命令面保持零命令、只读；无安装/卸载运行时、无下载/执行、无 secret 持久化、无新依赖。
- 未移动 NEXT；未 push。
