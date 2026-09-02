# strong-model-minimal-read-list（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 用途：给**后续强模型**的**最小阅读清单**——每个主任务只列 3~8 个必读文件，并明确「不必读」的长文档，降低 token 消耗
> 状态：📝 阅读建议（不构成任何主任务 PASS 结论）

---

## 0. 全局必读（**任何任务开工前都先读这 6 项**，约 1.2 万 token）

| # | 读什么 | 大小 | 为什么 |
|---|---|---|---|
| 1 | `logs/assist/free-model-prework-M2-M5-20260902-1055.md` §0/§3/§4/§5 | 16 KB（**只读这 4 节**） | 批次边界、5 项勘误、阻塞依赖图、9 条共性红线 K1~K9 |
| 2 | `logs/assist/errata-to-taskcards-20260902-1146.md` | 20 KB | 6 项勘误的**任务卡化**版本，含证据与验收命令 |
| 3 | `logs/assist/repo-sanity-audit-20260902-1146.md` §6 | 15 KB（只读 §6） | 结构事实速查：命令数、capabilities、目录、依赖 |
| 4 | `src-tauri/src/domain.rs` | 92 行 | 唯一已有的领域模型，所有新类型都加这里 |
| 5 | `src-tauri/src/workspace.rs` | 110 行 | 持久化 + 审计 + 1000 条上限；所有落盘任务都要复用 |
| 6 | `logs/baseline-2026-08-27.md` | 3.4 KB | clippy 13 warning + 主 JS 505 KB **硬门槛** |

**可选补充**（按任务）：`src-tauri/permissions/default-commands.toml`（59 命令，K1 红线入口）、`src-tauri/tauri.conf.json`（`withGlobalTauri` / `assetProtocol` 两个红线配置）。

---

## 1. M2 · 脚本库 / 小工具 / 图片

### 1.1 M2-6：5 个种子工具 HTML（从零编写）

| 必读（5） | 为什么 |
|---|---|
| `logs/assist/M2-tools-seed-html-prework-20260902-1146.md` | 本任务的**唯一权威卡**：HTML 骨架、CSS/JS 约束、契约、错误态、验收命令 |
| `logs/assist/M2-9.b-prework-20260902-1055.md` §4（逐工具验收表） | 每个工具的等价类与失败态 |
| `logs/assist/M2-7.b-prework-20260902-1055.md` §4.3（ToolMeta） | 工具元数据字段口径 |
| `logs/assist/errata-to-taskcards-20260902-1146.md` E1 | 为什么是「从零写」而非「补框架」 |
| `variants/` 下 **1 个** `.html`（任选） | 单文件自包含 HTML 的风格参照 |

| 不必读 | 原因 |
|---|---|
| `variants/` 全部 15 个 | 读 1 个足够 |
| `bridge.rs` 全文（2029 行） | 本任务不碰后端 |
| `grid_process.rs`（705 行） | 与本任务无关 |
| `超详细审核报告-严重问题与缺陷.md` | 历史长文档 |

**推荐模型**：json/base64/timestamp → `AI:BALANCED`；cron/regex → `AI:DEEP`。

### 1.2 M2-9：图片 UI 静态壳

| 必读（5） | 为什么 |
|---|---|
| `logs/assist/M2-image-ui-static-shell-20260902-1146.md` | 本任务权威卡：组件拆分、mock、截图点、反向用例 |
| `logs/assist/M2-1.a-prework-20260902-1055.md` | ImageRef 契约（mock 必须与之同构） |
| `src/components/workspace/ArtifactPanel.vue` | 现有面板的代码风格与布局参照 |
| `src/components/shared/ConfirmModal.vue` | 删除确认复用件 |
| `src/utils/format.ts` | 字节数格式化已存在，复用 |

| 不必读 | 原因 |
|---|---|
| `M2-2.b-prework-20260902-1055.md` | 已被本卡取代（口径更细） |
| `src-tauri/injected/collect.js` 全文 | 只需知道「网页侧会上报资源」这一事实 |
| `useBrowserStore.ts`（25 KB） | 与本任务无关 |

