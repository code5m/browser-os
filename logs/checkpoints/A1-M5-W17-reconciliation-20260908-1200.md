# A1 M5-W17 reconciliation checkpoint（DOCS ONLY · 2026-09-08 12:00 CST）

> Lane A1（M5-W17 · reconcile retained W16 outputs + W17 user-visible acceptance checklist/known limitations）
> 派发来源：`PARALLEL_COMMAND_BOARD.md` L1164-1215（M5-W17 Desktop Client Completeness and Home Recovery Dispatch）
> 配套交付：**W17 用户可见验收清单 / 已知限制** `logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`
> 整包交付：本 checkpoint + `logs/checkpoints/Lane-A1-M5-W17-20260908-1200.patch`（路径限定 A1 范围）
> A1 W17 立场 = **DOCS ONLY · 零产品代码 · 不 commit · 不 push**

---

## §1 任务接收

**W17 派发** = Desktop Client Completeness and Home Recovery Dispatch（board L1164-1215）：用户反馈已安装桌面客户端"快但感觉不完整"——首页缺少重要入口，需要在继续运行时平台工作前先让人能专注地人工走查一遍。

**A1 W17 任务边界（board L1185，冻结）**：
1. Reconcile retained W16 outputs
2. Write the W17 user-visible acceptance checklist / known limitations
3. Allowed scope：`AI-模型切换与接手清单.md` / `详细设计与实施计划.md` / `后续需求TODO.md` / `logs/checkpoints/`
4. **No product code**

**启动门禁实测**：

| 项 | 实测 | 判定 |
|---|---|---|
| `pwd` | `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3` | ✅ |
| branch | `master` | ✅ |
| `git fetch` + `git pull --ff-only` | 与 `origin/master` 一致，`052b18a`（`已经是最新的`） | ✅ |
| HEAD | `052b18a feat(M5): close W15 release readiness` | ✅ |
| W16 产出保留 | `A1-M5-W16-reconciliation-20260908-1100.md` + `A1-M5-W16-M6-WBS-proposal-20260908-1100.md` + `Lane-A1-M5-W16-20260908-1100.patch` 均在工作树（未 push，board L6 明确 retained） | ✅ |
| board `NEXT` | `M5-W17` desktop-client completeness and home recovery（board L8） | ✅ |

**工作树观察（A1 不动其他 lane）**：W15/W16 lane 产物共存（A2/A3/A4/A5/A6/A7/A8/A11 assist + checkpoint + patch）；**尚未见 W17 lane 产物**，A1 属 W17 早期 lane。A1 严格只在自身范围内改动，未触碰任何 `src/`、`src-tauri/`、`scripts/`、`package.json`、ACL/Capability。

---

## §2 整包改动清单（路径限定 A1 范围）

| # | 路径 | 类型 | 内容 |
|---|---|---|---|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | M | 标题链追加 `+ W17 active`；末尾新增 `[W17 active · 2026-09-08 12:00 CST]` 段（W16 retained 对账 + W17 有界性质表 + 6 条共享 AC + 11 lane 角色表 + 集成顺序 + A1 交付索引） |
| 2 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | M | 末尾新增 `[W17 verification scope · 2026-09-08 12:00 CST]` 段（W17 vs W15/W16 门禁差异 + **6 FAC 验证矩阵**（对应 6 条共享 AC）+ 7 条硬停止验证必跑） |
| 3 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | M | 末尾新增 `[W17 active · 2026-09-08 12:00 CST]` 段（W16 retained + 债务账立场 5 项 + **已知限制 5 条** + A1 交付索引） |
| 4 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | M | L25 新增 W17 ACTIVE 状态行（契约冻结 / 不碰本卡 / 运行时全锁） |
| 5 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | M | L25 新增 W17 ACTIVE 状态行（pure 维持 / ACL 末条不破） |
| 6 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | M | L25 新增 W17 ACTIVE 状态行（DEBT-04 closed-by-W14 维持 / A5 只改 `src/components/home/`） |
| 7 | `AI-模型切换与接手清单.md` | M | L1271 新增 A1 W17 update 行（紧邻 A1 W16 行之后） |
| 8 | `详细设计与实施计划.md` | M | L30 新增 A1 W17 update 行 |
| 9 | `后续需求TODO.md` | M | L19 新增 A1 W17 update 行 |
| 10 | `logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md` | **new** | **A1 W17 核心新交付**：6 组 24 项用户可见人工勾选项 + 5 条已知限制 + 通过判定口径（PASS / PASS_WITH_DEBT / BLOCKED）+ 责任矩阵 |
| 11 | `logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md` | **new** | 本 checkpoint |

