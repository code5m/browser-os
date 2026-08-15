// 初始化脚本（通过 initialization_script 注入，在页面 JS 之前执行）
// 功能：禁用系统原生右键菜单 → 注入自定义采集菜单（选区/整页）
(function () {
  if (window.__jzjd_injected) return;
  window.__jzjd_injected = true;

// ====== 1. 禁用 Tauri 原生右键菜单 + 显示自定义采集菜单（合并在 capture 阶段）======
document.addEventListener("contextmenu", function (e) {
  var t = e.target;
  // 不抢占输入框/文本域内的右键（用户可能需要粘贴等）
  if (t && (t.isContentEditable || t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;
  // 阻止默认（系统原生菜单 + Tauri DevTools 菜单）
  e.stopPropagation();
  e.stopImmediatePropagation();
  e.preventDefault();
  // 直接在此处显示自定义菜单（同一监听器，不存在事件被吞的问题）
  showMenu(e.clientX, e.clientY);
  return false;
}, true); // capture 阶段，优先级最高

  // ====== 统一拿 invoke（兼容多种全局注入形态）======
  function getInvoke() {
    try {
      var T = window.__TAURI__;
      if (T && T.core && typeof T.core.invoke === "function") return T.core.invoke.bind(T.core);
      var I = window.__TAURI_INTERNALS__;
      if (I && typeof I.invoke === "function") return I.invoke.bind(I);
    } catch (e) { console.error("[JZJD] getInvoke error:", e); }
    return null;
  }

  let menu = null;

  // 富文本保真：克隆选区 DOM → 绝对化链接/图片 → 内联样式 → 序列化
  function serializeSelection(sel) {
    if (!sel || sel.rangeCount === 0) return "";
    var range = sel.getRangeAt(0);
    var frag = range.cloneContents();
    var wrapper = document.createElement("div");
    wrapper.appendChild(frag);
    inlineStyles(wrapper);
    absolutize(wrapper, "img", "src");
    absolutize(wrapper, "a", "href");
    absolutize(wrapper, "source", "src");
    inlineImages(wrapper);
    wrapper.querySelectorAll("script,style,iframe,noscript").forEach(function (n) { n.remove(); });
    return wrapper.innerHTML;
  }

  function inlineStyles(root) {
    root.querySelectorAll("*").forEach(function (el) {
      var tag = el.tagName.toLowerCase(), s = el.style;
      if (tag === "b" || tag === "strong") s.fontWeight = "bold";
      if (tag === "i" || tag === "em") s.fontStyle = "italic";
      if (tag === "u") s.textDecoration = "underline";
      if (tag === "code" || tag === "pre") { s.fontFamily = "monospace"; s.background = "#f4f4f4"; s.padding = "2px 4px"; s.borderRadius = "4px"; }
      if (tag === "img") { s.maxWidth = "100%"; s.height = "auto"; }
      el.removeAttribute("class");
    });
  }

  function absolutize(root, selector, attr) {
    root.querySelectorAll(selector + "[" + attr + "]").forEach(function (el) {
      try { var v = el.getAttribute(attr); if (v) el.setAttribute(attr, new URL(v, location.href).href); } catch (_) {}
    });
  }

  // 图片转 base64 内联（跨域图片会降级保留绝对 URL）
  function inlineImages(root) {
    Array.from(root.querySelectorAll("img[src]")).forEach(function (img) {
      var src = img.getAttribute("src");
      if (!src || src.startsWith("data:")) return;
      try {
        var abs = new URL(src, location.href).href;
        if (abs.startsWith("http")) {
          fetch(abs, { mode: "no-cors" })
            .then(function (r) { return r.blob(); })
            .then(function (b) {
              var rd = new FileReader();
              rd.onload = function () { img.setAttribute("src", rd.result); };
              rd.readAsDataURL(b);
            })
            .catch(function () {});
        }
      } catch (_) {}
    });
  }

  function showMenu(x, y) {
    removeMenu();
    menu = document.createElement("div");
    menu.id = "jzjd-menu";
    menu.style.cssText =
      "position:fixed;z-index:2147483647;left:" + x + "px;top:" + y +
      "px;background:#fff;border:1px solid #ccc;box-shadow:0 2px 8px rgba(0,0,0,.25);" +
      "border-radius:6px;padding:4px;font-size:13px;min-width:160px;";

    var sel = window.getSelection();
    var text = sel ? sel.toString().trim() : "";

    if (text) {
      addItem(menu, "📋 保存选区到成果库", function () {
        var html = serializeSelection(sel);
        collect(text, html, document.title);
      });
      addItem(menu, "📝 选区存为 Markdown 笔记", function () {
        saveNote(text);
      });
    }
    addItem(menu, "📄 保存整页到成果库", function () {
      var bodyHtml = document.body ? document.body.innerHTML : "";
      collect(document.body ? document.body.innerText : "", bodyHtml, document.title);
    });
    addItem(menu, "💻 打开终端", function () {
      var invoke = getInvoke();
      if (!invoke) {
        toast("⚠ 未检测到桥环境（请确认应用正常运行）");
        return;
      }
      invoke("request_open_terminal")
        .then(function () { toast("💻 已切换到终端视图"); })
        .catch(function (err) { toast("❌ 打开终端失败: " + err); console.error("[JZJD] invoke error:", err); });
    });

    document.body.appendChild(menu);

    // 菜单位置修正（不超出视口）
    var rect = menu.getBoundingClientRect();
    if (rect.right > window.innerWidth) menu.style.left = (x - rect.width) + "px";
    if (rect.bottom > window.innerHeight) menu.style.top = (y - rect.height) + "px";
  }

  function addItem(parent, label, onClick) {
    var btn = document.createElement("div");
    btn.textContent = label;
    btn.style.cssText = "padding:7px 14px;cursor:pointer;white-space:nowrap;border-radius:3px;";
    btn.onmouseenter = function () { btn.style.background = "#eef3ff"; };
    btn.onmouseleave = function () { btn.style.background = "transparent"; };
    btn.onclick = function () { onClick(); removeMenu(); };
    parent.appendChild(btn);
  }

  function collect(text, html, title) {
    var invoke = getInvoke();
    if (!invoke) {
      toast("⚠ 未检测到桥环境（请确认应用正常运行）");
      console.error("[JZJD] __TAURI__ =", window.__TAURI__, "__TAURI_INTERNALS__ =", window.__TAURI_INTERNALS__);
      return;
    }
    invoke("collect_selection", {
      url: location.href,
      title: title || document.title,
      text: text,
      html: html,
    })
      .then(function () { toast("✅ 已保存到本地成果库（带溯源）"); })
      .catch(function (err) { toast("❌ 保存失败: " + err); console.error("[JZJD] invoke error:", err); });
  }

  // 选区一键存为 Markdown 笔记（默认笔记目录，文件名=时间戳+选中内容摘要）
  function saveNote(text) {
    var invoke = getInvoke();
    if (!invoke) {
      toast("⚠ 未检测到桥环境（请确认应用正常运行）");
      return;
    }
    invoke("save_note", {
      url: location.href,
      title: document.title,
      text: text,
    })
      .then(function (path) { toast("✅ 笔记已保存: " + path); })
      .catch(function (err) { toast("❌ 保存失败: " + err); console.error("[JZJD] invoke error:", err); });
  }

  function removeMenu() {
    if (menu) { menu.remove(); menu = null; }
  }
  document.addEventListener("click", function (e) {
    if (menu && !menu.contains(e.target)) removeMenu();
  });
  window.addEventListener("scroll", removeMenu, true);
  document.addEventListener("keydown", function (e) { if (e.key === "Escape") removeMenu(); });

  function toast(msg) {
    var t = document.createElement("div");
    t.textContent = msg;
    t.style.cssText =
      "position:fixed;bottom:24px;left:50%;transform:translateX(-50%);" +
      "background:rgba(0,0,0,.85);color:#fff;padding:9px 18px;border-radius:20px;" +
      "z-index:2147483647;font-size:13px;max-width:80vw;";
    document.body.appendChild(t);
    setTimeout(function () { t.remove(); }, 2500);
  }

  // ====== target=_blank 链接点击拦截（修复"网页内链接不跳转"）======
  // 背景：wry 0.55 在 WebKitGTK 下，<a target="_blank"> 走 decide_policy 的
  // NewWindowAction 策略类型，但 wry 的 handler 只处理 NavigationAction（见
  // wry/webkitgtk/mod.rs:549-553，`_ => return false` 忽略 NewWindowAction），
  // 导致 target=_blank 链接被 WebKit 默认 deny、点了没反应（on_new_window 不触发）。
  // 修复：在 capture 阶段拦截 target=_blank 的 <a> 点击，改为在当前页导航。
  document.addEventListener("click", function (e) {
    var a = e.target && e.target.closest ? e.target.closest("a") : null;
    if (!a || !a.href) return;
    // 只处理 target=_blank（新窗口）链接；普通链接让 WebKit 原生处理
    if (a.target !== "_blank") return;
    // 不拦截带下载/特殊协议的链接
    if (a.hasAttribute("download")) return;
    e.preventDefault();
    e.stopPropagation();
    e.stopImmediatePropagation();
    // 在当前页跳转（相当于 target=_self）
    window.location.href = a.href;
    return false;
  }, true); // capture 阶段，在页面 JS 之前拦截

  console.log("[JZJD] 采集脚本已注入，右键即可保存。invoke 可用:", !!getInvoke());
})();
