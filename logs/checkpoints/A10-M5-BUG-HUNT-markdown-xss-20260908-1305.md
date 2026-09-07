# A10 · M5-BUG-HUNT markdown 链接 XSS 修复 — Checkpoint

```text
LANE: A10
STATUS: PASS
SCOPE: src/utils/markdown.ts（改）, scripts/check-markdown-xss-logic.mjs（新）, 受影响渲染器 FileEditor.vue:29 / FilePanel.vue:52（经 useWorkspaceStore.ts 调 renderMd，根因在 markdown.ts，渲染器无需改动）
DELIVERED: 闭环 B11-3 markdown 链接 URL 属性注入 XSS（escapeAttr 转义引号 + 外部链接补 rel="noopener noreferrer"）；headless 测试 16/16 PASS；npm run build PASS；未重写 markdown 渲染器
VERIFY: node scripts/check-markdown-xss-logic.mjs: MARKDOWN_XSS_RESULT=PASS(通过 16 失败 0); npm run build: ✓ built in 3.26s(index 183.38kB); git diff --check src/utils/markdown.ts: 干净(0); read_lints markdown.ts: 0 诊断
METRICS: N/A（markdown.ts 为单函数点改动，主 chunk 增量≈0；build 183.38kB 与 W17 基线 169.45kB 差异由其它在制 lane 贡献，非本修复）
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A10-M5-BUG-HUNT-markdown-xss-20260908-1305.patch（git diff HEAD -- markdown.ts + 新测试脚本，7792B）
RISKS: (1) 仅修 markdown.ts 链接路径；B11-4/5/6 中 Artifact/Resource/UnifiedTabBar 的 target=_blank 缺 rel 属 board L1249「ordinary-model work after confirmed findings」范畴，不在 A10 scope(L1246 仅 markdown.ts + affected renderers + tests)，交 A0/A9；(2) 渲染器 FileEditor.vue/FilePanel.vue 经 v-html 消费 renderMd 输出，已确认根因仅在 markdown.ts，渲染器无代码改动；(3) 修复保留正则 scheme 限定 http/https，javascript: 等不会被链接化（已断言）；(4) 未接 pre-merge.sh（属 A9/A0 接线职责，避免跨 lane 改 pre-merge）
NO_PUSH: confirmed
```
