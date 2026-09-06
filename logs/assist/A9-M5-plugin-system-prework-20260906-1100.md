# A9 M5-10/11/12 插件系统预研（M5 Dispatch Now 整包交付）

> 生成：2026-09-06 11:00 CST · Lane A9（M5 prework only · docs only）
> BASE：`5ca8f9f`（HEAD，`docs(M5): dispatch architecture prework lanes`；M4 集成基线 `a1a2061` 已推送 `origin/master`）
> 性质：**可照着实现的契约草案 + 任务拆分 + 安全阻塞项**。零产品代码（未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh` 及三份主文档）。
> 锚定事实：所有「已落地/已冻结」陈述均来自对当前工作树的只读核对（§0），非推断。
> 配套：`A9-M5-plugin-form-feasibility-20260906-0700.md`（形态裁定，本文继承其结论）· `A9-M5-A13plus-cards-20260906-0010.md`（A18/A19 卡）· `详细设计与实施计划.md` §7.3 #15 与 M5-10/11/12。

---

## 0. 事实基线（2026-09-06 实测，只读核对）

| 项 | 已落地事实（文件:行） | 对 M5-10/11/12 的意义 |
|---|---|---|
| 全局 Tauri | `src-tauri/tauri.conf.json:12` `"withGlobalTauri": true`；`:15` `"csp": null` | 任何 webview 都能拿到 `window.__TAURI__`；form② webview 插件隔离的前提风险点（SB-1） |
| 全局 Tauri 引用点 | `grep -rn "__TAURI__" src/` 仅 **1 处**（`src/stores/useSystemStore.ts:84` 的 `event.listen("tauri://focus",...)`） | 主应用对全局 Tauri 依赖极薄，可在 workspace 下沉（A13）时封死为显式注入 |
| ACL 末条 | `src-tauri/permissions/default-commands.toml` 尾条恒为 `list_artifact_images` | 插件命令一律插其前（K1 坑位②） |
| 命令数量 | `default-commands.toml` 已含 `db_*`/`task_*` 等 100+ 命令 | 插件命令须逐个进 ACL 白名单 |
| PluginManifest | `grep -rn "PluginManifest" src-tauri/src/domain.rs` → **无**（类型尚未定义） | M5-10 须在 `domain.rs` 新增 `PluginManifest`/`PluginState`/`AclLevel`/权限类型 |
| 执行通道 | `src-tauri/src/script_runner.rs:843` `start_run` / `:882` `start_command`（M2-4 单执行路径） | form③ 执行体复用此通道 → **零第二执行路径** |
| 两段式闸门 | `src-tauri/src/bridge.rs:688-790` `request_sync`/`confirm_sync` | 插件安装/启用确认范式必须复用 |
| 审计 | `src-tauri/src/workspace.rs:89-110` `audit.json` 上限 **1000 条 FIFO** | 高频 `plugin_invoke` 须走独立 `plugin-audit.json`（K5） |
| 凭据 | `src-tauri/src/keyring_store.rs` `KeyringStore`（keyring 3） | 插件配置敏感项走 keyring 引用，不落明文（K3） |
| capabilities | `src-tauri/capabilities/` 仅 `browser-remote.json`、`default.json` | form② 需新增 `plugin-*.json`；须一并清理 `default.json` 残留 `browser` label（SB-2） |
| 关闭序 | `ShutdownCoordinator`（`shutdown.rs`）已有；A7 `stop-scheduler` 注册在 `kill-running-scripts` 之前 | 插件无独立进程（form③），退出收口沿用既有协调器 |

> 上述事实以 `a1a2061` 为基准（M4 数据库+调度代码已推送）。A9 仅写 `logs/assist/`，与任何 lane 未提交产物无文件交集。

---

## 1. 形态裁定（继承 `A9-M5-plugin-form-feasibility-20260906-0700.md`）

| 形态 | 2026-09-02 旧 prework 假设 | M5 Dispatch Now 裁定（本文继承） |
|---|---|---|
| ① 独立进程 / stdio | `plugin-runtime-taskcard` 的**主线** | **否决**：新增长生命周期子进程通道违反「禁第二执行路径」红线（`详细设计` M4 护栏）。除非 A0 显式放宽并新增 `PluginProcessTable`+`kill-plugins`，否则不实现 |
| ② webview 内插件 | 待判定 | **条件可行，仅作 UI 承载壳**：承载插件管理面板用 webview，但**执行仍走 M2-4**；前提是对插件 webview 关闭全局 Tauri 注入（SB-1）。若全局 Tauri 无法按 webview 关闭 → 仍否决 |
| ③ 声明式插件（无第三方代码加载） | 提及但非主线 | **默认形态（推荐）**：插件 = manifest 声明 `kind` + 指向已有 `ScriptMeta`/`ToolMeta`/`SkillDef`/`SessionHandler` id + 静态 `acl_level`；**不加载外部代码、不开新进程**，100% 复用 M2-4 + M2-9 执行通道与既有确认/审计/沙箱 |

**与 2026-09-02 旧卡的关系**：`plugin-runtime-taskcard-20260902-1146.md` 与 `plugin-permission-taskcard-20260902-1146.md` 中的「形态①进程/stdio」「manifest 含 `bin/` 可执行 + `form: process`」「`PluginProcessTable`」「`setsid`/`env_clear` 进程组」等内容 **已被本预研 SUPERSEDED**（裁定为否决/不实现）。其权限分级模型（§4.1 `PermissionLevel`）、fail-closed 判定算法（§4.2）、授权记录（§4.3）仍可整体沿用，但执行载体从"独立进程"改为"复用 M2-4 的执行引用"。`M5-15.a-prework`（form①②③，推荐①）亦需按本裁定更新为"默认③、②作壳、①否决"。

---

## 2. M5-10 插件 manifest 与生命周期（契约）

### 2.1 `PluginManifest`（声明式，form③ 专属）

```rust
// 位置：src-tauri/src/domain.rs（M5-10.a 新增）；serde rename_all="snake_case"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginKind {
    Script,          // 包装 M2-3.a ScriptMeta
    Tool,            // 包装 M2-7.b ToolMeta
    Skill,           // 包装 M5-12.a SkillDef
    SessionHandler,  // 包装 #14 会话处理器（尚未实现）
    // ❌ 不提供 Process / NativeLib / Webview 变体（与红线一致）
}

