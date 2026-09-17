# 19 · SECOND-REVIEW-EVIDENCE — 第二轮新证据总账

> 汇总第二轮 A–F 与 Final Reviewer 新建立的全部源码证据。
> 目的：为第二轮每一条裁决提供**可回溯的源码锚点**，并暴露各 Agent 证据编号的冲突。

---

## 0. ⚠ 证据编号冲突与命名空间约定（必读）

第二轮各复核者**独立编号**，导致 `SR-EVID-0001..003x` 在 **B / D / F 三方之间严重撞号**（例如 `SR-EVID-0001` 同时被 B、D、F 用于完全不同的证据）。

**本文件与 18 采用强制命名空间约定**：

| 复核者 | 命名空间 | 原始编号范围 | 所在文档 |
|---|---|---|---|
| Reviewer A（代笔：Chief） | `SR-EVID-A-xxx`（本轮未单独编号，引用 B/C/D/E/F 证据） | — | `17A-S4-S3-REVIEW.md` |
| Reviewer B（生命周期） | **`SR-EVID-B001..B038`** | 原文 `SR-EVID-0001..0038` | `17B-LIFECYCLE-REVIEW.md` |
| Reviewer C（状态真源） | **`SR-EVID-C201..C217`** | 原文 `SR-EVID-0201..0217` | `17C-STATE-TRUTH-REVIEW.md` |
| Reviewer D（IPC） | **`SR-EVID-D001..D036`** | 原文 `SR-EVID-0001..0036` | `17D-IPC-REVIEW.md` |
| Reviewer E（检查器） | **`SR-EVID-E001..E037`** | 原文 `SR-EVID-E001..E037` | `17E-CHECKER-REVIEW.md` |
| Reviewer F（ADR/契约/迁移） | **`SR-EVID-F001..F033`** | 原文 `SR-EVID-0001..0033` | `17F-ADR-CONTRACT-REVIEW.md` |
| Final Reviewer（终审） | **`SR-EVID-FR-0001..0020`** | 原文即此 | `18-SECOND-REVIEW-SYNTHESIS.md` |

**规则**：跨文档引用一律使用**命名空间形式**；单文档内部可沿用原文局部编号。后续任何自动化工具若解析本目录，必须按命名空间消歧，否则会误配证据。

---

## 1. 证据量总览

| 复核者 | 新证据条数 | 主要覆盖域 | 关键结论贡献 |
|---|---|---|---|
| B | 38 | 生命周期（Tab/Grid/PTY/Workspace） | 推翻 05 的 `S4=0`；`create_grid` 无条件 destroy 升 FACT；`UpdateRect:423` 升 FACT |
| C | 17 | 状态真源 / 正交性 | `gridOpen+mainView` 组合合法；纠正 4 项"重复真源"误判 |
| D | 36 | IPC 契约 | **发现 `move_path` BROKEN**；推翻 `BROKEN=0` 与"注册↔ACL 牢固"；CONSISTENT 39→51 |
| E | 37 | 检查器可执行性 | RULE-012 拟议正则 0 命中；RULE-011-B 100% 误报；ready 仅 2 条 |
| F | 33 | ADR / 目标契约 / 迁移计划 | 发现两门禁脚本未接线；`check-lifecycle-contract.py` 零覆盖；≥8 个引用脚本不存在 |
| FR | 20 | 终审再采样 | 决定性坐实 `move_path`；收窄宫格分叉范围；发现 `toggleGridToolbar` 死导出 |
| **合计** | **181** | | |

**分类分布（约）**：FACT ≈ 150；INFERENCE ≈ 31。
**置信分布（约）**：HIGH ≈ 160；MEDIUM/MEDIUM-HIGH ≈ 18；LOW ≈ 3。

---

## 2. 按主题索引（证据 → 裁决 → 锚点）

### 2.1 宫格退出意图分叉（FINAL_S4-1）
| 证据 | 命名空间 | 锚点 | 结论 |
|---|---|---|---|
| 分叉成立 | `SR-EVID-B026` / `SR-EVID-F002` / **`SR-EVID-FR-0001`** | `HomeLaunchers.vue:42-54`(L47) vs `ActivityBar.vue:150-177`(L171-173) | 同一意图 HIDE vs DESTROY |
| 视图切换链零 destroy | `SR-EVID-B024` / `SR-EVID-F001` / **`SR-EVID-FR-0003`** | `useLayoutStore.ts:193-199`；`useBrowserStore.ts:646-655`, `:657-664` | Tab/PTY 侧红线成立 |
| closeGridAll 副作用与顺序 | `SR-EVID-B023` / **`SR-EVID-FR-0002`** | `useBrowserStore.ts:483-511` | `tabActivate` 早于 `mainView` 复位；体内无 `schedulePosition(` |
| 范围收窄（死导出） | **`SR-EVID-FR-0012`** | `useLayoutStore.ts:210`(定义)/`:358`(返回)；全仓 `toggleGridToolbar` 仅 2 命中 | `toggleGridToolbar` 零调用者 ⇒ 不是活出口 |
| `ActivityBar:398` 非分叉 | **`SR-EVID-FR-0001`** | `ActivityBar.vue:398` | 是显式"关闭宫格"按钮 |
| 注释漂移 | `SR-EVID-B038` | `ActivityBar.vue:156-157` | 注释声称"离开宫格自动关闭"，代码无此实现 |

