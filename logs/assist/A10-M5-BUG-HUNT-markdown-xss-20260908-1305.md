# A10 · M5-BUG-HUNT markdown 链接 XSS 修复（START CODE）

> Lane: A10（M5 独立安全复审 / BUG-HUNT 转为 START CODE） · Wave: **M5-BUG-HUNT Follow-up Dispatch**（board §BUG-HUNT L1231-1252；base `052b18a`）
> A10 BUG-HUNT 任务（board L1246）：**`src/utils/markdown.ts` + affected renderers + tests | 用安全 URL 解析/转义 + 外部链接 `rel=noopener` 闭环已确认的 markdown 链接属性/XSS 路径；不要重写整个 markdown 渲染器。**
> 交付时间：2026-09-08 ~13:05 CST · BASE=`052b18a`（W15 已 push）
> 整包交付：修复 `markdown.ts` + 新测试 `scripts/check-markdown-xss-logic.mjs` + 本复核 + checkpoint + 二进制补丁。**未 push（仅 A0 可推）**。

## 0. Lane Output Template（机器可读结论）

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

## 1. 根因（B11-3，P1 XSS）

`src/utils/markdown.ts` 行内链接生成：

```ts
.replace(/\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g, '<a href="$2" target="_blank">$1</a>');
```

- `$2`（URL）取自 `https?://[^)]+` 后**原样插入 `href="..."` 且未转义引号**；`[x](https://a.com/"onmouseover="alert(1))` 经 `v-html` 注入 `onmouseover` 属性执行（BUG-HUNT-SUMMARY.md L62）。
- 缺 `rel="noopener noreferrer"`，外部 `target="_blank"` 可被反劫持（B11-4/5/6）。
- 链接文本 `$1` 在 `inline()` 调用前已由 `esc()`（转义 `& < >`）处理，安全；仅 URL 未处理引号。

受影响渲染器（消费链）：`useWorkspaceStore.ts` 的 `renderMd()`（行 485/493/498/560）→ `mdHtml`/`inlineHtml` → `FileEditor.vue:29` / `FilePanel.vue:52` 的 `v-html`。**根因仅在 `markdown.ts`，渲染器无需改。**

## 2. 修复（最小外科手术，未重写渲染器）

`src/utils/markdown.ts`：

1. 新增 HTML 属性上下文转义辅助（仅 URL 未经 esc 处理引号；`& < >` 已由 `esc` 处理，这里补引号防属性注入）：
   ```ts
   const escapeAttr = (s: string) =>
     s.replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
   ```
2. 链接生成改为 replacer 函数，对 URL 转义并补 `rel`：
   ```ts
   .replace(
     /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g,
     (_m, text: string, url: string) =>
       `<a href="${escapeAttr(url)}" target="_blank" rel="noopener noreferrer">${text}</a>`,
   );
   ```
- **scheme 限定**：正则仍限定 `https?://`，`javascript:` 等不会被链接化（已断言）。
- **无二次转义**：`escapeAttr` 不处理 `&`（已由 `esc` 转 `&amp;`），查询串 `&` 在 href 内保持单 `&amp;`。
- 其它 markdown 渲染（标题/粗体/斜体/代码块/列表）完全保留 —— **未重写渲染器**，符合 board L1246 「Do not rewrite the markdown renderer」。

## 3. 测试（headless，加载真实 renderMd）

新增 `scripts/check-markdown-xss-logic.mjs`（复用 A9 既有 TS 加载模式：native TS strip 优先，回退 esbuild），覆盖 16 断言：