### 1.3 M2-3：脚本库 UI 静态壳

| 必读（5） | 为什么 |
|---|---|
| `logs/assist/M2-script-library-ui-static-shell-20260902-1146.md` | 本任务权威卡 |
| `logs/assist/M2-3.a-prework-20260902-1055.md` | ScriptMeta 契约 |
| `logs/assist/script-execution-safety-taskcard-20260902-1146.md`（**只读 §4.4 参数校验 + §6 禁止事项**） | 前端校验要知道后端会重校验，危险参数口径要一致 |
| `src/components/workspace/AuditPanel.vue` | 审计展示位的现状与风格 |
| `src/stores/useWorkspaceStore.ts` | Pinia store 写法参照 |

| 不必读 | 原因 |
|---|---|
| `M2-5.b-prework-20260902-1055.md` | 已被本卡取代 |
| `script-execution-safety-taskcard` 全文（约 12 KB） | 实现执行通道时才需要；做 UI 只需 §4.4 与 §6 |

### 1.4 M2-5：工具库 UI 静态壳

| 必读（5） | 为什么 |
|---|---|
| `logs/assist/M2-tool-library-ui-static-shell-20260902-1146.md` | 本任务权威卡 |
| `logs/assist/M2-7.b-prework-20260902-1055.md` §4.3（ToolMeta）+ §5.R3 | 字段口径 + `withGlobalTauri` 红线 |
| `logs/assist/errata-to-taskcards-20260902-1146.md` E1 + E5 | 文件不存在 + 安全红线 |
| `src/components/system/AppPanel.vue` | 同类「卡片网格 + 启动」面板的参照 |
| `src/components/shared/ConfirmModal.vue` | 卸载确认复用件 |

| 不必读 | 原因 |
|---|---|
| `src-tauri/src/grid_process.rs` | 与本任务无关 |
| `tauri-browser-tabs/` 子 crate | 除非接手子 webview 加载通道（那是另一张卡） |

### 1.5 M2-1/M2-2：脚本执行通道（**强模型任务**）

| 必读（7） | 为什么 |
|---|---|
| `logs/assist/script-execution-safety-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `logs/assist/M2-3.a-prework-20260902-1055.md` | ScriptMeta 契约 |
| `logs/assist/M3-terminal-shutdown-taskcard-20260902-1146.md`（**必读 §4.4 进程组 + §4.2 注册项**） | 依赖它的 ShutdownCoordinator；进程组 kill 范式复用 |
| `src-tauri/src/bridge.rs:688-790` | 两段式安全闸门范式（约 100 行，**不要读全文**） |
| `src-tauri/src/workspace.rs` | 审计落盘 + 1000 条上限 |
| `src-tauri/src/keyring_store.rs`（25 行） | 凭据存储（K3） |
| `src-tauri/capabilities/default.json` | `shell:allow-spawn args:true` 的现状 |

| 不必读 | 原因 |
|---|---|
| `bridge.rs` 全文（2029 行） | 只读 688-790 段 |
| `sync.rs`（287 行） | 只读闸门范式即可，不必读 sync 实现 |
| `grid_process.rs` | 无关 |

---

## 2. M3 · 终端

### 2.1 M3-1/M0-2：退出收口 ShutdownCoordinator（**P0，阻塞 5 个下游**）

| 必读（6） | 为什么 |
|---|---|
| `logs/assist/M3-terminal-shutdown-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `src-tauri/src/main.rs:585-615` + `:660-679` | `CloseRequested` 与 `.run()` 的**当前写法**（约 50 行） |
| `src-tauri/src/bridge.rs:1917-2027` | 终端全部实现（约 111 行，**必读**） |
| `logs/assist/M3-1.a-prework-20260902-1055.md` §2.5/§2.6 | 退出接入点的原始盘点 |
| `src/components/system/TerminalPane.vue`（124 行） | 前端卸载语义 |
| `src-tauri/src/grid_process.rs`（**只读 `shutdown_all` 相关部分**） | 现有的子进程收口范式 |

