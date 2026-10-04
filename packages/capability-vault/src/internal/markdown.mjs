// Vault 自有的安全 Markdown 渲染器。原始 HTML 一律转义，仅生成固定标签。
// 支持笔记阅读所需的标题、代码、强调、列表、外链和 Obsidian wiki link。

const escapeHtml = value => value
  .replace(/&/g, '&amp;')
  .replace(/</g, '&lt;')
  .replace(/>/g, '&gt;');

const escapeAttr = value => value
  .replace(/&/g, '&amp;')
  .replace(/"/g, '&quot;')
  .replace(/</g, '&lt;')
  .replace(/>/g, '&gt;');

function safeWikiTarget(value) {
  const target = value.trim();
  if (!target || /^[a-z][a-z\d+.-]*:/i.test(target) || target.startsWith('/')) return '';
  try { return escapeAttr(encodeURI(target)); } catch { return ''; }
}

function inline(value) {
  return value
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
    .replace(/\*([^*]+)\*/g, '<i>$1</i>')
    .replace(/\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g, (_raw, text, url) =>
      `<a href="${escapeAttr(url)}" target="_blank" rel="noopener noreferrer">${text}</a>`)
    .replace(/\[\[([^\]\n]+)\]\]/g, (_raw, body) => {
      const [rawTarget, rawLabel] = body.split('|');
      const target = safeWikiTarget(rawTarget);
      return target ? `<a href="${target}">${rawLabel || rawTarget}</a>` : (rawLabel || rawTarget);
    });
}

export function renderVaultMarkdown(source) {
  const lines = source.replace(/\r\n/g, '\n').split('\n');
  let html = '';
  let inCode = false;
  let list = '';
  const closeList = () => {
    if (list) html += `</${list}>`;
    list = '';
  };

  for (const raw of lines) {
    if (raw.startsWith('```')) {
      closeList();
      inCode = !inCode;
      html += inCode ? '<pre><code>' : '</code></pre>';
      continue;
    }
    if (inCode) {
      html += `${escapeHtml(raw)}\n`;
      continue;
    }
    const line = escapeHtml(raw);
    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading) {
      closeList();
      const level = heading[1].length;
      html += `<h${level}>${inline(heading[2])}</h${level}>`;
      continue;
    }
    const unordered = /^[-*]\s+(.*)$/.exec(line);
    const ordered = /^\d+[.)]\s+(.*)$/.exec(line);
    if (unordered || ordered) {
      const next = ordered ? 'ol' : 'ul';
      if (list !== next) {
        closeList();
        list = next;
        html += `<${list}>`;
      }
      html += `<li>${inline((unordered || ordered)[1])}</li>`;
      continue;
    }
    closeList();
    const quote = /^&gt;\s?(.*)$/.exec(line);
    if (quote) {
      html += `<blockquote>${inline(quote[1])}</blockquote>`;
      continue;
    }
    if (!line.trim()) {
      html += '<br/>';
      continue;
    }
    html += `<p>${inline(line)}</p>`;
  }
  closeList();
  if (inCode) html += '</code></pre>';
  return html;
}
