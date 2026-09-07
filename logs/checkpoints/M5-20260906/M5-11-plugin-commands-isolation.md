# M5-11 插件命令与隔离（install/permissions/audit/uninstall）

> 子卡 ID：**M5-11** · 需求 #15 · `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A18**
> 父卡：`详细设计与实施计划.md` L575（`M5-11 插件命令与隔离`）
> 主预研：`logs/assist/M5-15.a-prework-20260902-1055.md` §4 · `logs/assist/A9-M5-plugin-form-feasibility-20260906-0700.md`
> 配套：`M5-10-plugin-manifest-lifecycle.md`（manifest 后端）· `M5-12-plugin-ui.md`（管理 UI）
>
> **W3** BLOCKED（待 M5-10 manifest 解析 + M5-2 capability.rs + 确认闸门）· **W4** ACTIVE（A9 W4 仍 SUPPORT DOCS ONLY）· **W5** ACTIVE（A9 W5 仍 SUPPORT DOCS ONLY）· **W6** PUSHED · **A9 升级为 START PRODUCT CODE** · `5f92ece` 拣入（M5-11 commands_isolation shell + 5 stub 命令 ACL 占位 + capability 校验骨架 + audit shape 仅 key_hash_only；详见本卡顶部 `[W6 next-card acceptance criteria]` 段；与 M5-10 W6 共享 hard stop 集）· **W7** RECONCILIATION（A3 W7 mcp_* 3 命令 + A11 W7 pre-merge FAIL 3 red lights + A6 W7 wiring 在 `6c1f30e` / `daa10f6` / `a29b796` 已拣入 master；A9 M5-10/11 在 W7 仍无新命令落地，5 stub 维持 `Err("not-implemented-in-W6")`；A1 W7 整包本卡修订未进 master，留 W8 整包合并拣入；详见 `M5-0-overview.md` 顶部 `[W7 reconciliation]` 段 + `M5-13-verification-matrix.md` 顶部 `[W7 verification scope]` 段）· **W8** ACTIVE（**A9 W8 = POLICY REVIEW ONLY** —— 与 M5-10 W8 共享 review scope：plugin 命令 ACL 维护 / stub 错误结构不变 / audit shape 维持 key_hash_only / 无 install/enable/delete 真实命令；**不**写 plugin runtime 命令；A3 W8 = HOLD/NO ASSIGNMENT；详见 `M5-0-overview.md` 顶部 `[W8 active]` 段 + `M5-13-verification-matrix.md` 顶部 `[W8 verification scope]` 段）
> **W8 reconciliation**（2026-09-07 14:30 CST · A0 拣入）：A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`**（53 files +6038 -101）+ **`94e763e fix(M5-W8,A3): close MCP policy phase debt`**（MCP policy phase debt 关闭 · `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 verification delta 功能性 ALL_PASS）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（3 份 W8 patch 文件空白规范化）4 commit 中拣入本卡 W8 修订：① A9 W8 plugin policy review 落地（与 M5-10 共享 review scope，commands_isolation 维持 pure，5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_get_or_put_or_delete` 维持 `Err("not-implemented-in-W6")`）；② A1 W7 reconciliation 整包合并拣入（A11 W7 pre-merge FAIL 3 red lights 配方归档）；③ A1 W8 reconciliation 整包合并拣入（本卡头部 + [W8 active] 段 + 顶部 history 链更新）；**A1 W7 + W8 整包合并拣入 9 + 11 = 20 文件均含本卡修订**。
> **W9** ACTIVE（**A9 W9 = PLUGIN REVIEW ONLY** —— plugin surface 复审：commands_isolation 维持 pure / stub 错误结构不变 / audit shape 维持 key_hash_only / 无 install/enable/delete 真实命令；**不**实施 plugin runtime；A3 W9 = POLICY/REVIEW ONLY MCP policy current-phase green + 后续 M5-2.b 卡预备，不实施 rmcp server/listener/network；详见 `M5-0-overview.md` 顶部 `[W9 active · 2026-09-07 14:30 CST]` 段 + `M5-13-verification-matrix.md` 顶部 `[W9 verification scope]` 段）
> **W10** ACTIVE（**A9 W10 = PLUGIN RUNTIME PLAN ONLY**（plugin commands_isolation runtime dispatch 卡预备：install/enable/delete/list 命令序列 + permission isolation + audit redaction + 5 stub 真实化路径；**不**实施 plugin runtime）；**plugin install/enable/delete/download runtime 仍 LOCKED**（W10 Hard Stop L207）；详见 `M5-0-overview.md` 顶部 `[W10 active · 2026-09-07 16:00 CST]` 段 runtime-lock 状态表）
> **W10 PUSHED**（2026-09-07 18:30 CST · A0 拣入 `5226aad` = HEAD）：A0 在 **`ba78092 feat(M5-W10,A3): MCP stdio-prep skeleton (feature-gated, read-only, std-only)`**（A3 W10 实施期）+ **`5226aad feat(M5): integrate W10 MCP stdio prep`**（A0 W10 整包合并拣入）两 commit 中拣入 W10：① A9 W10 plugin commands_isolation runtime plan only 落地（commands_isolation 维持 pure / 5 stub `plugin_invoke/cancel/permissions_get/audit_list/storage_get_or_put_or_delete` 维持 `Err("not-implemented-in-W6")` / audit shape 维持 key_hash_only）；② A1 W10 reconciliation 整包合并拣入；③ A11 W10 verification delta 收口（`check-plugin-policy.py --self-test` PASS(ACTIVE=6) + 5 stub 不变）；**M5-11 关联债 DEBT-04（plugin commands_isolation runtime）= W11+ 仍挂账**。
> **W11** ACTIVE（**A9 W11 = PLUGIN DOCS/POLICY ONLY**（W12/W13 plugin commands_isolation staged cards 预备；**不**实施 plugin runtime / 不破 K1 ACL 末条恒为 `list_artifact_images`）；**plugin install/enable/delete/download runtime 仍 LOCKED**（W11 Hard Stop L207）；详见 `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段 runtime-lock 状态表 10 行）

