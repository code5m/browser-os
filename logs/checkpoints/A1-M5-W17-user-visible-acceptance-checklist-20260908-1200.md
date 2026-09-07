# A1 M5-W17 用户可见验收清单 / 已知限制（DOCS ONLY · 2026-09-08 12:00 CST）

> Lane A1（M5-W17 · Desktop Client Completeness and Home Recovery Dispatch）
> 派发来源：`PARALLEL_COMMAND_BOARD.md` L1164-1215（M5-W17 Desktop Client Completeness and Home Recovery Dispatch）
> A1 W17 任务行（board L1185）：*Reconcile retained W16 outputs and write the W17 user-visible acceptance checklist/known limitations. No product code.*
> 配套交付：**A1 W17 reconciliation checkpoint** `logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md`
> 立场：**DOCS ONLY · 零产品代码 · 不 commit · 不 push**
> 用途：供 **A8 实跑验收**、**A11 独立矩阵引用**、**A0 拣入期人工走查**。清单项必须**由人真跑勾选**；A1 不代填、不伪造结果。

---

## §0 一页结论

W17 是 M5 里**第一个真实前端产品代码波**（有界：桌面启动体验 + 首页 + 导航 + shell 兜底），但**运行时权限面维持全锁**。本清单把 board 六条共享 AC 展开为 **6 组 24 项用户可见检查项**（人工勾选）+ **5 条已知限制** + **1 条通过判定口径**。

**最关键的两条风险**（A1 必须前置提示）：
1. **体积余量仅 0.06pp**（基线 `total_bytes_pct=25.14`，上限 25.2%）—— A5/A6/A7 任一新增 chunk 都可能触顶；触顶必须**报精确 delta 并瘦身**，**禁止抬上限**。
2. **首页容易滑向"营销页"**——AC-2 明确要求"exposes the existing principal work areas as usable routes… It must not become a marketing page"；验收时必须逐项确认每个 launcher 都**真的能进入既有功能**，而非装饰性入口。

---

## §1 基线与前提

| 项 | 值 |
|---|---|
| mainline | `052b18a`（W15 release-readiness PUSHED and accepted） |
| 工作树 | 含 W16 全部 retained 文档产出（未 push） |
| W17 性质 | 有界**前端/启动体验**产品代码波；**不新增运行时权限** |
| 集成顺序 | `A2 -> A3 -> A5 -> A6 -> A7 -> A4/A9 -> A8/A10/A11 -> A0` |
| 体积基线 | `total_bytes_pct=25.14` ≤ **25.2%**（一次性上限），`cargo_warnings` delta = 0 |
| 门禁原则 | 超限报精确 delta，**不抬上限**（board L1178） |

**前置确认（勾选后才能进入 §2）**：

- [ ] `git fetch origin && git pull --ff-only` 成功，HEAD = A0 集成后的 W17 提交（或已应用供验收的补丁）
- [ ] 工作树无与验收无关的脏文件（其他 lane 的 W16/W17 产物应已被 A0 集成或明确挂起）
- [ ] 已记录本机环境：OS / 桌面环境 / 屏幕宽度（窄窗口验收需 ≤ 某阈值，记录实际值）/ Node 与 Rust 版本

---

## §2 用户可见验收清单（6 组 · 24 项）

> 每项三态勾选：`PASS` / `FAIL` / `N/A`。`FAIL` 必须附**实际看到的现象**（不写"不符合预期"这类空话），`N/A` 必须写原因。

### 组 A · 启动（AC-1 · 责任 A2 · 实跑 A8/A11）

- [ ] **A-1** 从干净状态执行桌面 dev 启动助手（如 `run-gui.sh`），**不出现** `localhost:1421` connection refused
- [ ] **A-2** 助手能**检测**到已存在的 Vite dev server 并复用（不会盲目再起一个）
- [ ] **A-3** 助手在 dev server 未就绪时会**等待**就绪后再拉起桌面端（不是固定 sleep 后硬拉）
- [ ] **A-4** 退出桌面端后，助手**只清理自己启动的** dev server；用户自己预先启动的 server **仍在运行**（ownership-safe，是 AC-1 最容易错的一条）
- [ ] **A-5** release 行为**不变**：仍加载 bundled assets，不依赖 dev server（release 构建下不出现任何 dev-only 路径）
- [ ] **A-6** 启动失败时给出**可读的错误提示**（不是静默失败/空白窗口），且提示中**不含**本地绝对路径、端口凭据、敏感 URL/query

