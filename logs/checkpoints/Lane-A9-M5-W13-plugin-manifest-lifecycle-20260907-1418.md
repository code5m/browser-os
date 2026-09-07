# Lane A9 — M5-W13 插件 manifest 生命周期 Stage-I（Manifest Lifecycle, Local-Only）

- **Lane / 分派**：A9，`START PRODUCT CODE`
- **BASE**：`3c3f460`（master，会话起始工作树已 `git pull --ff-only` 同步）
- **HEAD**：未 push（仅 A0 可 push）；交付物 = 本检查点 + 同名列 `.patch`
- **时间戳**：2026-09-07 14:18

## 一、本卡范围（防越界）

Plugin Runtime Stage-I = **只管本地状态**，红线圈定如下，**均未实现（刻意不碰）**：
- 无 `plugin_invoke` / 执行 / 动态加载 / 网络下载 / 监听 / daemon。
- 无 model / Agent-Skill / MCP 全量 / graph build-write-export。
- 真验签（Ed25519 校验签名值）与资源解包属后续运行时 wave；本 wave 仅做**结构校验**。
- 视图层永不出现：签名原文（`value`）、资源绝对路径、公钥原文（仅 16-hex 指纹）。

## 二、交付物（原子同包）

### 后端（Rust，src-tauri）
1. **`src/domain.rs`**（W13 DTO 区）
   - 新增 `PluginState` / `PluginEntry` / `PluginCapability` / `PluginSignature` /
     `PluginManifest` / `PluginResourceMeta` / `PluginSummary` / `PluginCapabilityView` /
     `PluginSignatureView` / `PluginDetail` / `TrustedKeyRecord` —— 与后端 `domain.rs` 1:1 镜像，
     **无** `value` / `pubkey` / `resource_path` 字段。
   - `PluginRecord` 重构为**派生字段**（不再整份嵌套 `PluginManifest`）：
     `metadata` 与 `signature.value` 永不入库（闭环 A4 F-A4-2 / F-A4-4）。
   - 常量单一真源：`MAX_TEXT_FIELD_BYTES = 64KiB`、`HASH_PREFIX_LEN = 16`。

2. **`src/plugin.rs`**（纯策略切片，零执行面）
   - `validate_plugin_manifest`：反向域名 id + 形态③同源 entry + capability 白名单 + 签名结构 +
     `metadata` **体量边界 + `contains_credential_leak`**（此前只体量，注释与代码漂移，已闭环）。
   - `verify_plugin_signature_structure`：纯结构校验（不验签）。
   - 登记簿：内存 `Vec<PluginRecord>` + `session::atomic_write`（tmp+rename）落 `plugins.json`；
     损坏/非 JSON 触发 `corrupt_registry_recovers_to_empty` 恢复为 `[]`，错误只报稳定码。
   - `trusted_keys_add/list/remove`：只存 `fingerprint`（sha256 前 16 hex），**不落公钥原文**。
   - `detail_of` / `summary_of`：脱敏视图（`value` / `metadata` / 路径不出）。
   - 错误脱敏：`redact_preview`（len + sha256 前缀，不回显原文），`CredentialLeak` 载荷只放**字段名**
     （闭环 A4 F-A4-1 / F-A4-3）。

3. **`src/bridge.rs`**：8 条命令（每条约 10 行，含 `check_invocation_source` 来源校验 + 脱敏审计）。
   - `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` /
     `plugin_keys_add` / `plugin_keys_list` / `plugin_keys_remove`。
   - 审计 detail 仅 `id` / 计数 / 稳定 `code`，无路径 / 正文 / 裸签名。

4. **`src/main.rs`**：8 条命令注册 + `mod plugin;`。

5. **`src-tauri/permissions/default-commands.toml`（ACL）**：8 条 `plugin_*` 插在末条
   `list_artifact_images` **之前**（K1 红线：plugin 命令 ACL 尾恒为该审计镜像末条）。

### 前端（TS）
6. **`src/types.ts`**：W13 DTO（禁字面量，1:1 镜像后端 `domain.rs`）。
7. **`src/bridge.ts`**：8 个 `plugin*` 包装（UI 禁止裸 invoke），来源校验由后端命令层兜底。

### 门禁夹具（机器可判）
8. **`scripts/check-plugin-policy.py`**：新增 6 条 ACTIVE 码
   - `PLUGIN_CMD_ACL_PARITY`（命令集与 ACL 一致 + 末条 `list_artifact_images`）
   - `PLUGIN_CMD_SOURCE_CHECK`（每命令过来源校验）
   - `PLUGIN_NO_EXEC_SURFACE`（插件域无执行面）
   - `PLUGIN_AUDIT_REDACTED`（审计不回显禁记字段）
   - `PLUGIN_REGISTRY_ATOMIC_WRITE`（落盘走 atomic_write）
   - `PLUGIN_DTO_NO_SIG_VALUE`（DTO 禁 `value` / `pubkey` / `resource_path`）
   - 含 `--self-test` 双向自检（好样本零违规 + 坏样本全检出，含变异防呆）。

## 三、闭环的 A4 隐私缺陷（A4 W13 隐私评审交付给我）

| A4 码 | 缺陷 | 闭环方式 |
|---|---|---|
| F-A4-1 | 错误串原样回显 `id` / `entry_url` / `algorithm` | `redact_preview` 脱敏预览 |
| F-A4-2 / F-A4-4 | `metadata` 只体量边界、密钥随登记簿落盘 | `metadata` 过 `contains_credential_leak` + `PluginRecord` 不整份落 `PluginManifest` |
| F-A4-3 | `CredentialLeak` 载荷带命中密文（`Debug` 可回显） | 载荷只放字段名（如 `"plugin.metadata"`） |

闭环后 `python3 scripts/check-plugin-privacy.py --expect-pending` = **NONE**（5 个 pending 码位均未检出），
A4 要求的「修复后转 ACTIVE + 接入 `--expect-pending`」条件已满足。

## 四、验证证据（本次本地复跑）

- `cargo test --manifest-path src-tauri/Cargo.toml` → **431 passed; 0 failed**（含本卡 28 条 plugin 单测）。
- `python3 scripts/check-plugin-policy.py` → `PLUGIN_POLICY=PASS`（ACTIVE=7 / PENDING=5）。
- `python3 scripts/check-plugin-policy.py --self-test` → `ALL_PASS`。
- `python3 scripts/check-plugin-privacy.py --expect-pending` → `NONE`。
- `python3 scripts/check-plugin-privacy.py --self-test` → `PASS`。
- `cargo fmt --check` → 干净（无格式漂移）。
- `git diff --check` → 仅机器证据除外，本卡 8 文件无冲突标记。

## 五、集成核对（交给 A0）

本卡只触碰共享文件（domain.rs / bridge.rs / main.rs / ACL / types.ts / bridge.ts），A0 合并时注意保留
其它 lane 在同一文件上的已提交改动（我用 `git diff <本卡文件>` 生成补丁，已排除其它 lane 产物）。

- ACL 末条仍为 `list_artifact_images`（本卡 8 条命令插在其前）。
- 无新增 `tauri::async_runtime::spawn` / `std::process` / `reqwest` / `eval` / 动态加载面。

## 六、剩余挂账（非本卡，交后续 wave / A0）

- W13 真验签（Ed25519 验 `signature.value`）、资源包解包与比对 —— 后续运行时 wave。
- 插件 UI（Lane A8 W12，独立交付，本卡不碰前端 UI）。
