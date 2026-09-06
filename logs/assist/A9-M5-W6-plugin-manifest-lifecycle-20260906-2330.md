# A9 · M5-W6 Delta — 插件 manifest/生命周期策略切片（START PRODUCT CODE）

> 生成：2026-09-06 23:30 CST · Lane A9（M5-W6 · START PRODUCT CODE · 整包交付）
> BASE：`77b1e3e`（HEAD：`docs(M5): dispatch W6 graph UI and plugin lanes`；已与 `origin/master` 同步）
> 性质：**策略切片产品代码 + 文档**。W6 A9 由 W4/W5 的 SUPPORT DOCS ONLY 升级为 START PRODUCT CODE：实现 M5-10/M5-11 插件 manifest/生命周期**策略切片**（DTO + 纯校验 + 生命周期状态机 + 权限清单规则 + 策略脚本）。
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W6 Parallel Dispatch → A9：**START PRODUCT CODE** — Implement M5-10/M5-11 plugin manifest/lifecycle policy slice: DTOs, validation, lifecycle state machine, permission manifest rules, policy script. | Allowed: `src-tauri/src/domain.rs`, optional `src-tauri/src/plugin.rs`, `src-tauri/src/security_policy.rs`, `scripts/check-plugin-policy.py`, `scripts/pre-merge.sh`, focused Rust tests/checkpoint. | Must Deliver: Policy self-test/default PASS, bounded metadata, no secrets, no network/install runtime.（行 159）
> 配套前序：W4 `A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md`（已拣入，对齐 A5 Agent/Skill 策略 + 能力碎片化发现）；W5 `A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md`（已对照 A7 实际代码复核，对齐 A6 UI + A7 graph）。

---

## 0. 一句话结论

按 W4/W5 既定设计落地插件 manifest/生命周期**纯策略切片**：`domain.rs` 新增 `PluginManifest`/`PluginEntry`/`PluginCapability`/`PluginSignature`/`PluginState` DTO；新 `plugin.rs` 提供 `validate_plugin_manifest`（schema 边界 + 凭据泄露 + 形态③入口 + 能力白名单 + 哈希/签名结构）、`verify_plugin_signature_structure`（仅结构，真验签留运行时）、`can_transition`/`transition`（状态机，无文件 I/O）、`permission_preview_for_plugin`（复用 A5 `PermissionPreview`）；`security_policy.rs` 加 `PLUGIN_CAPABILITY_V1` 单一真源 + `check_plugin_capabilities`；新 `check-plugin-policy.py`（1 ACTIVE + 5 PENDING）接入 `pre-merge.sh`。**零运行时安装/卸载/下载/执行、零新增命令、零 secret、零新依赖**；`cargo check` 仅 3 条他 lane 既有告警，11/11 单测通过，策略脚本 self-test/default 全 PASS。

---

## 1. 实现清单（W6 A9 产品代码）

| 文件 | 改动 | 内容 |
|---|---|---|
| `src-tauri/src/domain.rs` | 追加（L2117 后） | `PluginManifest`/`PluginEntry`/`PluginCapability`/`PluginSignature`/`PluginState` DTO；`PluginSignature` 默认 `algorithm="Ed25519"`；`PluginState` 8 态枚举（rename snake_case，可序列化） |
| `src-tauri/src/plugin.rs` | **新增** | 纯函数切片：`DANGEROUS_PLUGIN_CAPABILITIES` + `capability_acl_level` + `permission_preview_for_plugin`；`validate_plugin_manifest`；`verify_plugin_signature_structure`；`can_transition`/`transition`；边界辅助；`#[cfg(test)]` 11 例 |
| `src-tauri/src/security_policy.rs` | 扩展 | `PolicyError` 增 `InvalidPluginManifest`/`InvalidPluginSignature`/`PluginStateTransition`（含 Display）；`PLUGIN_CAPABILITY_V1`（空，单一真源）+ `check_plugin_capabilities` + 单测 |
| `src-tauri/src/main.rs` | 1 行 | `mod plugin;` |
| `scripts/check-plugin-policy.py` | **新增** | 插件策略夹具（ACTIVE+PENDING 码位，--self-test/默认/--expect-pending） |
| `scripts/pre-merge.sh` | 2 处 | 默认门禁 + 自检接入 `check-plugin-policy.py` |

