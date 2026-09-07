# Lane A9 — M5-W15 Release Readiness · 冻结的 W13 后端契约复核（Frozen W13 backend-contract review; no backend changes）

- **Lane / 分派**：A9（M5-W15 Release Readiness Dispatch，行 1177：`Frozen W13 backend-contract review; no backend changes.`）
- **BASE**：`886ea29`（master，W14 已合入；`git fetch origin && git pull --ff-only` 已同步，与 `origin/master` 一致）
- **交付物**：本复核笔记（契约冻结确认 + W14/W15 零后端改动核验）；无产品代码，未提交未 push
- **时间戳**：2026-09-07 15:05
- **前置**：`A9-M5-W14-plugin-contract-review-20260907-1430.md`（W14 契约核对，结论仍成立，本卡续做 W15 冻结确认）

---

## 一、复核结论（一句话）

截至 W15（HEAD=`886ea29`），W13 冻结的后端插件契约**仍完整冻结、未被 W14/W15 触碰**；W14 仅改前端 + 政策脚本 + 文档（零 `src-tauri/` 改动），W15 工作树后端亦零改动。8 后端命令 ↔ 8 `bridge.ts` 方法 ↔ DTO 集合与 ACL 8 条目**全部未变**，A6（W14 Plugin Manager UI）仅经 `src/bridge.ts` 消费冻结契约（无裸 invoke）。后端契约**已具备发版就绪（release-ready）状态，W15 无需任何后端动作**。

---

## 二、零后端改动核验（实测）

| 范围 | 核验命令 | 结果 |
|---|---|---|
| W14 是否改后端 | `git diff --stat a7eefbb 886ea29 -- src-tauri/` | **空**（W14 仅 `src/components/plugin/*`、`src/stores/usePluginStore.ts`、`src/utils/pluginUi.ts`、layout、`scripts/*`、`logs/*`、主文档） |
| W15 工作树后端改动 | `git diff --stat HEAD -- src-tauri/` | **空**（A9 本卡零 `src-tauri/` 改动，符合 W15 硬停） |
| W14 提交文件清单 | `git show --stat 886ea29` | 仅前端/脚本/文档/指标 JSON；无 `src-tauri/**`、`permissions/*.toml` 改动 |

→ W13 后端契约面在 W14、W15 两波均**未被修改**。

---

## 三、冻结契约面复核（与 W14 复核逐字一致，未漂移）

### 3.1 后端命令（`src-tauri/src/bridge.rs`，8 条，每条带 `check_invocation_source`）
`plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_keys_add` / `plugin_keys_list` / `plugin_keys_remove`
（`grep -c "pub fn plugin_" src-tauri/src/bridge.rs` = **8**，与 W14 复核一致）

### 3.2 前端方法（`src/bridge.ts:718-739`，1:1 映射，无裸 invoke）
`pluginInstall` / `pluginEnable` / `pluginDisable` / `pluginList` / `pluginGet` / `pluginKeysAdd` / `pluginKeysList` / `pluginKeysRemove`

### 3.3 UI 仅经 bridge 消费（W15 新增核验）
`grep -rn "invoke(\|@tauri-apps/api/core\|window.__TAURI__" src/components/plugin/ src/stores/usePluginStore.ts src/utils/pluginUi.ts` → **NO_RAW_INVOKE**。
UI 对受信任密钥使用客户端字段 `note`（非 W13 文档旧名 `label`，G4 已正确消解，见 `usePluginStore.ts:173/183`）。

### 3.4 DTO（`domain.rs` + `types.ts`，脱敏构造，未变）
`PluginSummary` / `PluginDetail` / `PluginCapabilityView`(`acl_level`) / `PluginSignatureView`(无 `value`) / `PluginResourceMeta`(无绝对路径) / `TrustedKeyRecord`(仅 sha256 前 16hex 指纹) / `PluginManifest`(仅作 `pluginInstall` 入参，`value` 收不存、禁渲染)。

### 3.5 ACL（`permissions/default-commands.toml:130-137`）
8 条 `plugin_*` 仍插在末条 `list_artifact_images` **之前**（K1 红线，未漂移）。

---

## 四、W14 复核登记的缺口现状（仍属 W15 冻结范围外）

W14 复核登记 G1–G5（见 `A9-M5-W14-plugin-contract-review-20260907-1430.md` §四）：

- **G1**（受信任密钥 join）：客户端用 `signature.key_id` × `pluginKeysList()` 推导「signed by trusted key」，W15 仍建议 A6 客户端实现，**不触发后端改动**。
- **G2**（vendor/author 展示）：manifest 无 vendor 字段，后续 wave 后端补字段。**W15 硬停禁止扩张运行时权限/新增后端字段**，保持延后**。
- **G3**（校验失败原因）：仅稳定码/脱敏的 `error`/`last_error` 字段需后端补；**W15 冻结范围外**，保持延后。
- **G4**（label→note）：已消解，UI 绑定 `note`。
- **G5**（Summary 缺 `installed_at`）：UI 复用 `updated_at`，或后续 wave 后端补；W15 不处理。

→ 上述缺口在 W15 无需、也不得由后端补齐（W15 硬停：`no new command or runtime surface`）。它们不影响发版就绪判定。

---

## 五、W15 硬停合规

W15 硬停（`PARALLEL_COMMAND_BOARD.md:1181`）：禁 plugin invoke/execution、dynamic loading、network download/listener、daemon、model call、Agent/Skill execution、MCP expansion、graph write/export、background worker、raw Tauri invoke、sensitive rendering/persistence。

- A9 本卡**零产品代码**，仅交付此笔记；未触碰任何被禁运行时面。
- 复核对象（W13 后端契约）本身即 Local-only Stage-I，无上述任一运行时能力，与 W15 硬停天然一致。

---

## 六、发版就绪裁定

1. **后端契约冻结状态确认**：W13 8 命令 / 8 bridge 方法 / DTO / ACL 8 条目在 W14、W15 均无改动，契约稳定可发版。
2. **零后端回归风险**：W14 仅前端，W15 无后端工作树改动，无契约漂移。
3. **A6 消费合规**：UI 仅经 `src/bridge.ts` 调用冻结方法，无裸 invoke，字段绑定（`note`）正确。
4. **遗留展示增强（G2/G3/G5）** 属后续 wave，不阻塞 W15 发版。
5. A9 本卡交付**仅此笔记**，无补丁（无代码改动）；checkpoint/patch 仅含本笔记文档。

---

## 七、FORBID 遵守记录

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh` 及三份主文档。
- 未移动 `NEXT`（现 M5-W15 Release Readiness，属 A0/A11 链）。
- 未提交、未 push；本文件为新增独立文档，与工作树中 W13/W14 已提交产物及其他 lane 未提交产物无交集。
- 所有「已冻结/已核验」陈述均以 `git diff --stat` / `grep` 实测结果标注。
