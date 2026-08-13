// 资源扫描脚本：页面加载完成后收集其引用的外部资源（js/css/svg 等），
// 通过 invoke("report_resources") 上报给 Rust 后端，再转发到主窗口展示。
(function () {
  if (window.__jzjd_res_injected) return;
  window.__jzjd_res_injected = true;

  function abs(u) {
    try { return new URL(u, location.href).href; } catch (_) { return u; }
  }

  function collect() {
    var items = [];
    var seen = {};

    function push(type, url) {
      if (!url) return;
      var a = abs(url);
      // 跳过 data: 内联与 about: 等
      if (a.startsWith("data:") || a.startsWith("about:") || a.startsWith("blob:")) return;
      if (seen[a]) return;
      seen[a] = true;
      items.push({ res_type: type, url: url, absolute: a });
    }

    // JS
    document.querySelectorAll("script[src]").forEach(function (n) { push("script", n.getAttribute("src")); });
    // CSS
    document.querySelectorAll('link[rel="stylesheet"][href], link[type="text/css"][href]').forEach(function (n) { push("stylesheet", n.getAttribute("href")); });
    document.querySelectorAll('link[rel="preload"][as="style"][href]').forEach(function (n) { push("stylesheet", n.getAttribute("href")); });
    // 图片
    document.querySelectorAll("img[src]").forEach(function (n) { push("image", n.getAttribute("src")); });
    // SVG（内联 <svg> 不报；外部 <svg> 通过 <use>/<image> 或 object）
    document.querySelectorAll('object[type="image/svg+xml"][data], embed[type="image/svg+xml"][src]').forEach(function (n) {
      push("svg", n.getAttribute("data") || n.getAttribute("src"));
    });
    document.querySelectorAll("use[xlink\\:href]").forEach(function (n) { push("svg", n.getAttribute("xlink:href")); });
    // iframe / frame
    document.querySelectorAll("iframe[src], frame[src]").forEach(function (n) { push("iframe", n.getAttribute("src")); });
    // 媒体
    document.querySelectorAll("audio[src], video[src]").forEach(function (n) { push("media", n.getAttribute("src")); });
    document.querySelectorAll("source[src]").forEach(function (n) { push("media", n.getAttribute("src")); });
    // 字体
    document.querySelectorAll('link[rel="preload"][as="font"][href], link[rel="stylesheet"][href]').forEach(function (n) {
      // 字体多在 CSS 内，这里只抓显式 preload 字体
      if (n.getAttribute("as") === "font") push("font", n.getAttribute("href"));
    });

    var invoke = null;
    try {
      var T = window.__TAURI__;
      if (T && T.core && typeof T.core.invoke === "function") invoke = T.core.invoke.bind(T.core);
      var I = window.__TAURI_INTERNALS__;
      if (!invoke && I && typeof I.invoke === "function") invoke = I.invoke.bind(I);
    } catch (_) {}
    if (!invoke) { console.warn("[JZJD-RES] 无 invoke 可用，跳过上报"); return; }

    invoke("report_resources", { page_url: location.href, items: items })
      .catch(function (e) { console.warn("[JZJD-RES] 上报失败:", e); });
  }

  // 页面加载后扫描；SPA 路由变化后再扫一次
  function safeCollect() { try { collect(); } catch (e) { console.warn("[JZJD-RES]", e); } }
  if (document.readyState === "complete" || document.readyState === "interactive") {
    setTimeout(safeCollect, 800);
  } else {
    window.addEventListener("load", function () { setTimeout(safeCollect, 800); });
  }
  // 资源可能延迟加载，2.5s 后再补扫一次
  setTimeout(safeCollect, 2500);
  // SPA：监听 URL 变化
  var _push = history.pushState, _replace = history.replaceState;
  if (_push) history.pushState = function () { var r = _push.apply(this, arguments); safeCollect(); return r; };
  if (_replace) history.replaceState = function () { var r = _replace.apply(this, arguments); safeCollect(); return r; };
  window.addEventListener("popstate", function () { setTimeout(safeCollect, 800); });

  console.log("[JZJD-RES] 资源扫描脚本已注入");
})();
