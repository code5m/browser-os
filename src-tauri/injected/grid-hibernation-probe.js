// Executed only through the Grid child's native read-only eval_result.
// A URL alone is not proof that a page's in-memory state is recoverable.
// Only a truly empty about:blank DOM is eligible for automatic process reclaim.
(() => {
  try {
    const emptyBody =
      document.body !== null &&
      document.body.children.length === 0 &&
      (document.body.textContent || '').trim() === '';
    const noInteractiveState = !document.querySelector(
      'script,iframe,frame,canvas,audio,video,input,textarea,select,button,form,' +
      'img,object,embed,[contenteditable],[role="textbox"]'
    );
    const safe =
      location.href === 'about:blank' &&
      document.readyState === 'complete' &&
      emptyBody &&
      noInteractiveState &&
      !window.onbeforeunload &&
      window.__browserosPreventHibernate !== true;
    return JSON.stringify({ safe: safe === true, url: location.href });
  } catch (_error) {
    return JSON.stringify({ safe: false, url: '' });
  }
})()