| 不必读 | 原因 |
|---|---|
| `main.rs` 全文（31 KB / 约 900 行） | 只读上述两段 |
| `bridge.rs` 全文 | 只读 1917-2027 |
| `grid_process.rs` 全文（705 行） | 只读 shutdown 部分 |

### 2.2 M3-2：term_resize（E2 + E3 同 PR）

| 必读（5） | 为什么 |
|---|---|
| `logs/assist/M3-terminal-resize-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡，含 `TerminalSession.master` 方案与前端节流代码 |
| `src-tauri/src/bridge.rs:114-117` + `:1926-1995` + `:2011-2016` | `TerminalSession` / `term_spawn` / `term_resize` 三处 |
| `src/components/system/TerminalPane.vue`（124 行） | `ResizeObserver` + `fitAddon` 现状 |
| `src/stores/useSystemStore.ts`（7 KB） | 需要在这里加 `resizeShell` |
| `src/bridge.ts:195-220` | `termResize` 的定义与终端事件订阅 |

| 不必读 | 原因 |
|---|---|
| `M3-4.b-prework-20260902-1055.md` | 已被本卡细化取代 |
| `@xterm/xterm` 源码 | 只用 `proposeDimensions()` / `resize()` 两个 API |

### 2.3 M3-3：终端历史与不落盘

| 必读（4） | 为什么 |
|---|---|
| `logs/assist/M3-terminal-history-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡，含测试夹具 |
| `src-tauri/src/bridge.rs:1958-1981` | 读取线程现状（emit 完即弃） |
| `src/components/system/TerminalPane.vue` | 加清空入口与 `scrollback` 的位置 |
| `src/stores/useSystemStore.ts` | `termLines` 遗留清理点 |

| 不必读 | 原因 |
|---|---|
| `M3-4.b-prework` 全文 | 已被本卡取代 |
| `sync.rs` / `grid_process.rs` | 无关 |

---

## 3. M4 · 数据库 / 定时任务

### 3.1 M4-1~M4-4：数据库 schema / 迁移 / 回滚

| 必读（6） | 为什么 |
|---|---|
| `logs/assist/database-schema-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡，含 schema 草案与验收脚本草案 |
| `logs/assist/M4-14.a-prework-20260902-1055.md` §5-bis | 迁移回滚范式 |
| `src-tauri/src/workspace.rs` | 现有 JSON 落盘点（迁移对账基准） |
| `src-tauri/src/domain.rs` | 现有模型（迁移目标） |
| `src-tauri/src/keyring_store.rs` | K3 凭据不入库 |
| `logs/assist/M5-13.a-prework-20260902-1055.md` | 图谱共享同一存储选型 |

| 不必读 | 原因 |
|---|---|
| `logs/assist/M5-15.a-prework` | 无关（那是插件权限） |
| `bridge.rs` 全文 | 只需 688-790 的闸门范式 |

**前置**：必须先人工拍板 D1~D4（存储选型）。

### 3.2 M4-5~M4-8：定时任务

| 必读（6） | 为什么 |
|---|---|
| `logs/assist/scheduled-task-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `logs/assist/M4-58.a-prework-20260902-1055.md` | 契约草案（调度/幂等/失败恢复/审计拆分） |
| `logs/assist/script-execution-safety-taskcard-20260902-1146.md` §4.4/§4.7 | 复用 `run_script` 通道与进程组范式 |
| `src-tauri/src/main.rs:598-605` | 常驻线程的挂载位置 |
| `src-tauri/src/workspace.rs` | 持久化范式（**注意其非原子写是反例**） |
| `logs/assist/M3-terminal-shutdown-taskcard-20260902-1146.md` §4.2 | `SchedulerShutdown` 的注册方式 |

| 不必读 | 原因 |
|---|---|
| `main.rs` 全文 | 只读 598-605 |
| 前端 `.vue` 全部 | 本任务后端为主 |

---

## 4. M5 · 协议 / Skill / 图谱 / 插件

