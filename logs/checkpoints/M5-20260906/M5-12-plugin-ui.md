# M5-12 插件管理 UI（权限清单 / 配置 / 状态）

> 子卡 ID：**M5-12** · 需求 #15 · `[S3|LEVERAGE:1|COMPLEX|AI:NORMAL|R:high]`
> 责任 Lane 候选：**A19**
> 父卡：`详细设计与实施计划.md` L576（`M5-12 插件管理 UI`）
> 主预研：暂无 prework 文档
> 配套：`M5-10-plugin-manifest-lifecycle.md`（manifest 后端）· `M5-11-plugin-commands-isolation.md`（命令与隔离）
>
> **W3** BLOCKED（待 M5-10 解析 + M5-11 命令基元）· **W4** ACTIVE（A19 W4 仍 SUPPORT DOCS ONLY）· **W5** ACTIVE（A19 W5 仍 SUPPORT DOCS ONLY）· **W6** ACTIVE（A19 W6 仍 SUPPORT DOCS ONLY —— W6 仅 A8 M5-9 + A9 M5-10/M5-11 产品代码 lane，**无 plugin UI lane 承接**；详见本卡顶部 `[W6 status]` 段）· **W7** RECONCILIATION（A3 W7 mcp_* 3 命令 + A11 W7 pre-merge FAIL 3 red lights + A6 W7 wiring 在 `6c1f30e` / `daa10f6` / `a29b796` 已拣入 master；A19 W7 仍 SUPPORT DOCS ONLY；plugin UI 待 W8+ A19 派发：plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项；A1 W7 整包本卡修订未进 master，留 W8 整包合并拣入；详见 `M5-0-overview.md` 顶部 `[W7 reconciliation]` 段 + `M5-14-debt-ledger.md` §10 DEBT-04）· **W8** ACTIVE（**A19 W8 仍 SUPPORT DOCS ONLY** —— **W8 仅 A5 START PRODUCT CODE 硬化 Agent/Skill read-only bridge + A6 START UI DOCS/LOGIC 实施 Agent/Skill 面板消费 + 纯 UI helper tests + A8 UI POLISH/TEST + A9 POLICY REVIEW；无 plugin UI lane 承接**；plugin UI 待 W9+ A19 派发，详见 `M5-0-overview.md` 顶部 `[W8 active]` 段 + `M5-14-debt-ledger.md` §10 DEBT-04 + §10.4 W8+ batch）
> **W8 reconciliation**（2026-09-07 14:30 CST · A0 拣入）：A0 在 **`4d7be97 feat(M5): integrate W8 command bridge polish`**（53 files +6038 -101）+ **`94e763e fix(M5-W8,A3): close MCP policy phase debt`**（MCP policy phase debt 关闭 · `MCP_NO_RMCP_SERVER` + `--expect-current-gaps` gate + ACTIVE=8 PENDING=0）+ **`a840fcb docs(A11): M5-W8 verification delta — functional GREEN, 3 red lights all trace to A3 W7/W8`**（A11 W8 verification delta 功能性 ALL_PASS）+ **`97118d6 chore(M5): normalize W8 patch evidence whitespace`**（3 份 W8 patch 文件空白规范化）4 commit 中拣入本卡 W8 修订：① A1 W7 reconciliation 整包合并拣入（DEBT-04 plugin UI 仍 0% ACTIVE 残留债在本卡 [W7 reconciliation] 段）；② A1 W8 reconciliation 整包合并拣入（本卡头部 + [W8 active] 段 + 顶部 history 链更新）；**A1 W7 + W8 整包合并拣入 9 + 11 = 20 文件均含本卡修订**。
> **W9** ACTIVE（**A19 W9 仍 SUPPORT DOCS ONLY** —— **W9 仅 A2 REVIEW + A3 POLICY/REVIEW + A4 PRIVACY + A5 PRODUCT CODE SMALL（Agent/Skill hardening）+ A6 UI LOGIC（Agent/Skill 面板消费）+ A7 GRAPH CONTRACT DOCS + A8 GRAPH UI SMALL + A9 PLUGIN REVIEW + A10 SECURITY FINAL + A11 FINAL VERIFICATION + A1 DOCS；无 plugin UI lane 承接**；plugin UI 待 W10+ A19 派发：plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项；DEBT-04 收口推 W10，详见 `M5-0-overview.md` 顶部 `[W9 active · 2026-09-07 14:30 CST]` 段 + `M5-14-debt-ledger.md` §10 DEBT-04 + §10.4 W9+ batch）
> **W10** ACTIVE（**A9 W10 PLUGIN RUNTIME PLAN ONLY** + **A19 仍 SUPPORT DOCS ONLY**（plugin UI 待 W10+ A19 派发：plugin 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览 6 项；DEBT-04 收口推 W10+）；**plugin UI runtime 仍 LOCKED**（W10 Hard Stop L207）；详见 `M5-0-overview.md` 顶部 `[W10 active · 2026-09-07 16:00 CST]` 段 runtime-lock 状态表）
> **W10 PUSHED**（2026-09-07 18:30 CST · A0 拣入 `5226aad` = HEAD）：A0 在 **`ba78092 feat(M5-W10,A3): MCP stdio-prep skeleton (feature-gated, read-only, std-only)`**（A3 W10 实施期）+ **`5226aad feat(M5): integrate W10 MCP stdio prep`**（A0 W10 整包合并拣入）两 commit 中拣入 W10：① A9 W10 plugin runtime plan only + A19 仍 SUPPORT DOCS ONLY（plugin UI 待 W10+ A19 派发 6 项）；② A1 W10 reconciliation 整包合并拣入；③ A11 W10 verification delta 收口；**M5-12 关联债 DEBT-04（plugin UI 6 项仍 0% ACTIVE）= W11+ 仍挂账**。
> **W11** ACTIVE（**A9 W11 = PLUGIN DOCS/POLICY ONLY**（W12/W13 plugin staged cards 预备）+ **A19 仍 SUPPORT DOCS ONLY**（plugin UI 6 项仍待 A19 派发；**A11 board 写为 A19** 而非 A9，名称按 board L200）；**plugin UI runtime 仍 LOCKED**（W11 Hard Stop L207）；详见 `M5-0-overview.md` 顶部 `[W11 active · 2026-09-07 18:30 CST]` 段 runtime-lock 状态表 10 行；A19 W11 仍 SUPPORT DOCS ONLY）
> **W11 PUSHED**（2026-09-07 20:30 CST · A0 拣入 `269269a` = HEAD）：A0 在 **`269269a feat(M5): integrate W11 MCP stdio dry-run hardening`** 中拣入 W11；MCP stdio dry-run 硬化 + A5 Agent/Skill 执行锁 policy（PENDING=6）；`cargo test --features mcp mcp_server` 21/21 + `check-mcp-policy.py` ACTIVE=11/PENDING=0；A1 W11 reconciliation 整包已合并拣入；**plugin UI runtime 仍 LOCKED**（W12 Hard Stop L209）。
> **W12** ACTIVE（**A9 W12 = PLUGIN DOCS ONLY**（W12/W13 plugin staged cards 细化仅在 W11 反馈改变 blocker 时）+ **A19 仍 SUPPORT DOCS ONLY**（plugin UI 6 项仍待 A19 派发）；**plugin UI runtime 仍 LOCKED**（W12 Hard Stop L209）；详见 `M5-0-overview.md` 顶部 `[W12 active · 2026-09-07 20:30 CST]` 段 runtime-lock 状态表 11 行）
> **W12 PUSHED**（2026-09-07 23:55 CST · A0 拣入 `3c3f460` = HEAD）：A0 在 **`3c3f460 feat(M5): integrate W12 graph live-query readonly bridge/UI`** 中拣入 W12：图谱 live-query 只读 3 命令 + A8 UI 消费；`cargo test graph` 15/15 + full cargo 414/414 + `check-graph-policy.py` ACTIVE=8 + graph UI logic 113/113 + npm build PASS + build metrics 22.26% ≤ 23%；A1 W12 reconciliation 整包合并拣入；**plugin UI runtime 仍 LOCKED**（W13 Hard Stop）+ **A19 仍 SUPPORT DOCS ONLY**（plugin UI 6 项仍 0% ACTIVE）。
> **W13** ACTIVE（**A9 W13 = START PRODUCT CODE NARROW**（**plugin stage-I manifest 生命周期后端** —— `plugin_install` / `plugin_enable` / `plugin_disable` / `plugin_list` / `plugin_get` / `plugin_key_registry` 6 命令 + 本地状态机）+ **A19 仍 SUPPORT DOCS ONLY**（plugin UI 6 项仍 0% ACTIVE，**仍待 A19 派发**；A0 W12/W13 期间未派发 A19 plugin UI 产品代码；plugin UI 6 项 = 列表 / 详情 / 安装向导 / 启用停用 / 审计查询 / 权限预览，**全部依赖** W13 stage-I 真实后端 6 命令落地）；**plugin UI runtime 仍 LOCKED**（W13 Hard Stop）；详见 `M5-0-overview.md` 顶部 `[W13 active · 2026-09-07 23:55 CST]` 段 runtime-lock 状态表 13 行；**DEBT-04（plugin UI 6 项仍 0% ACTIVE）= W14+ 仍挂账**；**W13 plugin 范围冻结 = 后端 6 条 stage-I 命令（**不**含 UI 实施）**（详见 [W13 plugin UI still locked] 段）

