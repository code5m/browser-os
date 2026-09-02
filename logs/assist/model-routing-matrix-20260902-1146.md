# model-routing-matrix（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 用途：输出后续任务的**模型路由矩阵**——哪些免费/快模型可做、哪些只能强模型做、哪些必须人工 GUI 验收
> 状态：📝 路由建议（**不构成任何主任务的 PASS 结论**）

---

## 1. 路由档位定义

| 档位 | 适用 | 典型能力 |
|---|---|---|
| `AI:FAST` | 低风险、模式化、量大 | 文档整理、清单、grep 类静态检查脚本、mock 数据 |
| `AI:BALANCED` | 中等复杂度、单领域 | 前端组件、单文件工具、状态机、表单 |
| `AI:DEEP` | 跨层、涉及安全/并发/生命周期/所有权 | Rust 进程管理、协议实现、迁移、调度、插件运行时 |
| `AI:DEEP-xhigh` | 方案评审 / 高风险决策 | 安全隔离方案、权限模型、存储选型、协议评审 |
| `HUMAN` | 无法自动验证 | GUI 交互、进程回收、时间控制、视觉验收、安全放行 |

---

## 2. 主任务路由矩阵

### 2.1 M2 · 脚本库 / 小工具 / 图片

| 任务 | 文档 | 推荐模型 | 免费/快模型可做？ | 需人工 GUI | 说明 |
|---|---|---|---|---|---|
| M2-6 5 个种子 HTML（json/base64/timestamp） | `M2-tools-seed-html-prework` | `AI:BALANCED` | ✅ 大部分可做 | ✅ 必须 | 纯前端、逻辑简单；离线实跑必须人工 |
| M2-6 5 个种子 HTML（cron） | 同上 | `AI:DEEP` | ⚠️ 勉强 | ✅ 必须 | 边界多：6 段/越界/DST/未来推算 |
| M2-6 5 个种子 HTML（regex） | 同上 | `AI:DEEP` | ❌ 不建议 | ✅ 必须 | 六种语言转义 + 灾难性回溯保护 |
| M2-9 图片 UI 静态壳 | `M2-image-ui-static-shell` | `AI:BALANCED` | ✅ 可做 | ✅ 必须（14 截图点） | 组件化工作；SVG XSS 需人工验证 |
| M2-3 脚本库 UI 静态壳 | `M2-script-library-ui-static-shell` | `AI:BALANCED` | ✅ 可做 | ✅ 必须（14 截图点） | 状态机 + 表单，模式固定 |
| M2-5 工具库 UI 静态壳 | `M2-tool-library-ui-static-shell` | `AI:BALANCED` | ✅ 可做 | ✅ 必须（15 截图点） | 网格 + 搜索 + 多状态 |
| M2-1/M2-2 **脚本执行通道** | `script-execution-safety-taskcard` | `AI:DEEP` | ❌ **不可** | ✅ 必须 | 命令注入防线；安全评审建议 `AI:DEEP-xhigh` |
| M2-5/M2-7 `list_tools` + 打包嵌入 | `M2-7.b` + `M2-tools-seed-html` | `AI:DEEP` | ❌ 不可 | ✅ 必须 | `include_dir!` + `asset:` 协议 + CSP |

### 2.2 M3 · 终端

| 任务 | 文档 | 推荐模型 | 免费/快模型可做？ | 需人工 GUI | 说明 |
|---|---|---|---|---|---|
| **M3-1/M0-2 退出收口（ShutdownCoordinator）** | `M3-terminal-shutdown-taskcard` | `AI:DEEP`（评审 `AI:DEEP-xhigh`） | ❌ **不可** | ✅ 必须 | **P0，阻塞 5 个下游**；进程生命周期 + 事件循环 |
| M3-2 term_resize（后端 E2 + 前端 E3） | `M3-terminal-resize-taskcard` | `AI:DEEP` | ❌ 不可 | ✅ 必须（vim/top/htop） | Rust 所有权 + xterm fit 链路 |
| M3-3 终端历史与不落盘 | `M3-terminal-history-taskcard` | `AI:BALANCED` | ✅ 可做 | ✅ 必须 | 前端改动为主 |
| M3-4 终端低风险体验 | `M3-4.b` | `AI:BALANCED` | ✅ 可做 | ✅ 必须 | — |

