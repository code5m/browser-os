# A9 M5-10 插件形态 Wave-0 可行性判定（Batch Implementation Dispatch 交付）

> 生成：2026-09-06 07:00 CST · Lane A9（M5 prework only · docs only）
> 性质：**Wave-0 文档层判定**（Board "现即可做文档层"）。零产品代码。
> 目的：在 M4 PASS 前用实测事实把"插件形态①/②/③"判清楚，给 A0 一个可裁定的结论，而非让 A16/A18 在 M4 PASS 后盲目开工。
> 锚定事实：来自对当前工作树权限/配置/ACL 的只读核对（§0）。
> 配套：`A9-M5-A13plus-cards-20260906-0010.md`（A18 卡细化）· `详细设计与实施计划.md` §7.3（#15 插件系统）· `AI-模型切换与接手清单.md`（capabilities 残留标签）。

---

## 0. 事实基线（2026-09-06 07:00 实测，只读核对）

| 项 | 已落地事实（文件:行） | 对插件形态的意义 |
|---|---|---|
| 全局 Tauri | `src-tauri/tauri.conf.json:12` `"withGlobalTauri": true` | 每个 webview（含远程）都能拿到 `window.__TAURI__` 全局对象 |
| 全局 Tauri 引用点 | `src/stores/useSystemStore.ts:84` 仅 **1 处** `__TAURI__.event.listen("tauri://focus", ...)` | 主应用对全局 Tauri 依赖极薄，**可封**为显式注入（A13 评估项） |
| 权限集划分 | `src-tauri/capabilities/default.json`（windows=["main"]，含 `default-commands`）· `browser-remote.json`（webviews=["tab-*","grid-*"]，remote urls，含 `remote-collect`） | 权限按 **webview label 通配 + remote url** 静态绑定 |
| 远程命令集 | `src-tauri/permissions/remote-collect.toml` 仅 `report_resources`/`report_title`/`report_grid_load_failed` | 远程 webview 默认**只能**回传类命令；写盘/开终端已移除 |
| ACL 末条 | `default-commands.toml` 尾条恒为 `list_artifact_images` | 插件命令插其前（坑位②） |
| 命令数量 | 当前 `default-commands.toml` 已含 `db_*`/`task_*` 等 100+ 命令 | 插件 webview 若共享 `default-commands` 即拿到全部能力 |
| CSP | `tauri.conf.json:15` `"csp": null` | 远程页面可跑任意 JS（但 invoke 受 capability 限制） |
| 执行通道 | `src-tauri/src/script_runner.rs:843` `start_run` / `:882` `start_command`（M2-4 单执行路径） | 插件执行体若复用此通道即无第二执行路径 |

---

## 1. 三种形态定义

| 形态 | 描述 | 关键依赖 |
|---|---|---|
| ① 独立进程（stdio） | 插件以独立子进程运行，经 stdio JSON-RPC 与主进程通信；主进程按插件 manifest 代理白名单命令 | 需新增长生命周期 stdio 通道（**第二执行路径风险**） |
| ② webview 内插件 | 插件以 webview（`plugin-*` label）加载，直接调 `window.__TAURI__.core.invoke` | 依赖 capability 静态降权 + 运行时二次校验 |
| ③ 声明式插件（无第三方代码加载） | 插件 = manifest 声明 `kind=script\|tool\|skill\|session-handler` + 指向已有 `ScriptMeta`/`ToolMeta`/`SkillDef` id + 静态 `acl_level`；不加载外部代码、不开新进程 | 复用 M2-4/M2-9 执行通道与既有确认闸门 |

---

## 2. 各形态可行性判定（实测事实驱动）

### 2.1 形态②（webview 内插件）——**条件可行，但有硬约束**

- `withGlobalTauri=true`（`tauri.conf.json:12`）使 `window.__TAURI__` 在**所有** webview（含远程 `tab-*`/`grid-*`）可见。因此**不能**靠"插件 webview 拿不到 Tauri 全局"做隔离 → 隔离必须靠两层：
  1. **静态 capability**：在 `capabilities/` 新增 `plugin.json`，`webviews=["plugin-*"]`，`permissions=[core:default, plugin-runtime]`（`plugin-runtime.toml` 仅含插件所需最小命令集，**不能**含 `write_file`/`run_script`/`db_*` 等副作用命令）。Tauri v2 capability 支持 label 通配，故**静态声明可行**。
  2. **运行时二次校验**：capability 是静态、无法按插件动态授予（插件 A 可读文件、插件 B 不可）。命令处理层须按 `plugin_id` 查 manifest `acl_level` 做二次校验（复用 `security_policy::check_invocation_source` 风格，非新机制）。
- **硬约束**：Tauri capability 在编译期声明，无法运行时动态增删权限；若需要"按插件动态权限"，必须走运行时二次校验（见上 2）。
- **结论**：形态② 可行，但权限粒度受 capability 静态性限制，需运行时校验兜底。**不引入第二执行路径**（执行仍走 M2-4）。

