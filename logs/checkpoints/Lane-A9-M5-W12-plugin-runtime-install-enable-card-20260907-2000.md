# A9 · M5-W12 Plugin Runtime Stage-I Staged Card — Checkpoint

> Lane：A9（PLUGIN DOCS/POLICY ONLY）
> Wave：M5-W12（Plugin Runtime Staged Dispatch · Stage-I install/enable/disable/list/get）
> 时间：2026-09-07 20:00 CST
> 性质：**规划卡片 checkpoint · 无产品代码改动 · 未 push**

## 1. 交付物

- 卡片：`logs/assist/A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md`
  - 5 核心命令（`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get`）+ 3 keyring 命令（`plugin_keys_add/list/remove`）= 8 命令
  - 端到端执行序列（install/enable/disable/list/get 4 段伪代码）
  - 签名失败模式（FM-1~FM-10）
  - 存储上限（复用 W6/A2 W2 单一真源）
  - 审计脱敏（`audit.json` 摘要 shape，禁记 params/result 正文/公钥本身/keyring 凭据）
  - UI 依赖文档化（`PluginManager.vue` / `TrustedKeyManager.vue` / 闸门弹窗复用）
  - 14 项 hard stops（含 HS-13/HS-14/HS-15 严守 W12 切分边界）
  - 16 项 Rust 单测 + 3 新增 ACTIVE 政策码 + 6 项 UI 集成测试

## 2. 与 W10 计划的对应

| W10 § | 主题 | W12 落点 |
|---|---|---|
| §1 GOAL | 升级 0 命令只读切片为受控运行时 | W12 = Stage-I（install/enable/disable/list/get） |
| §2.1 | 5 核心命令 | 全部覆盖（含端到端伪代码） |
| §2.2 | 18 命令集 | W12 收 8 命令（install/enable/disable/list/get + 3 keyring） |
| §2.3 | install 端到端 | 完整 15 步伪代码 |
| §3 FM-1~FM-10 | 签名失败模式 | 全部覆盖 + W12 必落 FM-5/FM-6 |
| §4 存储上限 | 8 项 | W12 覆盖 6 项（W13 接管 plugin-invokes.json） |
| §5 审计脱敏 | audit shape | 覆盖 install/enable/disable/keyring 子集 |
| §6 UI 依赖 | 7 文件 | 文档化 7 文件 + 明确 W12 UI 切分（详情/历史/storage/权限 留 W13） |
| §7 HS-1~HS-12 | 12 项硬停 | 全部继承 + 新增 HS-13/HS-14/HS-15 严守切分 |
| §8 Tests | 单测 14 + 策略 + UI | 16 项单测 + 3 新 ACTIVE 码 + 6 项 UI |

## 3. 与冻结段的对齐

- **M5-10 W6 freeze**：`PluginManifest` / `PluginState` / `PluginCapability` / `PluginSignature` DTOs + `validate_plugin_manifest` / `can_transition` / `transition` / `permission_preview_for_plugin` 纯函数 —— **W12 端到端序列直接复用，0 漂移**。
- **M5-11 W6 freeze**：5 stub 命令 ACL + capability 校验骨架 + `PluginInvokeRecord` shape —— **W12 不重定义，invoke 留 W13**。
- **A2 W2 constants**：`MAX_TEXT_FIELD_BYTES=64KiB` / `MAX_PLUGIN_CAPABILITIES=5` / `HASH_HEX_LEN_SHORT=16` / `MAX_PLUGIN_ID_BYTES=256` —— **W12 全部复用，0 字面量**。
- **A3 W3 两段式确认闸门**（`bridge.rs:688-790`）：W12 `plugin_install` / `plugin_enable` 复用同款 pattern。
- **A4 W4 审计基元**：W12 `audit.json` 摘要 shape 与 `plugin-invokes.json` 摘要字段对齐（`payload_key_hash` 16-hex）。
- **A5 W4 capability.rs**：`PLUGIN_CAPABILITY_V1`（首期空集合 fail-closed）—— W12 enable 时双层校验（manifest ⊆ PLUGIN_CAPABILITY_V1）。
- **K1/K3/K5/K7** 严守：ACL 末条恒 `list_artifact_images`、凭据禁记入 storage/audit、详情页禁折叠、capability.rs 单一真源。