### 2.3 M4 · 数据库 / 定时任务

| 任务 | 文档 | 推荐模型 | 免费/快模型可做？ | 需人工 GUI | 说明 |
|---|---|---|---|---|---|
| **存储选型决策（D1~D4）** | `database-schema-taskcard` §9 | `AI:DEEP-xhigh` + **HUMAN 拍板** | ❌ 不可 | ✅ 必须（决策） | 影响 M4 与 M5 图谱两处 |
| M4-1~M4-4 schema + 迁移 + 回滚 | `database-schema-taskcard` | `AI:DEEP` | ❌ 不可 | ✅ 必须 | 数据无价；备份恢复演练必须人工 |
| M4-5~M4-8 定时任务调度 | `scheduled-task-taskcard` | `AI:DEEP` | ❌ 不可 | ✅ 必须（misfire/退出） | 调度语义 + 持久化 + 进程 |
| 验收脚本实现（X 系列 + 静态类） | `acceptance-script-drafts` | `AI:FAST` / `AI:BALANCED` | ✅ 可做 | ❌ 否 | grep 类静态检查 |

### 2.4 M5 · 协议 / Skill / 图谱 / 插件

| 任务 | 文档 | 推荐模型 | 免费/快模型可做？ | 需人工 GUI | 说明 |
|---|---|---|---|---|---|
| **形态②可行性判定** | `plugin-permission-taskcard` §5 步骤 0 | `AI:DEEP-xhigh` + HUMAN | ❌ 不可 | ✅ 必须（决策） | 决定 HTML 工具加载方式与插件形态 |
| `withGlobalTauri` 整改 | `plugin-permission-taskcard` | `AI:DEEP` | ❌ **不可** | ✅ 必须（DevTools） | 安全红线；改动面覆盖全前端 |
| M5-1~M5-3 A2P/A2A 协议 | `A2P-A2A-protocol-taskcard` | `AI:DEEP` | ❌ 不可 | ✅ 必须 | 协议 + 子进程 + 并发 + 权限 |
| M5-4~M5-6 Agent-Skill 契约 | `agent-skill-contract-taskcard` | `AI:DEEP` | ❌ 不可 | ✅ 必须 | 与 A2A 共用能力层 |
| M5-7~M5-9 图谱数据模型 | `graph-model-taskcard` | `AI:DEEP`（阶段一抽取 `AI:BALANCED`） | ❌ 不可 | ✅ 必须（隐私/容量） | 依赖存储选型 |
| M5-10~M5-12 插件运行时 | `plugin-runtime-taskcard` | `AI:DEEP`（评审 `AI:DEEP-xhigh`） | ❌ **不可** | ✅ 必须（安全） | zip-slip / 进程隔离 / 权限 |

---

## 3. 免费/快模型「可做」清单（**本批次立场：趁免费额度优先清这些**）

| # | 任务 | 推荐档位 | 产出 | 阻塞依赖 |
|---|---|---|---|---|
| 1 | 3 个简单种子工具（json / base64 / timestamp） | `AI:BALANCED` | 3 个 HTML | 无（但完全离线验收需人工） |
| 2 | 图片 UI 静态壳实现 | `AI:BALANCED` | 7 个 `.vue` + mock | 无 |
| 3 | 脚本库 UI 静态壳实现 | `AI:BALANCED` | 8 个 `.vue` + mock | 无 |
| 4 | 工具库 UI 静态壳实现 | `AI:BALANCED` | 7 个 `.vue` + mock | 无 |
| 5 | 终端历史与不落盘 | `AI:BALANCED` | 前端改动 | 无（可与 E4 并行） |
| 6 | 验收脚本：X 系列 7 个 | `AI:FAST` | 7 个 `.sh` | 无 |
| 7 | 验收脚本：M2 静态类 3 个 | `AI:BALANCED` | 3 个 `.sh` | 无 |
| 8 | 死依赖清理（`strip-ansi-escapes` + `termLines`） | `AI:FAST` | 2 处删除 | 需先 grep 确认零引用 |
| 9 | 图谱阶段一确定性抽取 | `AI:BALANCED` | `extract.rs` | 存储选型已定 + M4 已完成 |