### 2.2 形态①（独立进程 stdio）——**与红线冲突，不推荐**

- 本质是新增长生命周期子进程通道（stdio over `run_command` 是一次性执行，不适用长连接）。这**违反**「禁第二执行路径」红线（M4 护栏/`详细设计` K 系列：`scheduler must reuse M2-4 execution channels; no second shell/process execution path`）。
- 若要保留，唯一合规做法是挂在 M2-4 执行通道上 → 但 stdio 长连接 ≠ `start_run` 一次性模型，**仍是第二路径**。
- **结论**：形态① 与现有红线冲突；除非 A0 显式放宽"允许插件独立进程并归入新 `PluginProcessTable` + 协调器 `kill-plugins`"，否则**否决**。若放行，需补：进程回收归入 `ShutdownCoordinator`（`kill-plugins` 序在 `kill-running-scripts` 之后）、独立审计、崩溃隔离测试。

### 2.3 形态③（声明式插件）——**推荐，默认形态**

- 插件**不加载任何第三方代码**：`PluginManifest{kind, entry_ref(指向已有 ScriptMeta/ToolMeta/SkillDef id), acl_level, config_schema}`；`plugin_enable/disable/uninstall` 只切换 manifest 状态 + 注册/注销能力到 MCP 工具集与 Skill 列表。
- 执行体 100% 复用 M2-4 `run_script`/`run_command` 与 M2-9 工具箱 → **零第二执行路径**、零新进程、继承既有确认闸门/审计/路径沙箱。
- 第三方代码能力由既有「脚本库/工具箱」承担（脚本库本身就是"用户自己加的可执行代码"，已有确认/审计/路径沙箱红线）。
- **结论**：形态③ 与现有红线完全一致，是 M5-10 的**首选落地形态**。

---

## 3. 裁定建议（交 A0）

| ID | 问题 | 建议 |
|---|---|---|
| P-D1 | 插件形态选 ①/②/③？ | **默认③**（声明式），② 作 Web UI 承载壳（插件管理面板用 webview，但执行仍走 M2-4）；① 否决除非显式放宽红线 |
| P-D2 | capability 静态性导致的"按插件动态权限"如何兜底？ | 选②承载 UI 时，命令处理层按 `plugin_id`+manifest `acl_level` 做运行时二次校验（复用 `check_invocation_source` 风格） |
| P-D3 | 插件能否加载远程/本地任意代码？ | **否**（形态③）。远程拉取仅作 manifest 引用，执行体必须是已存在的 Script/Tool/Skill（复用既有红线） |
| P-D4 | 插件卸载清理范围？ | 删 `workspace/plugins/*.json` + 注销能力 + 清运行记录；**不删**被引用的 Script/Tool/Skill（共享资产）；审计 `plugin_uninstall` |

---

## 4. 每种形态对 M5-10/11/12 的影响面

| 模块 | 形态③（推荐）影响 | 备注 |
|---|---|---|
| M5-10 manifest/生命周期 | `PluginManifest` 仅声明 + 静态 `acl_level`；load/unload/reload = manifest 读写 + 能力注册/注销 | 无进程管理 |
| M5-11 命令与隔离 | `plugin_list/install/enable/disable/uninstall`；install=写 manifest+校验签名/来源+默认 `enabled=false`；隔离靠 M2-4 + 运行时 `acl_level` 二次校验 | 无独立沙箱进程 |
| M5-12 管理 UI | 复用 M4 数据库/调度面板模式（M2-9 工具箱风格）；权限清单来自 manifest `acl_level` | 复用 `check-*-ui-logic.mjs` 范式 |

> 若 A0 裁定形态①（否决情形除外）：M5-11 需新增 `PluginProcessTable` + `kill-plugins` 协调器序（`kill-running-scripts` 之后）+ 独立 clippy/审计/崩溃测试；A9 不在此文档展开，交 A18 实施期。

---

## 5. 与既有 Open Item 的关系

- `AI-模型切换与接手清单.md` 记载「capabilities 残留 `browser` label（致命2/中危26，未修）」：形态③ 不新增 capability label，但**若选②需新增 `plugin-runtime`**，应一并清理 `browser` 残留 label（避免能力漂移，K1）。
- `withGlobalTauri` 全局暴露（§0）：形态③ 不依赖全局 Tauri（插件 webview 用显式 `invoke` 注入），但建议 A13 在 workspace 下沉评估时**封死** `src/stores/useSystemStore.ts:84` 的全局引用，统一走桥封装（降低攻击面）。

---

## 6. FORBID 遵守记录

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh` 及三份主文档。
- 未移动 `NEXT`（现 M4 batch implementation mode）。
- 未提交、未 push；与工作树中 A18 相关未提交产物无交集（A18 尚未开工）。
- 「已冻结」陈述以 file:line 标注；「判定/建议」明确区分并交 A0 裁定。
