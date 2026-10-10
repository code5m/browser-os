# SCR-20261011 BrowserOS 共享纯时间格式化

## 语义检查

- **现有意图/状态**：无新增；此改动不创建 Store、写入者或新的功能入口。
- **类型与输出契约**：`formatRelative(iso: string | null | undefined, nowMs: number): string` 不变。
- **唯一实现**：`src/shared/pure/time/relative.ts`。
- **向后兼容**：`taskUi.ts` 仍导出原命名；`agentSkillUi.ts` 的 `runRelative` 保持原名和参数。
- **守护**：`scripts/check-shared-pure-time.mjs` 保证真实代码时间边界及唯一实现，`npm run check` 接入。
- **非等价候选**：不同字节单位/精度、错误消息与密钥结构脱敏皆不纳入统一工具。
- **体积判定**：保留 `measure-build-metrics.py` 原始阈值与基线；任何增长只能通过优化降低，不以变更阈值通关。
