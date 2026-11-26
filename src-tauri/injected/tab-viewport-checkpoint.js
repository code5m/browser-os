(() => {
      const media = [...document.querySelectorAll('audio,video')].some(el => !el.paused && !el.ended);
      const controls = [...document.querySelectorAll('input,textarea,select')];
      const dirty = controls.some(el => {
        if (el instanceof HTMLInputElement) {
          const type = (el.type || '').toLowerCase();
          if (type === 'password') return el.value.length > 0;
          if (type === 'file') return !!el.files?.length;
          if (type === 'hidden') return false;
          if (type === 'checkbox' || type === 'radio') return el.checked !== el.defaultChecked;
        }
        if (el instanceof HTMLSelectElement) return [...el.options].some(o => o.selected !== o.defaultSelected);
        return 'defaultValue' in el && el.value !== el.defaultValue;
      });
      const focused = document.activeElement && document.activeElement !== document.body &&
        document.activeElement.matches('input,textarea,select,[contenteditable="true"]');
      const richEditor = [...document.querySelectorAll('[contenteditable="true"]')]
        .some(el => (el.textContent || '').trim().length > 0);
      // Embedded documents, canvas applications, framework-driven textboxes,
      // dialogs and even empty rich editors can carry non-serializable state.
      // URL + scroll is insufficient to restore them: skip rather than guess.
      const complexState = !!document.querySelector(
        'iframe,frame,canvas,embed,object,[contenteditable]:not([contenteditable="false"]),[role="textbox"],[aria-modal="true"],dialog[open]'
      );
      const protectedPage = media || dirty || focused || richEditor || complexState ||
        !!window.onbeforeunload || window.__browserosPreventHibernate === true;
      return JSON.stringify({
        protected: protectedPage || document.readyState !== 'complete',
        address: location.href,
        scrollX: Number(window.scrollX) || 0,
        scrollY: Number(window.scrollY) || 0
      });
    })()