- B11-3：XSS 载荷 `[x](https://a.com/"onmouseover="alert(1))` → 无裸 `onmouseover="`、含 `&quot;`、单 `<a`、含 `rel="noopener noreferrer"`。
- 正常外部链接结构精确：`<a href="https://example.com" target="_blank" rel="noopener noreferrer">text</a>`。
- 查询串 `&` → 单 `&amp;`（无 `&amp;amp;` 二次转义）。
- URL 内 `<`/`>` 不突破属性边界（转 `&lt;`/`&gt;`）。
- `javascript:` scheme 不被渲染为链接。
- 链接文本 `<b>` 被 esc（防元素内容 XSS）。
- 代码块内链接不渲染。
- 不回归基础 markdown（标题/粗体/斜体）。

## 4. 验证（命令 → 结果）

| 检查 | 命令 | 结果 |
|---|---|---|
| markdown XSS 逻辑 | `node scripts/check-markdown-xss-logic.mjs` | **MARKDOWN_XSS_RESULT=PASS（通过 16，失败 0）** ✅ |
| 前端编译回归 | `npm run build` | **✓ built in 3.26s**（index 183.38kB）✅ |
| 空白检查 | `git diff --check -- src/utils/markdown.ts` | 干净（exit 0）✅ |
| Lint | `read_lints src/utils/markdown.ts` | 0 诊断 ✅ |
| 跨 lane 未改渲染器 | `git diff --name-only HEAD`（markdown.ts 改 + 新脚本） | 渲染器 FileEditor/FilePanel 无改动 ✅ |

## 5. 与 W17 Closeout 的衔接

A10 在 W17 Acceptance Closeout（已完成，PASS）中曾**非阻塞标记**两个 Rust 文件（`script_runner.rs` / `tauri-browser-tabs/linux.rs`）越出 W17 scope。本波确认它们即为 **A0 在 BUG-HUNT 直接 DONE 的 B3-1（script_runner 进程组回收）/ B9-1（linux.rs WebKitGTK 死锁修复）**（board L1237）——属 A0 直接修复，非 stray drift、非特权扩张，标记消解。

## 6. 范围与边界（诚实）

- **在 scope**：`markdown.ts` 链接 XSS 闭环 + 新测试脚本。
- **渲染器**：`FileEditor.vue`/`FilePanel.vue` 为 `renderMd` 的 `v-html` 消费者，根因仅在 `markdown.ts`，**无代码改动**（符合「最小外科手术」）。
- **out-of-scope（交 A0/A9）**：B11-4/5/6 中 `target="_blank"` 缺 `rel=noopener` 还涉及 Artifact/Resource 渲染器与 `UnifiedTabBar` 原始 URL 渲染，属 board L1249「ordinary-model work after confirmed findings are recorded」，不在 A10 L1246 scope；A10 仅在 markdown 链接路径一并补了 `rel`，其余由后续 ordinary-model 工作收口。
- **未接 pre-merge.sh**：门禁接线属 A9/A0 职责（与 A9 W17 closeout 同一处理方式），A10 不跨 lane 改 `pre-merge.sh`，避免冲突；建议在 A0 集成时把 `node scripts/check-markdown-xss-logic.mjs` 接入 `pre-merge.sh` 的 UI 逻辑测试区。

## 7. 结论

A10 M5-BUG-HUNT **STATUS=PASS**：B11-3 markdown 链接属性注入 XSS 已闭环（`escapeAttr` 转义 + `rel=noopener`），修复最小、未重写渲染器；headless 测试 16/16 PASS、`npm run build` PASS、`git diff --check` 干净、0 lint；二进制补丁已生成。工作树其余在制 lane 改动原样保留，未 push。

## 8. 声明

- HEAD=`052b18a`，本地与 origin/master 一致（`git pull --ff-only` 因上游多分支跟踪配置报「无法快进到多个分支」误报，工作树实际与远端一致，无需拉取）。
- 仅改 `src/utils/markdown.ts` + 新增 `scripts/check-markdown-xss-logic.mjs`；未改任何渲染器、未改 pre-merge.sh、未新增 Tauri 命令/ACL/bridge/依赖。
- 未 commit、未 push（W17 BUG-HUNT 规则：仅 A0 可 push）。