---

## [W6 next-card acceptance criteria · 2026-09-06 19:25 CST] A9 M5-11 W6 实施期 acceptance criteria（commands_islolation shell + 5 命令 ACL stub + capability 校验骨架 + audit shape · 不接真实 install runtime / audit 仅 key_hash 不带 value / 不破 K1 ACL 末条恒为 list_artifact_images）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L160（**A9 M5-W6** *"Implement M5-10/M5-11 plugin manifest/lifecycle policy slice"*）+ L164-170 硬约束 + A3 W3 MCP 两段式确认闸门（`src-tauri/src/bridge.rs:688-790` 既有 pattern）+ A2 W2 constants centralized + A4 W4 agent_memory 审计基元（audit 仅 key_hash）。
> **与 M5-10 W6 的边界（A9 W6 同 lane 双卡派发）**：
> - **M5-10 W6 范围**（见 `M5-10-plugin-manifest-lifecycle.md` 顶部 [W6 next-card AC] 段）：PluginManifest DTOs + validation + lifecycle state machine + permission manifest rules + policy script；**0 新命令**。
> - **M5-11 W6 范围（本卡）**：commands_islolation shell + 5 命令 ACL stub（**不**接入真实业务 handler）+ capability 校验骨架（基于 M5-10 AC-3 PermissionManifestRule）+ plugin-invokes.json audit shape（基于 A4 W4 agent_memory 审计基元）；**仍 0 命令接业务**，W6 仅完成"命令落地骨架 + ACL stub + audit shape"，**真实 install/uninstall runtime 在 W7+**。
> **消费依赖（已落地，A9 W6 可直接接入）**：
> - **A3 W3 MCP 两段式确认闸门**（`12f1cff` 拣入）—— A9 W6 5 命令 stub 复用同款闸门 pattern。
> - **A4 W4 agent_memory 审计基元**（`1610939` 拣入）—— A9 W6 plugin-invokes.json audit **复用** key_hash only 模式，**不**写 value 字段。
> - **M5-10 W6 PermissionManifestRule**（同 lane A9）—— A9 W6 capability 校验骨架直接调 `M5-10::permission_manifest_rule::check(...)` 纯函数。
> - **A2 W2 constants**（`712a14c`）—— A9 W6 audit key 长度 / plugin-invokes.json 容量上限**复用**既有常量。
> **A1 W6 角色**：A1 W6 **不**改 §1~§11 决策史；仅在头部加本 `[W6 next-card acceptance criteria]` 段，**明确 A9 W6 实施期 4 项 AC + 5 项 hard stops**（与 M5-10 W6 共享 hard stop 集），供 A9 / A10 / A11 / A0 验收。

