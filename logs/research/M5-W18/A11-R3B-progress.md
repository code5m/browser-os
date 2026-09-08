# A11 · M5-W18-R3B 进度账本（蓝图 §9 场景账本 + 验收追踪）

> Lane A11 · R3B 最终验收打包 · 基线 `origin/master` `d96b9b5` · 不 push
> 本文件记录 R3B 波次各 lane 状态、9 项 R3B 发现处置、与 A11 验收矩阵实跑结果。

## 1. 本 Lane 交付物

| 文件 | 用途 |
|---|---|
| `A11-R3B-integration-manifest.json` | 验收真源：门限、A0 裁定、参考钉版、lane 状态、9 项发现、检查项 |
| `A11-R3B-acceptance-matrix.py` | 机器探针：**解析真实 A1 HTML 的 `.frame` 文本**，覆盖尺寸/Git/a11y/折叠/卫生/快捷键/几何/钉版/A10 检查器 |
| `A11-R3B-acceptance-checklist.md` | 用户评审清单 |
| `A11-R3B-run-20260908.out` | 矩阵实跑结果（针对当前 R3 原型，暴露缺口） |
| `A11-R3B-checkpoint.md` | Lane 检查点 |

## 2. 对 A0 关键批语的回应

A0 R3 审计指出 R3 的 A11 矩阵"只在参考文档里找词，不证明原型覆盖"。本 R3B 矩阵改为：

- **真实 DOM 解析**：用 div 配平法提取每个 `.frame` 块文本（排除报告正文 h2/frame-meta/汇总表），Git 单元与无障碍属性只在原型 UI 文本/CSS 内判定。
- **A10 检查器内嵌**：直接运行 A10 修正后的自包含检查器并纳入门禁。
- **几何公式机算**：按 A0 公式 `(w-28)/w`、`(h-60-24)/h` 在六尺寸实算。
- **钉版结构校验**：manifest 参考钉版须为 `cee14e9`，否则 WARN（R3B-07）。

## 3. Lane 状态（R3B 上游尚未提交）

| Lane | 分支 | R3 HEAD | R3B 状态 | 阻断? |
|---|---|---|---|---|
| A1 | codex/m5-w18-a1 | ee2b8fd | PENDING | ✅ 阻断（交付修正原型） |
| A2 | codex/m5-w18-a2 | 63b0a6e | PENDING | — |
| A3 | codex/m5-w18-a3 | 0258c5b | PENDING | — |
| A4 | codex/m5-w18-a4 | 8e77799 | PENDING | — |
| A5 | codex/m5-w18-a5 | 6bc4942 | PENDING | — |
| A6 | codex/m5-w18-a6 | d5eb144 | PENDING | — |
| A7 | codex/m5-w18-a7 | 6e1f2ba | PENDING | — |
| A8 | codex/m5-w18-a8 | 0db481a | PENDING | — |
| A9 | codex/m5-w18-a9 | 23d12cd | PENDING | — |
| A10 | codex/m5-w18-a10 | 3a86834 | PENDING | ✅ 阻断（独立复核 + 检查器全绿） |
| A11 | codex/m5-w18-a11 | d96b9b5 | PROVISIONAL_PENDING_UPSTREAM | — |

## 4. R3B 发现处置

| ID | 严重 | 主题 | 归属 | 状态 | 矩阵探针 |
|---|---|---|---|---|---|
| R3B-01 | high | Git 覆盖不全 | A1 | OPEN | G-worktrees/conflicts/patch/amend/reset-revert |
| R3B-02 | high | 原型卫生（donor 字体 + 检查器 FAIL） | A1/A10 | OPEN | H-font, A10-gate |
| R3B-03 | high | 折叠隐藏活动条 | A1/A3/A4 | OPEN | C-strip |
| R3B-04 | high | 无障碍不可证 | A1 | OPEN | A-aria, A-focus |
| R3B-05 | med | 几何口径未冻结 | A0/A2 | OPEN | GEO, GEO-status |
| R3B-06 | med | 快捷键冲突 | A6/A1 | OPEN | K-binding |
| R3B-07 | med | 参考钉版不一致 | A10/A7 | OPEN | M-pin |
| R3B-08 | med | 缺 1200×800 / 900×600 | A1/A2 | OPEN | S-1200x800, S-900x600 |
| R3B-09 | med | 原生可行性未验证 | A0/用户 | RESERVED_NATIVE | （允许 NOT_RUN） |

## 5. 矩阵实跑结果（A11-R3B-run-20260908.out，针对当前 R3 原型）

```
20 PASS / 0 FAIL / 12 GAP_CONFIRMED / 2 WARN / 0 NOT_RUN
OPEN R3B FINDINGS: R3B-01, R3B-02, R3B-03, R3B-04, R3B-05, R3B-06, R3B-08
GATE: FAIL   (PROVISIONAL_PENDING_UPSTREAM)
```

逐项：`S-1200x800`/`S-900x600` GAP；`G-worktrees`/`G-conflicts`/`G-patch`/`G-amend`/`G-reset-revert` GAP；
`A-aria` GAP；`C-strip` GAP；`H-font` GAP；`K-binding` GAP；`A10-gate` GAP（exit=1）；
`GEO-status` WARN（26px≠24px）；`A-focus` WARN（无 focus-visible 规则）；`M-pin` PASS（`cee14e9`）；
`H-brand` PASS（UI 文本干净，A10 检查器对正文更严）。

## 6. 后续依赖（待上游落地后 A11 重跑定稿）

1. A1 提交 R3B 修正原型（14 Git 单元 + amend/reset-revert、role/aria、六尺寸含 900×600、折叠保留 28px 条、中性字体栈、`Ctrl+K`=地址）。
2. A10 提交 R3B 独立复核 + 重跑自包含检查器（须全绿，且无 open high）。
3. A11 重跑 `A11-R3B-acceptance-matrix.py --manifest ... --report`：当 `GAP_CONFIRMED=0` 且 `WARN` 均可解释（仅 R3B-09 原生预留），将 manifest `status` 改为 `FINAL` 并出最终包。
4. A0 集成并开片交付用户评审。

## 7. 风险

- **上游未动**：本包当前运行于 R3 原型，缺口即上游待办；非 A11 自身缺陷。
- **A10 检查器更严**：要求连报告正文品牌串也清除，A1 修正版须注意。
- **不 push**：A11 仅提交本 lane 研究产物，集成/push 归 A0。
