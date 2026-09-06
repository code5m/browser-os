# M5-10 插件 manifest 与生命周期（签名/load/unload）

> 子卡 ID：**M5-10** · 需求 #15（插件系统）· `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A18**
> 父卡：`详细设计与实施计划.md` L574（`M5-10 插件 manifest 与生命周期`）
> 主预研：`logs/assist/M5-15.a-prework-20260902-1055.md` · `logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`（**默认形态③声明式**）
> 配套：`M5-11-plugin-commands-isolation.md`（命令与隔离）· `M5-12-plugin-ui.md`（管理 UI）

---

## 0. 编号与锚定

- 批次任务号 `M5-10`；需求号 #15；WBS L574 一致。
- 依赖：A0 拍形态决策（**默认形态③声明式**）+ M5-2 ✅（capability.rs 共用）
- **关键决策交 A0 拍**：① 形态（**默认形态③**——Tauri v2 不支持按 webview 关全局，**形态②放弃**）② 签名方案（**默认 Ed25519 + 本地受信任 keys 目录**）

---

## 1. GOAL

冻结 `PluginManifest` schema（id / version / entry / capabilities / hash / signature / min_app_version）；实现插件加载/卸载生命周期（manifest 解析 → 签名校验 → capability 校验 → 加载到 `plugins_dir()` → 注册到 `capability.rs` → 启用 / 停用 / 卸载）；**默认形态③**——插件是"声明式资源包"（图标 + 入口 URL + 权限声明），**不**是独立 webview，**不**是独立 stdio 进程。

---

## 2. READ

1. `logs/assist/M5-15.a-prework-20260902-1055.md`（**全读**，manifest/签名/形态）
2. `logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`（**全读**，三形态可行性）
3. `M5-2-rmcp-mcp-policy.md` §4.2-4.3（capability.rs 共用）
4. `M5-4-agent-skill-runtime.md` §4.2（`SkillExec` 禁内联，K6）
5. `src-tauri/src/workspace.rs`（`plugins_dir()`）
6. `src-tauri/src/keyring_store.rs`（签名 key 存放）
7. `src-tauri/src/domain.rs`（契约根，**全读**）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/plugin_manifest.rs` | **新增** | Manifest 解析 + 校验 + 签名验证 |
| `src-tauri/src/plugin_lifecycle.rs` | **新增** | load / unload / enable / disable 状态机 |
| `src-tauri/src/plugin_signature.rs` | **新增** | Ed25519 签名 + 验证 + 本地受信任 keys |
| `src-tauri/src/plugins/` | **新增目录** | `plugins_dir()` 默认 `~/.local/share/.../plugins/` |
| `src-tauri/src/keys/trusted-pubkeys.json` | 持久化 | 受信任公钥列表（受 keyring 保护访问） |
| `src-tauri/src/domain.rs` | 新增类型 | `PluginManifest` / `PluginState` / `PluginCapability` / `PluginSignature` |
| `src-tauri/src/capability.rs` | 扩展 | 加 `PLUGIN_CAPABILITY_V1` 列表（首期空，按需逐个评估） |
| `src-tauri/src/bridge.rs` | 修改 | `plugin_install` / `plugin_uninstall` / `plugin_list` / `plugin_get` / `plugin_enable` / `plugin_disable` / `plugin_keys_add/list/remove`（**全进 ACL**） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 `plugin_*` 共 10 条于 `list_artifact_images` 之前 |
| `src/types.ts` | 新增 | TS 镜像 `PluginManifest` |

---

## 4. 关键契约

### 4.1 形态决策（默认形态③ 声明式）

| 形态 | 内容 | 决策 |
|---|---|---|
| ① 独立 stdio 进程 | 插件 = 独立可执行 + stdio 通信 | **否决**（与"无第二执行路径"冲突） |
| ② 独立 webview（host 进程） | 插件 = webview，访问 `window.__TAURI__` | **否决**（`withGlobalTauri` 全局未按 webview 关闭，跨插件泄露） |
| **③ 声明式资源包**（默认） | 插件 = 图标 + 入口 URL + 权限声明 + JS 钩子 | **采用** |

**形态③插件的本质**：
- 入口 URL 走既有 webview 框架（不开新 webview）
- JS 钩子通过受控 `bridge.ts` 子集调用（不允许裸 `invoke`）
- 资源文件（图标/文案）走 `workspace/` 同款隔离

### 4.2 `PluginManifest` schema

```rust
pub struct PluginManifest {
    pub id: String,                  // 反向域名规范
    pub version: String,             // semver
    pub display_name: String,
    pub description: String,
    pub min_app_version: String,     // semver，过期拒绝
    pub entry: PluginEntry,          // 形态③
    pub capabilities: Vec<PluginCapability>,  // 引用 capability.rs
    pub hash: String,                // 资源包 sha256
    pub signature: PluginSignature,  // Ed25519
    pub metadata: serde_json::Value,
}
pub struct PluginEntry {
    pub entry_url: String,           // 形态③ 入口 URL（与既有 webview 同源）
    pub icon: String,                // 图标相对路径
}
pub struct PluginCapability {
    pub capability: String,          // 引用 capability.rs 常量
    pub reason: String,              // 为什么需要此 capability
}
pub struct PluginSignature {
    pub algorithm: String,           // "Ed25519"
    pub key_id: String,              // 引用 trusted-pubkeys.json 的 key
    pub value: String,               // base64 签名
    pub signed_at: DateTime<Utc>,
}
```

### 4.3 生命周期状态机

```
Discovered → Validating → Signed(OK|Failed) → Loaded → Enabled → Disabled
                                                    ↓
                                                Uninstalled