> 验证全绿：`cargo fmt --check` 通过；`cargo check --locked` 仅 3 条他 lane 既有告警（grid_process 等，非本切片）；`cargo test --manifest-path src-tauri/Cargo.toml plugin` → **11 passed**；`python3 scripts/check-plugin-policy.py --self-test` → `PLUGIN_SELF_TEST=ALL_PASS`（ACTIVE=1 PENDING=5）；默认模式 → `PLUGIN_POLICY=PASS`；`bash -n scripts/pre-merge.sh` 通过；K1 `list_artifact_images` 仍为 ACL 末条（L118）。

---

## 2. 关键设计决策（承 W4/W5，并对齐 M5-10/11 卡）

1. **能力白名单单一真源落 `security_policy.rs`**：`PLUGIN_CAPABILITY_V1` 放在 `security_policy.rs`（与 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1` 同文件，贴合 security_policy.rs §M5-4/5 头注释「Skill / Agent / MCP / Plugin / A2A 共用本文件」）。MCP 的 `MCP_CAPABILITY_V1` 仍在 `domain.rs`——这是 W4 §4 已记录的**能力真源碎片化待收口项**；W6 不破坏既有 A3/A5 门禁，故 Plugin 随 Skill/Agent 落 security_policy.rs。完整收口（建 `capability.rs` 统一四处）仍交 A0 裁决（W4 §4 延续）。check-plugin-policy.py 的 `PLUGIN_CAP_SINGLE_DEF` 守 `PLUGIN_CAPABILITY_V1` 全仓**唯一定义**。
2. **形态③在类型与校验层即排除 form①/②**：`PluginEntry.entry_url` 仅允许 `http(s)://`/`tool://`；`validate_plugin_manifest` 拒绝 `javascript:`/`data:`/`file:` 等。`check-plugin-policy.py` 的 `PLUGIN_FORM_THREE` 守插件模块无 `window.__TAURI__` 越权 / 独立 webview 自起 / `std::process` 独立进程。不引入 `form` 枚举字段（保持与 M5-10 §4.2 schema 一致）。
3. **签名仅结构校验（W6 不引加密 crate、不联网）**：`verify_plugin_signature_structure` 校验 `algorithm=="Ed25519"` + `key_id` 非空 + `value` 为 base64 字符集（不解码）+ `signed_at` 为 ISO8601 边界。**真 Ed25519 验签 + `trusted-pubkeys.json` 信任根（SB-11）由运行时 lane 在本地完成**。这与 W6 FORBID「no signature enforcement beyond pure validation unless fully local」一致，且未新增 `ed25519-dalek` 依赖（Cargo.toml 当前无该 crate，不破「无新依赖」）。
4. **生命周期状态机纯函数（无文件 I/O）**：`PluginState` 8 态 + `can_transition` 合法边（承 M5-10 §4.3：`Discovered→Validating→SignedOk→Loaded→Enabled⇄Disabled`；`SignedFailed/任意→Uninstalled`）；`transition` 受控 fail-closed。**不做 plugins_dir 解包、不读写磁盘清单、不启用/卸载真实插件**（W6 FORBID）。
5. **权限预览复用 A5 `PermissionPreview`**：`permission_preview_for_plugin` 返回 `PermissionPreview { gate: AclLevel, capabilities: Vec<String> }`，闸门取 capabilities 中最高风险档（`DANGEROUS_PLUGIN_CAPABILITIES` → `Dangerous`，其余 → `Confirm`，无能力 → `Safe`）。与 A5 `AgentDef::permission_preview` 同形，供 A6 `PermissionPreviewModal.vue`（W5 已对齐）直渲——**Agent/Skill/Plugin 三态一个类型一个组件一个 `AclLevel` 真源**（W5 §2 闭环）。
6. **能力→闸门档映射为占位集中**：`DANGEROUS_PLUGIN_CAPABILITIES` 初为空集合占位，能力逐个评估追加 `PLUGIN_CAPABILITY_V1` 时同步在此标记风险档，不另立文件（与单一真源一致）。映射函数 `capability_acl_level` 置于 `plugin.rs`（复用 `AclLevel` 来自 domain.rs）；未改动 A5 `agent.rs` 的既有映射，避免跨 lane 编辑（W4 §4 收敛建议仍适用）。

