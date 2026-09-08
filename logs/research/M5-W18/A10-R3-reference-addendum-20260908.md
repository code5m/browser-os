# A10 — M5-W18-R3 复核补充（第 2 波）：次级参照许可 + 原型自包含门禁

- Lane：A10；分支 `codex/m5-w18-a10`；起点 `200f0f1`；本波接续 `2a0722b`
- 时间：2026-09-08
- 触发：`PARALLEL_COMMAND_BOARD.md` 当前派发仍为 `M5-W18-R3-UX`，且 `origin/master` **尚无 A2–A9 的 R3 产物**（已核查：`git ls-tree origin/master -- logs/research/M5-W18` 无 R3 文件）。A10 终审继续等待，本波做**不依赖他 lane**的独立核验。

---

## 1. 次级参照的可核验性（A10 许可职责）

R3 卡片列出两个次级比较参照。核验结果：

| 参照 | 卡片声明 | A10 核验 | 证据 | 结论 |
|---|---|---|---|---|
| SourceGit（`sourcegit-scm/sourcegit`） | MIT | **已核实 MIT** | `raw.githubusercontent.com/sourcegit-scm/sourcegit/master/LICENSE` HTTP 200，首行 `The MIT License (MIT) Copyright (c) 2026 sourcegit` | 许可可用；仍须固定 SHA 与文件级 provenance 才能 COPY/ADAPT |
| slio-git | MIT / Apache-2.0 | **不可核验 → UNVERIFIED** | 官网 `https://slio-git.skwang.uk/` 底部与下载区标注 `v0.0.26 · Open Source · MIT / Apache-2.0`，源码链接 `https://github.com/sk-wang/slio-git`；但 `api.github.com/repos/sk-wang/slio-git` 返回 **404**，且 `raw.githubusercontent.com/sk-wang/slio-git/{main,master}/{LICENSE,LICENSE-MIT,LICENSE-APACHE,README.md}` **全部 404** | **在拿到可访问仓库 + 固定 SHA + 许可文件之前，slio-git 不得作为 COPY/ADAPT 来源**；其分类只能记为 `REIMPLEMENT_FROM_BEHAVIOR(UNVERIFIED)` |

**F16（新增，Medium）**：卡片第 45 行要求"A10 记录文件级 provenance、适用许可/条款、NOTICE 义务"。slio-git 的许可目前只由**营销站点**声明，源码不可达 ⇒ 不满足该要求。交 A7/A0：要么提供可访问仓库（含 pinned SHA），要么在映射表中把 slio-git 全部单元标 `UNVERIFIED`，不得给出 `COPY`。

> 备注：核验期间 GitHub API 匿名配额已耗尽（`rate_limit.core.remaining = 0`），但 slio-git 的响应是**格式正确的 404 JSON**，非限流导致；SourceGit 的 LICENSE 走 raw（不受该配额影响）已成功。

---

## 2. 交叉验证：slio-git 的能力集合 vs A10 §5 可实现性矩阵

即使源码不可核验，其官网公开的**技术栈与能力清单**仍可作为"行为可达性"的**第二来源**，用来交叉检验主报告 §5 的判定：

| slio-git 公开能力/栈 | 与本产品对照 | 对 §5 判定的支撑 |
|---|---|---|
| **Rust + git2-rs (libgit2)** | 本产品已是 Rust + `git2 = "0.19"` | ✅ 与 §5 "git2 已就位、无需新依赖、无第二条执行路径"一致 |
| three-pane merge（冲突解决） | 本产品无 | ✅ 支撑 §5 冲突解决 = **C**（可达但需净新增 UI） |
| interactive rebase todo（pick/reword/squash/fixup/drop + continue/skip/abort） | 本产品无 | ✅ 支撑 §5 交互式 rebase = **C**（sequencer 状态机） |
| commit graph 可视化 + 搜索/键盘导航 | 本产品无 | ✅ 支撑 §5 log/graph = **B** |
| unified / side-by-side diff（字符级） | 本产品仅文本 diff | ✅ 支撑 §5 diff = **B**，并指出字符级差异需额外算法 |
| 丰富右键菜单（分组） | 本产品无命令注册表 | ✅ 支撑 F10（注册表为净新增基础设施） |
| Iced 0.14 + wgpu / Metal | 本产品 Vue + WebView | ❌ **UI 层不可移植**，只能按行为重实现 |