---

## [W6 status · 2026-09-06 19:25 CST] A19 M5-12 W6 仍 SUPPORT DOCS ONLY（无 plugin UI lane 承接；W7+ 待 A9 W6 plugin backend 落地后由 A0 决定派发）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L136-171（**M5-W6 Parallel Dispatch**：仅 **A8** *START PRODUCT CODE*（M5-9 graph UI）+ **A9** *START PRODUCT CODE*（M5-10/M5-11 plugin manifest/lifecycle policy slice）—— **未**列 A19 *START PRODUCT CODE*；其它 9 lane 全部 SUPPORT/REVIEW/VERIFICATION）+ L7（*"Current NEXT: M5-W6 parallel implementation; Lane A8 owns M5-9 graph UI pure logic/panel shell, Lane A9 owns M5-10/11 plugin manifest/lifecycle policy slice"* —— **未**提 A19 plugin UI）。
> **A19 W6 仍 SUPPORT DOCS ONLY 的原因**：
> - **A9 W6 plugin manifest/lifecycle policy slice 仍在落 schema / stub / ACL** —— 真实 install/uninstall runtime 在 W7+；M5-12 plugin UI 必须消费**已落地的真实 manifest / 真实 lifecycle state / 真实 5 命令 handler** 才能做（plugin 列表 / 安装向导 / 启用停用 / 审计查询 / 权限预览 等 UI 元素**全部依赖** W7+ 的真实 plugin runtime 后端）。
> - **A8 W6 已经在做 M5-9 graph UI** —— UI lane 仅 A8 一条；M5-12 plugin UI 不能与 A8 W6 抢资源（A19 UI 范式 = A6 W5 / A8 W6 复用一致，但 plugin 域独立，避免 W6 期间双 UI lane 互相干扰）。
> - **d3 / npm 依赖** —— A8 W6 不引 d3 已在 W6-HS4 严守；A19 W6 plugin UI 若开须 1+ 轮 npm 引入决议（W7+）。
> **A19 W6 应做的（轻量）**：
> ① 重读 A9 W6 output（A9 W6 实施期产出物）确认 manifest DTO / lifecycle state machine / 5 命令 ACL stub / plugin-invokes.json audit shape 实际形态
> ② 重读 A8 W6 graph UI 范式（`f99d2eb` 拣入的 Agent/Skill panel shell + `4b438ef` 拣入的 graph 模型/常量/隐私双扫）确认 UI 模式
> ③ 在 `logs/assist/A19-M5-W6-*.md` 出 W6 plugin UI card delta（**仅 docs**，**不**写 plugin UI 代码），**不**碰 `src/components/plugin/**` / `src/stores/plugin*.ts` / `src/types.ts` / `src/bridge.ts`
> ④ 在本卡 `M5-12-plugin-ui.md`（如需）补 W6 status 行（**不**动 §1~§11）
> **A19 W6 不应做的**：① 写 plugin 列表 / 安装向导 / 启用停用 / 审计查询 / 权限预览任何 UI 组件 ② 写 `usePluginStore.ts` 状态管理 ③ 改 §1~§11 决策史 ④ 改本根卡（M5-0）/M5-10/M5-11 任何 AC 段 ⑤ 改三份主文档 / ACL / Capability / pre-merge.sh / Cargo.toml / package.json
> **A1 W6 不修订范围（本卡）**：
> - **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W6 **不动**（决策史保持 W0 原文；A19 W6 status 在本顶部段单列）。
> - **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W6 不动。
> - **`NEXT` 标记**：A0 调度权；A1 不改字面值。
> - **A1 W6 强停止**：本卡本轮**仅**加本 `[W6 status]` 段 + 头部状态行；**不**写 next-card AC（与 M5-9/10/11 不同——M5-12 在 W6 没有产品代码 lane 承接）。

