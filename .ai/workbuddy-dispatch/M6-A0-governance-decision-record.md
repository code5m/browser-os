# M6 — A0 Governance Decision Record

> Version: 2026-09-12
> 来源：Owner / Chief Architect 正式裁决
> 性质：治理收口记录 + A0 待执行事项
> 配套：`M6-browser-core-specification.md`、`M6-browser-core-dispatch-plan.md`

---

## 1. 命名裁决

**裁定：整条 Browser Core 命名为 `M6`，废弃 `PLANS.md` 旧 Phase 2 / Phase 3 / Phase 5 编号。**

理由：已确认两套 Phase 编号冲突（`PLANS.md` 的 Phase 0~7 与后续 Phase 03/04/05），且 `PLANS.md` 为空壳模板。

| 编号 | 状态 |
|---|---|
| `PLANS.md` 旧 Phase 0~7 | 作废。保留文件，标注 DEPRECATED，不再作为派发依据 |
| **M6** = Browser Core Abstraction | 当前阶段 |
| **M7** = Workspace Retrieval Foundation | NEXT_CAPABILITY，排队，**不取消** |

路线：稳定浏览器 → M6 Browser Core → M7 Workspace Retrieval → Agent/RAG/Skill。

**A0 待执行**：在 `PLANS.md` 顶部标注该文件已 DEPRECATED，指向 `M6-browser-core-specification.md`。

---

## 2. R3B 裁决：DEFERRED

**裁定：R3B Browser-First Workbench UI = `DEFERRED`。**

含义（Owner 原文）：

- 不废弃
- 不作为当前 UI 改版目标
- 不阻塞 Browser Core
- 不要求为了 Browser Core 接受 R3B UI
- 未来若重新启动 Workbench UI，应建立在 Browser Core 能力之上

**依赖关系记录**：R3B 依赖 M6 的 `WebViewSafeShell` / `BrowserViewportAnchor`（R3B-09 原生 WebView 焦点/遮挡/resize 悬置的根因）；M6 不依赖 R3B。因此先做 M6 对 R3B 是纯收益。

---

## 3. Product Code LOCK 治理（关键）

### 3.1 现状

`WORKSPACE_IDENTITY.md` 当前写明：

> Product code remains locked until the corrected prototype is accepted by the user.

上下文：该段落位于 `NEXT: M5-W18-R3B; W19: CLOSED` 与 `A0 verdict: REVISE_TARGETED` 之下，指向 WORKBENCH_BLUEPRINT 产品方向。

### 3.2 裁定

**禁止通过「假装 ACCEPT R3B」解除该 LOCK。**

该 LOCK 的**原文作用域为 W18 / W19 / R3B 产品改版路线**。M6 属 architecture migration（Owner 第 6 条：不是 product redesign，不改变 Tab 生命周期/关闭策略/布局/UI），**不应被该 LOCK 错误阻塞**。

### 3.3 A0 待执行（本波不越权）

本波**未修改** `WORKSPACE_IDENTITY.md`。建议 A0 执行以下两项之一：

**选项 A（推荐）— 收窄作用域**

将原文：

```
Product code remains locked until the corrected prototype is accepted by the user.
```

改为：

```
Product code remains locked for the W18/W19/R3B workbench product-redesign
route until the corrected prototype is accepted by the user.
Scope note (2026-09-12 Owner ruling): this lock does NOT block architecture
migration work that preserves product semantics. See
.ai/workbuddy-dispatch/M6-A0-governance-decision-record.md.
M6 (Browser Core Abstraction) is authorized under this carve-out.
M7 (Workspace Retrieval) remains queued, not authorized yet.
```

**选项 B — 保留原 LOCK，显式列出豁免**

保留原文，追加一段 `LOCK EXEMPTIONS`，列出 M6 及其不变量 I-1~I-12。

### 3.4 阻塞判定

