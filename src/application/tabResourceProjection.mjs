/**
 * Single logical projection for the tab strip.
 *
 * Browser tabs retain their one-WebView-per-tab physical Owner and module
 * tabs retain Layout's logical Owner. This is NOT a WebView pool, automatic
 * session save, or a replacement for native resource lifecycle. Every
 * resource includes the original owner and unmodified opaque ID for routing.
 */
export function projectTabResources(browserTabs, moduleTabs, view, activeBrowser, activeModule, order) {
  const resources = [];
  const seen = new Set();
  for (const tab of browserTabs) {
    const key = 'web:' + tab.id;
    if (!tab.id || seen.has(key)) throw Error('TAB_ID_DUPLICATE');
    seen.add(key);
    resources.push({
      key, id:tab.id, owner:'browser', kind:'web',
      title:tab.title || tab.url, path:null, icon:'web',
      active:view === 'browser' && activeBrowser === tab.id,
      nativeWebviewId:tab.id
    });
  }
  for (const tab of moduleTabs) {
    const key = 'module:' + tab.id;
    if (!tab.id || seen.has(key)) throw Error('TAB_ID_DUPLICATE');
    seen.add(key);
    resources.push({
      key, id:tab.id, owner:'layout', kind:'module',
      title:tab.label, path:tab.path || null, icon:tab.icon,
      active:view === tab.view && activeModule === tab.id,
      nativeWebviewId:null,
      view:tab.view
    });
  }
  if (order) {
    const current = new Set(resources.map(item => item.key));
    for (let i = order.length - 1; i >= 0; i--) {
      if (!current.has(order[i])) order.splice(i, 1);
    }
    for (const item of resources) {
      if (!order.includes(item.key)) order.push(item.key);
    }
    const position = new Map(order.map((key, index) => [key, index]));
    resources.sort((a, b) => position.get(a.key) - position.get(b.key));
  }
  return resources;
}
export function tabResourceByKey(resources, key) {
  return resources.find(item => item.key === key) || null;
}
