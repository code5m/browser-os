#!/usr/bin/env node
// ---------------------------------------------------------------------------
// check-vault-logic.mjs — @browser-os/capability-vault 包内 domain logic 行为门禁
//
// 复用 Frontend M2 Pilot 既有的 Vault 领域行为断言（resolveNote / noteLinks /
// searchNotes），从包内 private internal 导入，证明迁移后逻辑逐字节一致。
// 这是 PACKAGE_VALIDATION 的一部分（不依赖整个 App 即可验证 Domain）。
// ---------------------------------------------------------------------------
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { resolveNote, noteLinks, searchNotes } from "../src/internal/vault.mjs";

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

const notes = [
  { path: "a.md", text: "alpha beta" },
  { path: "b.md", text: "gamma" },
  { path: "c.md", text: "alpha again" },
];
// 8) 空查询返回全部
assert.strictEqual(searchNotes(notes, "").length, 3);
// 9) 正文命中
assert.deepStrictEqual(
  searchNotes(notes, "alpha").map((r) => r.path).sort(),
  ["a.md", "c.md"],
);
// 10) 路径命中
assert.deepStrictEqual(searchNotes(notes, "b.md").map((r) => r.path), ["b.md"]);

console.log("VAULT_LOGIC=PASS (10/10 domain assertions)");
