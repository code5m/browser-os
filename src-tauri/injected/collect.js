// 注入到“浏览子窗口”的内容脚本：右键选区 → 保存到成果库
// IPC 仅对 label="browser" 窗口经窗口能力（capabilities/default.json）放行
(function () {
  if (window.__jzjd_injected) return;
  window.__jzjd_injected = true;

  const Tauri = window.__TAURI__ || window.__tauri__;
  let menu = null;

  document.addEventListener("contextmenu", function (e) {
    e.preventDefault();
    const sel = window.getSelection();
    const text = sel ? sel.toString() : "";
    const html = serializeSelection(sel);
    showMenu(e.clientX, e.clientY, text, html);
  });

  // 富文本保真：克隆选区 DOM → 绝对化链接/图片 → 内联关键计算样式 → 序列化
  function serializeSelection(sel) {
    if (!sel || sel.rangeCount === 0) return "";
    const range = sel.getRangeAt(0);
    const frag = range.cloneContents();
    const wrapper = document.createElement("div");
    wrapper.appendChild(frag);

    // 用原始选区节点的计算样式来内联（克隆后拿不到 live 样式，故按标签近似还原）
    inlineStyles(wrapper);
    absolutize(wrapper, "img", "src");
    absolutize(wrapper, "a", "href");
    absolutize(wrapper, "source", "src");

    // 去掉 script/style/iframe 等不安全或无意义节点
    wrapper.querySelectorAll("script,style,iframe,noscript").forEach((n) => n.remove());
    return wrapper.innerHTML;
  }

  // 内联常见排版样式，保证脱离原页面后仍保有基本外观
  const STYLE_PROPS = [
    "font-weight", "font-style", "font-size", "color", "background-color",
    "text-align", "text-decoration", "line-height", "list-style-type",
    "border", "padding", "margin",
  ];
  function inlineStyles(root) {
    // 对克隆树里的每个元素，按标签给出保守的内联样式
    root.querySelectorAll("*").forEach((el) => {
      const tag = el.tagName.toLowerCase();
      const s = el.style;
      if (tag === "b" || tag === "strong") s.fontWeight = "bold";
      if (tag === "i" || tag === "em") s.fontStyle = "italic";
      if (tag === "u") s.textDecoration = "underline";
      if (tag === "code" || tag === "pre") {
        s.fontFamily = "monospace";
        s.background = "#f4f4f4";
        s.padding = "2px 4px";
        s.borderRadius = "4px";
      }
      if (tag === "img") {
        s.maxWidth = "100%";
        s.height = "auto";
      }
      // 保留元素自带的关键 class 无意义（脱离页面 CSS），故清空 class 避免残留引用
      el.removeAttribute("class");
    });
    void STYLE_PROPS; // 保留字段说明，未来可切换到 getComputedStyle 精确内联
  }

  // 把相对 URL 绝对化，脱离原站点仍可访问
  function absolutize(root, selector, attr) {
    root.querySelectorAll(selector + "[" + attr + "]").forEach((el) => {
      try {
        const v = el.getAttribute(attr);
        if (v) el.setAttribute(attr, new URL(v, location.href).href);
      } catch (_) {}
    });
  }

  function showMenu(x, y, text, html) {
    removeMenu();
    if (!text) return;
    menu = document.createElement("div");
    menu.style.cssText =
      "position:fixed;z-index:2147483647;left:" + x + "px;top:" + y +
      "px;background:#fff;border:1px solid #ccc;box-shadow:0 2px 8px rgba(0,0,0,.2);" +
      "border-radius:6px;padding:4px;font-size:13px;";
    const btn = document.createElement("div");
    btn.textContent = "保存到成果库";
    btn.style.cssText = "padding:6px 12px;cursor:pointer;";
    btn.onclick = function () {
      if (!Tauri || !Tauri.core) {
        toast("未检测到桥环境");
        return removeMenu();
      }
      Tauri.core
        .invoke("collect_selection", {
          url: location.href,
          title: document.title,
          text: text,
          html: html,
        })
        .then(() => toast("已保存到本地成果库（带溯源）"))
        .catch((err) => toast("保存失败: " + err));
      removeMenu();
    };
    menu.appendChild(btn);
    document.body.appendChild(menu);
  }

  function removeMenu() {
    if (menu) {
      menu.remove();
      menu = null;
    }
  }
  document.addEventListener("click", removeMenu);

  function toast(msg) {
    const t = document.createElement("div");
    t.textContent = msg;
    t.style.cssText =
      "position:fixed;bottom:20px;left:50%;transform:translateX(-50%);" +
      "background:rgba(0,0,0,.8);color:#fff;padding:8px 16px;border-radius:20px;" +
      "z-index:2147483647;font-size:13px;";
    document.body.appendChild(t);
    setTimeout(() => t.remove(), 2000);
  }
})();
