# A9 · M5-W13 Plugin Runtime Stage-II Staged Card — Checkpoint

> Lane：A9（PLUGIN DOCS/POLICY ONLY）
> Wave：M5-W13（Plugin Runtime Staged Dispatch · Stage-II delete/storage/audit/invoke/upgrade/rollback + UI 收口）
> 时间：2026-09-07 20:00 CST
> 性质：**规划卡片 checkpoint · 无产品代码改动 · 未 push**

## 1. 交付物

- 卡片：`logs/assist/A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md`
  - 10 命令（`plugin_delete`/`plugin_uninstall` 2 + `plugin_storage_get/put/delete` 3 + `plugin_audit_list` 1 + `plugin_permissions_get/set` 2 + `plugin_invoke`/`plugin_invoke_cancel` 2）+ `plugin_rollback` 1（**A0 拍 18 vs 19**）= 11（或 10）命令
  - 端到端执行序列（invoke 7 步判定 + 升级 7 步 + 回滚 6 步伪代码）
  - 关键契约（两层隔离 + invoke 7 步 + 升级/回滚 + 审计 + storage）
  - 存储上限（8 项含 `plugin_storage.json` 64KiB / `plugin-invokes.json` 500 FIFO / `manifest.previous.json` 32KiB）
  - 审计脱敏（`plugin-invokes.json` shape + `audit.json` 摘要 + `CredentialLeak` Display）
  - UI 全量收口（10 文件清单 + `PluginDetail.vue` / `PluginHistoryModal.vue` / `PluginStorageEditor.vue` / `PluginUpgradePanel.vue` W13 必落）
  - 19 项 hard stops（含 HS-13~HS-19 严守 W13 切分 + W12 已落约束继承）
  - 22 项 Rust 单测 + 8 新增 ACTIVE 政策码 + 10 项 UI 集成测试

## 2. 与 W10 计划的对应

| W10 § | 主题 | W13 落点 |
|---|---|---|
| §1 GOAL | 升级 0 命令只读切片为受控运行时 | W13 = Stage-II（delete/storage/audit/invoke/upgrade/rollback + UI） |
| §2.1 | 5 核心命令 | W12 落；W13 接管 `plugin_delete`/`plugin_uninstall` |
| §2.2 | 18 命令集 | W13 收 10/11 命令 + W12 8 命令 = 18/19 命令全集 |
| §2.3 install | 端到端 | W12 落；W13 接管升级/回滚 |
| §3 FM-1~FM-10 | 签名失败模式 | 全部继承 + W13 补 FM-8 升级验证 |
| §4 存储上限 | 8 项 | W13 接管 `plugin-invokes.json` 500 FIFO + `plugin_storage` 64KiB + `manifest.previous.json` 32KiB |
| §5 审计脱敏 | audit shape | W13 补 `plugin-invokes.json` 完整 shape + `key_hash_only: true` type-system 锁 |
| §6 UI 依赖 | 7 文件 | W13 收口 4 个新文件（详情/历史/storage/升级回滚）+ 全量 12 i18n key |
| §7 HS-1~HS-12 | 12 项硬停 | 全部继承 + 新增 HS-13~HS-19 |
| §8 Tests | 单测 14 + 策略 + UI | 22 项单测 + 8 新 ACTIVE 码 + 10 项 UI |

## 3. 与冻结段的对齐

- **M5-10 W6 freeze**：复用 `PluginManifest` / `PluginState` / `PluginCapability` / `PluginSignature` DTOs。
- **M5-11 W6 freeze**：复用 5 stub 命令 ACL + capability 校验骨架 + `PluginInvokeRecord` shape（`payload_key_hash` 16-hex + `key_hash_only: true`）。
- **A2 W2 constants**：复用 `MAX_TEXT_FIELD_BYTES=64KiB` / `MAX_PLUGIN_CAPABILITIES=5` / `HASH_HEX_LEN_SHORT=16`。
- **A3 W3 两段式确认闸门**：W13 `plugin_invoke` Dangerous 复用同款 pattern + keyring 二次认证。
- **A4 W4 审计基元**：`plugin-invokes.json` 500 FIFO / 64KiB 与 `audit.json` shape 对齐。
- **A5 W4 capability.rs**：`PLUGIN_CAPABILITY_V1` + `capability_acl_level` 3 档（Low/Medium/Dangerous）—— W13 invoke 双层校验 + risk→闸门映射。
- **K1/K3/K5/K7** 严守：ACL 末条恒 `list_artifact_images`、凭据禁记入 storage/audit、详情页禁折叠、capability.rs 单一真源、升级保留 storage/workspace。

