# A10 · M5-W13 Plugin Runtime Stage-I Manifest Lifecycle — Security Review Checkpoint

> 检查点：2026-09-07 20:30 CST · Lane A10（SECURITY REVIEW）
> 基线 HEAD `3c3f460`（ff-only 已最新），工作树干净，未提交未推送。

## VERIFY（启动门禁）
- `WORKSPACE_ID=BACKV3_MAIN` ✅ / 主目录 ✅ / `master` ✅ / `git pull --ff-only` 已最新 ✅ / 工作树干净 ✅
- 遵守「仅 A0 push」。

## SCOPE
board §M5-W13 Lane A10（row 207）：review W13 plugin lifecycle for execution bypass / path escape / secret echo / ACL·source-check gaps / build warning·metric regression.

## 交付物
- `logs/assist/A10-M5-W13-plugin-runtime-security-review-20260907-2030.md`（完整复核）
- 本检查点
- 单补丁：`logs/checkpoints/Lane-A10-M5-W13-plugin-runtime-security-review-20260907-2030.patch`

## VERDICT = PASS_WITH_DEBT（基础层 PASS + 运行时层 PENDING）

### 已闭环（基础层）
- `plugin.rs`（W6 纯策略切片，`mod plugin;` @ main.rs:14）满足：纯函数无执行路径、形态③（`http/https/tool://`）、能力白名单 `PLUGIN_CAPABILITY_V1` 空集合 fail-closed（security_policy.rs:2126）、`contains_credential_leak` 守凭据（:2088）、签名仅结构、状态机 fail-closed、metadata ≤64KiB（domain.rs:1019 单一真源）。
- `check-plugin-policy.py` ACTIVE=1/PENDING=5，`--self-test` ALL_PASS，接 pre-merge.sh:451-454,588-590。

### 未闭环（债 / 待办）
- **F-3（范围缺口）**：W13 Stage-I 生命周期命令（`plugin_install/enable/disable/list/get` + trusted-key）当前**不在树中**（bridge.rs / main.rs / default-commands.toml 零 plugin 命令/ACL；A9 W13 交付仅为 Stage-II 规划卡）。execution bypass / path escape / ACL·来源校验 / 审计脱敏 四项运行时判定须 A9 命令 wave 落地后 A10 复审。
- **F-5（文档不一致）**：构建阈值 board 23% vs A9 HS-12 22% → A0 裁决统一。
- **F-6（夹具缺口·中高）**：`check-plugin-policy.py:121-123` PENDING 违规仅扫 `plugin.rs`，不扫 `bridge.rs`；命令 wave 落地后第二执行路径护栏对实现层失效。修复交 A0 指派 A9（扩展扫描 bridge.rs + 增 `PLUGIN_CMD_SOURCE_CHECK`/`PLUGIN_ACL_TAIL` 码位）。
- **F-4**：供 A9 命令 wave 的前向安全门禁清单（8 条，映射 W13 硬停 + A9 HS-1..HS-19）。
- **F-7（INFO）**：`is_allowed_entry_url` 放行 `http://` 明文，建议落地时收窄为 https/tool 或 localhost。

## 修复指派（均非 A10 权限 / 共享文件冲突）
- F-3：A0 确认 A9 是否本批补实现；若补，A10 复审。
- F-5：统一构建阈值口径。
- F-6：指派 A9 扩展 `check-plugin-policy.py` 扫描 bridge.rs。

## 红线对照（摘要）
| 硬停 | 判定 |
|---|---|
| 191 执行旁路 | ✅ 基础层；⏳ 运行时 PENDING |
| 192 凭据回显 | ✅ 切片无审计；脱敏基元就位 |
| 256 新命令 source-check+ACL+main+bridge/types+policy+tests | ⏳ PENDING（无新命令） |
| 构建指标 23% / 告警不增 | ✅ 无 W13 产品代码新增 |
| 仅 A0 push | ✅ |

## 声明
本批零产品代码、零策略脚本改动（F-6 修复交 A0 指派 A9，避免改高冲突共享脚本）。未实跑 cargo build（W13 无代码新增；基础层 W6 已合且编译通过）。