---

## 0. 编号与锚定

- 批次任务号 `M5-12`；需求号 #15；WBS L576 一致。
- 依赖：M5-10 ✅ + M5-11 ✅（18 条命令稳定）
- 前端栈：Vue 3 + Pinia

---

## 1. GOAL

实现插件管理 UI：① 插件市场/列表（按 source / 状态过滤）② 详情（manifest + 资源 + 权限清单）③ 安装/启用/禁用/卸载（必弹闸门）④ 受信任公钥管理（trusted-pubkeys.json 编辑）⑤ 插件运行历史。所有写操作必走 M5-10/11 闸门。

---

## 2. READ

1. `M5-10-plugin-manifest-lifecycle.md` §4.5（两段式确认）
2. `M5-11-plugin-commands-isolation.md` §4.1（18 条命令）
3. `src/components/agent/PermissionPreviewModal.vue`（M5-6 已有，**全读**——权限弹窗范式）
4. `src/components/agent/SkillManager.vue`（M5-6 已有，**全读**——列表/详情范式）
5. `src/bridge.ts`（18 条新命令的 TS 包装，**全读**）
6. `src/types.ts`（`PluginManifest` TS 镜像）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/components/plugin/PluginManager.vue` | **新增** | 主面板（市场/列表/过滤） |
| `src/components/plugin/PluginDetail.vue` | **新增** | 详情（manifest + 资源 + 权限） |
| `src/components/plugin/PluginPermissionPanel.vue` | **新增** | 权限清单（逐项可视化） |
| `src/components/plugin/TrustedKeyManager.vue` | **新增** | 受信任公钥增删改 |
| `src/components/plugin/PluginHistoryModal.vue` | **新增** | 运行历史 |
| `src/stores/usePluginStore.ts` | **新增** | 插件状态（列表缓存、闸门确认态、trusted keys 缓存） |
| `src/router/plugin.ts` | **新增** | 插件路由 |
| `src/locales/zh-CN.json` `src/locales/en-US.json` | 扩展 | 12 个 i18n key |
| `src/styles/plugin.scss` | **新增** | 样式（dark/light 双主题） |

---

## 4. 关键契约

### 4.1 主面板布局

- 顶部 Tab：已安装 / 失败 / 全部
- 列表项：图标 + 名称 + 版本 + 状态徽章（Enabled/Disabled/Failed）+ 操作按钮
- 状态徽章颜色：绿（Enabled） / 灰（Disabled） / 红（Failed）
- 操作：详情、启用/禁用、卸载、（重新）安装

### 4.2 详情面板

- 必显项（**禁**折叠）：id / version / display_name / description / 来源 / capabilities（逐项） / hash / signature 状态 / 安装时间 / 资源列表
- 权限清单单独成面板（`PluginPermissionPanel.vue`），含风险等级（与 MCP/Skill 范式一致）

### 4.3 安装/启用/卸载 必弹闸门

- 复用 `PermissionPreviewModal.vue` 范式（M5-6）
- 必显：插件名称 + 版本 + capabilities（**逐项**）+ 风险等级 + 来源 + hash 前 8 位
- 二次确认（keyring 二次认证）用于 `Dangerous`
- 取消 = 拒绝

### 4.4 受信任公钥管理

- 增：粘贴公钥 + key_id + 备注
- 删：需 keyring 二次认证
- 改：仅可改"备注"，不能改 key_id 或公钥本身（防替换攻击）

### 4.5 闸门确认态管理

- `usePluginStore.pendingConfirms: Map<id, {action, payload, expires_at}>`
- 与 M5-6 Agent 范式一致

---

## 5. FORBID

- **不**让任何 UI 路径直接走 Tauri `invoke`（必须经 `bridge.ts`）
- **不**让闸门弹窗被折叠/隐藏/默认确认
- **不**让 trusted-pubkeys.json 编辑 UI 绕过 keyring 二次认证
- **不**让 dark/light 主题冲突
- **不**让 i18n 缺 key
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 文件结构
ls src/components/plugin src/stores/usePluginStore.ts 2>&1

# B. 反向用例
# N1: 启用 Dangerous 插件不弹闸门 → 阻断
# N2: 卸载插件不弹闸门 → 阻断
# N3: 改 trusted key 的公钥本身 → 阻断
# N4: 删 trusted key 不弹 keyring 认证 → 阻断
# N5: 裸 invoke → 阻断
# N6: dark/light 主题崩 → 阻断
# N7: i18n 缺 key → 阻断

# C. e2e 冒烟
pnpm test:unit
pnpm test:e2e

# D. 编译与基线
pnpm build
pnpm tsc --noEmit
pnpm lint
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 5 个新组件 + 1 store 落地 | 命令 A |
| 2 | 闸门必显（含 keyring 二次认证） | 单测 N1/N2/N4 |
| 3 | 公钥编辑防替换 | 单测 N3 |
| 4 | UI 仅经 `bridge.ts` | CI 断言 N5 |
| 5 | dark/light 主题适配 | 单测 N6 |
| 6 | i18n 双语齐 | 单测 N7 |
| 7 | `pnpm test:unit` + `pnpm test:e2e` 全绿 | 命令 C |
| 8 | `pnpm build` 无错 | 命令 D |
| 9 | `pre-merge.sh` ALL_PASS | 复用既有 |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 闸门被绕过 | 阻断（红线） |
| 裸 `invoke` 出现 | 阻断（必须经 bridge.ts） |
| 公钥可改 | 阻断（防替换攻击） |
| dark/light 主题崩 | 阻断 |
| i18n 缺 key | 阻断 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L576 `[ ]` → `[x]`
2. `后续需求TODO.md` §15 状态 `DONE`
3. `AI-模型切换与接手清单.md` NEXT 移至 M6（**待 A0 拍**）
4. `logs/checkpoints/M5-12.a-2026MMDD-HHMM.md`

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A19 实施填
- **NEXT**：M5-13 验证矩阵 + M5-14 债务账（A1 本批 M5-W0 最后两卡）；M6 待 A0 拍

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