---

## 3. 策略脚本 `check-plugin-policy.py`（镜像 A5 `check-agent-skill-policy.py` 范式）

| 类别 | 码位 | 守什么 |
|---|---|---|
| ACTIVE | `PLUGIN_CAP_SINGLE_DEF` | `PLUGIN_CAPABILITY_V1` 在 security_policy.rs 唯一定义（能力单一真源，不漂移/不重复） |
| PENDING（插件域存在才判） | `PLUGIN_SECOND_PATH` | 插件模块无第二执行路径（`std::process`/`Command::new`/`tokio::spawn`） |
| | `PLUGIN_INLINE_SHELL` | 无内联脚本/裸 shell（`InlineScript`/`RawShell`/`sh -c`） |
| | `PLUGIN_SIG_BYPASS` | 无跳过/绕过签名校验（`todo!`/`unimplemented!`/`panic!`/`unwrap_or(true)`） |
| | `PLUGIN_FORM_THREE` | 仅形态③（无 `window.__TAURI__` 越权 / 独立 webview / `std::process` 独立进程） |
| | `PLUGIN_NO_SECRETS` | manifest/生命周期代码不持有凭据字段（`password`/`secret`/`token`/`api_key` 等） |

探针 `_plugin_present`：domain.rs 含 `PluginManifest` / security_policy.rs 含 `PLUGIN_CAPABILITY_V1` / plugin.rs 存在 → 任一即在，PENDING 码位自动接管（与 A5 同族探针范式）。门禁只扫 `src-tauri/src/plugin.rs`（避免 domain.rs 等共享文件误伤）。三模式：`--self-test`（1 好样本 + 5 坏样本变异防呆）、默认（真仓判定）、`--expect-pending`（PENDING 集合未变化）。已接入 `pre-merge.sh` 默认门禁与自检。

---

## 4. FORBID / 冲突规避（W6 切片）

- 仅改/新增 W6 允许的 6 个文件；未碰 `bridge.ts`/`types.ts`/graph UI/卡片（A1/A8 在 reconcile 与写 graph UI，工作树已见 `M5-0-overview.md`/graph 组件修改态，本切片不触）。
- **无安装/卸载运行时**：`plugin.rs` 不触碰 `plugins_dir`、不读写磁盘清单、不调 `std::fs` 解包；`transition` 不持久化。
- **无下载/执行/启用真实插件**：纯逻辑；无 `tauri::api` 下载、无命令调用。
- **prefer 不新增命令**：W6 未加任何 Tauri 命令（M5-10/11 的 18 条命令属运行时 lane A18/A19，不在本切片）；故 ACL 末条 `list_artifact_images`（L118）未变，K1 守。
- **无 secret 持久化/泄露**：复用 `security_policy::contains_credential_leak`；`metadata` 体量受 `MAX_TEXT_FIELD_BYTES`；`PluginManifest` 无凭据字段；审计/日志不记正文（本切片无审计写入）。
- **无新依赖**：仅用既有 `serde`/`serde_json`/`std`；base64/semver/chrono/ed25519 均不引入（手动校验）。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。

---

## 5. 验收（只读 + 测试，零运行时副作用）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check        # 期望：通过
cargo check --manifest-path src-tauri/Cargo.toml --locked 2>&1 | tail -3   # 期望：仅他 lane 既有告警，Finished
cargo test --manifest-path src-tauri/Cargo.toml plugin 2>&1 | grep "test result"   # 期望：11 passed
python3 scripts/check-plugin-policy.py --self-test                 # 期望：PLUGIN_SELF_TEST=ALL_PASS
python3 scripts/check-plugin-policy.py                             # 期望：PLUGIN_POLICY=PASS
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：118（K1 末条未变）
```

结果：fmt 通过；check 仅 3 条他 lane 告警（本切片 `#![allow(dead_code)]` 未新增）；11/11 通过；策略 self-test/default PASS；K1 末条未变。

