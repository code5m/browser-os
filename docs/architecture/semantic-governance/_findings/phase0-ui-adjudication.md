# Phase 0 — Agent C：UI Checker Debt Adjudication

日期：2026-09-17
范围：仅裁决，不修改 `check-ui.mjs`，不修改 `FilePanel.vue`（src 默认 READ-ONLY）。

## C1. FilePanel fixed overlay

证据：
- `src/components/workspace/FilePanel.vue:280` `.move-confirm { position: fixed; inset: 0; ... z-index: 50; }`
- `src/components/workspace/FilePanel.vue:243` `<div class="move-confirm" v-if="ws.moveConfirm.show" ...>`
- `node scripts/check-ui.mjs` 输出：`[P2] src/components/workspace/FilePanel.vue:282 新的 position:fixed 浮层 ".move-confirm" 不在已批准允许列表中`
- `node scripts/check-ui.mjs --self-test` → `UI_CHECK_SELF_TEST: PASS`（检查器自身健康）

裁决：**REAL_VIOLATION**（真实产品违规），检查器规则正确，非 STALE_CHECKER_RULE。

理由：
1. `check-ui.mjs` 的 `ALLOWED_FIXED` 允许列表是针对「经源码验证的、已知不覆盖原生 WebView 的浮层」
   （boot-overlay / shell-error / status-bar / 各 approved modal-mask / ctx-menu 等）。
   `.move-confirm` 使用裸 `position: fixed; inset: 0`（全屏覆盖），既不在允许列表，也未采用项目既有的
   已批准 modal 模式（如 `ScriptRunDialog` 的 `.run-mask`、`PermissionPreviewModal` 的 `.modal-mask`、
   `PluginManager` 的 `.pm-modal`）。因此它是非符合规范的实现。
2. 这是**真实的 `position:fixed` 全屏浮层**，违反 PROJECT-RULES 规则 3.8（HTML 浮层禁止覆盖原生浏览器
   WebView 区域）。检查器按字面规则正确拦截。
3. 它不是 STALE_CHECKER_RULE：规则语义与源码策略一致，且检查器自测通过（无过宽/误报）。

处理（Phase 0 不修业务代码）：
- **不得**将 `.move-confirm` 加入 `ALLOWED_FIXED`（§19 禁止 allow-list 洗绿；无 owner/expiry/evidence 依据）。
- 记录为 **PRE_EXISTING_PRODUCT_DEBT**，建议后续（Phase 1 或独立产品 PR）将 `.move-confirm` 重构为
  已批准的 modal 模式（`.modal-mask` + `.move-confirm-box`），而非裸 `fixed; inset:0`。
- 仓库策略门因此 FAIL 属预期（见 Phase 0 Acceptance：REPOSITORY_POLICY_GATE = FAIL because PRE_EXISTING_PRODUCT_DEBT）。

## C2. UI checker 规则一致性

检查项相互重叠/冲突评估：

| 规则 | 文件 | 关注点 | 与 .move-confirm 关系 |
|---|---|---|---|
| position:fixed 不在 ALLOWED_FIXED | check-ui.mjs | 新 fixed 浮层覆盖 WebView | 触发（z-index 50 < 1000，仅 fixed 规则命中） |
| 高 z-index (>=1000) 不在 ALLOWED_HIGH_ZINDEX | check-ui.mjs | 高 z-index 覆盖 WebView | 不触发（50 < 1000） |
| toast-pop DEPRECATED | check-ui.mjs | 废弃浮层 | 无关 |
| 已新建页签 Toast DEPRECATED | check-ui.mjs | 废弃提示 | 无关 |
| native webview overlay | check-native-webview-overlay.mjs | 原生 UI 覆盖 webview | 不同关注点，不冲突 |
| SessionCloseDialog 规则 | 已在 Phase 0-A 移除 stale 规则 | — | C 不重新触碰（§9 禁止） |

结论：规则之间**无重叠、无冲突**。`check-ui.mjs` 的 fixed 规则是唯一命中 `.move-confirm` 的规则，
其语义自洽且经自测验证。无需要合并/删除的规则。

## 状态

- REAL_VIOLATION：FilePanel `.move-confirm` 全屏 fixed 浮层（PRE_EXISTING_PRODUCT_DEBT，记录，不修）。
- STALE_CHECKER_RULE：无。
- LEGITIMATE_EXCEPTION：无（不应作为例外加入 allow-list）。
- NEEDS_REDESIGN：无（重构建议属产品侧后续工作，非 Phase 0）。