// 权限等级（累进：高等级隐含低等级），与 M5-12.a AclLevel 同源共用一份
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AclLevel { Read, Write, Execute, Network }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermission {
    pub commands: Option<Vec<String>>, // 允许调用的命令白名单（优先于等级）
    pub fs_roots: Option<Vec<String>>,  // 允许读写的文件系统根（canonicalize 前缀校验）
    pub net_hosts: Option<Vec<String>>, // host 白名单（不支持通配）
    pub db_conn_ids: Option<Vec<String>>, // 允许引用的 M4 连接 id
    pub keyring_prefixes: Option<Vec<String>>, // 允许引用的 keyring 条目前缀（不暴露凭据本体）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,                  // 反向域名或 slug，全局唯一
    pub name: String,
    pub version: String,             // semver
    pub author: Option<String>,
    pub kind: PluginKind,
    pub entry_ref: String,           // 指向既有 ScriptMeta/ToolMeta/SkillDef/SessionHandler 的 id
    pub acl_level: AclLevel,         // 声明等级（安装时展示）
    pub permissions: Vec<PluginPermission>,
    pub config_schema: Option<serde_json::Value>,
    pub min_host_version: Option<String>,
    // 无 process/bin、无 webview entry、无第三方代码路径
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState { Installed, Enabled, Disabled, Error { reason: String } }
```

> **form③ 关键点**：`entry_ref` 指向的实体必须**已存在**（`ScriptMeta`/`ToolMeta`/`SkillDef`/`SessionHandler`），插件不携带、不加载任何第三方代码。第三方代码能力由既有脚本库/工具箱/Skill 承担（它们本身已走确认闸门+审计+路径沙箱）。

### 2.2 存储与扫描加载

- 目录：`app_data_dir()/mvp-browser-os/workspace/plugins/<id>/`，每插件一份 `manifest.json` + `grant.json`（授权记录）+ `config.json`（用户配置）。
- 启动扫描：`plugin_load_all()` 遍历 `workspace/plugins/` 解析 manifest → 校验结构 → 校验 `entry_ref` 指向实体存在 → 载入 `PluginState`（默认 `Disabled`）。
- **load/unload/reload = manifest 读写 + 能力注册/注销**，不启动任何进程：
  - 注册：把插件的 `acl_level` + `permissions.commands` 合并进 #7 MCP 工具集与 #12 Skill 列表（仅**展示/编排**层注册，真正执行仍经 M2-4 引用）。
  - 注销：从 MCP/Skill 列表移除该插件贡献的能力。
- 原子写：复用 `src-tauri/src/session.rs:30` `atomic_write`（temp+rename）；`manifest.json` 半截损坏 → 备份为 `.corrupt` 不静默清空，应用仍可启动。

### 2.3 签名/来源（首期）

- 首期**只支持本地路径安装**，不做远程市场（供应链 SB-10）。
- `manifest_hash`：安装时计算并写入 `grant.json`；manifest 内容变更（含权限扩大/缩小）→ 授权失效 → 插件自动 `Disabled` + 提示重新确认（防作者事后提权）。
- 可选签名：M5 首期**不实现**签名校验，但须在 `grant.json` 记录来源路径（本地路径即可），交 A0 裁定是否引入签名。

### 2.4 生命周期状态机

```
install → (Installed/Disabled) → enable → Enabled → (disable) → Disabled → uninstall
                                                      ↓
                                                  error → 记录 reason → 可 disable 重试
```

- **硬约束**：安装后**不得自动启用**（`installed` 即 `Disabled`，需用户显式 `enable`）。

---

## 3. M5-11 插件命令与隔离

### 3.1 命令清单（全部进 ACL，插 `list_artifact_images` 前）

| 命令 | 作用 | 红线备注 |
|---|---|---|
| `plugin_list` | 列出已安装插件 + 状态 | 只读 |
| `plugin_install(path)` | 解析+校验+写 manifest+写 grant+审计；**默认 Disabled** | 校验能力⊆白名单；不自动启用 |
| `plugin_enable(id, grants)` | 展示声明能力 → 用户确认授予 → 能力注册到 MCP/Skill | 运行时二次校验 |
| `plugin_disable(id)` | `enabled=false`；不影响在飞 | 资源释放、能力注销 |
| `plugin_uninstall(id)` | 删 manifest/config/grant + 注销能力 + 审计 | **不删**被引用的共享实体 |
| `plugin_config_get/set(id, key, val)` | 读写 `config.json`；敏感项走 keyring 引用 | K3 |

每条命令必须：过 `check_invocation_source`（与 M4 同款）、进 ACL、审计脱敏。

### 3.2 隔离模型（form③：复用 M2-4，无第二执行路径）

- 插件"执行"=`plugin_enable` 把 `entry_ref` 注册为可编排能力；真正触发时由宿主经 `script_runner::start_run`/`start_command`（`:843`/`:882`）执行既有实体。**插件代码永远拿不到 `AppHandle`，也永远不能调任意 `#[tauri::command]`**（红线，与 2026-09-02 旧卡 §6 禁止项 2 一致）。
- **运行时二次校验 `can_invoke(plugin, command)`**（fail-closed，复用 `security_policy::check_invocation_source` 风格，非新机制）：

```
1) 命令是否在 ACL 白名单（default-commands.toml）？ 否 → Deny(COMMAND_NOT_IN_ACL)
2) 命令是否在 plugin.permissions.commands 白名单（若声明）？ 否 → Deny(NOT_DECLARED)
3) 命令 risk 等级 ≤ plugin.acl_level？ 否 → Deny(LEVEL_EXCEEDED)
4) 涉及文件系统？路径 canonicalize 后是否在 fs_roots 前缀内？ 否 → Deny(FS_OUT_OF_ROOT)
5) 出网？目标 host 是否精确匹配 net_hosts（不支持通配）？ 否 → Deny(NET_HOST_NOT_ALLOWED)
6) risk > Read 或命令自带闸门？ 是 → RequireConfirm（走 request_sync/confirm_sync 两段式）
7) → Allow，且记审计
```

**硬规则**：默认拒绝（任何未显式通过步 1-6 的调用一律 `Deny`，不"警告但执行"）；声明≠授予（以 `grant.json` 运行时记录为准，不以 manifest 为准）；路径 `canonicalize()`+前缀校验防 `..`/软链接逃逸；网络 host **不支持通配**。

### 3.3 授权记录（Grant）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginGrant {
    pub plugin_id: String,
    pub granted_level: AclLevel,             // 用户实际授予（可低于声明）
    pub granted_permissions: Vec<PluginPermission>,
    pub granted_at: DateTime<Utc>,
    pub manifest_hash: String,               // manifest 变更 → 授权失效
    pub source_path: String,                 // 本地安装来源路径（供应链溯源）
}
```

### 3.4 审计契约

| action | 时机 |
|---|---|
| `plugin_install_request` | 生成预览 |
| `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_uninstall` | 状态变更 |
| `plugin_invoke` | 插件触发执行（含 plugin_id/command/decision） |
| `plugin_invoke_denied` | **被拒调用也记**（发现恶意插件的关键线索） |

- `detail` 含 `plugin_id`/`command`/`decision`/`reason`，**禁含** SQL 正文/密码/DSN/token/cookie/Authorization/Set-Cookie（K3）。
- **K5 防刷爆**：`plugin_invoke` 高频明细走独立 `plugin-audit.json`（`workspace/plugins/<id>/audit.json` 或全局 `plugin-audit.json`），摘要进 `audit.json`。拒绝调用同样记 `plugin_invoke_denied`（可低频，但不得刷爆主 `audit.json`）。

---

## 4. M5-12 插件管理 UI 与清理

### 4.1 UI（复用 M4 数据库/调度面板模式 + M2-9 工具箱风格）

- 面板位置：现有 workspace 风格 UI（复用 `M2-tool-library-ui-static-shell` 卡片/详情形态）。
- 展示：插件列表（名称/版本/状态/来源）、权限清单（来自 manifest `acl_level` + `permissions`）、配置表单（`config_schema` 驱动）、运行状态、故障诊断（Error reason）。
- 操作：安装（本地路径）→ 权限确认弹窗 → 启用/禁用/卸载（卸载二次确认）。
- live 调用须等 M5-11 后端 DTO 稳定；否则留 skeleton + 文档化内部 TODO，**不**写可见的"功能说明"文案。
- 纯逻辑 helper：权限标签格式化、config_schema→表单、状态/来源展示、卸载确认态；单测放 `scripts/check-plugin-ui-logic.mjs`（复用 `check-database-ui-logic.mjs` / `check-scheduler-ui-logic.mjs` 范式）。

### 4.2 清理归属（uninstall 范围）

- **删除**：`workspace/plugins/<id>/` 下 `manifest.json` + `grant.json` + `config.json`；从 MCP 工具集与 Skill 列表注销该插件贡献的能力；清该插件运行记录。
- **不删**：被 `entry_ref` 指向的 `ScriptMeta`/`ToolMeta`/`SkillDef`/`SessionHandler`（共享资产，其他插件/用户可能仍引用）。
- 审计 `plugin_uninstall`（保留审计历史，不删）。

---

## 5. 插件任务拆分（subcards，供 A0 签发 M5 实现卡）

> 命名沿用 `A9-M5-A13plus-cards`：M5-10/11 归 **A18**（runtime），M5-12 归 **A19**（UI）。Wave：文档层现可做，产品代码在 M4 PASS 后、A13 workspace 下沉评估之后。

| 子卡 | 目标 | 依赖 | 必改文件（仅实现期） |
|---|---|---|---|
| M5-10.a | `PluginManifest`/`PluginKind`/`AclLevel`/`PluginState`/`PluginPermission` 类型 + `workspace/plugins/` 扫描加载 + 默认 `Disabled` | M4 PASS | `domain.rs` |
| M5-10.b | 签名/来源（manifest_hash 绑定 grant）+ load/unload/reload 能力注册/注销到 MCP(#7)/Skill(#12) | M5-10.a、#7、#12 | `domain.rs`、`bridge.rs`、ACL |
| M5-11.a | `plugin_list/install/enable` + 两段式确认闸门 + 默认 `Disabled` + 写审计 | M5-10.a、M2-4、M2-7/9 | `bridge.rs`、`domain.rs`、ACL |
| M5-11.b | `plugin_disable/uninstall` + 运行时二次校验 `can_invoke` + `plugin-audit.json` 防刷爆 + 清理（不删共享实体） | M5-11.a、`security_policy.rs` | `bridge.rs`、`plugin_runtime.rs`(或并入 domain)、ACL、policy 脚本 |
| M5-12.a | 权限清单/配置 UI（skeleton + 纯逻辑，后端 DTO 稳定后 live） | M5-11 后端 DTO | `src/components/**`、`src/stores/**`、`src/bridge.ts`、`src/types.ts` |
| M5-12.b | 状态/来源/故障诊断 UI + `check-plugin-ui-logic.mjs` | M5-12.a | 同 M5-12.a |

**合并顺序**：M5-10.a → M5-10.b → M5-11.a → M5-11.b → M5-12.a/b（A19 与 A18 可并行收尾）。

---

## 6. 安全阻塞项（security blockers，交 A0 裁定）

| ID | 阻塞项 | 影响形态 | 建议处置 |
|---|---|---|---|
| SB-1 | `withGlobalTauri=true`（`tauri.conf.json:12`）+ `csp=null` | form② | form③ 不依赖全局 Tauri；若选 form② 承载 UI，须对插件 webview 关闭全局注入（且全局 Tauri 须能按 webview 粒度关闭，否则 form② 仍否决）。建议 A13 在 workspace 下沉时封死 `useSystemStore.ts:84` 全局引用，统一走桥封装 |
| SB-2 | `capabilities` 残留 `browser` label（`AI-模型切换与接手清单.md` 记载未修） | form② | 新增 `plugin-*.json` capability 时一并清理 `browser` 残留 label，避免能力漂移（K1） |
| SB-3 | Tauri capability 编译期静态声明，无法运行时按插件动态授予 | form②/③ | form③ 用运行时 `can_invoke` 二次校验（§3.2）兜底；form② 同 |
| SB-4 | 「禁第二执行路径」红线 | form① | form① 否决；form③ 天然满足（复用 M2-4） |
| SB-5 | ACL 末条恒 `list_artifact_images` | 全部 | 插件命令插其前（坑位②） |
| SB-6 | 凭据不进 manifest/审计/前端 state（K3） | 全部 | `config.json` 敏感项走 keyring 引用不落明文；`detail` 脱敏 |
| SB-7 | `audit.json` 1000 上限（K5） | 全部 | 高频 `plugin_invoke` 走独立 `plugin-audit.json` |
| SB-8 | 禁 npm/tokio 新依赖；新增 Cargo 依赖需 A0 指派 | 全部 | M5-10/11 仅用现有 `serde`/`keyring`/`std`，不引新 crate |
| SB-9 | 每命令 source check + ACL | 全部 | 6 条插件命令全过 `check_invocation_source` 且进 ACL |
| SB-10 | 插件市场/远程安装 = 供应链攻击面 | 全部 | 首期仅本地路径安装，不做远程市场 |

---

## 7. 反向用例（M5-10/11/12）

| # | 场景 | 期望 |
|---|---|---|
| N1 | 插件声明 `acl_level=Read`，调用写命令 | `Deny(LEVEL_EXCEEDED)` + 记 `plugin_invoke_denied` |
| N2 | 插件声明 `commands=["list_artifacts"]`，调 `read_file` | `Deny(NOT_DECLARED)` |
| N3 | `fs_roots=["$WORKSPACE"]`，传 `../../etc/passwd` | `Deny(FS_OUT_OF_ROOT)`（canonicalize 后逃逸） |
| N4 | 软链接指向 `/etc` | `Deny(FS_OUT_OF_ROOT)` |
| N5 | `net_hosts=["api.example.com"]`，请求 `evil.com` | `Deny(NET_HOST_NOT_ALLOWED)` |
| N6 | `net_hosts=["*.example.com"]` | 解析即拒绝（不支持通配） |
| N7 | 安装后查状态 | `Installed` 且 **Disabled** |
| N8 | manifest 权限扩大后重装/重载 | 授权失效 → 自动 `Disabled` + 提示重新确认 |
| N9 | 插件被禁用后触发执行 | `Deny(PLUGIN_DISABLED)` |
| N10 | 卸载插件 | 删 manifest/config/grant + 注销能力；**保留**被引用 Script/Tool/Skill；审计历史保留 |
| N11 | 调 ACL 不存在的命令 | `Deny(COMMAND_NOT_IN_ACL)` |
| N12 | 插件尝试 `kind=Process`/`NativeLib` | 拒绝加载（不支持形态） |
| N13 | id 冲突 | 拒绝安装或要求先卸载（不静默覆盖） |
| N14 | 应用退出时插件处于 Enabled | 无独立进程（form③），随既有 `ShutdownCoordinator` 收口；无残留 |
| N15 | 插件高频调用（每秒 100 次） | `plugin-audit.json` 承载明细，主 `audit.json` 不爆 |
| N16 | `entry_ref` 指向不存在的 Script/Skill | 加载/启用拒绝 `ENTRY_REF_NOT_FOUND`，不猜测执行 |

---

## 8. 验收命令（供实现期，本文未执行）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核（已执行，结论见 §0）
grep -n "withGlobalTauri" src-tauri/tauri.conf.json               # true（SB-1 风险点）
grep -c "tauri::command" src-tauri/src/bridge.rs                 # 基线 100+（含 db_*/task_*）
tail -1 src-tauri/permissions/default-commands.toml               # list_artifact_images（K1 末条）
grep -rn "PluginManifest" src-tauri/src/domain.rs || echo "none" # 类型待加

