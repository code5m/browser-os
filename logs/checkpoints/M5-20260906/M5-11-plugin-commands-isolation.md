# M5-11 插件命令与隔离（install/permissions/audit/uninstall）

> 子卡 ID：**M5-11** · 需求 #15 · `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A18**
> 父卡：`详细设计与实施计划.md` L575（`M5-11 插件命令与隔离`）
> 主预研：`logs/assist/M5-15.a-prework-20260902-1055.md` §4 · `logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`
> 配套：`M5-10-plugin-manifest-lifecycle.md`（manifest 后端）· `M5-12-plugin-ui.md`（管理 UI）

---

## 0. 编号与锚定

- 批次任务号 `M5-11`；需求号 #15；WBS L575 一致。
- 依赖：M5-10 ✅（manifest 解析 + 生命周期稳定）+ M5-2 ✅（capability.rs + 确认闸门）

---

## 1. GOAL

把插件操作封装为 Tauri 命令（含 10 条 M5-10 命令 + 5 条本卡新增），强制两层隔离：① capability 隔离（每个插件只能调用自己 manifest 中声明的 capability）② 资源隔离（每个插件的 `workspace/` 子目录相互隔离，与用户主工作区不交叉）；统一走 `capability.rs` 公共白名单 + 两段式确认闸门 + 独立 `plugin-invokes.json` 审计。

---

## 2. READ