**F17（新增，Medium，交 A7/A0）**：slio-git 在**实现栈**上比 Rebased 更贴近本产品（同为 Rust + libgit2），适合作为"Git 工作流行为/交互粒度"的**可行性参照**；而 Rebased 作为**产品语义与信息架构**参照（卡片原意）仍然成立。建议 A7 在映射表中**区分两类参照角色**，不要混用"COPY 潜力"与"行为参照"。

**F18（新增，Medium，依赖红线）**：slio-git 栈中的 `iced`、`wgpu`、`similar`、`syntect`、`notify`、`tokio` 在本产品当前状态下**全部 BLOCKED**：

- R3 卡第 26 行：研究原型"不改动依赖、ACL、capability、原生运行时或用户数据"；
- M4 A2 裁决 F-1：**不引 tokio 直接依赖**；
- `WORKBENCH_BLUEPRINT-20260908.md` §7：体积阈值 25.2%，余量约 346B（主报告 F05）。

⇒ 任何"借鉴 slio-git 实现"的表述都必须落到**行为重实现**，不得变成新依赖提案；若要引入，须独立立项并给出预算来源。

---

## 3. G8 原型自包含门禁：脚本与基线

新增研究资产：`logs/research/M5-W18/A10-R3-prototype-selfcontainment-check.py`

检查项（对齐主报告 §8 的 G8 与 R3 卡第 26 行）：

| 码 | 含义 |
|---|---|
| `PROTO_NO_NETWORK` | 无外部 `src/href/url()` 网络引用（w3 命名空间除外） |
| `PROTO_NO_NATIVE_INVOKE` | 无 `@tauri-apps` / `__TAURI__` / `invoke(` / `bridge.invoke` |
| `PROTO_NO_PRODUCT_IMPORT` | 不 import 产品源码（`../../src/`、`@/`、`src/components|stores|utils`） |
| `PROTO_NO_NPM_IMPORT` | 无 vue/react/d3/tauri 等裸包 import（原型须免构建直接打开） |
| `PROTO_NO_PERSIST` | 不用 `localStorage/sessionStorage/indexedDB/cookie` |
| `PROTO_NO_BRANDING` | 无 `Rebased/IntelliJ/JetBrains` 品牌串（仅允许作为来源引注） |

```bash
python3 logs/research/M5-W18/A10-R3-prototype-selfcontainment-check.py
# [PASS] logs/research/M5-W18/A1-wireframe-prototype-R2B.html
# [PASS] logs/research/M5-W18/A5-R2B-wireframe.html
# RESULT: PASS (2/2 files clean)
```

**基线结论**：既有研究原型**已满足** G8，说明该门禁对 R3 是**可达且不苛刻**的；R3 新原型（A1 的 `A1-R3-prototype.html` 等）交付时应以 exit 0 为门槛（A11 可直接复用本脚本做验收）。

---

## 4. 对 A10 终审的影响

- 主报告 §8 闸门 **G1/G3/G8** 现在都有可执行依据：G8 = 本节脚本；G1/G3 = 主报告 §2.5 分类门槛 + 品牌串检查（脚本已含 `PROTO_NO_BRANDING`）。
- 新增发现 **F16（slio-git 不可核验）**、**F17（参照角色需分层）**、**F18（依赖红线）** 并入发现登记；F16/F17 交 A7，F18 交 A0。
- 终审仍未开始：等待 A2–A9 的 R3 产物落到 `master`。

## 5. 本波范围声明

- 仅新增 A10 lane 自有研究文件（1 个 md + 1 个 py）；**零产品代码、零依赖/ACL/capability 改动；未 push**。
- 未触碰他 lane 文件；未修改已提交的 `2a0722b` 内容（历史保留，符合蓝图 §8 第 8 条）。