---

## 6. Lane Output Template

```text
LANE=A9
STATUS=PASS
BASE=77b1e3e (HEAD: docs(M5): dispatch W6 graph UI and plugin lanes; 已 git pull --ff-only 同步 origin/master)
HEAD=logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.patch
FILES=src-tauri/src/domain.rs, src-tauri/src/plugin.rs, src-tauri/src/security_policy.rs,
      src-tauri/src/main.rs, scripts/check-plugin-policy.py, scripts/pre-merge.sh,
      logs/assist/A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md,
      logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md
VERIFY=cargo check 仅 3 条他 lane 告警；11/11 plugin 单测通过；check-plugin-policy.py self-test/default PASS；
      K1 list_artifact_images 仍为 ACL 末条(118)；fmt 通过
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md
MERGE_NOTES=见 §7
NEXT=A18 (M5-10/11 运行时：18 命令/plugins_dir 解包/真 Ed25519 验签/trusted-pubkeys.json) / A19 (M5-12 UI)，
      在 §2 策略切片之上接入；能力真源统一收口(capability.rs) 仍交 A0 裁决
```

## 7. MERGE_NOTES

- **冲突**：无。本切片文件与 W6 其他 lane（A1 卡片 reconcile / A8 graph UI / A2/A4/A5/A7/A10 评审）无交集；`domain.rs` 追加在 graph 常量之后，`security_policy.rs` 仅增 plugin 段，`main.rs` 仅加 `mod plugin;`，`pre-merge.sh` 仅加 2 段。
- **与 W4/W5 衔接**：W4 已对齐 A5 Agent/Skill 策略（含能力真源碎片化发现）；W5 已对齐 A6 UI（权限预览复用 `PermissionPreview`）+ A7 graph（节点注册表加 `Plugin` 建议等）。本切片把 W4/W5 的设计**落地为可编译、可测、受门禁守护的产品代码切片**，未偏离前序结论。
- **对 A18/A19 的实施契约**：运行时 lane 在 `plugin.rs` 之上加 `plugins_dir` 解包、`verify_plugin_signature`（真 Ed25519，信任根 `keys/trusted-pubkeys.json` 经 KeyringStore，fail-closed）、18 条 Tauri 命令（每加必过 source check + ACL + 前端 bridge/types + 策略覆盖 + 同包测试，优先少加）；`PluginState` 经 `transition` 驱动；`permission_preview_for_plugin` 供 `PermissionPreviewModal.vue` 渲染；`PLUGIN_CAPABILITY_V1` 能力逐个评估时同步在 `DANGEROUS_PLUGIN_CAPABILITIES` 标记风险档。
- **能力真源收口**：`PLUGIN_CAPABILITY_V1` 落 security_policy.rs 是 W4 §4 碎片化待收口项的最小改动落点；完整 `capability.rs` 统一（MCP+Skill+Agent+Plugin+A2A）仍交 A0 裁决。

## 8. 交付索引

- 产品代码：`src-tauri/src/domain.rs`(plugin DTO 追加)、`src-tauri/src/plugin.rs`(新增)、`src-tauri/src/security_policy.rs`(PLUGIN_CAPABILITY_V1 + PolicyError 变体)、`src-tauri/src/main.rs`(mod plugin)、`scripts/check-plugin-policy.py`(新增)、`scripts/pre-merge.sh`(接入)。
- 文档：`logs/assist/A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md`（本文件）、`logs/checkpoints/Lane-A9-M5-W6-plugin-manifest-lifecycle-20260906-2330.md`。
- 上游：W4 `A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md`（已拣入）、W5 `A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md`（已对照 A7 复核）。
- A9 插件 docs 链完整：`W0(1100)→W1(1530+1630 已集成)→W3(1730 已拣入+graph 契约)→W4(1820 已拣入)→W5(1905 已对照 A7 复核)→W6(2330 策略切片产品代码)`。manifest/lifecycle 已由设计对齐（A5/A6/A7）转为受编译+单测+策略门禁守护的可集成切片；运行时 lane A18/A19 在其上接入。
