# Lane A9 — M5-W10 插件运行时 Dispatch Card Checkpoint（PLUGIN RUNTIME PLAN ONLY）

> 生成：2026-09-07 17:30 CST · Lane A9（M5-W10 · PLUGIN RUNTIME PLAN ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W10 → A9（行 200）。W10 硬停：Plugin/Agent execution remains locked；only A3 may touch MCP runtime-prep。
> BASE：`3792115`（HEAD：`feat(M5): integrate W9 runtime-free polish`）

```
LANE=A9
STATUS=PASS (PLUGIN RUNTIME PLAN ONLY；零产品代码；交付插件运行时 dispatch 卡片)
BASE=3792115
HEAD=logs/checkpoints/Lane-A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.patch
FILES=logs/assist/A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md,
      logs/checkpoints/Lane-A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md
VERIFY=见 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md
MERGE_NOTES=见下
NEXT=A18/A19 运行时 wave（须 A0 显式新 dispatch 开启；受 HS-1~HS-12 + R-N1~R-N14 + I-N1~I-N7 约束）
```

## 卡片覆盖（W10 A9 要求项）

- **install/enable/delete/list 命令序列**（§2）：核心 4 生命周期命令 + 依赖 14 命令（共 18，原子同包，插 `list_artifact_images` 前 K1）+ install 端到端伪代码。
- **签名失败模式**（§3）：FM-1~FM-10（algorithm/key_id/base64/ISO8601/真验签/hash/版本/变更/公钥越权/绕过），FM-1~4 已被当前 `verify_plugin_signature_structure` 覆盖，FM-5~6 运行时新增。
- **存储上限**（§4）：复用 `MAX_TEXT_FIELD_BYTES`/`MAX_PLUGIN_CAPABILITIES=5`/`HASH_HEX_LEN_SHORT=16` 单一真源；storage.json ≤64KiB、plugin-invokes.json 500 FIFO ≤64KiB。
- **审计脱敏**（§5）：`payload_key_hash` only、`CredentialLeak` Display `<redacted>`、审计/日志/前端只用 Display 非 Debug（W9 B6）、storage_put 拒凭据。
- **UI 依赖**（§6）：M5-12 六组件 + 闸门弹窗复用 + bridge.ts/types.ts + 主题/i18n。
- **Hard Stops**（§7）：HS-1~HS-12（含 W10 本 wave 不实现 HS-11、build≤22% HS-12）。
- **Tests**（§8）：R-N1~R-N14 单测 + 策略脚本扩展（PLUGIN_RUNTIME_GUARD/PLUGIN_MANIFEST_HASH_PINNED）+ I-N1~I-N7 UI/集成。

## Verify（本卡片为 PLAN，无产品代码可跑；验收参照项）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 当前 mainline 插件命令面仍为 0 命令只读切片（卡片规划对象，未实现）
grep -rniE "plugin_install|plugin_enable|plugin_delete|plugin_download|plugins_dir" src-tauri/src/ | grep -v "tests\|doc"   # 期望：仅文档/红线声明
grep -niE "plugin" src-tauri/permissions/default-commands.toml   # 期望：0 命中（W10 未开 runtime）
python3 scripts/check-plugin-policy.py --self-test   # 期望：PLUGIN_SELF_TEST=ALL_PASS（暂不新增 RUNTIME 码）
python3 scripts/check-plugin-policy.py               # 期望：PLUGIN_POLICY=PASS
```

## Merge Notes

- **冲突**：无。本卡片零产品代码（PLAN ONLY）；未触 W10 他 lane 文件（A3 `mcp*`、A7 graph 卡、A1  reconciliation 等）。
- **能力真源**：`PLUGIN_CAPABILITY_V1` 落 security_policy.rs、MCP 在 domain.rs（W4/W6 碎片化待收口，完整 `capability.rs` 交 A0 裁决，见 §0）。
- **开波许可**：插件运行时 wave（A18/A19）须 A0 显式新 dispatch 开启（W10 硬停 HS-11 仍禁）；本卡片为其派发依据。
- 未 push（board Merge Rule：仅 A0 推送）。

## FORBID 遵守（W10）

- 仅新增规划卡片 + checkpoint；**未写任何产品代码**（含 `plugin.rs`/`bridge.rs`/UI/脚本）。
- 未改 ACL/`capability.rs`/`pre-merge.sh`/三份主文档。
- 未移动 `NEXT`；未 push。