> ⚠️ 第 1~5 项虽可由快模型实现，**验收仍需人工 GUI**（截图点 + 反向用例）。

---

## 4. 只能强模型做（**快模型碰不得**）

| # | 任务 | 为什么只能强模型 | 最低档位 |
|---|---|---|---|
| 1 | ShutdownCoordinator（退出收口） | 进程生命周期 + Tauri 事件循环 + 并发 + 幂等；做错 = 全应用泄漏 | `AI:DEEP` |
| 2 | 脚本执行通道（`run_script`） | 命令注入防线；一处错 = 任意命令执行 | `AI:DEEP` |
| 3 | `withGlobalTauri` / 插件隔离整改 | 安全红线；需全前端回归 + DevTools 验证 | `AI:DEEP` |
| 4 | 插件运行时 | zip-slip + 进程隔离 + 权限；安全评审 | `AI:DEEP` + 评审 |
| 5 | A2P/A2A 协议 | 协议 + 子进程 + 并发 + 权限 | `AI:DEEP` |
| 6 | 数据库 schema / 迁移 / 回滚 | 数据无价；错误不可逆 | `AI:DEEP` |
| 7 | 定时任务调度 | 幂等 + 错过补偿 + 持久化 + 进程 | `AI:DEEP` |
| 8 | 图谱模型 | 依赖存储选型 + 隐私过滤 | `AI:DEEP` |
| 9 | term_resize（E2+E3） | Rust 所有权 + xterm fit 链路 | `AI:DEEP` |
| 10 | cron / regex 两个种子工具 | 边界用例多 | `AI:DEEP` |

---

## 5. 必须人工参与的项（**AI 不得代签**）

| 类别 | 具体项 | 为什么 |
|---|---|---|
| **GUI 视觉验收** | 图片 UI 14 截图点 / 脚本库 14 点 / 工具库 15 点 | 布局、视觉、交互手感无法脚本化 |
| **终端交互** | `vim` / `top` / `htop` 的 resize 表现 | 需人眼判断残影与错位 |
| **进程回收** | `ps -ef` 检查孤儿（`sleep` / `yes` / 脚本 / 插件） | 需真实操作应用关闭 |
| **时间控制** | 定时任务的 misfire 三策略 | 需关闭应用 / 系统休眠 |
| **安全验证** | DevTools 执行 `window.__TAURI__`、注入 payload、恶意插件包 | 安全结论需人确认 |
| **数据操作** | 数据库备份恢复演练、迁移回滚演练 | 涉及真实数据，不可逆 |
| **决策拍板** | 存储选型（D1~D4）、形态②可行性、能力白名单范围 | 业务/风险权衡 |
| **隐私验收** | 图谱 `restricted` 是否泄漏、导出是否含私密 | 需人工审查样本 |

---

## 6. 路由决策速查

```
新问题进来，按以下顺序判定：

1) 是否涉及进程/并发/所有权/生命周期？          是 → AI:DEEP
2) 是否涉及安全边界（注入/权限/CSP/路径逃逸）？   是 → AI:DEEP（+ xhigh 评审）
3) 是否涉及不可逆数据操作（迁移/删除/覆盖）？     是 → AI:DEEP + HUMAN
4) 是否跨 ≥3 层（Rust + 前端 + 配置）？          是 → AI:DEEP
5) 是否纯前端组件/状态机/表单？                  是 → AI:BALANCED
6) 是否文档/清单/grep 类脚本/mock 数据？          是 → AI:FAST
7) 是否需要看屏幕/操作 GUI/判断视觉？            是 → HUMAN（叠加在上述任一档位上）
```

---

## 7. 与批次任务编号的对应