### 2.2 `gridOpen` / `mainView` 组合合法性（推翻 round-1）
| 证据 | 命名空间 | 锚点 |
|---|---|---|
| 组合被设计保留 | `SR-EVID-C201` / **`SR-EVID-FR-0004`** | `useBrowserStore.ts:319-324`（带注释守卫） |
| 语义化为"冻结" | `SR-EVID-C203` / **`SR-EVID-FR-0005`** | `syncFreeze:618-631`（L628）；L620 死变量 |
| 三处守卫正当处理 | `SR-EVID-C204` / **`SR-EVID-FR-0006`** | `useBrowserHost.ts:64/99/109` |
| 派生未单点化 | `SR-EVID-C208` / `SR-EVID-F010` | `useBrowserHost:64/99/109`、`ActivityBar:358/369`、`useGridArchiveStore:48` |
| 双 writer 跨 store | `SR-EVID-C209` | `useLayoutStore:211` + `useBrowserStore:488` |

### 2.3 IPC：`move_path` BROKEN（★ 决定性）
| 证据 | 命名空间 | 锚点 |
|---|---|---|
| FE 封装 | `SR-EVID-D001`族 / **`SR-EVID-FR-0007`** | `src/bridge.ts:507-509` |
| FE 调用 + UI 可达 | **`SR-EVID-FR-0007`** | `useWorkspaceStore.ts:679-690`(L684)；`FilePanel.vue:242-258`(L256) |
| Rust 实现 | **`SR-EVID-FR-0007`** | `fs_cmds.rs:20-21` |
| 注册 | `SR-EVID-D001`族 / **`SR-EVID-FR-0007`** | `main.rs:1432` |
| **ACL 缺失** | `SR-EVID-D015/0016/0017` / **`SR-EVID-FR-0007/0008`** | `default-commands.toml` L34↔L35 缺 `"move_path"`；全部 capabilities 0 命中 |
| 计数核对 148−147=1 | **`SR-EVID-FR-0008`** | `main.rs:1403-1552`；`default-commands.toml:4-146` |
| `create_grid` 契约闭合 | `SR-EVID-D001..D006` | `bridge.ts:534-535` ↔ `bridge.rs:3851` ↔ `main.rs:1440` ↔ ACL L42 |
| 无守卫 typed 占位 7 条 | `SR-EVID-D021/0022` / **`SR-EVID-FR-0019`** | `bridge.ts:401/403/417-418/420-421/431-432/462/463` |
| 6 条已落地只读命令 | `SR-EVID-D033` | `bridge.rs:6529-6623`；`main.rs:1518-1523`；ACL L121-126 |
| 正则盲区量化 | `SR-EVID-D019` / `SR-EVID-E014` | 161 invoke 点中仅 49 可见 |
| `sync_browser_scene` 重分类 | `SR-EVID-D025/0026/0027` | 代码 0 命中；`docs/AI/00-Architecture.md:261` APPROVED_TARGET |

### 2.4 检查器执行力
| 证据 | 命名空间 | 锚点 | 结论 |
|---|---|---|---|
| 两脚本未接入门禁 | `SR-EVID-F020` / **`SR-EVID-FR-0010`** | `pre-merge.sh` 检索 0 命中 | ready 数今日实际为 0 |
| G2 + G1#4 两处 FAIL | `SR-EVID-E003/E004` / **`SR-EVID-FR-0011`** | `check-grid-close-logic.mjs:112`、`:88` | round-1 只报 G2 |
| RULE-012 正则 0 命中 | `SR-EVID-E030` / **`SR-EVID-FR-0017`** | `useLayoutStore.ts:146/338` | 当日假 PASS |
| RULE-011-B 100% 误报 | `SR-EVID-E027` | `BookmarkPanel.vue:134` | 唯一命中是 `ref="passwordInput"` |
| 红线已由类型层保证 | `SR-EVID-E028` / **`SR-EVID-FR-0018`** | `types.ts:174-186` | RULE-011-B 否决 |
| RULE-004 实测 7 处 | `SR-EVID-E010` / **`SR-EVID-FR-0015`** | `App.vue:199` 等 7 处 | 低估 7 倍 |
| 白名单不可达 | `SR-EVID-E008` / **`SR-EVID-FR-0013`** | `useLayoutStore.ts:193-199/253-265/300-314` | RULE-003 白名单形同装饰 |
| 引用脚本不存在 | `SR-EVID-F033` / **`SR-EVID-FR-0020`** | `scripts/` 共 63 个 `check-*` | ≥8 个被引用脚本不存在 |
| 运行态检测可行 | `SR-EVID-E036` | `check-client-navigation-logic.mjs:19-60` | 推翻"只能源码解析"前提 |

