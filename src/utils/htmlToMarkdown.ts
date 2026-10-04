// 将采集到的回复 HTML 转为可归档 Markdown。
// 纯字符串解析不会执行 HTML；危险容器连同正文一起丢弃，链接协议采用白名单。

const FORBIDDEN_BLOCKS = /<(script|style|button|iframe|object|form|svg)\b[^>]*>[\s\S]*?<\/\1\s*>/gi;
const TOKENS = /<!--[\s\S]*?-->|<![^>]*>|<[^>]+>|[^<]+/g;

function decodeEntities(value: string): string {
  const named: Record<string, string> = {
    amp: "&",
    apos: "'",
    gt: ">",
    lt: "<",
    nbsp: " ",
    quot: '"',
  };
  return value.replace(/&(#x[\da-f]+|#\d+|[a-z]+);/gi, (_raw, entity: string) => {
    if (entity[0] !== "#") return named[entity.toLowerCase()] ?? _raw;
    const hex = entity[1]?.toLowerCase() === "x";
    const codepoint = Number.parseInt(entity.slice(hex ? 2 : 1), hex ? 16 : 10);
    if (!Number.isFinite(codepoint) || codepoint < 0 || codepoint > 0x10ffff) return "";
    try {
      return String.fromCodePoint(codepoint);
    } catch {
      return "";
    }
  });
}

function safeHref(tag: string): string {
  const match = /\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/i.exec(tag);
  const href = decodeEntities(match?.[1] ?? match?.[2] ?? match?.[3] ?? "").trim();
  if (!href || !/^(https?:\/\/|\/|#)/i.test(href)) return "";
  return href.replace(/[()\s]/g, (char) => encodeURIComponent(char));
}

function normalize(markdown: string): string {
  return markdown
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n[ \t]+/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

export function htmlToMarkdown(source: string): string {
  let html = source;
  for (let index = 0; index < 3; index++) html = html.replace(FORBIDDEN_BLOCKS, "");

  let output = "";
  let preDepth = 0;
  const lists: Array<{ ordered: boolean; index: number }> = [];
  const links: string[] = [];
  const ensureBreak = (count: number) => {
    const present = output.match(/\n*$/)?.[0].length ?? 0;
    if (present < count) output += "\n".repeat(count - present);
  };

  for (const token of html.match(TOKENS) ?? []) {
    if (!token.startsWith("<")) {
      const text = decodeEntities(token);
      output += preDepth ? text : text.replace(/\s+/g, " ");
      continue;
    }
    if (/^<!--|^<!/i.test(token)) continue;
    const parsed = /^<\s*(\/?)\s*([a-z\d]+)/i.exec(token);
    if (!parsed) continue;
    const closing = parsed[1] === "/";
    const tag = parsed[2].toLowerCase();

    if (!closing) {
      if (/^h[1-6]$/.test(tag)) {
        ensureBreak(2);
        output += `${"#".repeat(Number(tag[1]))} `;
      } else if (["p", "div", "section", "article", "header", "footer", "tr"].includes(tag)) {
        ensureBreak(2);
      } else if (tag === "br") {
        ensureBreak(1);
      } else if (tag === "strong" || tag === "b") {
        output += "**";
      } else if (tag === "em" || tag === "i") {
        output += "*";
      } else if (tag === "del" || tag === "s") {
        output += "~~";
      } else if (tag === "pre") {
        ensureBreak(2);
        output += "```\n";
        preDepth++;
      } else if (tag === "code" && preDepth === 0) {
        output += "`";
      } else if (tag === "ul" || tag === "ol") {
        lists.push({ ordered: tag === "ol", index: 0 });
        ensureBreak(1);
      } else if (tag === "li") {
        ensureBreak(1);
        const list = lists.at(-1);
        if (list?.ordered) output += `${++list.index}. `;
        else output += "- ";
      } else if (tag === "blockquote") {
        ensureBreak(2);
        output += "> ";
      } else if (tag === "a") {
        const href = safeHref(token);
        links.push(href);
        if (href) output += "[";
      } else if (tag === "hr") {
        ensureBreak(2);
        output += "---";
        ensureBreak(2);
      }
      continue;
    }

    if (/^h[1-6]$/.test(tag) || ["p", "div", "section", "article", "header", "footer"].includes(tag)) {
      ensureBreak(2);
    } else if (tag === "strong" || tag === "b") {
      output += "**";
    } else if (tag === "em" || tag === "i") {
      output += "*";
    } else if (tag === "del" || tag === "s") {
      output += "~~";
    } else if (tag === "pre") {
      preDepth = Math.max(0, preDepth - 1);
      ensureBreak(1);
      output += "```";
      ensureBreak(2);
    } else if (tag === "code" && preDepth === 0) {
      output += "`";
    } else if (tag === "ul" || tag === "ol") {
      lists.pop();
      ensureBreak(2);
    } else if (tag === "li" || tag === "blockquote" || tag === "tr") {
      ensureBreak(1);
    } else if (tag === "a") {
      const href = links.pop() ?? "";
      if (href) output += `](${href})`;
    }
  }

  return normalize(output);
}