---

## §3 W16 outputs retained 对账（A1 W17 第一项任务）

**board L6 原文**：*"Retain every M5-W16 documentation/review deliverable; M5-W17 desktop-client completeness and home recovery is now active."*

**对账结论**：

| W16 产出 | 保留状态 | A1 W17 处理 |
|---|---|---|
| `A1-M5-W16-M6-WBS-proposal-20260908-1100.md`（M6-1~M6-6 六候选工作流 + 波次排序 + 入口闸 11 项） | ✅ retained | 在 M5-0 `[W17 active]` 段与 M5-14 `[W17 active]` 段显式引用；**W17 不启动任何 M6 权限切片** |
| `A1-M5-W16-reconciliation-20260908-1100.md`（W15 accepted 对账 + 25.2% 门禁更正） | ✅ retained | 25.2% 上限在 M5-13 FAC-5.W17 与 M5-14 已知限制 L-5 中延续生效 |
| M5-0/13/14 的 25.0 → **25.2** 门禁更正 | ✅ retained 且生效 | W17 体积门禁基线以 25.2% 为准（`total_bytes_pct=25.14`，余量 0.06pp） |
| M5 final debt ledger 53 条 | ✅ 维持 | W17 预期 **0 新增**（有界前端波，不动 backend 契约与运行时权限面） |
| W15 遗留：手动 GUI/运行时证据未补录 | 未消，转交 | **W17 由 A8 START MANUAL QA 承接实跑**（board L1192）；永不伪造截图 |

---

## §4 W17 用户可见验收清单（A1 W17 第二项任务 · 核心新交付）

文件：`logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`

**设计原则**：
- 把 board 六条共享 AC **逐条展开为可人工勾选的用户可见检查项**（不写"符合 AC-1"这种无法执行的空话）
- 每项三态 `PASS` / `FAIL` / `N/A`；`FAIL` 必须附**实际看到的现象**，`N/A` 必须写原因
- 明确**不代跑、不代填、不伪造**（A1 只定义口径；执行由 A8 实跑 + A11 矩阵 + 各 lane 自测）

**6 组 24 项**：

| 组 | 对应 AC | 项数 | 责任 | 最关键项 |
|---|---|---|---|---|
| A 启动 | AC-1 | 6 | A2 码 / A8+A11 验 | **A-4 只清理自己启动的 server**（ownership-safe，最易错）· A-5 release 仍用 bundled assets |
| B 首页 | AC-2 | 6 | A3 store + A5 组件 / A9 测 / A8 跑 | **B-2 不是营销页** · B-4 抗畸形本地数据 · B-3 列表有界 |
| C 导航·窄窗口 | AC-3 | 5 | A6 + A7 / A8 跑 | C-3 窄窗口可用（记录实测宽度）· C-4 面板解析失败不留空白死区 |
| D 可达性·无敏感泄露 | AC-4 | 5 | A4 静态 + A9 DOM + A10 安全 | D-2 accessible name · D-4 无凭据/URL query/本地路径泄露 |
| E 体积门禁 | AC-5 | 3 | A11 主 | **E-1 ≤ 25.2%**（余量 0.06pp）· E-3 超限报 delta **不抬上限** |
| F 交付完整性 | AC-6 | 3 | 全 lane + A0 | F-2 冲突出 binary patch 不覆盖 · F-3 无 lane 自行 push |

