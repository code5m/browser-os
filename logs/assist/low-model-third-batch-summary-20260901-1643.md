# 第三批低模型任务汇总（压缩为强模型最小输入 + 验收草案）

> 生成：2026-09-01 16:43。分支：feature-M0-baseline。
> 目的：把前两批 assist 材料压成"强模型最小输入卡"+"验收脚本草案"，减少后续高模型 token。
> **全程未移动 NEXT（仍 M1-4）、未签任何主线 PASS、未改 src/src-tauri 产品代码。**

## 交付文件与 commit
| # | 文件 | commit | 状态 |
|---|------|--------|------|
| 1 | M1-4-minicard-20260901-1643.md | a7c1088 | DONE |
| 2 | M1-6-minicard-20260901-1643.md | a7c1088 | DONE |
| 3 | M1-8-minicard-20260901-1643.md | ca60f20 | DONE |
| 4 | M1-9-minicard-20260901-1643.md | ca60f20 | DONE |
| 5 | M2-4-minicard-20260901-1643.md | 0023814 | DONE |
| 6 | M2-8-minicard-20260901-1643.md | 0023814 | DONE |
| 7 | M3-terminal-minicard-20260901-1643.md | 1d31273 | DONE |
| 8 | high-risk-acceptance-matrix-20260901-1643.md | 262c22f | DONE |
| 9 | assist-index-20260901-1643.md | 本轮末提交 | DONE |
| 10 | low-model-third-batch-summary-20260901-1643.md | 本轮末提交 | DONE |

> 注：minicard 按"每 2~3 个文件一 commit"分组提交（a7c1088 / ca60f20 / 0023814 / 1d31273 / 262c22f），
> 索引与汇总独立提交，避免超大单提交。

## BLOCKED 情况
- 无 BLOCKED。全部 10 个文件基于真实代码检索（bridge.rs / domain.rs / sync.rs / security_policy.rs / §5 / §6 / §7）完成。

## 关键偏差（已如实记录，未粉饰）
- **§4.3 工具目录未落地**：M2-8-minicard 末尾标注 `src/tools/` 实际不存在，强模型须先建目录再 `include_dir`（见 M2-7-assist）。

## 强模型领取路径（最小输入）
1. 领取具体任务 → 读对应 **minicard（必读）** → 按需读 assist 详细版。
2. 写代码前看 **high-risk-acceptance-matrix** 对应反向用例 + 证据要求。
3. M4/M5 未到可启动（§8 锁定），其 assist 仅归档。

## 全局验证结果
- 开工 `git status --short --branch`：feature-M0-baseline，干净。
- 每次提交前 `git diff --check`：全部 EXIT=0（仅新增 logs/assist/ 文件）。
- 结束 `git status --short --branch`：干净，NEXT 仍 M1-4。
- 未改 src / src-tauri；未移动 NEXT；未宣称主线 PASS。

## 后续建议
- 强模型实现 M1-4 后可顺移 NEXT（由 DEEP 裁决，本批不代签）。
- 验收脚本（accept-*.sh / vitest / cargo test）可按本汇总 §8 矩阵落地，建议归入 `scripts/` + 前端测试套件。
