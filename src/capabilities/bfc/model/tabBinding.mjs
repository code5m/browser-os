// BFC must use the actual last selected BrowserOS web tab, never the first
// bilibili.com tab in an unrelated browser session.
export function isBilibiliPage(raw) {
  try {
    const value = new URL(raw);
    return value.protocol === 'https:' && value.port === '' &&
      ['www.bilibili.com', 'space.bilibili.com'].includes(value.hostname);
  } catch { return false; }
}

export function selectBfcReadbackTab(tabs, selectedId, requestedId = selectedId) {
  if (!selectedId) throw new Error('BFC_TAB_NOT_ACTIVE');
  if (!/^tab-[0-9]+$/.test(selectedId) || requestedId !== selectedId)
    throw new Error('BFC_TAB_ID_MISMATCH');
  const tab = tabs.find((item) => item.id === selectedId);
  if (!tab) throw new Error('BFC_TAB_ID_MISMATCH');
  if (!isBilibiliPage(tab.url)) throw new Error('BFC_FAVORITES_PAGE_REQUIRED');
  return tab;
}

export function safeBfcTabSummary(tab) {
  const url = new URL(tab.url);
  const path = url.pathname.slice(0, 200);
  const originalTitle = String(tab.title || '').replace(/[\r\n\t]/g, ' ');
  const title = (/^https?:\/\//i.test(originalTitle)
    ? `${url.origin}${path}` : originalTitle.replace(/(token|secret|password|auth|key)=[^&\s]+/gi, '$1=[redacted]')).slice(0, 80);
  return {
    tabId: tab.id, title, url: `${url.origin}${path}`,
    isFavorites: url.hostname === 'space.bilibili.com' && /^\/[0-9]{1,20}\/favlist(?:\/|$)/.test(path)
  };
}
