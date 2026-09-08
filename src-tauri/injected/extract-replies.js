(() => {
  const host = location.hostname.toLowerCase();
  const matches = domain => host === domain || host.endsWith('.' + domain);
  const adapters = [
    ['ChatGPT', ['chatgpt.com', 'chat.openai.com'], '[data-message-author-role="assistant"]'],
    ['Claude', ['claude.ai'], '[data-is-streaming] .font-claude-response, .font-claude-response'],
    ['Gemini', ['gemini.google.com'], 'model-response .markdown'],
    ['DeepSeek', ['chat.deepseek.com'], '.ds-markdown'],
    ['Kimi', ['kimi.com', 'kimi.moonshot.cn'], '.segment-assistant .markdown, .chat-content-item-assistant .markdown'],
    ['Qwen', ['chat.qwen.ai'], '.response-message-content'],
    ['Doubao', ['doubao.com'], '[data-testid="receive_message"]'],
  ];
  const adapter = adapters.find(([, domains]) => domains.some(matches));
  if (!adapter) return JSON.stringify({ error: 'UNSUPPORTED_SITE', replies: [] });
  if (document.querySelector('[data-testid="stop-button"], [aria-label="Stop generating"], [data-is-streaming="true"], .stop-button')) {
    return JSON.stringify({ error: 'REPLY_STREAMING', replies: [] });
  }
  const redactUrl = value => {
    try {
      const url = new URL(value);
      url.username = '';
      url.password = '';
      url.hash = '';
      for (const key of [...url.searchParams.keys()]) {
        if (/token|password|secret|signature|api.?key|authorization|credential/i.test(key)) {
          url.searchParams.set(key, 'REDACTED');
        }
      }
      return url.toString();
    } catch {
      return value;
    }
  };
  const redactText = value => value.replace(/https?:\/\/[^\s<>"'\x60()]+/g, raw => {
    let urlText = raw;
    let suffix = '';
    while (/[.,!?;:]$/.test(urlText)) {
      suffix = urlText.slice(-1) + suffix;
      urlText = urlText.slice(0, -1);
    }
    return redactUrl(urlText) + suffix;
  });
  const allCandidates = Array.from(document.querySelectorAll(adapter[2]));
  const truncatedByCount = allCandidates.length > 40;
  const candidates = allCandidates.slice(-40);
  const nodes = candidates.filter(n => !candidates.some(p => p !== n && p.contains(n)));
  const replies = [];
  let bytes = 0;
  let truncated = truncatedByCount;
  for (const node of nodes) {
    const copy = node.cloneNode(true);
    copy.querySelectorAll('button,script,style,svg,nav,[aria-hidden="true"]').forEach(n => n.remove());
    copy.querySelectorAll('a[href]').forEach(a => a.setAttribute('href', redactUrl(a.getAttribute('href') || '')));
    const walker = document.createTreeWalker(copy, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) walker.currentNode.nodeValue = redactText(walker.currentNode.nodeValue || '');
    const text = copy.textContent.trim();
    if (!text) continue;
    bytes += new TextEncoder().encode(copy.innerHTML).length;
    if (bytes > 240000) { truncated = true; break; }
    replies.push(copy.innerHTML);
  }
  return JSON.stringify({ provider: adapter[0], replies, truncated, error: replies.length ? null : 'NO_ASSISTANT_REPLY' });
})()