### 组 B · 首页 / Home（AC-2 · 责任 A3 store + A5 组件 · 测试 A9 · 实跑 A8）

- [ ] **B-1** 首页列出**既有主工作区**入口（如 workspace / graph / plugin / agent-skill / terminal 等既有能力），且每个入口**点击后真的进入对应功能**
- [ ] **B-2** 首页**不是营销页**：无标语式 hero、无虚构能力宣传、无"敬请期待"占位入口（占位入口若存在必须显式标注不可用原因）
- [ ] **B-3** 快捷方式 / 最近项**有界**：列表长度受显式常量上界约束（不是无上限增长），超出时按既定策略截断
- [ ] **B-4** 快捷方式 / 最近项**抗畸形本地数据**：手工写入损坏/超长/类型错误的本地数据后，首页**不崩溃、不白屏**，回退到稳定默认并（可选项）记录一条非敏感的告警
- [ ] **B-5** 空态 / 加载态 / 错误态**连贯且可读**：空列表有明确文案与下一步动作；加载中有可见指示；错误态给出可行动提示（非裸错误对象）
- [ ] **B-6** 首页**无后端契约改动**：不新增/不调用任何 Tauri 命令（只消费既有 `src/bridge.ts` 能力）

### 组 C · 导航与窄窗口（AC-3 · 责任 A6/A7 · 实跑 A8）

- [ ] **C-1** 侧边/活动栏能让用户**发现既有模块**（不存在"功能存在但找不到入口"的情况）
- [ ] **C-2** active 态**稳定**：切换面板后 active 高亮正确；刷新/重进后 active 态不丢失或能合理恢复
- [ ] **C-3** **窄窗口**下导航仍可用（不出现入口被永久裁剪且无替代路径）；记录实测窗口宽度阈值
- [ ] **C-4** 面板解析失败 / 路由失效时，**主区不出现空白死区**：有可读兜底文案（A7 责任）
- [ ] **C-5** 状态栏（`StatusBar.vue`）在启动/错误/兜底态下的呈现**可读且不误导**（不显示虚假"已就绪"）

### 组 D · 可达性与无敏感泄露（AC-4 · 责任 A4 静态 + A9 DOM + A10 安全）

- [ ] **D-1** 所有新增可见控件**键盘可达**（Tab 顺序合理，无键盘陷阱）
- [ ] **D-2** 所有新增可见控件**有 accessible name**（图标按钮/纯图形控件必须有 `aria-label` 或等价可访问名）
- [ ] **D-3** 沿用**既有视觉语言**（未引入新设计体系/新依赖/新图标库）
- [ ] **D-4** 新增 UI 与错误提示中**无敏感泄露**：不含凭据、token、原始敏感 URL/query 值、本地绝对路径、签名/公钥材料
- [ ] **D-5** `scripts/check-home-client-policy.py`（A4 新）PASS；`scripts/check-home-ui-logic.mjs`（A9 新）PASS

### 组 E · 体积门禁（AC-5 · 责任 A11 · 全 lane 自律）

- [ ] **E-1** `scripts/measure-build-metrics.py` 复测：`total_bytes_pct ≤ 25.2`（基线 25.14%，余量 **0.06pp**）
- [ ] **E-2** `cargo_warnings` delta = **0**
- [ ] **E-3** 若超限：已记录**精确 delta** 与归因文件，并**已瘦身或已上报**；**未**修改 `TOTAL_BYTES_GROWTH_LIMIT_PCT` 抬上限

### 组 F · 交付完整性（AC-6 · 全 11 lane + A0）

- [ ] **F-1** 每个 lane 交付整包：代码/测试（如分配）+ checkpoint + 精确命令与结果 + `git diff --check` + **binary patch**
- [ ] **F-2** 允许文件被他 lane 改动时**出 binary patch**，未覆盖他人改动、未跨 lane 解冲突
- [ ] **F-3** 无 lane 自行 commit / push（仅 A0）

---

## §3 已知限制（W17 明确不做 / 遗留）

> 这些是**设计边界**，不是缺陷。验收时不应因这些项判 FAIL；若产品认为需要，应另开波次而非在 W17 内扩张。

