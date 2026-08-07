// 注入到 Android「浏览子 WebView」的采集脚本（对应桌面 injected/collect.js）。
// 通过 window.AndroidBridge.collectSelection(json) 调到 NativeBridge，
// 命令名/参数结构与桌面一致，保证前端逻辑跨端复用。
(function () {
  if (window.__jzjd_injected) return;
  window.__jzjd_injected = true;

  document.addEventListener("contextmenu", function (e) {
    e.preventDefault();
    const sel = window.getSelection();
    const text = sel ? sel.toString() : "";
    if (!text) return;
    const html = serializeSelection(sel);
    showMenu(e.clientX, e.clientY, text, html);
  });

  function serializeSelection(sel) {
    if (!sel || sel.rangeCount === 0) return "";
    const wrapper = document.createElement("div");
    wrapper.appendChild(sel.getRangeAt(0).cloneContents());
    wrapper.querySelectorAll("script,style,iframe,noscript").forEach((n) => n.remove());
    // 绝对化图片/链接，脱离原站仍可访问
    ["img[src]", "source[src]", "a[href]"].forEach((q) => {
      wrapper.querySelectorAll(q).forEach((el) => {
        const attr = el.hasAttribute("src") ? "src" : "href";
        try {
          el.setAttribute(attr, new URL(el.getAttribute(attr), location.href).href);
        } catch (_) {}
      });
    });
    return wrapper.innerHTML;
  }

  let menu = null;
  function showMenu(x, y, text, html) {
    removeMenu();
    menu = document.createElement("div");
    menu.style.cssText =
      "position:fixed;z-index:2147483647;left:" + x + "px;top:" + y +
      "px;background:#fff;border:1px solid #ccc;border-radius:6px;padding:4px;font-size:14px;";
    const btn = document.createElement("div");
    btn.textContent = "保存到成果库";
    btn.style.cssText = "padding:8px 14px;";
    btn.onclick = function () {
      try {
        window.AndroidBridge.collectSelection(
          JSON.stringify({ url: location.href, title: document.title, text: text, html: html })
        );
        toast("已保存到本地成果库（带溯源）");
      } catch (err) {
        toast("保存失败: " + err);
      }
      removeMenu();
    };
    menu.appendChild(btn);
    document.body.appendChild(menu);
  }
  function removeMenu() { if (menu) { menu.remove(); menu = null; } }
  document.addEventListener("click", removeMenu);

  function toast(m) {
    const t = document.createElement("div");
    t.textContent = m;
    t.style.cssText =
      "position:fixed;bottom:24px;left:50%;transform:translateX(-50%);" +
      "background:rgba(0,0,0,.8);color:#fff;padding:8px 16px;border-radius:20px;z-index:2147483647;";
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 2000);
  }
})();