### W6 A9 M5-11 实施期 acceptance criteria（4 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **commands_islolation shell 冻结** | `src-tauri/src/plugin_runtime.rs`（或 `src-tauri/src/plugin_invoke.rs`，A9 W6 决定；A9 W6 范围**不**创建此文件也可，**不强制**）追加 5 个 stub 纯函数（**仅**返回 `Err("not-implemented-in-W6")`）：① `plugin_invoke(plugin_id, capability, payload) -> Result<Json, InvokeError>` ② `plugin_invoke_cancel(invoke_id) -> Result<(), InvokeError>` ③ `plugin_permissions_get(plugin_id) -> Result<PermissionSummary, InvokeError>` ④ `plugin_audit_list(plugin_id, page) -> Result<AuditPage, InvokeError>` ⑤ `plugin_storage_get/put/delete(plugin_id, key)`（3 个合一 stub） —— 5 个 stub 函数**仅**接受 source check + ACL 校验，**不**做真实 IO，**不**接 M5-10 真实 manifest / lifecycle / 真实 capability registry；**仅**保留签名 + 参数骨架 | `cargo test --manifest-path src-tauri/Cargo.toml plugin_runtime` PASS（如创建文件）+ A10 抽查 5 stub 均为纯函数 |
| AC-2 **5 命令 ACL stub**（**仅 ACL 条目，不接业务 handler**）| `src-tauri/permissions/default-commands.toml` 在 `list_artifact_images` **之前**插 5 条占位 ACL 条目（**仅**条目+name 字段；**不**在 `src-tauri/src/bridge.rs` / `main.rs` 注册 handler；**不**在 `src/bridge.ts` / `src/types.ts` 加 TS 镜像）：① `plugin_invoke` ② `plugin_invoke_cancel` ③ `plugin_permissions_get` ④ `plugin_audit_list` ⑤ `plugin_storage_get/put/delete`（合一）—— ACL 末条仍为 `list_artifact_images`（K1 严守）| `grep -nE 'plugin_invoke\|plugin_permissions_get\|plugin_audit_list\|plugin_storage' src-tauri/permissions/default-commands.toml` 命中 5 条 + `grep -n 'list_artifact_images' src-tauri/permissions/default-commands.toml` 末条仍在 |
| AC-3 **capability 校验骨架** | `src-tauri/src/plugin_runtime.rs`（如 A9 W6 创建）追加 `check_plugin_capability(plugin_id, capability) -> Result<(), CapabilityError>` 纯函数：① 接受 `&PluginManifest`（M5-10 DTO）+ `&CapabilityRegistry`（capability.rs 既有）② 调用 M5-10 AC-3 `PermissionManifestRule::check(...)` ③ 返回 `CapabilityError::NotInWhitelist` / `CapabilityError::ManifestMissing` / `CapabilityError::Disabled` 三种 —— **不**做真实插件调用 / **不**做 IO | 单测 + A10 抽查 + M5-10 AC-3 共享验证 |
| AC-4 **plugin-invokes.json audit shape** | ① `PluginInvokeRecord { invoke_id, plugin_id, capability, payload_key_hash, key_hash_only: true, status, at }` schema 在 `domain.rs` 冻结（**payload 仅 key_hash** 字段，**不**含 value；参考 A4 W4 审计基元）② `serialize_audit(record) -> serde_json::Value` 纯函数 ③ 容量上限：单条 `payload_key_hash` 16 字节 hex（**复用** A2 W2 `HASH_HEX_LEN_SHORT=16` 常量）；`plugin-invokes.json` 文件总字节 ≤ `MAX_TEXT_FIELD_BYTES=64KiB`（**复用** A2 W2 常量，超限 FIFO 裁剪）④ 不写 secret / token / DSN（隐私断言 0 命中）| `cargo test --manifest-path src-tauri/Cargo.toml plugin_runtime` PASS + A10 抽查 audit JSON 无 value 字段 |

