# Lane A9 — M5-W14 Plugin Manager UI · 后端契约复核（SUPPORT BACKEND REVIEW ONLY）

- **Lane / 分派**：A9，`SUPPORT BACKEND REVIEW ONLY`（**无产品代码**，本卡零 Rust/TS 改动）
- **BASE**：`a7eefbb`（master，W13 已合入；`git pull --ff-only` 已同步）
- **交付物**：本复核笔记（契约核对 + 缺失只读展示字段登记）；无补丁（无代码改动）
- **时间戳**：2026-09-07 14:30

## 一、复核结论（一句话）

W13 冻结契约（8 命令 ↔ 8 bridge 方法 ↔ DTO 集合）**完整、自洽、脱敏干净**，A6 可在**不改动后端**的前提下，仅用 `src/bridge.ts` 的 8 个方法 + 客户端 join 构建完整的只读 Plugin Manager UI。唯一需要后端才能补齐的展示字段（G2/G3）属 W14 冻结范围之外，交后续 wave。

## 二、冻结契约面（已逐项核验）

### 2.1 后端 8 命令（`src-tauri/src/bridge.rs` + `main.rs:1478-1485`）
每条均带 `check_invocation_source`（来源校验，仅受信任 main 窗口可调用）：
`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` /
`plugin_get` / `plugin_keys_add` / `plugin_keys_list` / `plugin_keys_remove`。

### 2.2 前端 8 方法（`src/bridge.ts:718-739`，1:1 映射，无裸 invoke）
`pluginInstall` / `pluginEnable` / `pluginDisable` / `pluginList` /
`pluginGet` / `pluginKeysAdd` / `pluginKeysList` / `pluginKeysRemove`。

### 2.3 DTO（types.ts / domain.rs，脱敏构造）
- `PluginSummary`：`id, version, display_name, state, capability_count, hash_prefix(16hex), updated_at`
- `PluginDetail`：`id, version, display_name, description, min_app_version, state, capabilities[], hash_prefix, signature, installed_at, updated_at, resource`
- `PluginCapabilityView`：`capability, reason, acl_level("safe"|"confirm"|"dangerous")`
- `PluginSignatureView`：`algorithm, key_id, status("structure_ok"|"structure_failed")` —— **无 `value`**
- `PluginResourceMeta`：`declared_hash, path_provided, verified` —— **无资源绝对路径**
- `TrustedKeyRecord`：`key_id, fingerprint(sha256 前16hex), note, added_at` —— **无公钥原文**
- `PluginManifest`（仅作 `pluginInstall` 入参）：含 `signature.value`，但后端**只收不存**，输出视图永不回显（单测 `!json.contains("\"value\"")` 已锁）。

### 2.4 ACL（`permissions/default-commands.toml`）
8 条 `plugin_*` 插在末条 `list_artifact_images` **之前**（K1 红线）。

## 三、A6 各视图所需数据 → 契约供给核对（零后端改动）

| UI 视图 | 调用 | 所需字段 | 供给 |
|---|---|---|---|
| 列表/筛选 | `pluginList(state?)` | Summary 全字段 | ✓ |
| 详情/检查 | `pluginGet(id)` | Detail 全字段 | ✓ |
| 能力风险展示 | — | `capabilities[].acl_level` | ✓（UI 逐项展示、禁折叠） |
| 启/停用 | `pluginEnable/Disable(id)` | id | ✓（返回新 Summary） |
| 安装（提供 manifest） | `pluginInstall(manifest, resourcePath?)` | 整份 manifest 作入参 | ✓（value 为输入态、禁渲染） |
| 受信任密钥管理 | `pluginKeysAdd/List/Remove` | TrustedKeyRecord[] | ✓ |

→ **结论**：所有只读展示与状态变更数据后端均已暴露，A6 **无需任何后端变更**。

## 四、缺失的只读展示字段登记（需后端才补齐 → W14 冻结范围外）

> 判定口径：下列字段如需出现在 UI，目前契约不提供；补之须改 `domain.rs`/`plugin.rs`/`types.ts`（W14 冻结 DTO + 禁新增生命周期命令）。故**不在本 wave 实现**，仅登记供 A0/A6 决策与后续 wave 排期。

- **G1（非后端缺口，交 A6 客户端 join）** —— 签名是否来自受信任密钥的指示：`PluginSignatureView` 无 `trusted` 布尔。UI 应在客户端用 `signature.key_id` 与 `pluginKeysList()` 结果做 join 推导「signed by trusted key: 是/否」。**无需后端改动**，列为 A6 实现责任。
- **G2（真缺口，可选）** —— `vendor`/`author` 展示：manifest 无 vendor 字段，仅 `id`（反向域名）。Stage-I 本地 MVP 可从 `id` 域名前缀推导出版方，或干脆省略；若需正式 vendor 展示，须后端补字段（后续 wave）。
- **G3（真缺口，UX）** —— 校验失败原因：`signed_failed` 态下 UI 无从得知为何失败（算法不匹配？key 空？manifest 非法？）。契约仅给 `state` + `signature.status` 代理。若需富文本失败说明，须后端补一个**仅稳定码/脱敏**的 `error`/`last_error` 字段（严禁回显 secret）；建议留待后续运行时 wave，且必须遵守 A4 脱敏纪律。
- **G4（命名差异，提醒 A6）** —— W13 文档曾称受信任密钥字段为 `label`；冻结 `types.ts` 实际字段为 `note`。A6 必须绑定 `note`，勿用 `label`。
- **G5（次要）** —— `installed_at` 仅在 Detail，不在 Summary。若列表想显示安装时间，UI 可复用 `updated_at`，或后续 wave 在 Summary 加 `installed_at`（后端改动）。

## 五、A6 硬停合规保证（基于冻结契约）

- **禁裸 invoke**：8 方法即全部入口，契约未暴露任何裸 `invoke` 直连。
- **禁执行/invoke/网络/下载/动态加载**：契约面无任何此类命令。
- **禁渲染签名/metadata/路径**：DTO 构造上即不含 `value`/`metadata`/资源路径；安装期 `manifest.signature.value` 为输入态，A6 必须当作不透明入参、**绝不渲染**。
- **命令名与 DTO 冻结**：A6 不得新增生命周期命令。

## 六、最终裁定

1. W13 契约**可被 A6 无后端改动消费**，完整支撑只读 Plugin Manager UI（列表/筛选/检查/启停/安装/受信任密钥管理）。
2. 唯一需后端介入的展示增强（G2/G3）属 W14 冻结外，登记待后续 wave；G1 由 A6 客户端 join 解决，不阻塞本 wave。
3. A9 本卡**零产品代码**，交付仅此复核笔记。无需补丁。
4. 建议 A0 在集成 A6 产物时，确认 A6 已：(a) 用 `note` 而非 `label`；(b) 对 `signature.key_id` 做受信任密钥 join（G1）；(c) 安装流程不渲染 `signature.value`。
