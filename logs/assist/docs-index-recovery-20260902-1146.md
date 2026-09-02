# docs-index-recovery（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 性质：**新建索引**（仓库内此前不存在任何 `assist-index-*.md`，见 `repo-sanity-audit-20260902-1146.md` §2.5/§3）
> 覆盖：`logs/assist/` 全部 15 份既有文档 + 本批次新增文档（本批次文档以「批次 4」标注，逐份产出后在本文件 §6 增补）

---

## 1. TASK：docs-index-recovery

把 `logs/assist/` 下的前置文档按 M1/M2/M3/M4/M5 分类，标注：**必读 / 可选读 / 仅归档 / 已过期·口径冲突**。

分类图例：

| 标记 | 含义 | 强模型处置 |
|---|---|---|
| 🔴 **必读** | 含契约/勘误/阻塞结论，不读会做错 | 开工前必读 |
| 🟡 **可选读** | 有参考价值，可延后 | 遇到具体问题时再读 |
| ⚪ **仅归档** | 历史快照/重复内容，正常流程无需读 | 跳过 |
| ⛔ **已过期/口径冲突** | 与 2026-09-02 实测冲突，**不可直接采信** | 只读其「冲突点说明」，以勘误为准 |

---

## 2. M0 / 全局

| 文档 | 分类 | 建议 |
|---|---|---|
| `logs/baseline-2026-08-27.md` | 🔴 **必读** | clippy 13 warning + 主 JS 505 KB **硬门槛**，所有任务验收都要对照 |
| `logs/assist/repo-sanity-audit-20260902-1146.md`（本批次） | 🔴 **必读** | pwd/branch/提交/文档存在性/NEXT 来源 |
| 本文档 `docs-index-recovery-20260902-1146.md` | 🔴 **必读** | 入口索引 |
| `详细设计与实施计划.md`（仓库根，38 KB） | 🟡 **可选读** | ⛔ §「5 个工具已落盘」部分已过期，见勘误 |
| `后续需求TODO.md`（仓库根，32 KB） | 🟡 **可选读** | 需求权威来源；⛔ §2 括号内「文件已落盘」表述已过期 |
| `PROJECT-RULES.md` | 🔴 **必读** | 项目规则 |
| ⛔ `AI-模型切换与接手清单.md` | ❌ **不存在** | 无法索引；编号口径校准阻塞项 |

---

## 3. M1（基础能力）

本仓库 `logs/assist/` 下**无 M1 文档**。M1 相关内容散落在：

| 内容 | 位置 | 分类 |
|---|---|---|
| 领域模型 `Artifact`/`AuditEntry`/`RepoConfig` | `src-tauri/src/domain.rs`（92 行） | 🔴 必读 |
| 持久化 + 审计 + 1000 条上限 | `src-tauri/src/workspace.rs`（110 行） | 🔴 必读 |
| 两段式安全闸门范式 | `src-tauri/src/bridge.rs:688-790` | 🔴 必读 |
| keyring 用法 | `src-tauri/src/keyring_store.rs`（25 行） | 🟡 可选读 |

---

## 4. M2（脚本库 / 小工具 / 图片）

| 文档 | 大小 | 锚定 WBS | 分类 | 说明 |
|---|---|---|---|---|
| `M2-1.a-prework-20260902-1055.md` | 14.6 KB | §4.4 **M2-8**（#10 图片） | 🔴 **必读** | ImageRef/MIME/大小/路径 契约冻结 + 反向用例 |
| `M2-2.b-prework-20260902-1055.md` | 10.1 KB | §4.4 **M2-9** | 🟡 可选读 | 图片预览 UI mock（**本批次 TASK-5 静态壳方案将取代并细化**） |
| `M2-3.a-prework-20260902-1055.md` | 15.8 KB | §4.1 **M2-1+M2-2**（#1/#4 脚本） | 🔴 **必读** | ScriptMeta/参数/危险参数/审计/取消超时 契约 |
| `M2-5.b-prework-20260902-1055.md` | 13.8 KB | §4.1 **M2-3** | 🟡 可选读 | 脚本库 UI mock（**本批次 TASK-6 取代细化**） |
| `M2-7.b-prework-20260902-1055.md` | 16.2 KB | §4.3 **M2-5+M2-6+M2-7**（#2 工具） | 🔴 **必读** | ToolMeta 清单 + `include_dir!` 打包；§5.R3 = `withGlobalTauri` 红线 |
| `M2-9.b-prework-20260902-1055.md` | 17.9 KB | §4.3 **M2-6** | 🔴 **必读** | 5 个种子工具**验收表**；§0 已修正「文件不存在」 |
| `M2-tools-seed-html-prework-20260902-1146.md`（本批次） | — | **M2-6** | 🔴 **必读** | 5 个 HTML **从零落盘**任务卡（含 HTML 结构/CSS-JS 约束/契约） |
| `M2-image-ui-static-shell-20260902-1146.md`（本批次） | — | **M2-9** | 🔴 必读（接图片 UI 时） | 图片 UI 静态壳方案 |
| `M2-script-library-ui-static-shell-20260902-1146.md`（本批次） | — | **M2-3** | 🔴 必读（接脚本 UI 时） | 脚本库 UI 静态壳方案 |
| `M2-tool-library-ui-static-shell-20260902-1146.md`（本批次） | — | **M2-5** | 🔴 必读（接工具箱 UI 时） | 工具库 UI 静态壳方案 |
| `script-execution-safety-taskcard-20260902-1146.md`（本批次） | — | **M2-1/M2-2** | 🔴 **必读** | 脚本执行安全卡（禁 `sh -c` 拼接、进程组回收、背压） |