### W6 A9 M5-11 实施期 hard stops（5 项，与 M5-10 共享集 + 本卡强化）

| HS | 约束 | 来源 |
|----|------|------|
| W6-HS1 | **5 命令仅 stub**（**不**接业务 handler / **不**在 `bridge.rs` / `main.rs` 注册；ACL 条目占位即可；真实 install/uninvoke runtime 在 W7+）| PARALLEL_COMMAND_BOARD L160 + L168 + L169（*"prefer no command in W6"*）|
| W6-HS2 | **audit 仅 key_hash 不带 value**（`PluginInvokeRecord.payload_key_hash` 16-hex sha256 摘要；**不**含 payload value / 敏感字段 / DSN / token）| A4 W4 audit 模式 + 隐私双扫 + K 隐私全网 |
| W6-HS3 | **capability 真源单点**（`check_plugin_capability` 接受 `&CapabilityRegistry` 参数；**不**在 `plugin_runtime.rs` 内嵌 capability 字面量白名单）| M5-10 W6 AC-3 + M5-2 §4.2-4.3 + A3 W3 |
| W6-HS4 | **不破 K1（ACL 末条恒为 `list_artifact_images`）** —— 5 条 stub ACL 必须插在 `list_artifact_images` **之前** | M5-11 §5 FORBID + 全局 K1 |
| W6-HS5 | **不接 M5-10 真实 lifecycle / 不做网络 / 不做下载 / 不做签名强制** —— 5 stub 函数**不**调 `transition()` 真实跑状态机（**仅**返回 `Err("not-implemented-in-W6")`）| PARALLEL_COMMAND_BOARD L168 + M5-10 W6 W6-HS1+HS2 |

### W6 验证清单（供 A11 收口）

- `cargo test --manifest-path src-tauri/Cargo.toml plugin_runtime` PASS（如 A9 W6 创建 module）
- `cargo test --manifest-path src-tauri/Cargo.toml` 全绿（无新增 warning > 0）
- `python3 scripts/check-plugin-policy.py --self-test` PASS（ACTIVE=6，**新增** 1 条 `PLUGIN_AUDIT_KEY_HASH_ONLY`，A9 W6 落地时同步加码）
- `python3 scripts/check-plugin-policy.py` PASS
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- `grep -nE 'plugin_invoke\|plugin_permissions_get\|plugin_audit_list\|plugin_storage' src-tauri/permissions/default-commands.toml` 命中 5 条 ACL stub
- `grep -nE 'plugin_invoke\|plugin_permissions_get\|plugin_audit_list\|plugin_storage' src-tauri/src/bridge.rs src-tauri/src/main.rs src/bridge.ts src/types.ts` **W6 期间 0 命中**（不接业务 handler；W7+ 才接）
- `grep -nE 'tauri::Manager\|std::fs::write\|std::fs::read\|reqwest\|ureq' src-tauri/src/plugin_runtime.rs src-tauri/src/plugin_invoke.rs` 0 命中（无真实 IO / 无网络）
- A11 比对 `domain.rs` `PluginInvokeRecord` 与 A4 W4 audit schema 0 drift
- **A10 复审 PASS**（5 stub 纯函数 / ACL 占位 / capability 真源单点 / audit 仅 key_hash / 无 IO / 无网络 / K1 严守）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W6-*.md`

### W6 A1 不修订范围（本卡）

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W6 **不动**（决策史保持 W0 原文；W6 AC 在本顶部段单列；M5-11 §3 WRITE 5 命令在 W6 **仅** ACL stub，**不**接业务 handler；真实 18 命令集在 W7+）。
- **三份主文档 / ACL 末条 / Capability / pre-merge.sh / scripts/**：A1 W6 不动（5 命令 ACL stub 由 A9 W6 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。

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