| 项 | 是否仍被 LOCK 阻塞 |
|---|---|
| M6 Browser Core（架构迁移，保语义） | **否**（有裁决依据，待 A0 落文案） |
| M7 Workspace Retrieval（新能力） | **待定** — 需 A0 确认是否属豁免范围 |
| R3B UI 改版 | 是（DEFERRED，保持锁定） |

---

## 4. PENDING 清理裁决

`docs/AI/00-Architecture.md` §2.3 原 4 条 PENDING 全部裁决：

| 原 PENDING | 裁决 | 依据 |
|---|---|---|
| 关闭 tab 是否自动保存并直接关闭 | **CLOSED / REJECTED** — 普通关闭 = no prompt + no persistent save + direct close | Phase 04 Owner 最终裁决；`PROJECT-RULES.md` [DEPRECATED] `auto_save_on_close` |
| 是否增加 recentlyClosed + Ctrl+Shift+T | **APPROVED / 已实现** — 内存栈（仅 url,title）+ Ctrl+Shift+T | Phase 04（commit `acf4add`） |
| Native WebView 模型 | **APPROVED_CURRENT：1 tab = 1 native WebView**；slot pool = `FUTURE / SEPARATE_DECISION`，本阶段禁止 | Owner 第 3 条 |
| Vue Shell 迁移方式 | **APPROVED：incremental migration**；full rewrite = `REJECTED` | Owner 第 4 条 |

**已执行**：`docs/AI/00-Architecture.md` §2.3 已更新（本波）。

**A0 follow-up**：`PROJECT-RULES.md` L152-155 有同样两条 PENDING，需 Project Rules Agent 同步（本波未擅自修改该锁定文件）。

---

## 5. WebView 模型裁决（含补充约束）

**裁定：APPROVED_CURRENT = 1 tab = 1 native WebView。禁止本阶段改为 1-4 Slot Pool。Slot Pool 进入 FUTURE / SEPARATE ARCHITECTURE DECISION。**

**Owner 补充约束（重要）**：

> 「每 Tab 一个 WebView」会影响 BrowserScene 设计，但**不意味着未来永远不能槽池化**。
> 正确设计应让 `BrowserScene` 描述「期望的浏览器场景」，而不是把底层「1 Tab 1 WebView」的实现细节永久焊死在上层 contract 里。
> 这样以后真证明槽池值得做，可以换 Runtime 实现，而不是再推翻 Vue Shell。

已写入 specification §4.3 硬约束。

---

## 6. Vue Shell 迁移裁决

**裁定：渐进迁移（incremental migration）。**

明确禁止：

- 整体重写 Vue Shell
- 推倒现有 `bridge.ts`
- 一次性替换全部 browser lifecycle
- 借架构重构改变现有稳定产品行为

迁移锚点：`src/bridge.ts`、`src/composables/useBrowserHost.ts`、当前 tab/webview lifecycle、Phase 04 close semantics、已通过的 deterministic gates。

方式：adapter / strangler 逐步收口。

> 补充澄清：「Vue Shell 任务」的准确含义是**现有 Shell 的职责收敛与渐进迁移**，不是再造一个新 Vue 应用。现有 Vue UI 本质上已经是 Shell。

---

## 7. checker 语义修正

见 `M6-browser-core-dispatch-plan.md` §6。

要点：引入 `implementationState`（NOT_IMPLEMENTED / PARTIAL / IMPLEMENTED_PASS / IMPLEMENTED_FAIL）与 `status` 解耦。**本波不修改 checker**，由 Runtime Checker Agent 在 A0 批准后、M6-G 前实施。

---

## 8. 遗留与未决

| # | 项 | 归属 |
|---|---|---|
| 1 | LOCK 作用域收窄文案落位 | A0 |
| 2 | `PLANS.md` 标注 DEPRECATED | A0 |
| 3 | `PROJECT-RULES.md` L152-155 PENDING 同步 | Project Rules Agent |
| 4 | checker 语义修正实施 | Runtime Checker Agent（A0 批准后） |
| 5 | M7 是否属 LOCK 豁免范围 | A0 确认 |
| 6 | 体积预算（25.2%，余量极小）对 M6 各批的约束 | 每批复测 |
