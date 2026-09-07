// 轻量 markdown 渲染（标题/粗体/斜体/行内代码/代码块/链接/列表/换行）

export function renderMd(src: string): string {
  const esc = (s: string) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  // HTML 属性上下文转义：仅链接 URL（$2）未经 esc 处理引号；& < > 已由 esc 处理，这里补引号防属性注入。
  const escapeAttr = (s: string) =>
    s.replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const lines = src.replace(/\r\n/g, "\n").split("\n");
  let out = "";
  let inCode = false;
  for (const raw of lines) {
    if (raw.startsWith("```")) {
      inCode = !inCode;
      out += inCode ? "<pre><code>" : "</code></pre>";
      continue;
    }
    if (inCode) {
      out += esc(raw) + "\n";
      continue;
    }
    let line = esc(raw);
    const h = line.match(/^(#{1,6})\s+(.*)$/);
    if (h) {
      const lv = h[1].length;
      out += `<h${lv}>${h[2]}</h${lv}>`;
      continue;
    }
    const li = line.match(/^[-*]\s+(.*)$/);
    if (li) {
      out += `<li>${inline(li[1])}</li>`;
      continue;
    }
    if (line.trim() === "") {
      out += "<br/>";
      continue;
    }
    out += "<p>" + inline(line) + "</p>";
  }
  return out;
  function inline(t: string): string {
    return t
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\*\*([^*]+)\*\*/g, "<b>$1</b>")
      .replace(/\*([^*]+)\*/g, "<i>$1</i>")
      // 外部链接（scheme 已被正则限定 http/https）：URL 经 escapeAttr 转义防属性注入 XSS(B11-3)，
      // 并加 rel="noopener noreferrer" 防 target=_blank 反劫持(B11-4/5/6)。链接文本 $1 已含 esc 结果，安全。
      .replace(
        /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g,
        (_m, text: string, url: string) =>
          `<a href="${escapeAttr(url)}" target="_blank" rel="noopener noreferrer">${text}</a>`,
      );
  }
}
