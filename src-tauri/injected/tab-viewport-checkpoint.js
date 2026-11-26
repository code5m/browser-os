(() => {
  // A bounded walk avoids copying arbitrarily large DOM collections or text.
  // Fail closed for complex application state, including observable shadow
  // roots. This is a heuristic, not a proof of absence of page-owned state.
  const address = location.href;
  const scrollX = window.scrollX, scrollY = window.scrollY;
  let protectedPage = true;
  try {
    if (document.readyState === 'complete' && document.documentElement &&
        Number.isFinite(scrollX) && Number.isFinite(scrollY) &&
        !window.onbeforeunload && window.__browserosPreventHibernate !== true) {
      protectedPage = false;
      const walker = document.createTreeWalker(document.documentElement, NodeFilter.SHOW_ELEMENT);
      let count = 0;
      let el = walker.currentNode;
      while (el) {
        if (++count > 10000) { protectedPage = true; break; }
        if (el.shadowRoot || el.matches(
          'iframe,frame,canvas,embed,object,[contenteditable]:not([contenteditable="false"]),[role="textbox"],[aria-modal="true"],dialog[open]'
        ) || (el === document.activeElement && el.matches('input,textarea,select'))) {
          protectedPage = true; break;
        }
        if (el instanceof HTMLMediaElement && !el.paused && !el.ended) {
          protectedPage = true; break;
        }
        if (el instanceof HTMLInputElement) {
          const type = el.type.toLowerCase();
          const dirty = type === 'file' ? !!el.files?.length
            : type === 'password' ? el.value.length > 0
            : type === 'hidden' ? false
            : type === 'checkbox' || type === 'radio' ? el.checked !== el.defaultChecked
            : el.value !== el.defaultValue;
          if (dirty) { protectedPage = true; break; }
        } else if (el instanceof HTMLTextAreaElement) {
          if (el.value !== el.defaultValue) { protectedPage = true; break; }
        } else if (el instanceof HTMLOptionElement) {
          if (el.selected !== el.defaultSelected) { protectedPage = true; break; }
        }
        el = walker.nextNode();
      }
    }
  } catch (_) { protectedPage = true; }
  return JSON.stringify({protected:protectedPage,address,scrollX,scrollY});
})()
