#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-vault-logic.mjs — @browser-os/capability-vault 包内 domain logic 行为门禁
//
// 验证 Vault 领域行为和包内安全 Markdown 渲染器。
// 这是 PACKAGE_VALIDATION 的一部分（不依赖整个 App 即可验证 Domain）。
// ---------------------------------------------------------------------------
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { resolveNote, noteLinks, searchNotes } from "../src/internal/vault.mjs";
import { renderVaultMarkdown } from "../src/internal/markdown.mjs";

registerHooks({
  resolve(specifier, context, next) {
    try {
      return next(specifier, context);
    } catch (e) {
      for (const ext of [".ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
      throw e;
    }
  },
});
globalThis.window = { setTimeout };

const paths = ["Home.md", "notes/One.md", "a/Duplicate.md", "b/Duplicate.md"];

// 1) 当前笔记作为锚点，相对链接可解析
assert.deepStrictEqual(resolveNote("One", "notes/Home.md", paths), ["notes/One.md"]);
// 2) 以 "/" 开头的路径视为 vault 外（文件系统绝对路径），按设计返回 [] 不解析
assert.deepStrictEqual(resolveNote("/notes/One.md", "notes/Home.md", paths), []);
// 3) 重复文件名歧义 → 列出全部候选
assert.deepStrictEqual(resolveNote("Duplicate", "notes/Home.md", paths).sort(), [
  "a/Duplicate.md",
  "b/Duplicate.md",
]);
// 4) 非 md 链接被忽略
assert.deepStrictEqual(resolveNote("mailto:x@y.z", "notes/Home.md", paths), []);
// 5) 含 # 锚点的链接解析到文件
assert.deepStrictEqual(resolveNote("One#section", "notes/Home.md", paths), ["notes/One.md"]);

const text = "# Title\nSee [[One]] and [[Missing Note]].\nAlso [ext](https://e.x/z).";
// 6) wikilink + markdown link 都被提取
assert.deepStrictEqual(noteLinks(text).sort(), ["Missing Note", "One", "https://e.x/z"]);
// 7) 限定规模（防构造型放大）
assert.ok(noteLinks("# ".repeat(300)).length <= 256);
// 8) 代码块与行内代码不产生图谱边
assert.deepStrictEqual(noteLinks("```\n[[Hidden]]\n```\n`[[Inline]]`\n[[Visible]]"), ["Visible"]);

const notes = [
  { path: "a.md", text: "alpha beta" },
  { path: "b.md", text: "gamma" },
  { path: "c.md", text: "alpha again" },
];
// 9) 空查询返回全部
assert.strictEqual(searchNotes(notes, "").length, 3);
// 10) 正文命中
assert.deepStrictEqual(
  searchNotes(notes, "alpha").map((r) => r.path).sort(),
  ["a.md", "c.md"],
);
// 11) 路径命中
assert.deepStrictEqual(searchNotes(notes, "b.md").map((r) => r.path), ["b.md"]);

// 12) 常用 Markdown 与 wiki link 保持可读
const rendered = renderVaultMarkdown("# Title\n- one\n- two\n[[notes/One|Open]]\n[Web](https://example.com)");
assert.ok(rendered.includes("<h1>Title</h1>"));
assert.ok(rendered.includes("<ul><li>one</li><li>two</li></ul>"));
assert.ok(rendered.includes('<a href="notes/One">Open</a>'));
assert.ok(rendered.includes('rel="noopener noreferrer"'));
// 13) 原始 HTML 与危险 wiki scheme 不会形成可执行标签或链接
const hostile = renderVaultMarkdown('<img src=x onerror=alert(1)> [[javascript:alert(2)|bad]]');
assert.ok(hostile.includes("&lt;img"));
assert.ok(!hostile.includes("<img"));
assert.ok(!hostile.includes('href="javascript:'));

console.log("VAULT_LOGIC=PASS (13/13 domain and rendering assertions)");
