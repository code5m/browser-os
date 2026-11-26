// Fixed read-only probe for an inactive ordinary renderer. The parent also
// excludes every generation with a prior page-affecting or unknown command.
(() => {
  try {
    const emptyBody = document.body !== null &&
      document.body.children.length === 0 &&
      (document.body.textContent || '').trim() === '';
    const safe = location.href === 'about:blank' &&
      document.readyState === 'complete' && history.length === 1 &&
      !document.hasFocus() && emptyBody &&
      !document.querySelector('script,iframe,frame,canvas,audio,video,input,textarea,select,button,form,img,object,embed,[contenteditable],[role="textbox"]') &&
      !window.onbeforeunload && window.__browserosPreventHibernate !== true;
    return JSON.stringify({ safe: safe === true, url: location.href });
  } catch (_) {
    return JSON.stringify({ safe: false, url: '' });
  }
})()