---

## 5. M3（终端）

| 文档 | 大小 | 锚定 WBS | 分类 | 说明 |
|---|---|---|---|---|
| `M3-1.a-prework-20260902-1055.md` | 19.4 KB | §5 **M3-1+M3-2+M3-3**（#9/#3） | 🔴 **必读** | 终端现状盘点：PTY/resize/history/进程归属/退出接入点 |
| `M3-4.b-prework-20260902-1055.md` | 15.0 KB | §5 **M3-4** | 🟡 可选读 | 低风险体验草案（历史上限/resize 静默窗口）→ **本批次 TASK-9/10 拆为独立可执行卡** |
| `M3-terminal-resize-taskcard-20260902-1146.md`（本批次） | — | **M3-2** | 🔴 **必读** | resize 正式整改卡（含 `TerminalSession.master` 方案） |
| `M3-terminal-history-taskcard-20260902-1146.md`（本批次） | — | **M3-3** | 🔴 **必读** | 历史/敏感过滤/不落盘策略 |
| `M3-terminal-shutdown-taskcard-20260902-1146.md`（本批次） | — | **M3-1 + M0-2** | 🔴 **最高必读** | PTY 生命周期 + ShutdownCoordinator，**阻塞 5 个下游** |

---

## 6. M4（数据库 / 定时任务）

| 文档 | 大小 | 锚定 WBS | 分类 | 说明 |
|---|---|---|---|---|
| `M4-14.a-prework-20260902-1055.md` | 19.2 KB | §6 **M4-1~M4-4**（#6） | 🔴 **必读** | schema/迁移/回滚/验收/风险；§5-bis 迁移回滚范式 |
| `M4-58.a-prework-20260902-1055.md` | 18.0 KB | §6.1 **M4-5~M4-8**（#11） | 🔴 **必读** | 调度/幂等/失败恢复/审计拆分 |
| `database-schema-taskcard-20260902-1146.md`（本批次） | — | **M4-1~M4-4** | 🔴 必读 | schema 草案 + 迁移回滚 + 备份兼容 |
| `scheduled-task-taskcard-20260902-1146.md`（本批次） | — | **M4-5~M4-8** | 🔴 必读 | 调度模型/幂等键/错过补偿/ShutdownCoordinator 接入 |

---

## 7. M5（A2P/A2A / Skill / 图谱 / 插件）