### 2.5 生命周期机制事实
| 证据 | 命名空间 | 锚点 |
|---|---|---|
| POSITION⇒SHOW（Tab） | `SR-EVID-B002` / **`SR-EVID-FR-0016`** | `bridge.rs:531-564`(L555) |
| POSITION⇒SHOW（Grid） | `SR-EVID-B015` / **`SR-EVID-FR-0016`** | `main.rs:391-425`(L423) |
| hide 一律 −30000，禁 set_visible(false) | `SR-EVID-B003` / `SR-EVID-F012` | `bridge.rs:566-597`（注释 566-569） |
| `create_grid` 无条件 destroy | `SR-EVID-B011` / **`SR-EVID-FR-0016`** | `bridge.rs:3854`（无守卫） |
| HIBERNATED 前端零镜像 | `SR-EVID-B006` | `bridge.rs:4559-4574` / `4663-4683` |
| ACTIVATE ⊇ CREATE | `SR-EVID-B007` | `bridge.rs:4665-4683` |
| PTY 存活于视图切换 | `SR-EVID-B031` | `TerminalPane.vue:223-233` |
| kill 正确 wait | `SR-EVID-B033` | `bridge.rs:4817-4829`；`terminal.rs:661-707` |
| Workspace 无原生生命周期 | `SR-EVID-B037` | 双向 0 命中 |

### 2.6 ADR / 契约 / 迁移
| 证据 | 命名空间 | 锚点 | 结论 |
|---|---|---|---|
| ADR-GRID-001 反例 | `SR-EVID-F002` / **`SR-EVID-FR-0001`** | `HomeLaunchers.vue:47` | 现状断言不成立 |
| checker 覆盖声明错误 | `SR-EVID-F030` / **`SR-EVID-FR-0009`** | `check-lifecycle-contract.py` 0 命中 | 须删除 L18 |
| SessionCloseDialog 实为两方 | `SR-EVID-F023` / `SR-EVID-E033` / **`SR-EVID-FR-0014`** | `check-session-persistence-policy.py:21-23, 230-231` | 该脚本站"撤销"一侧 |
| ADR-BROWSER-001 完全确认 | `SR-EVID-F007/0008/0009` | `useLayoutStore:188-190`；`useBrowserStore:149-151`；`UnifiedTabBar:147-158` | 维持 ACCEPTED |
| ADR-NATIVE-SHOW-001 完全确认 | `SR-EVID-F011..F014` | `bridge.rs:555/566-597`；`main.rs:391-431`；show 变体 0 命中 | 维持 ACCEPTED |
| PROPOSED ADR 已事实生效 | `SR-EVID-F026` | `semantic-governance/README.md:56` | 须与 ADR-SEVERITY-001 二选一 |

---

## 3. 与 round-1 证据的关系

- **round-1 证据保留有效**：01–16 中的目录学证据（定义点、读写者、行号）经 C 复核"高度准确"，绝大多数 FACT 成立。
- **round-1 证据被推翻的 8 处**（见 18 §8）：`BROKEN=0`、`EVID-022` 注册↔ACL 牢固、隐含 GATE PASS、`CLAIM-XC-01` 三方矛盾、`CLAIM-LC-01` 红线全部成立、ADR-GRID-001 L18 已覆盖、`CLAIM-MT-03` 多真源/S4、13/12 引用不存在脚本。
- **round-1 判级被修正**：`EVID-GR-02`/`EVID-GR-04` 由 INFERENCE → FACT（B `SR-EVID-B011/B015`）；`DUP-001`/`DUP-002` 由 S4 → S2（C）。

---

## 4. 证据记录规范（后续统一）

1. 每条必须同时记录 **file + symbol**（行号仅辅助），防止代码变化后失效。
2. 显式标注 **FACT / INFERENCE / RECOMMENDATION / UNVERIFIED**。
3. 跨文档引用必须使用**命名空间**（`SR-EVID-{B|C|D|E|F|FR}-{num}`）。
4. 每条须给出 **Confidence**（HIGH/MEDIUM/LOW）与 **Why it supports/challenges original claim**。
5. 禁止把设计建议写成现状；目标设计须显式标注 TARGET DESIGN / PROPOSED。

---

## 5. 未独立复核清单（诚实标注）

以下结论**采信复核者、终审未独立重采样**，不应作为强决策依据：
- 17B `SR-EVID-B021`（`show_for_focus` 缺 `hidden` 守卫）—— INFERENCE/MEDIUM。
- 17B `SR-EVID-B035`（PTY 泄漏窗口）—— INFERENCE/MEDIUM。
- 17D 的 `CONSISTENT` 桶内 51 条中的约 38 条（终审只逐条复核约 13 条 + 结构基准）。
- 17D 的 `DEAD_SURFACE`（`term_spawn`、`issue_intent`）。
- 17E/17F 的凭据 3 处直连点（`CredentialList.vue:44/:79-82`、`BookmarkPanel.vue:109`）。

---

**签收**：第二轮证据总账 · READ-ONLY · 全部 181 条证据均来自源码静态观测，未执行任何运行时代码（除既有检查器的静态推演），未修改任何产品代码或检查器。
