# Semantic Change Request：行内编辑保存快照

## New Semantic

- 名称：`inlineSavedText`；类型：State；分类：`CURRENT_FACT`。
- Owner：`useFileStore`。成功读取当前行内文件或成功保存时更新，用于比较 `inlineText` 并提示丢弃草稿。

## Why Existing Semantic Cannot Represent / Existing Alternatives

已检查 `inlineFile`、`inlineText`、`filePath`、`previewDir`。`inlineText` 是用户可编辑内容，不能同时表示最近读取或成功保存的内容；路径和目录也不能表示该快照。快照不表示磁盘当前版本，不能替代原子版本校验。

## State / Intent / Owner Impact

仅新增由既有 `useFileStore` 持有的内部快照，不新增 Owner、Intent 或原生副作用。打开和保存是唯一写入口；组件不能写入。`inlineText` 仍为编辑内容的唯一来源，没有新增第二个可编辑真源。

## Checker Impact / ADR

登记 `states.yaml` 中的现有事实，不改 checker 或放宽规则；不改变跨能力所有权，无需 ADR。

## Review and Limitations

本次由执行 Agent 按源码及针对性测试审查；未声称人工评审。快照只保护部分行内草稿，不证明外部变更检测、原子 CAS、关闭页签和进程退出保护。用户已授权 Agent 自行完成验证和合并。

## 验证

应运行 `node scripts/check-semantic-registry.mjs --self-test` 和 `node scripts/check-semantic-registry.mjs`，并运行行内草稿行为回归。