### 4.1 M5-1~M5-3：A2P / A2A 协议

| 必读（6） | 为什么 |
|---|---|
| `logs/assist/A2P-A2A-protocol-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `logs/assist/M5-7.a-prework-20260902-1055.md` | 能力清单 / stdio 传输 / McpGlobalPolicy 草案 |
| `logs/assist/script-execution-safety-taskcard-20260902-1146.md` §4.7 | 进程组 spawn/kill 范式复用 |
| `logs/assist/M3-terminal-shutdown-taskcard-20260902-1146.md` §4.2 | `AgentShutdown` 注册 |
| `src-tauri/src/bridge.rs:688-790` | 安全闸门范式 |
| `src-tauri/src/workspace.rs` | 审计落盘与 1000 条上限（K5） |

| 不必读 | 原因 |
|---|---|
| 前端 `.vue` 全部 | 本任务后端为主（`AINavPanel.vue` 仅作未来 UI 参照，可跳过） |

### 4.2 M5-4~M5-6：Agent-Skill 契约

| 必读（6） | 为什么 |
|---|---|
| `logs/assist/agent-skill-contract-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `logs/assist/M5-12.a-prework-20260902-1055.md` | SkillDef / 能力边界 / 流式回传草案 |
| `logs/assist/A2P-A2A-protocol-taskcard-20260902-1146.md` §4.3 | 能力白名单（**两处必须共用同一份**） |
| `logs/assist/script-execution-safety-taskcard-20260902-1146.md` §4.4 | 参数校验复用（危险参数口径一致） |
| `src-tauri/src/domain.rs` | 新类型加这里 |
| `src-tauri/src/keyring_store.rs` | password 参数的 keyring 引用 |

| 不必读 | 原因 |
|---|---|
| `M5-15.a-prework`（插件权限） | 权限模型不同源，别混 |

### 4.3 M5-7~M5-9：图谱数据模型