| 本批次 TASK | 产出文档 | 路由建议 |
|---|---|---|
| 1 repo-sanity-audit | `repo-sanity-audit-20260902-1146.md` | `AI:FAST`（已完成） |
| 2 docs-index-recovery | `docs-index-recovery-20260902-1146.md` | `AI:FAST`（已完成） |
| 3 errata-to-taskcards | `errata-to-taskcards-20260902-1146.md` | `AI:FAST`（已完成） |
| 4 M2-tools-seed-html-prework | `M2-tools-seed-html-prework-20260902-1146.md` | `AI:FAST` 出卡 / `AI:BALANCED`+`AI:DEEP` 实现 |
| 5 M2-image-ui-static-shell | `M2-image-ui-static-shell-20260902-1146.md` | `AI:FAST` 出卡 / `AI:BALANCED` 实现 |
| 6 M2-script-library-ui-static-shell | `M2-script-library-ui-static-shell-20260902-1146.md` | 同上 |
| 7 M2-tool-library-ui-static-shell | `M2-tool-library-ui-static-shell-20260902-1146.md` | 同上 |
| 8 M3-terminal-resize-taskcard | `M3-terminal-resize-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 9 M3-terminal-history-taskcard | `M3-terminal-history-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:BALANCED` 实现 |
| 10 M3-terminal-shutdown-taskcard | `M3-terminal-shutdown-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现（**P0**） |
| 11 plugin-permission-taskcard | `plugin-permission-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP`+xhigh 实现 |
| 12 script-execution-safety-taskcard | `script-execution-safety-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 13 scheduled-task-taskcard | `scheduled-task-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 14 database-schema-taskcard | `database-schema-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 15 A2P-A2A-protocol-taskcard | `A2P-A2A-protocol-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 16 agent-skill-contract-taskcard | `agent-skill-contract-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 17 graph-model-taskcard | `graph-model-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP` 实现 |
| 18 plugin-runtime-taskcard | `plugin-runtime-taskcard-20260902-1146.md` | `AI:FAST` 出卡 / `AI:DEEP`+xhigh 实现 |
| 19 acceptance-script-drafts | `acceptance-script-drafts-20260902-1146.md` | `AI:FAST` 出草案 / 实现按需 |
| 20 model-routing-matrix | 本文 | `AI:FAST`（已完成） |
| 21 strong-model-minimal-read-list | `strong-model-minimal-read-list-20260902-1146.md` | `AI:FAST`（已完成） |

---

## 8. 推荐执行顺序（按「解锁价值 / 风险」排序）

```
第 0 步（决策，人工）：
   D1~D4 存储选型 + 形态②可行性判定
        ↓
第 1 步（P0，AI:DEEP）：
   ShutdownCoordinator 退出收口 ────── 解锁 5 个下游
        ↓
第 2 步（低风险，AI:BALANCED，可并行）：
   3 个简单种子工具 + 三份 UI 静态壳 + 终端历史
        ↓
第 3 步（AI:DEEP）：
   run_script 执行通道 ──→ 定时任务 ──→ 数据库
        ↓
第 4 步（AI:DEEP）：
   term_resize（E2+E3 同 PR）
        ↓
第 5 步（安全整改，AI:DEEP + 评审）：
   withGlobalTauri 整改
        ↓
第 6 步（AI:DEEP，风险递增）：
   A2P/A2A 协议 ──→ Agent-Skill ──→ 图谱 ──→ 插件运行时
        ↓
全程（AI:FAST）：
   验收脚本（X 系列优先）
```

---

## 9. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 中 | 强模型也可能把「安全类」任务做漏 | 安全类一律走 `AI:DEEP` + `AI:DEEP-xhigh` 评审 + 人工验收三重 |
| 中 | 快模型做 UI 静态壳时「顺手」接了真实通道 | 静态壳有 grep 红线校验（零 `invoke`），已写入验收脚本 M2-S3 |
| 中 | 路由被当作「可以跳步」的依据 | 依赖图（上一批次 §4 + 本文 §8）是硬约束：上游未完成不得开工 |
| 低 | 档位划分过粗 | 按 §6 速查流程逐条判定，不只看表格 |

---

## 10. 验收命令（本文为路由建议，无可执行命令）

```bash
# 本文不含可执行的验收命令；其作用是路由决策参考。
# 各任务的实际验收命令见对应任务卡的「验收命令」小节。
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
ls -1 logs/assist/*-20260902-1146.md | wc -l      # 本批次产出文档数
```

---

## 11. 失败动作

| 失败 | 动作 |
|---|---|
| 某任务无法判定档位 | 取**更高**档位（保守） |
| 强模型产出未通过人工验收 | 按任务卡的「失败动作」回退或补做，不得降档重试 |
| 快模型任务做不动 | 升级档位，不得硬凑产出 |

---

## 12. 推荐模型

本文本身：`AI:FAST`（已完成）。
路由结论的执行监督：人工。
