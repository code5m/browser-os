#!/usr/bin/env node
// 回复归档 HTML → Markdown 安全与常用结构回归。

import assert from "node:assert/strict";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import os from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SOURCE = join(ROOT, "src/utils/htmlToMarkdown.ts");

async function load() {
  if (!existsSync(SOURCE)) return null;
  try {
    return await import(pathToFileURL(SOURCE).href);
  } catch {}
  try {
    const esbuild = await import("esbuild");
    const { code } = esbuild.transformSync(readFileSync(SOURCE, "utf8"), {
      loader: "ts",
      format: "esm",
    });
    const output = join(os.tmpdir(), `html-to-markdown.${process.pid}.mjs`);
    writeFileSync(output, code);
    return await import(pathToFileURL(output).href);
  } catch {
    return null;
  }
}

const loaded = await load();
assert.equal(typeof loaded?.htmlToMarkdown, "function", "htmlToMarkdown must load");
const convert = loaded.htmlToMarkdown;

assert.equal(convert("<h2>Title</h2><p>Hello <strong>world</strong>.</p>"), "## Title\n\nHello **world**.");
assert.equal(convert("<ol><li>One</li><li>Two</li></ol>"), "1. One\n2. Two");
assert.equal(convert("<pre><code>&lt;x&gt;\n2</code></pre>"), "```\n<x>\n2\n```");
assert.equal(convert('<a href="https://example.com/a b">safe</a>'), "[safe](https://example.com/a%20b)");
assert.equal(convert('<a href="javascript:alert(1)">unsafe</a>'), "unsafe");

const hostile = convert('<script>alert(1)</script><img src=x onerror=alert(2)><p>kept</p><iframe>secret</iframe>');
assert.equal(hostile, "kept");
assert.ok(!/script|iframe|onerror|alert/.test(hostile));
assert.equal(convert("<p>A&nbsp;&amp;&nbsp;B &#x1f642;</p>"), "A & B 🙂");

console.log("HTML_TO_MARKDOWN_RESULT=PASS (7/7)");
