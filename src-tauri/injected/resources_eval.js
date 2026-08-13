(function () {
  function abs(u) {
    try { return new URL(u, location.href).href; } catch (_) { return u; }
  }
  function push(items, seen, type, url) {
    if (!url) return;
    var a = abs(url);
    if (a.startsWith("data:") || a.startsWith("about:") || a.startsWith("blob:")) return;
    if (seen[a]) return;
    seen[a] = true;
    items.push({ res_type: type, url: url, absolute: a });
  }
  var items = [];
  var seen = {};
  try {
    document.querySelectorAll("script[src]").forEach(function (n) { push(items, seen, "script", n.getAttribute("src")); });
    document.querySelectorAll('link[rel="stylesheet"][href], link[type="text/css"][href]').forEach(function (n) { push(items, seen, "stylesheet", n.getAttribute("href")); });
    document.querySelectorAll('link[rel="preload"][as="style"][href]').forEach(function (n) { push(items, seen, "stylesheet", n.getAttribute("href")); });
    document.querySelectorAll("img[src]").forEach(function (n) { push(items, seen, "image", n.getAttribute("src")); });
    document.querySelectorAll('object[type="image/svg+xml"][data], embed[type="image/svg+xml"][src]').forEach(function (n) { push(items, seen, "svg", n.getAttribute("data") || n.getAttribute("src")); });
    document.querySelectorAll("use[xlink\\:href]").forEach(function (n) { push(items, seen, "svg", n.getAttribute("xlink:href")); });
    document.querySelectorAll("iframe[src], frame[src]").forEach(function (n) { push(items, seen, "iframe", n.getAttribute("src")); });
    document.querySelectorAll("audio[src], video[src]").forEach(function (n) { push(items, seen, "media", n.getAttribute("src")); });
    document.querySelectorAll("source[src]").forEach(function (n) { push(items, seen, "media", n.getAttribute("src")); });
    document.querySelectorAll('link[rel="preload"][as="font"][href]').forEach(function (n) { push(items, seen, "font", n.getAttribute("href")); });
    // CSS 中引用的图片/字体（尽力扫描）
    Array.from(document.styleSheets).forEach(function (sheet) {
      try {
        Array.from(sheet.cssRules || []).forEach(function (rule) {
          if (!rule.cssText) return;
          var re = /url\(["']?([^"')]+)["']?\)/g, m;
          while ((m = re.exec(rule.cssText))) { push(items, seen, "css-asset", m[1]); }
        });
      } catch (_) {}
    });
  } catch (e) {}
  // 结果经全局 Tauri 注入的 invoke 回传（conf 已开 withGlobalTauri，window.__TAURI__.core.invoke 可用）
  try {
    window.__TAURI__.core.invoke("report_resources", { page_url: location.href, items: items });
  } catch (e) {}
})()