# B. 红线：本预研不得改动 src-tauri 核心（立即跑应为空）
git status --porcelain -- src-tauri/src/ src-tauri/Cargo.toml src-tauri/tauri.conf.json

# C. 契约静态检查（实现后跑）
grep -n "pub struct PluginManifest" src-tauri/src/domain.rs
grep -n "fn can_invoke" src-tauri/src/plugin_runtime.rs           # 或 domain.rs
grep -n "canonicalize" src-tauri/src/plugin_runtime.rs            # 路径逃逸防护
grep -c "plugin_list\|plugin_install\|plugin_enable\|plugin_disable\|plugin_uninstall" \
  src-tauri/permissions/default-commands.toml                       # 期望 5（均插 list_artifact_images 前）

# D. 权限判定单测（实现后跑，覆盖 §7 N1~N6/N9/N11）
# cargo test plugin_perm_

# E. K5 防刷爆（实现后跑）：高频 plugin_invoke 后主 audit.json 不爆
python3 - <<'PY'
import json,os
p=os.path.expanduser("~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/audit.json")
d=json.load(open(p)); denied=[e for e in d if e["action"]=="plugin_invoke_denied"]
print("denied recorded:",len(denied))
PY

# F. 编译与基线（实现后跑）
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | grep -c warning   # 不得 > 基线
```

---

## 9. FORBID 遵守记录

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh` 及三份主文档（A1/A6 高冲突位）。
- 未移动 `NEXT`（现 M5-W0 文档层，产品代码冻结）。
- 未提交、未 push；本文件为新增独立文档，与工作树中 A2/A3/A4/A6/A7/A10/A11 未提交产物无交集。
- 所有「已冻结/已落地」陈述均以来源 file:line 标注；「裁定/建议/开放项」明确区分并交 A0 裁定（§1、§6）。
- 本预研 SUPERSEDED 了 2026-09-02 `plugin-runtime-taskcard`/`plugin-permission-taskcard` 的 form① 进程假设，继承 `A9-M5-plugin-form-feasibility-20260906-0700.md` 的形态③ 裁定。

---

## 本交付（A9 M5 Dispatch Now 整包）索引

- `logs/assist/A9-M5-plugin-system-prework-20260906-1100.md`（本文件）—— M5-10/11/12 契约 + 任务拆分 + 安全阻塞项。
- 继承：`logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`（形态裁定）。
- 配套卡：`A9-M5-A13plus-cards-20260906-0010.md`（A18 runtime / A19 UI）。