## 4. 政策脚本扩展（实施 wave 须落）

`scripts/check-plugin-policy.py` 须由 PENDING=5 升 ACTIVE≥4：

- `PLUGIN_RUNTIME_GUARD`（PENDING→ACTIVE）：禁 `plugins_dir` 直写 / 禁 `std::process` / 禁网络拉取
- `PLUGIN_MANIFEST_HASH_PINNED`（新增 ACTIVE）：`hash` 须 == entry 校验值
- `PLUGIN_TRUSTED_KEYS_KEYRING`（新增 ACTIVE）：`trusted-pubkeys.json` 读写须经 keyring

`PLUGIN_SIG_BYPASS` / `PLUGIN_NO_SECRETS` / `PLUGIN_FORM_THREE` / `PLUGIN_CAP_SINGLE_DEF` 维持。

## 5. 切分边界（严守）

- **HS-13**：W12 仅 install/enable/disable/list/get + 3 keyring 命令；其余 10 命令（invoke/permissions/audit/storage/delete）**全部留 W13**。
- **HS-14**：W12 不写 `plugin-invokes.json` / `plugin_storage` / 详情 modal / 升级回滚。
- **HS-15**：W12 不动 A0 已拣入的 6 commit 集（`5f92ece`/`6c1f30e`/`daa10f6`/`a29b796`/`ba78092`/`5226aad`）。

## 6. 验证状态（本卡生成期）

- 卡片文件存在性：`test -f logs/assist/A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md` PASS
- 工作树状态：未污染（仅新增 `logs/assist/A9-M5-W12-*.md` + `logs/assist/A9-M5-W13-*.md` + 本 checkpoint + patch）
- 政策脚本未改（`check-plugin-policy.py` ACTIVE=1 PENDING=5 维持）
- ACL 未改（`default-commands.toml` 无 `plugin_install`/`plugin_enable`/`plugin_disable` 等命令）
- `plugin.rs` / `bridge.rs` / `main.rs` 未改
- UI / `bridge.ts` / `types.ts` / 三份主文档未改
- 未移动 `NEXT`、未 push

## 7. 与他 lane 协同

- **A2 W11**（已拣入 draft）：A2 W11 笔记提及 `core/bin` 边界，与本卡 HS-2/HS-10 形态③ + 无第二执行路径一致。
- **A4 W11**（已拣入）：A4 W11 笔记 `logs/assist/A4-M5-W11-privacy-review-20260907-1900.md` 提 A3 stdio-prep，本卡 W12 不触 A3 域（HS-2 严守）。
- **A5 W11**（已拣入）：A5 W11 笔记 `logs/assist/A5-M5-W11-20260907-1935.md` 与本卡能力白名单 `PLUGIN_CAPABILITY_V1` 对齐。
- **A7 W11**（已拣入）：A7 W12 graph impl plan 与本卡 plugin 域 0 交叉。
- **A8 W11**（已拣入）：A8 graph UI polish 与本卡 UI 切分一致。
- **A10 W11**（已拣入）：A10 W11 笔记 `logs/assist/A10-M5-W11-20260907-1910.md` 复审范畴，本卡供其对照。

## 8. 交付索引

- 卡片：`logs/assist/A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md`
- 姊妹卡：`logs/assist/A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md`
- 本 checkpoint：`logs/checkpoints/Lane-A9-M5-W12-plugin-runtime-install-enable-card-20260907-2000.md`
- 姊妹 checkpoint：`logs/checkpoints/Lane-A9-M5-W13-plugin-runtime-delete-storage-card-20260907-2000.md`
- 承接链：W10 dispatch 汇总卡 → W12（本卡）/ W13（姊妹卡）→ 实施 wave 待 A0 显式新 dispatch
