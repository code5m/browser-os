# Lane A9 — M5-W6 整包交付（插件 manifest/生命周期策略切片，START PRODUCT CODE）Checkpoint

> 生成：2026-09-06 23:30 CST · Lane A9（M5-W6 · START PRODUCT CODE · 整包交付）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W6 Parallel Dispatch → A9：Implement M5-10/M5-11 plugin manifest/lifecycle policy slice.
> BASE：`77b1e3e`（HEAD：`docs(M5): dispatch W6 graph UI and plugin lanes`；已 git pull --ff-only 同步）

```
LANE=A9
STATUS=PASS
BASE=77b1e3e (HEAD: docs(M5): dispatch W6 graph UI and plugin lanes; 已 git pull --ff-only 同步 origin/master)
HEAD=logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.patch
FILES=src-tauri/src/domain.rs, src-tauri/src/plugin.rs, src-tauri/src/security_policy.rs,
      src-tauri/src/main.rs, scripts/check-plugin-policy.py, scripts/pre-merge.sh,
      logs/assist/A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md,
      logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md
VERIFY=见 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md
MERGE_NOTES=见下
NEXT=A18 (M5-10/11 运行时) / A19 (M5-12 UI)
```

## 本切片交付（A9 M5-W6 整包，策略切片产品代码）

| 要求（M5-W6 A9） | 落点 |
|---|---|
| DTOs（manifest/entry/capability/signature/state） | `domain.rs` 追加 `PluginManifest`/`PluginEntry`/`PluginCapability`/`PluginSignature`/`PluginState` |
| validation（纯函数） | `plugin.rs`：`validate_plugin_manifest`（schema+凭据+形态③+白名单+哈希/签名结构）、`verify_plugin_signature_structure`（仅结构） |
| lifecycle state machine | `plugin.rs`：`can_transition`/`transition`（8 态，无文件 I/O，fail-closed） |
| permission manifest rules | `plugin.rs`：`permission_preview_for_plugin` 复用 A5 `PermissionPreview`；`capability_acl_level` 风险档映射 |
| policy script | `scripts/check-plugin-policy.py`（1 ACTIVE `PLUGIN_CAP_SINGLE_DEF` + 5 PENDING）；接入 `pre-merge.sh` |
| 无安装/卸载文件突变运行时 / 无下载插件 / 无真签名加密（除非全本地） | 纯逻辑切片；真 Ed25519 留运行时 lane；未引加密 crate |
| bounded metadata / no secrets / no network/install runtime | `MAX_TEXT_FIELD_BYTES` 边界 + `contains_credential_leak`；无 `std::fs` 解包、无联网、无命令 |

## Verify（编译 + 单测 + 策略门禁 + 只读）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check      # 期望：通过
cargo check --manifest-path src-tauri/Cargo.toml --locked 2>&1 | tail -3   # 期望：仅他 lane 既有告警，Finished
cargo test --manifest-path src-tauri/Cargo.toml plugin 2>&1 | grep "test result"   # 期望：11 passed
python3 scripts/check-plugin-policy.py --self-test               # 期望：PLUGIN_SELF_TEST=ALL_PASS (ACTIVE=1 PENDING=5)
python3 scripts/check-plugin-policy.py                           # 期望：PLUGIN_POLICY=PASS
bash -n scripts/pre-merge.sh                                      # 期望：OK
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：118（K1 末条未变）
```

结果：fmt 通过；check 仅 3 条他 lane 告警（本切片 `#![allow(dead_code)]` 未新增）；**11/11 plugin 单测通过**；策略 self-test/default PASS；K1 末条未变；pre-merge.sh 语法 OK。

## Merge Notes

- **冲突**：无。仅 W6 允许文件；与 A1 卡片 reconcile / A8 graph UI / A2/A4/A5/A7/A10 评审无交集。`domain.rs` 追加在 graph 常量后，`security_policy.rs` 仅增 plugin 段，`main.rs` 仅加 `mod plugin;`。
- **能力真源**：`PLUGIN_CAPABILITY_V1` 落 security_policy.rs（与 SKILL/AGENT 同文件），MCP 仍在 domain.rs——W4 §4 碎片化待收口项；完整 `capability.rs` 统一交 A0 裁决。`PLUGIN_CAP_SINGLE_DEF` 守唯一定义。
- **与 W4/W5 衔接**：设计已对齐 A5 策略 + A6 UI（权限预览复用 `PermissionPreview`）+ A7 graph；本切片将其落地为可编译/可测/受门禁守护的切片。
- **对 A18/A19**：运行时在其上接 plugins_dir 解包、真 Ed25519 验签（信任根 `keys/trusted-pubkeys.json` 经 KeyringStore，fail-closed）、18 命令（每加必过 source/ACL/bridge/策略/测试）、`PluginState` 经 `transition` 驱动、`permission_preview_for_plugin` 供 `PermissionPreviewModal.vue`。

## FORBID 遵守

- 仅改/新增 W6 允许 6 文件；未触 bridge.ts/types.ts/graph UI/卡片/M5-10/11 卡。
- 无安装/卸载运行时、无下载/执行/启用真实插件、无新增命令（K1 末条未变）、无 secret 持久化、无新依赖。
- 未移动 NEXT；未提交、未 push（board Merge Rule：仅 A0 可推送）。