| 必读（6） | 为什么 |
|---|---|
| `logs/assist/graph-model-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `logs/assist/M5-13.a-prework-20260902-1055.md` | GraphNode/GraphEdge/邻接表/两阶段抽取草案 |
| `logs/assist/database-schema-taskcard-20260902-1146.md` §4.5/§5 | 迁移体系与原子写（图谱表挂进同一 SQLite） |
| `src-tauri/src/domain.rs` | `Artifact` 是当前唯一可用的上游实体 |
| `src-tauri/src/workspace.rs` | 持久化的孤儿问题根源 |
| `logs/assist/A2P-A2A-protocol-taskcard-20260902-1146.md` §4.3 | Agent 可见性的隐私默认值 |

| 不必读 | 原因 |
|---|---|
| `sync.rs` | 无关 |
| 前端 `.vue` | 本阶段无图谱 UI 卡 |

**前置**：存储选型已拍板 + M4 数据库已落地。

### 4.4 M5-10~M5-12：插件运行时 + 权限整改

| 必读（7） | 为什么 |
|---|---|
| `logs/assist/plugin-runtime-taskcard-20260902-1146.md`（**全读**） | 本任务权威卡 |
| `logs/assist/plugin-permission-taskcard-20260902-1146.md`（**全读**） | `withGlobalTauri` 整改 + 可行性判定步骤 |
| `logs/assist/M5-15.a-prework-20260902-1055.md` §5 | PluginManifest / 权限分级 / fail-closed |
| `src-tauri/tauri.conf.json` | 两个红线配置（`withGlobalTauri` / `assetProtocol.scope` / `csp: null`） |
| `src-tauri/capabilities/default.json` + `browser-remote.json` | 现状 capability |
| `src-tauri/permissions/default-commands.toml` | 59 个命令（拆分对象） |
| `logs/assist/M3-terminal-shutdown-taskcard-20260902-1146.md` §4.2/§4.4 | `PluginShutdown` + 进程组范式 |

| 不必读 | 原因 |
|---|---|
| `tauri-browser-tabs/` 子 crate 源码 | 除非接管子 webview 加载通道 |

---

## 5. 跨任务：验收脚本实现

| 必读（3） | 为什么 |
|---|---|
| `logs/assist/acceptance-script-drafts-20260902-1146.md` | 35 个脚本的检查项/输入/退出码/失败信息/CI 化判定 |
| `logs/baseline-2026-08-27.md` | 阈值来源 |
| `logs/assist/strong-model-minimal-read-list-20260902-1146.md`（本文 §0） | 全局必读 6 项 |

---

## 6. ⛔ 全局「不必读」清单（**所有任务都跳过**）

| 文件/目录 | 大小 | 为什么不必读 |
|---|---|---|
| `src-tauri/src/bridge.rs` 全文 | 78 KB / 2029 行 | 按任务读对应区段；各任务卡已标好行号 |
| `src-tauri/src/main.rs` 全文 | 31 KB / 约 900 行 | 只读相关区段 |
| `src-tauri/src/grid_process.rs` 全文 | 29 KB / 705 行 | 仅 #3 宫格相关，M2~M5 任务均不涉及 |
| `src-tauri/src/sync.rs` 全文 | 11 KB / 287 行 | 只需 `bridge.rs:688-790` 的闸门范式 |
| `src/store/useBrowserStore.ts` | 25 KB | 仅浏览器相关 |
| `超详细审核报告-严重问题与缺陷.md` | 16 KB | 历史长文档，除 M0-6 依赖清理外无关 |
| `5种重构方案.md` / `重构方案-最终版.md` / `重构方案与实践流程.md` | 合计 ~30 KB | 历史方案，已尘埃落定 |
| `宫格崩溃多进程改造-*.md`（3 份） | 合计 ~50 KB | 仅 #3 相关 |
| `prototype.html` / `prototype-index.html` | 49 KB | 原型，非实现参照 |
| `gen_variants.py` | 74 KB | 变体生成脚本，无关 |
| `variants/` 全部 15 个 HTML | — | **读 1 个**作风格参照即可 |
| `tauri-browser-tabs/` 子 crate | — | 除非接管子 webview 加载通道 |
| `src-tauri/target/**` | — | 构建产物 |

---

## 7. 给强模型的 5 条硬提示

1. **先读 §0 全局 6 项，再读对应任务的 3~8 项**，其余一律跳过。总阅读量可控制在 **2~4 万 token**。
2. **每个任务卡第「验收命令」小节的命令都标了「未执行」**——不要把它们当已通过的证据；实现后必须真跑。
3. **改 `domain.rs` 时新字段必须 `#[serde(default)]`**，否则历史 `workspace/*.json` 会被 `load_artifacts` 静默丢弃（`workspace.rs:53` 的 `if let Ok` 吞错）。
4. **新增 `#[tauri::command]` 必须同步进 `permissions/default-commands.toml`**（K1）——漏加被 ACL 静默拒绝，无日志，极难排查。
5. **先做退出收口再动功能**（M3-1.a / TASK-10 只有少量改动，却解锁 5 个下游任务）。

---

## 8. 验收命令（本文为阅读建议，无可执行命令）

```bash
# 本文不含可执行验收命令；作用是降低后续强模型的探索成本。
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 估算阅读量（字节）
wc -c logs/assist/free-model-prework-M2-M5-20260902-1055.md \
      logs/assist/errata-to-taskcards-20260902-1146.md \
      src-tauri/src/domain.rs src-tauri/src/workspace.rs \
      logs/baseline-2026-08-27.md | tail -1
```

---

## 9. 失败动作

| 失败 | 动作 |
|---|---|
| 某任务缺必读文件（如主文档不存在） | 按 `repo-sanity-audit` §3 处置：记录实测输出，**不创建替代主文档** |
| 必读文件超过 8 个 | 说明任务拆分不够；拆任务而非加阅读量 |
| 读了「不必读」的长文档导致上下文溢出 | 回到本文 §6 清单，重新只加载必读项 |

---

## 10. 推荐模型

本文本身：`AI:FAST`（已完成）。