```

- `Discovered`：扫描 `plugins_dir()` 找到新 manifest
- `Validating`：schema 校验 + `min_app_version` 校验
- `Signed`：签名验证（trusted-pubkeys.json 必须在 key）
  - **Failed**：拒绝加载 + 写 `audit.json` + 通知 UI
- `Loaded`：解压到 `plugins_dir()/<id>/<version>/` + 校验 hash
- `Enabled`：注册到 `capability.rs` + UI 可见
- `Disabled`：保留资源但移除 capability（可重新启用）
- `Uninstalled`：删 `plugins_dir()/<id>/`

### 4.4 签名验证

- 签名算法：**Ed25519**（默认）
- 公钥存 `~/.local/share/.../keys/trusted-pubkeys.json`（keyring 保护读写）
- 首次安装要求用户提供 pubkey（或开发者自带）
- 升级时必须验证 `hash` + `signature` 同时通过
- `manifest_hash` 变更 → 已启用插件自动 `Disabled`，等用户重新确认

### 4.5 二次确认（与 MCP/Skill 范式一致）

- 任何 `plugin_install` / `plugin_enable` → 走两段式确认闸门
- 必须显式列 `capabilities`（**逐项**）+ 来源 + 风险

---

## 5. FORBID

- **不**采用形态①/形态②（A9 feasibility 已决）
- **不**让插件接触 `window.__TAURI__` 全局（K1 + 形态③）
- **不**让 `PluginManifest.hash` 与 `signature` 校验任一失败时仍加载
- **不**让 `capability.rs` 漂移（A2P / A2A / Skill / Plugin / Agent 共用）
- **不**让 `trusted-pubkeys.json` 被未授权修改
- **不**让 `manifest_hash` 变更不触发重新确认
- **不**破 K1（ACL 末条恒为 `list_artifact_images`）/K3/K5
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
ls src-tauri/src/plugins src-tauri/src/keys 2>&1
grep -nE "PluginManifest|plugin_lifecycle" src-tauri/src | head

# B. ACL 末条
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -5

# C. 反向用例
# N1: 形态② 插件（webview 访问 window.__TAURI__）→ 阻断
# N2: 签名验证失败 → 拒绝加载 + 审计
# N3: min_app_version 过期 → 拒绝加载
# N4: trusted-pubkeys.json 不含 key_id → 拒绝
# N5: 资源包 hash 与 manifest 不一致 → 拒绝
# N6: manifest_hash 变更 → 插件自动 Disabled
# N7: 插件安装不弹闸门 → 阻断

# D. 性能基线
# 10 插件安装 < 5s
# 100 插件列表 < 100ms

# E. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# F. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | `PluginManifest` / `PluginState` 落 `domain.rs` | `grep` 命中 |
| 2 | 形态③ 是唯一形态；形态①/② 实现不存在 | `grep` 0 命中 |
| 3 | 签名验证失败拒绝加载 | 单测 N2 |
| 4 | `min_app_version` 过期拒绝 | 单测 N3 |
| 5 | `trusted-pubkeys.json` 缺失 key_id 拒绝 | 单测 N4 |
| 6 | 资源包 hash 校验失败拒绝 | 单测 N5 |
| 7 | `manifest_hash` 变更触发 Disabled | 单测 N6 |
| 8 | 安装必弹闸门 | 单测 N7 |
| 9 | ACL 末条仍为 `list_artifact_images` | 命令 B |
| 10 | `cargo test` 全绿 | 命令 E |
| 11 | `pre-merge.sh` ALL_PASS | 命令 F |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 形态①/② 出现 | **红线失守**：与 K1/形态决策冲突 |
| 签名校验被绕过 | **红线失守**：阻断 |
| `trusted-pubkeys.json` 越权改 | 立即回滚 + 审计 + 提示用户检查 |
| `manifest_hash` 变更不触发确认 | 阻断（用户数据安全） |
| 闸门被绕过 | 阻断 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L574 `[ ]` → `[x]`
2. `后续需求TODO.md` §15 状态 `PARTIAL`（留 `M5-11/12`）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-11`
4. `logs/checkpoints/M5-10.a-2026MMDD-HHMM.md`
5. `M5-14-debt-ledger.md` 增项：第三方签名服务（是否首期做） / 插件商店

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A18 实施填
- **NEXT**：M5-11（命令与隔离），同 A18 拆卡

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