**通过判定**：六组 `FAIL`=0 且 `N/A` 经 A0 认可 + 5 条已知限制显式确认 + A11 复测体积 + A8 与 A11 结论不冲突（冲突以实跑证据为准）。
**PASS_WITH_DEBT**：非阻断 FAIL 经 A0 挂账，或体积未超限但余量耗尽待决策。
**BLOCKED**：运行时权限被扩张 / 体积超限且抬上限 / 首页沦为营销页或存在装饰性不可用入口 / 启动助手破坏用户自有 dev server。

---

## §5 W17 有界性核对（A1 必须显式记账）

| 维度 | W17 状态 | A1 记录 |
|---|---|---|
| 允许 | 有界前端/启动体验产品代码（A2 启动助手 / A3 首页 store+utils / A5 首页组件 / A6 导航 / A7 shell 兜底） | ✅ 已在 M5-0 `[W17 active]` 段 + M5-13 FAC 表标注 |
| 禁止 | 命令执行 / plugin 调用 / 动态加载 / 远程下载·监听 / daemon / 模型调用 / Agent-Skill 执行 / MCP live runtime / graph 写·导出 / 后台 worker | ✅ 已在 M5-13 硬停止必跑 2 + M5-14 已知限制 L-2 标注 |
| 冻结 | 新 Tauri 命令 / bridge capability / ACL 条目 / 文件系统权限 / 网络特权 / 新依赖 = **零** | ✅ 已在 M5-13 硬停止必跑 1 标注 |
| 不启动 | 任何 M6 权限切片 | ✅ 已在 M5-0 + M5-13 硬停止必跑 4 标注 |
| 门禁 | 25.2% 上限 + warning delta=0；超限报 delta 不抬上限 | ✅ 已在 M5-13 FAC-5.W17 + M5-14 L-5 + 验收清单 E 组标注 |

---

## §6 A1 W17 立场

- **DOCS ONLY**：零产品代码。未触 `src/`、`src-tauri/`、`scripts/`、`package.json`、`permissions/default-commands.toml`、`capabilities/default.json`、任何 ACL/Capability。
- **不代跑、不代填、不伪造**：验收清单只定义口径与判定规则；实跑结果由 A8 / A11 / 各 lane 回填。
- **不越界解冲突**：W17 lane 允许文件若已被他 lane 改动，A1 只出 binary patch（本次 A1 范围为纯文档，未出现冲突）。
- **不移动 `NEXT`**：board L8 `NEXT=M5-W17` 由 A0 管理。
- **不 commit、不 push**：整包留工作树交 A0 集成。

---

## §7 整包交付与自检

- ✅ `logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md`（本 checkpoint）
- ✅ `logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`（W17 用户可见验收清单 / 已知限制）
- ✅ `logs/checkpoints/Lane-A1-M5-W17-20260908-1200.patch`（路径限定 A1 范围）
- ✅ 自检：`git diff --check` 干净；`git apply --check --reverse` 通过；`grep -c "^diff --git"` 计数核对

**patch 生成（路径限定）**：

```bash
git add -N logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md \
           logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md
git diff --binary -- 详细设计与实施计划.md AI-模型切换与接手清单.md 后续需求TODO.md \
  logs/checkpoints/M5-20260906 \
  logs/checkpoints/A1-M5-W17-reconciliation-20260908-1200.md \
  logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md \
  > logs/checkpoints/Lane-A1-M5-W17-20260908-1200.patch
```

**M5-0/13/14 与子卡修订载体**：M5-0 标题链 + `[W17 active]` 段 · M5-13 `[W17 verification scope]` 段 · M5-14 `[W17 active]` 段 · M5-10/11/12 L25 状态行 · 3 主文档 update 行。

---

## §8 关联指针

- W17 派发：`PARALLEL_COMMAND_BOARD.md` L1164-1215
- W17 用户可见验收清单：`logs/checkpoints/A1-M5-W17-user-visible-acceptance-checklist-20260908-1200.md`
- A1 W16（retained）：`logs/checkpoints/A1-M5-W16-reconciliation-20260908-1100.md`
- A1 W16 M6 WBS 提案（retained）：`logs/checkpoints/A1-M5-W16-M6-WBS-proposal-20260908-1100.md`
- M5-0 `[W17 active]` 段 / M5-13 `[W17 verification scope]` 段 / M5-14 `[W17 active]` 段（三卡末尾）