## 4. 政策脚本扩展（实施 wave 须落）

`scripts/check-plugin-policy.py` 须由 W12 末 ACTIVE=4 升 ACTIVE≥11：

| 码 | 类型 | 描述 |
|---|---|---|
| `PLUGIN_AUDIT_KEY_HASH_ONLY` | PENDING→ACTIVE | `plugin-invokes.json` 写必带 `key_hash_only: true` |
| `PLUGIN_STORAGE_NO_SECRETS` | PENDING→ACTIVE | `plugin_storage_put` value 必过 `contains_credential_leak` |
| `PLUGIN_INVOKE_CAPABILITY_GATE` | PENDING→ACTIVE | `plugin_invoke` 必双层 capability 校验 |
| `PLUGIN_PATH_ESCAPE` | PENDING→ACTIVE | `plugin_invoke` params 路径必在 `plugins_dir/<id>/*` 子树 |
| `PLUGIN_UPGRADE_KEEP_STORAGE` | PENDING→ACTIVE | 升级保留 `storage.json`/`workspace/` |
| `PLUGIN_ROLLBACK_MANIFEST_PREV` | PENDING→ACTIVE | `plugin_rollback` 必读 `manifest.previous.json` |
| `PLUGIN_INVOKE_FIFO_500` | PENDING→ACTIVE | `plugin-invokes.json` 必 500 FIFO / 64KiB |
| `PLUGIN_DANGEROUS_KEYRING` | PENDING→ACTIVE | Dangerous capability 启用/invoke 必 keyring 二次认证 |
| W12 既有 3 ACTIVE + W6 既有 1 ACTIVE + 既有 4 维持 | 维持 | — |

`pre-merge.sh` 接入并 ALL_PASS（ACTIVE=11+）。

## 5. 切分边界（严守）

- **HS-13**：W13 仅 delete/storage/audit/invoke/permissions/upgrade/rollback + UI 全量收口；install/enable/disable/list/get 已在 W12 落。
- **HS-14**：W13 不写 install 真实解包（W12 已落）。
- **HS-15**：W13 升级保留 storage.json/workspace（**禁**升级丢数据，K1）。
- **HS-16**：`plugin-invokes.json` type-system 锁 `key_hash_only: true`（**禁**后续 lane 改 false）。
- **HS-17**：`plugin_invoke` Dangerous 能力必须 keyring 二次认证（**禁**仅 Confirm）。
- **HS-18**：W13 不动 A0 已拣入的 6 commit 集。
- **HS-19**：`plugin_rollback` 是否并入 18 命令集交 A0 拍（本卡默认 19 落地）。

## 6. 验证状态（本卡生成期）

- 卡片文件存在性：`test -f logs/assist/A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md` PASS
- 工作树状态：未污染（仅新增 2 张 staged cards + 2 个 checkpoint + 2 个 patch）
- 政策脚本未改（`check-plugin-policy.py` ACTIVE=1 PENDING=5 维持）
- ACL 未改（`default-commands.toml` 无 `plugin_*` 命令）
- `plugin.rs` / `bridge.rs` / `main.rs` 未改
- UI / `bridge.ts` / `types.ts` / 三份主文档未改
- 未移动 `NEXT`、未 push

## 7. 与他 lane 协同

- **A2 W11 / A4 W11 / A5 W11 / A7 W11 / A8 W11 / A10 W11**：均已拣入或待 A0 拣入；本卡 W13 与其无 cross-domain 冲突。
- **A3 W10 stdio-prep**（`ba78092`+`5226aad` 已拣入）：A3 不触 plugin 域。
- **A18/A19 实施 wave**（W12/W13 接收方）：W12 必须先落地 install/enable/disable/list/get + 3 keyring；W13 仅在 W12 闭合后开工。
- **A1 reconciliation**：本卡 W13 与 W12 配对；A1 W12 reconciliation 整包合并拣入时同步收 W13 头部 + 切分边界。

## 8. 交付索引

- 卡片：`logs/assist/A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md`
- 姊妹卡：`logs/assist/A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md`
- 本 checkpoint：`logs/checkpoints/Lane-A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md`
- 姊妹 checkpoint：`logs/checkpoints/Lane-A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md`
- 承接链：W10 dispatch 汇总卡 → W12（姊妹） → W13（本卡） → 实施 wave 待 A0 显式新 dispatch