1. `logs/assist/M5-15.a-prework-20260902-1055.md` §4（**全读**，命令/权限/审计/隔离）
2. `M5-10-plugin-manifest-lifecycle.md` §4.3-4.5（生命周期 + 二次确认）
3. `M5-2-rmcp-mcp-policy.md` §4.2-4.5（capability.rs + 确认闸门 + 审计）
4. `src-tauri/src/bridge.rs:688-790`（两段式确认闸门）
5. `src-tauri/src/workspace.rs`（路径解析 + 隔离范式）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/bridge.rs` | 修改 | 新增 `plugin_invoke` / `plugin_invoke_cancel` / `plugin_permissions_get/set` / `plugin_audit_list` / `plugin_storage_get/put/delete`（5 条） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 5 条新命令于 `list_artifact_images` 之前（**末条恒为 `list_artifact_images`**） |
| `src-tauri/src/plugin_runtime.rs` | **新增** | 调用时 capability 校验 + 资源隔离 + `capability.rs` 联合判定 |
| `src-tauri/src/plugin_storage.rs` | **新增** | 插件独立 KV（`plugins_dir()/<id>/storage.json`） |
| `src-tauri/src/plugin-invokes.json` | 持久化 | 独立 500 上限 FIFO |
| `src/bridge.ts` `src/types.ts` | 新增 | TS 镜像 |

---

## 4. 关键契约

### 4.1 5 条新命令

| 命令 | 入参 | 出参 | 闸门 | ACL 风险 |
|---|---|---|---|---|
| `plugin_invoke` | `{ id, capability, params }` | `Result` | **按 capability risk** | 按 `capability` |
| `plugin_invoke_cancel` | `{ invoke_id }` | `bool` | — | Low |
| `plugin_permissions_get` | `{ id }` | `PluginPermission[]` | — | Low |
| `plugin_permissions_set` | `{ id, allowed: bool }` | `bool` | **Confirm**（开关 capability） | High |
| `plugin_audit_list` | `{ id, filter? }` | `InvokeRecord[]` | — | Low |
| `plugin_storage_get` | `{ id, key }` | `Value` | — | Low |
| `plugin_storage_put` | `{ id, key, value }` | `bool` | — | Low |
| `plugin_storage_delete` | `{ id, key }` | `bool` | — | Low |

> **注**：上表 8 条（不是 5 条），加 M5-10 的 10 条共 **18 条**。**M5-11 增量 8 条**。

### 4.2 两层隔离

**capability 隔离**：
- `plugin_invoke` 时强制校验 `manifest.capabilities` 是否含目标 capability
- 校验失败 → `CAPABILITY_NOT_ALLOWED`（与 MCP 一致）
- `plugin_permissions_set` 受 `capability.rs` 二次校验（开关 capability 不能超出 host 自身能力）

**资源隔离**：
- 插件独立目录：`plugins_dir()/<id>/workspace/`
- 插件独立 storage：`plugins_dir()/<id>/storage.json`（`atomic_write`）
- 插件独立审计：`plugin-invokes.json`（按 plugin_id 分组索引）
- **禁**跨插件读 workspace / storage
- **禁**插件读用户主 workspace（除非显式 capability = `workspace.read`）

### 4.3 `plugin_invoke` 判定顺序（与 MCP §4.3 复用）

1. 插件状态 = `Enabled`？否 → `PLUGIN_DISABLED`
2. 目标 capability 在 `manifest.capabilities`？否 → `CAPABILITY_NOT_ALLOWED`
3. capability 在 `capability.rs` 白名单？否 → `CAPABILITY_NOT_ALLOWED`
4. 风险等级与 `policy` 比对（同 MCP §4.3 第 2-4 步）
5. `requires_confirm` → 走两段式闸门
6. 资源路径校验（含 `..` / 越界 → 拒绝）
7. 执行 → 写 `plugin-invokes.json` + 摘要 `audit.json`

### 4.4 审计契约

- `plugin_invoke` 写 `plugin-invokes.json`（独立 500 上限）
- 摘要进 `audit.json`（`action=plugin_invoke`）
- `detail` 含 `id` / `capability` / `params_summary` / `result` / `duration_ms`
- 禁记 params / result 正文（K3 + 体量防爆）

### 4.5 升级/回滚

- 升级：保留 `storage.json` + `workspace/`，仅替换 manifest 与资源
- 回滚：恢复上次 `manifest` + `storage.json` + `workspace/`
- `manifest_hash` 变更 → 已启用插件自动 `Disabled`（M5-10 §4.3）

---

## 5. FORBID

- **不**让 `plugin_invoke` 绕过 capability 校验
- **不**让跨插件 / 跨用户主 workspace 越权访问
- **不**让 `plugin_storage_put` 存 token/密码（K3）
- **不**让 ACL 末条不再是 `list_artifact_images`（K1）
- **不**让 `capability.rs` 在多文件定义
- **不**让升级/回滚丢失用户数据
- **不**破 K1/K3/K5
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. ACL 末条
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -5

# B. 8 条新增
grep -nE "plugin_(invoke|invoke_cancel|permissions_get|permissions_set|audit_list|storage_get|storage_put|storage_delete)" src-tauri/bridge.ts

# C. 反向用例
# N1: 插件 invoke 未声明的 capability → CAPABILITY_NOT_ALLOWED
# N2: 跨插件读 workspace → 阻断
# N3: 插件读用户主 workspace 无 workspace.read capability → 阻断
# N4: plugin_storage_put 写 token → 阻断
# N5: 升级后 storage.json 丢失 → 阻断
# N6: 资源路径含 .. → 阻断
# N7: CLI 路径调用 plugin_invoke Dangerous → 拒绝

# D. 审计不被刷爆
python3 - <<'PY'
import json, os
p = os.path.expanduser("~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/plugin-invokes.json")
if os.path.exists(p):
    d = json.load(open(p))
    print("plugin-invokes entries:", len(d), "(上限 500)")
PY

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
| 1 | 8 条新命令全部入 `bridge.rs` + `bridge.ts` | 命令 B |
| 2 | ACL 末条仍为 `list_artifact_images` | 命令 A |
| 3 | capability 校验生效 | 单测 N1 |
| 4 | 资源隔离（跨插件 / 越界） | 单测 N2/N3/N6 |
| 5 | 凭据不入 `storage.json` | 单测 N4 |
| 6 | 升级不丢 storage | 单测 N5 |
| 7 | CLI 路径拒绝 | 单测 N7 |
| 8 | 审计上限 500 | 命令 D |
| 9 | `cargo test` 全绿 | 命令 E |
| 10 | `pre-merge.sh` ALL_PASS | 命令 F |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 越权访问 workspace | **红线失守**：阻断 |
| `plugin_storage` 存凭据 | 立即修 + 清泄露 + 提示用户轮换 |
| 升级丢数据 | 阻断（K1） |
| 闸门被绕过 | 阻断 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L575 `[ ]` → `[x]`
2. `后续需求TODO.md` §15 状态 `PARTIAL`（留 `M5-12` UI）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-12`
4. `logs/checkpoints/M5-11.a-2026MMDD-HHMM.md`

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A18 实施填
- **NEXT**：M5-12（插件管理 UI），A19

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