| # | 已知限制 | 性质 | 去向 |
|---|---|---|---|
| L-1 | 首页/launcher **只暴露既有主工作区**，不新增业务能力 | W17 边界（board L1175） | 新能力走 M6（见 `A1-M5-W16-M6-WBS-proposal-20260908-1100.md`） |
| L-2 | **运行时权限仍全锁**：plugin invoke / 命令执行 / 动态加载 / 远程下载·监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP live runtime / graph 写·导出 / 后台 worker | board L1170 hard stop | M6 入口闸（产品负责人书面批准 + A10 verdict + A11 骨架） |
| L-3 | dev 启动助手**只改善本地 dev 体验**，不改 release 资产加载路径 | W17 AC-1 明确（board L1174） | 分发/签名属 M6-6 |
| L-4 | **窄窗口/HiDPI/多平台（Windows / macOS）未实跑验证** | 环境限制（本次仅 Linux 本机） | A8 诚实记录已测环境；跨平台分发属 M6-6 |
| L-5 | 体积余量 **0.06pp**，任何新 chunk 都可能触顶；W17 后若继续增长需新决策 | 门禁现实（25.2% 一次性上限） | A11 每波复测；抬上限需 A0 新决策 |

**补充：W15 遗留开放项**（不属 W17 但需并读）——手动 GUI/运行时证据在 W15 未补录，W17 由 **A8 START MANUAL QA**（board L1192）承接实跑；**永不伪造截图**。

---

## §4 通过判定口径

**W17 用户可见验收通过 = 同时满足**：

1. §1 三条前置确认全部勾选
2. §2 中 **A/B/C/D/E/F 六组**：每组 `FAIL` 数 = **0**；`N/A` 项必须写明原因且经 A0 认可
3. §3 五条已知限制已被**显式确认**（不是被忽略）
4. 体积门禁（E-1/E-2）由 A11 独立复测确认，且 **E-3 未被违反**
5. A8 实跑记录与 A11 矩阵结论**不冲突**；若冲突，以**实跑证据**为准并记录分歧

**判定为 `PASS_WITH_DEBT` 的情形**（须写明债务条目与去向）：
- 出现非阻断 `FAIL` 且 A0 认可挂账（例如某窄窗口阈值下的布局瑕疵，但功能仍可通过其他路径完成）
- 体积未超限但余量耗尽（需在下一波前给出瘦身或抬上限的 A0 决策）

**判定为 `BLOCKED` 的情形**：
- 任一"运行时权限面被扩张"（新增命令/ACL/bridge capability/网络特权/新依赖/执行能力）
- 体积超限且未报 delta 或直接抬上限
- 首页落入"营销页"或存在不可用的装饰性入口（AC-2 硬违约）
- 启动助手破坏用户自有 dev server（AC-1 硬违约）

---

## §5 责任与不越界

| 角色 | 在本清单中的职责 |
|---|---|
| **A1（本卡作者）** | 定义清单与判定口径；**不代跑、不代填、不伪造结果** |
| **A2/A3/A5/A6/A7** | 自测并回填本卡对应组（A/B/C）的实际命令与结果 |
| **A4/A9** | 出策略/DOM 级检查脚本并回填 D 组 |
| **A8** | **实跑原生客户端**并逐项勾选（永不伪造截图）；记录实测窗口宽度 |
| **A10** | 安全复核（D-4 + 硬停止 1/2/3） |
| **A11** | 独立复测 E 组 + 汇总整包矩阵 |
| **A0** | 集成、认可 `N/A`/挂账、判定 `PASS` / `PASS_WITH_DEBT` / `BLOCKED` |

---

## §6 关联指针

- W17 派发：`PARALLEL_COMMAND_BOARD.md` L1164-1215
- A1 W17 reconciliation checkpoint：`logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md`
- A1 W16（retained）M6 WBS 提案：`logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md`
- A1 W16 reconciliation（retained）：`logs/checkpoints/A1-M5-W16-reconciliation-20260908-1100.md`
- M5-0 `[W17 active]` 段：`logs/checkpoints/M5-20260906/M5-0-overview.md` 末尾
- M5-13 `[W17 verification scope]` 段：`logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` 末尾
- M5-14 `[W17 active]` 段：`logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` 末尾
