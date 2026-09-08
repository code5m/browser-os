# A11 · M5-W18-R3B 检查点

- Lane: A11（RESEARCH_AND_PROTOTYPE，验收打包）
- Wave: M5-W18-R3B（A0 退回的定向修订波，verdict `REVISE_TARGETED`）
- Worktree: `/home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3`
- Branch: `codex/m5-w18-a11`
- Base: `origin/master` `d96b9b5`
- HEAD (本提交): 见 `git rev-parse HEAD`（A11 仅新增 R3B 研究产物，未改产品代码）
- Pushed: 否（仅 A0 可 push）
- Generated: 2026-09-08T15:30+0800

## 1. 本次做了什么

按 R3B 卡「A11 - Final acceptance package」与 A0 审计的关键批语（Git/无障碍探针必须审真实 A1 HTML，而非参考文档找词），重新产出 A11 验收包：

1. **`A11-R3B-acceptance-matrix.py`**（新写）：标准库 HTML 解析器，用 div 配平法提取每个 `.frame` 块真实 UI 文本，机检——
   - 六目标尺寸是否渲染（1920/1440/1366/1200/1024/900×…）；
   - 14 Git 单元 + amend/reset-revert 是否在原型 UI 可见；
   - `role=`/`aria-*` 与 `:focus-visible` 是否存在；
   - 折叠语义（隐藏工具窗口 / 保留 28px 条 / 恢复入口）；
   - 卫生（UI 无 donor 品牌 / CSS 无 donor 字体）；
   - 快捷键（`Ctrl+K` 误绑 palette 检测）；
   - A0 几何公式六尺寸实算；
   - 参考钉版 `cee14e9` 结构校验；
   - 内嵌运行 A10 修正后的自包含检查器并纳入门禁。
2. **`A11-R3B-integration-manifest.json`**：编码 A0 全部裁定（六尺寸、几何公式、60/24/28px、快捷键、`cee14e9` 钉版、`COPY=0/ADAPT=4/REIMPLEMENT=10`、9 项 R3B 发现、11 lane 状态）。
3. **`A11-R3B-acceptance-checklist.md`**：用户评审清单（回应 A0 批语，映射 R3B-01..09）。
4. **`A11-R3B-progress.md`**：进度账本（lane 状态、发现处置、矩阵实跑结果、后续依赖）。
5. **`A11-R3B-run-20260908.out`**：矩阵实跑结果。

## 2. 实跑结果（针对当前 R3 原型）

A11 矩阵（A11-R3B-acceptance-matrix.py --manifest ...）：
```
21 PASS / 0 FAIL / 12 GAP_CONFIRMED / 2 WARN / 0 NOT_RUN
OPEN R3B FINDINGS: R3B-01, R3B-02, R3B-03, R3B-04, R3B-05, R3B-06, R3B-08
GATE: FAIL   (PROVISIONAL_PENDING_UPSTREAM)
```

A10 权威闭门审计（A10-R3B-closure-audit.py @ facc4b9，A0 九裁定 P1–P9）：
```
PROBES: 0/9 PASS   FINDINGS: HIGH=24 MEDIUM=25   GATE: FAIL
```
主阻断 = **A1 修正原型未交付**（worktree 仍 `ee2b8fd`），导致 P4/P5/P6 及其余探针连带 FAIL。

缺口即上游 R3B 待办：A1 须补 worktree/conflicts/patch/amend/reset-revert、加 role/aria、加 1200×800 与 900×600、折叠保留 28px 条、换中性字体栈、把 `Ctrl+K` 改回地址；A10 须待 A1 落地后重跑审计至全绿。

## 3. 如何复跑

```bash
cd /home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3
python3 logs/research/M5-W18/A11-R3B-acceptance-matrix.py \
  --manifest logs/research/M5-W18/A11-R3B-integration-manifest.json --report
# 或仅看 JSON 汇总：
python3 logs/research/M5-W18/A11-R3B-acceptance-matrix.py --json
```

当 A1 R3B 修正原型与 A10 R3B 复核落地后，重跑上述命令；若 `GAP_CONFIRMED=0` 且 WARN 仅剩 R3B-09 原生预留，将 manifest `status` 改为 `FINAL` 即定稿。

## 4. 风险与后续依赖

- 上游（A1 R3B 原型、A10 R3B 复核）尚未在本工作区提交；本包当前以 R3 原型为输入暴露缺口，属预期。
- A10 检查器比本包更严（报告正文品牌串也判违规），A1 修正版须连同正文清除 "Rebased" 等并改中性字体栈。
- 不 push；集成与开片由 A0 执行。W19 仍 `CLOSED`。

## 5. 范围边界（硬约束）

- 仅研究/验收产物，零产品代码、零依赖/ACL/capability/native runtime/用户数据改动。
- 未编辑他 lane 文件。
- 未 push。
