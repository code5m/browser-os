# M5-12 插件管理 UI（权限清单 / 配置 / 状态）

> 子卡 ID：**M5-12** · 需求 #15 · `[S3|LEVERAGE:1|COMPLEX|AI:NORMAL|R:high]`
> 责任 Lane 候选：**A19**
> 父卡：`详细设计与实施计划.md` L576（`M5-12 插件管理 UI`）
> 主预研：暂无 prework 文档
> 配套：`M5-10-plugin-manifest-lifecycle.md`（manifest 后端）· `M5-11-plugin-commands-isolation.md`（命令与隔离）

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