| 文档 | 大小 | 锚定 WBS | 分类 | 说明 |
|---|---|---|---|---|
| `M5-7.a-prework-20260902-1055.md` | 18.5 KB | §7 **M5-1~M5-3**（#7） | 🔴 **必读** | A2P/A2A 能力清单/stdio 传输/McpGlobalPolicy |
| `M5-12.a-prework-20260902-1055.md` | 17.5 KB | §7.1 **M5-4~M5-6**（#12） | 🔴 **必读** | SkillDef/能力边界/流式回传/记忆层 |
| `M5-13.a-prework-20260902-1055.md` | 19.0 KB | §7.2 **M5-7~M5-9**（#13） | 🔴 **必读** | GraphNode/GraphEdge/SQLite 邻接表/两阶段抽取 |
| `M5-15.a-prework-20260902-1055.md` | 18.9 KB | §7.3 **M5-10~M5-12**（#15） | 🔴 **必读** | PluginManifest/权限分级/fail-closed；§5.R1 = withGlobalTauri |
| `free-model-prework-M2-M5-20260902-1055.md` | 16.6 KB | 汇总 | 🔴 **必读** | 编号映射表 + 5 项勘误 + 阻塞依赖图 + 9 条共性红线 K1~K9 |
| `A2P-A2A-protocol-taskcard-20260902-1146.md`（本批次） | — | **M5-1~M5-3** | 🔴 必读 | 消息结构/能力声明/权限/超时/取消/错误码 |
| `agent-skill-contract-taskcard-20260902-1146.md`（本批次） | — | **M5-4~M5-6** | 🔴 必读 | SkillMeta/输入输出/权限/安装启用/版本/审计 |
| `graph-model-taskcard-20260902-1146.md`（本批次） | — | **M5-7~M5-9** | 🔴 必读 | 节点/边/事件/来源/增量/查询/隐私过滤/容量 |
| `plugin-runtime-taskcard-20260902-1146.md`（本批次） | — | **M5-10~M5-12** | 🔴 必读 | 安装包结构/manifest/权限/隔离/生命周期/卸载 |
| `plugin-permission-taskcard-20260902-1146.md`（本批次） | — | **M5 + #2 工具** | 🔴 **必读** | `withGlobalTauri=true` 整改卡（跨 M2/M5） |

---

## 8. ⛔ 已过期 / 口径冲突清单（**强模型切勿采信**）

| 冲突点 | 出处 | 实测反证 | 以何为准 |
|---|---|---|---|
| 「5 个种子工具 HTML 已落盘 `src-tauri/src/tools/`」 | `详细设计与实施计划.md:144/178`；`后续需求TODO.md:37-51` | `ls src-tauri/src/tools` → No such file；`find -name "*tool*.html"` 零命中 | `errata-to-taskcards-20260902-1146.md` E1 |
| 「脚本库/工具箱已可加载」 | 同上 | 59 个命令中无 `list_tools`/`run_script`；前端无对应面板 | 同上 + `repo-sanity-audit §6.2` |
| 「`term_resize` 可用」 | 命令白名单已注册，看似可用 | `bridge.rs:2013-2016` 参数全吞，从不调 `MasterPty::resize` | E2 |
| 「终端支持窗口自适应」 | `TerminalPane.vue` 有 `ResizeObserver + fit()` | 前端 fit 只改 xterm，**不通知 PTY**；`termResize` 零调用 | E3 |
| 「关闭窗口已释放子进程」 | `main.rs:591` 有 `CloseRequested` | 只 `grid_manager.shutdown_all()`，**不碰 terminals**；`.run()` 无 `RunEvent` | E4 |
| 「`strip-ansi-escapes` 用于终端输出过滤」 | `Cargo.toml` 声明 | `src-tauri/src` 零引用；`bridge.rs:1942` 注释说明有意移除 | E6 |

---

## 9. 索引维护规则（写给后续模型）

1. 新增 `logs/assist/*.md` 时**必须**在本文档对应小节追加一行，并标注分类。
2. 若某文档被新文档取代，老文档改标 ⚪ **仅归档** 或 ⛔，并注明取代者。
3. 若发现新的「文档说法 vs 实测」冲突，**不得修改老文档**，只在 `errata-to-taskcards-*.md` 与本文档 §8 追加。
4. 删除文档=禁止。索引只能增不能减。

---

## 10. 验收命令

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 索引覆盖度：列出的文档数 >= 实际文件数
ls -1 logs/assist/*.md | wc -l
grep -c "prework-20260902-1055.md" logs/assist/docs-index-recovery-20260902-1146.md
# 每份 1055 批次文档都应出现在索引中
for f in logs/assist/*-20260902-1055.md; do
  n=$(basename "$f")
  grep -q "$n" logs/assist/docs-index-recovery-20260902-1146.md || echo "MISSING-IN-INDEX: $n"
done
```

预期：`MISSING-IN-INDEX` 零输出。

---

## 11. 失败动作

| 情况 | 动作 |
|---|---|
| 出现 `MISSING-IN-INDEX` | 补索引行，**不得**删除被漏索引的文档 |
| 某文档无法判定分类 | 暂标 🟡 可选读 + 备注「待人工判定」 |
| 文档被判定为 ⛔ | 保留文件，仅在索引标注；不删不改 |

---

## 12. 推荐模型

`AI:FAST`（已完成）。索引口径校准（等 `AI-模型切换与接手清单.md` 补齐后）→ `AI:BALANCED` + 人工。
