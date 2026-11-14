const invokeLine = "const invoke = window.__TAURI__?.core?.invoke; if (typeof invoke !== 'function') throw new Error('BFC_BROWSER_BRIDGE_UNAVAILABLE');";
export function buildLoginProbeScript(ticket) {
  const encodedTicket = JSON.stringify(String(ticket));
  return String.raw`(async () => {
    const ticket = ${encodedTicket};
    const invoke = window.__TAURI__?.core?.invoke;
    const root = document.documentElement;
    // An eval dispatch acknowledgement is NOT proof of a successful page IPC.
    // The trusted main window treats an absent readback reply as injection failure.
    if (typeof invoke !== 'function') {
      if (root) root.dataset.browserosBfcError = 'BFC_LOGIN_PROBE_INJECTION_FAILED';
      return;
    }
    try {
      const blockedPattern = /(?:^|\b)412(?:\b|$)|访问被拒绝|请求被拦截|安全验证|人机验证|访问过于频繁|操作频繁|风控验证/i;
      const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
      let ready = false;
      let login_status = 'unknown';
      let mid = null;
      let loginControlSeen = false;
      for (let attempt = 0; attempt < 40; attempt++) {
        if (window.__browserosBfcCancelScan === ticket) return;
        ready = (location.hostname === 'www.bilibili.com' || location.hostname === 'space.bilibili.com') &&
          document.readyState !== 'loading' && Boolean(document.body);
        if (ready) {
          const header = document.querySelector('#bili-header-container, .bili-header__bar, header, [class*="bili-header"]');
          const text = String(document.title || '') + '\n' + String(header?.innerText || '').slice(0, 5000);
          const body = String(document.body?.innerText || '').slice(0, 5000);
          if (blockedPattern.test(text + '\n' + body)) { login_status = 'access_limited'; break; }
          // Only a signed-in header avatar or a narrow public loginInfo field
          // identifies the logged-in account. A visited profile UID is NOT
          // evidence of the current session's identity.
          const anchors = Array.from(header?.querySelectorAll('a[href*="space.bilibili.com"]') || []);
          const avatar = anchors.find((node) => node.querySelector('img') ||
            /avatar|user-face|header-entry/i.test(String(node.className || '')));
          const href = String(avatar?.getAttribute('href') || '');
          mid = href.match(/space\.bilibili\.com\/(\d{1,20})(?:[/?#]|$)/)?.[1] || null;
          const publicMid = window.__INITIAL_STATE__?.loginInfo?.mid;
          if (!mid && /^\d{1,20}$/.test(String(publicMid ?? '')))
            mid = String(publicMid);
          const signedInPublicInfo = window.__BILI_USER_INFO__;
          if (!mid && signedInPublicInfo?.isLogin === true &&
              /^\d{1,20}$/.test(String(signedInPublicInfo.mid ?? '')))
            mid = String(signedInPublicInfo.mid);
          if (mid) { login_status = 'logged_in'; break; }
          // Bilibili can render the signed-out action as a styled div/span
          // rather than an anchor or button. Treat ONLY a visible, exact
          // header login control as an explicit logged-out signal, and still
          // wait for delayed public login MID during the bounded probe.
          if (Array.from(header?.querySelectorAll(
                'a,button,[role="button"],[class*="login"],[class*="Login"]'
              ) || []).some((node) => {
                const rects = node.getClientRects?.();
                if (rects && rects.length === 0) return false;
                return /^(?:登\s*录|立即登录)$/.test(String(node.innerText || '').trim()) ||
                  /passport\.bilibili\.com\/login/i.test(String(node.getAttribute('href') || ''));
              })) loginControlSeen = true;
        }
        await wait(250);
      }
      // A short-lived login placeholder is not proof of logged-out state.
      if (ready && login_status === 'unknown' && loginControlSeen) login_status = 'logged_out';
      await invoke('bfc_readback_submit', {
        ticket, payload: { kind: 'login', login_status, mid }
      });
      if (root) root.dataset.browserosBfcLoginProbe = login_status;
    } catch {
      if (root) root.dataset.browserosBfcError = 'BFC_LOGIN_PROBE_INJECTION_FAILED';
      try {
        await invoke('bfc_readback_submit', {
          ticket, payload: { kind: 'abort', message_code: 'BFC_LOGIN_PROBE_INJECTION_FAILED' }
        });
      } catch { /* The trusted main window will detect missing acknowledgement. */ }
    }
  })();`;
}

export function buildFavoritesReadbackScript(ticket, mid) {
  const encodedTicket = JSON.stringify(String(ticket));
  const encodedMid = JSON.stringify(String(mid));
  return `(async () => {
    ${invokeLine}
    const ticket = ${encodedTicket};
    const expectedMid = ${encodedMid};
    const isCancelled = () => window.__browserosBfcCancelScan === ticket;
    const blockedPattern = /(?:^|\\b)412(?:\\b|$)|访问被拒绝|请求被拦截|安全验证|人机验证|访问过于频繁|操作频繁|风控验证/i;
    const url = () => new URL(location.href);
    const abort = async (message_code) => {
      try { await invoke('bfc_readback_submit', { ticket, payload: { kind: 'abort', message_code } }); }
      catch (error) { document.documentElement.dataset.browserosBfcError = 'BFC_BROWSER_SCRIPT_ERROR'; }
    };
    const observedPageNumber = () => {
      const u = url();
      const fromUrl = Number(u.searchParams.get('pn') || u.searchParams.get('page') || 0);
      if (Number.isInteger(fromUrl) && fromUrl > 0) return fromUrl;
      const pager = Array.from(document.querySelectorAll('[class*="pager"],[class*="pagination"],[class*="pagenation"]'))
        .find((node) => node.getClientRects().length);
      const active = pager && pager.querySelector('[aria-current="page"],[class*="active"],[class*="current"]');
      const activeNumber = Number(String(active?.innerText || '').trim());
      return Number.isInteger(activeNumber) && activeNumber > 0 ? activeNumber : null;
    };
    const pageNumber = () => observedPageNumber() || 1;
    const pageControls = () => {
      const roots = Array.from(document.querySelectorAll('[class*="pager"],[class*="pagination"],[class*="pagenation"]'))
        .filter((node) => node.getClientRects().length);
      const root = roots.sort((a,b) => b.querySelectorAll('a,button').length - a.querySelectorAll('a,button').length)[0] || null;
      const numeric = root ? Array.from(root.querySelectorAll('a,button,[role="button"]'))
        .map((node) => ({node, value:Number(String(node.innerText || '').trim())}))
        .filter((item) => Number.isInteger(item.value) && item.value > 0) : [];
      const next = root && Array.from(root.querySelectorAll('a,button,[role="button"]'))
        .find((node) => /下一页|next/i.test(String(node.innerText || '') + ' ' + String(node.getAttribute('aria-label') || '') + ' ' + String(node.className || '')));
      return {root, numeric, next};
    };
    const reportedTotal = () => {
      // Prefer the visibly selected folder; body-wide counts can refer to
      // unrelated videos, views or other collections.
      const selected = Array.from(document.querySelectorAll(
        '.fav-item.active,.fav-item.cur,.favlist-item.active,.favlist-item--active,' +
        '[class*="fav-item"][class*="selected"],[class*="fav-item"][class*="active"]'
      ));
      for (const folder of selected) {
        if (!folder.getClientRects().length) continue;
        const match = String(folder.innerText || '').trim().match(/(?:收藏夹|收藏|\\n)\\s*[（(]?\\s*([\\d,]{1,8})\\s*(?:条|个|视频)?\\s*[）)]?\\s*$/);
        if (match) {
          const value = Number(match[1].replaceAll(',', ''));
          if (Number.isSafeInteger(value) && value >= 0) return value;
        }
      }
      for (const selector of ['.fav-count', '.fav-info .count', '.favlist-info .count', '.favlist-title .count', '[class*="fav-count"]']) {
        const node = document.querySelector(selector);
        if (!node || !node.getClientRects().length) continue;
        const match = String(node.innerText || '').trim().match(/^\\s*([\\d,]{1,8})\\s*(?:条|个|视频)?\\s*$/);
        if (match) {
          const value = Number(match[1].replaceAll(',', ''));
          if (Number.isSafeInteger(value) && value >= 0) return value;
        }
      }
      // Read only the current folder's detail header. Other sidebar folders,
      // recommendations and account-wide totals are not collection evidence.
      const headers = Array.from(document.querySelectorAll('.fav-info,.favlist-info,.favlist-title,.fav-header-info'))
        .filter((node) => node.getClientRects().length);
      const patterns = [
        /视频数\\s*[:：]\\s*([\\d,]+)/,
        /共\\s*([\\d,]+)\\s*(?:个视频|条视频|个收藏|条收藏)/,
        /总计\\s*([\\d,]+)\\s*(?:个视频|条视频|个收藏|条收藏)/,
        /([\\d,]+)\\s*个视频/
      ];
      const values = new Set();
      for (const header of headers) {
        const text = String(header.innerText || '').slice(0, 2000);
        for (const pattern of patterns) {
          const match = text.match(pattern);
          if (match) {
            const value = Number(match[1].replaceAll(',', ''));
            if (Number.isSafeInteger(value) && value >= 0) values.add(value);
          }
        }
      }
      return values.size === 1 ? [...values][0] : null;
    };
    const findFavoritesContainer = () => {
      const selectors = [
        '#fav-video-list','.fav-video-list','.fav-list','.fav-list-container',
        '.favlist-content','.fav-content','.fav-video-list-container',
        '[class*="fav-video-list"]','[class*="fav-list"]','[id*="fav-list"]'
      ];
      const candidates = selectors.flatMap((selector) => Array.from(document.querySelectorAll(selector)));
      return candidates
        .filter((node) => node.getClientRects().length && node.querySelectorAll('a[href*="/video/BV"]').length > 0)
        .sort((a,b) => b.querySelectorAll('a[href*="/video/BV"]').length - a.querySelectorAll('a[href*="/video/BV"]').length)[0] || null;
    };
    const extractItems = (container) => {
      const items = [];
      const seen = new Set();
      const isMetric = (value) => {
        const text = value.replaceAll(',', '').trim();
        // Character classes avoid losing regex escapes in the injected template.
        const quantity = '[0-9]+(?:[.][0-9]+)?[ ]*[万亿千百]?';
        const duration = '[0-9]{1,2}(?::[0-9]{2}){1,2}';
        return new RegExp('^' + quantity + '(?:播放|次|观看)?$').test(text)
          || new RegExp('^' + quantity + '[ ]+' + duration + '$').test(text)
          || new RegExp('^' + duration + '$').test(text)
          || new RegExp('^' + quantity + '(?:[ ]+[0-9]+)?[ ]+' + duration + '$').test(text)
          || /^(?:播放|收藏|评论|弹幕|点赞)(?:数)?[ :：]*[0-9]+(?:[.][0-9]+)?[万亿千百]?$/.test(text);
      };
      const cleanTitle = (value) => {
        const text = String(value || '').replace(/[\\s]+/g, ' ').trim();
        if (!text || isMetric(text) || /^BV[0-9A-Za-z]{10}$/.test(text)) return '';
        return text.slice(0, 1000);
      };
      for (const anchor of container.querySelectorAll('a[href*="/video/BV"]')) {
        const href = String(anchor.getAttribute('href') || '');
        const bvid = href.match(/\\/video\\/(BV[0-9A-Za-z]{10})/)?.[1];
        if (!bvid || seen.has(bvid)) continue;
        const card = anchor.closest('[class*="fav"],[class*="video-card"],li') || anchor.parentElement || anchor;
        const titleNodes = [
          ...Array.from(card.querySelectorAll?.('[class*="title"],[class*="name"]') || []),
          anchor,
        ];
        const textLines = String(card.innerText || anchor.innerText || '').split('\\n').map(cleanTitle);
        const title = [
          ...titleNodes.flatMap((node) => [node.getAttribute?.('title'), node.getAttribute?.('aria-label'), node.innerText]),
          ...textLines,
        ].map(cleanTitle).find(Boolean) || '标题未获取';
        seen.add(bvid);
        items.push({bvid, title});
      }
      return items;
    };
    const isBlocked = () => blockedPattern.test(String(document.title || '') + '\\n' + String(document.body?.innerText || '').slice(0, 5000));
    let paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
    const waitForPageChange = async (beforeSignature, expectedCount, targetPage) => {
      let changedAt = 0;
      let lastSignature = '';
      const startedAt = Date.now();
      let diagnostic = null;
      paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
      delete document.documentElement.dataset.browserosBfcPaginationDiagnostic;
      for (let attempt = 0; attempt < 80; attempt++) {
        await new Promise((resolve) => setTimeout(resolve, 250));
        if (isCancelled() || isBlocked()) return false;
        const container = findFavoritesContainer();
        const items = container ? extractItems(container) : [];
        const signature = items.map((item) => item.bvid).join('|');
        const observedPage = observedPageNumber();
        const links = container ? Array.from(container.querySelectorAll('a[href*="/video/BV"]')) : [];
        // 只保留计数与页码，不记录网页正文、账号、URL、BV 或凭据。
        diagnostic = { targetPage, observedPage, expectedItems: expectedCount ?? null,
          uniqueItems: items.length, videoLinks: links.length,
          visibleVideoLinks: links.filter((node) => node.getClientRects?.().length > 0).length,
          containerFound: Boolean(container), contentChanged: Boolean(signature && signature !== beforeSignature),
          waitedMs: Date.now() - startedAt };
        if (signature && signature !== beforeSignature &&
            (observedPage === null || observedPage === targetPage) &&
            (expectedCount === undefined || items.length <= expectedCount)) {
          if (!changedAt || signature !== lastSignature) changedAt = Date.now();
          lastSignature = signature;
          if (Date.now() - changedAt >= (expectedCount !== undefined && items.length < expectedCount ? 3000 : 1200)) return true;
        } else {
          changedAt = 0;
        }
      }
      if (diagnostic) {
        document.documentElement.dataset.browserosBfcPaginationDiagnostic = JSON.stringify(diagnostic);
        paginationFailureCode = !diagnostic.containerFound ? 'BFC_FAVORITES_CONTAINER_UNRECOGNIZED'
          : !diagnostic.contentChanged || (diagnostic.observedPage !== null && diagnostic.observedPage !== targetPage)
            ? 'BFC_PAGINATION_NOT_ADVANCED'
            : expectedCount !== undefined && diagnostic.uniqueItems !== expectedCount
              ? 'BFC_PAGINATION_ITEM_COUNT_MISMATCH' : 'BFC_PAGINATION_UNSTABLE';
      }
      return false;
    };
    try {
      const currentUrl = url();
      const pathSegments = currentUrl.pathname.split('/').filter(Boolean);
      if (currentUrl.hostname !== 'space.bilibili.com' ||
          pathSegments[0] !== expectedMid || pathSegments[1] !== 'favlist') {
        await abort('BFC_FAVORITES_PAGE_REQUIRED');
        return;
      }
      if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
      // Native-only acknowledgement: eval dispatch alone does not mean the
      // Bilibili child WebView executed or had a working scoped Tauri bridge.
      await invoke('bfc_readback_submit', { ticket, payload: { kind: 'favorites_start' } });
      let container = null;
      for (let attempt = 0; attempt < 60; attempt++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        container = findFavoritesContainer();
        if (container) break;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }

      const total = reportedTotal();
      if (total === null || !Number.isSafeInteger(total) || total < 0 || total > 5000) {
        await abort('BFC_TOTAL_UNKNOWN');
        return;
      }
      const controls = pageControls();
      let current = pageNumber();
      if (observedPageNumber() === null && total !== 0) { await abort('BFC_START_FROM_FIRST_PAGE'); return; }
      if (current !== 1) {
        const first = controls.numeric.find((item) => item.value === 1)?.node;
        if (!first) { await abort('BFC_START_FROM_FIRST_PAGE'); return; }
        const beforeSignature = extractItems(container).map((item) => item.bvid).join('|');
        first.click();
        if (!await waitForPageChange(beforeSignature, undefined, 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
      }

      const firstItems = extractItems(container);
      const pageSize = firstItems.length;
      if (total > 0 && pageSize === 0) { await abort('BFC_FAVORITES_PAGE_EMPTY'); return; }
      const pagerText = String(pageControls().root?.innerText || '');
      const declaredPages = Number(pagerText.match(/共\\s*([0-9]+)\\s*页/)?.[1] || 0);
      const expected = declaredPages || (total === 0 ? 1 : Math.ceil(total / pageSize));
      if (expected < 1 || expected > 300) { await abort('BFC_PAGINATION_LIMIT'); return; }
      const folderId = url().searchParams.get('fid') || 'unknown';
      const titleNode = document.querySelector('[class*="fav"][class*="title"],[class*="fav-title"]');
      const folderName = String(titleNode?.innerText || document.title || '当前收藏夹').trim().slice(0,128);
      const pages = [];
      const seenAcrossPages = new Set();
      const submitPages = (kind) => invoke('bfc_readback_submit', { ticket, payload: {
        kind, expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName, pages
      }});
      let itemsCaptured = 0;
      for (let page = 1; page <= expected; page++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        if ((url().searchParams.get('fid') || 'unknown') !== folderId || reportedTotal() !== total) {
          await abort('BFC_COLLECTION_CHANGED'); return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
        const items = page === 1 ? firstItems : extractItems(container);
        const expectedCount = Math.max(0, Math.min(pageSize, total - (page - 1) * pageSize));
        if (items.length > expectedCount) { await abort('BFC_READBACK_PAGE_COUNT_MISMATCH'); return; }
        if (items.some((item) => seenAcrossPages.has(item.bvid))) {
          await abort('BFC_READBACK_CROSS_PAGE_DUPLICATE'); return;
        }
        items.forEach((item) => seenAcrossPages.add(item.bvid));
        pages.push({page, items});
        itemsCaptured += items.length;
        await invoke('bfc_readback_submit', {
          ticket,
          payload: { kind: 'progress', pages_captured: page, items_captured: itemsCaptured,
            expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName }
        });
        await submitPages('checkpoint');
        // 数量缺口或单轮三页预算触发停止，保留部分数据，不自动扫描全部收藏。
        if (items.length !== expectedCount || (page >= 3 && page < expected)) {
          await submitPages('partial'); return;
        }
        if (page === expected) break;
        const pager = pageControls();
        if (!pager.next || pager.next.hasAttribute('disabled') ||
            pager.next.getAttribute('aria-disabled') === 'true' ||
            /disabled/i.test(String(pager.next.className || ''))) {
          await abort('BFC_PAGINATION_INCOMPLETE');
          return;
        }
        const beforeSignature = items.map((item) => item.bvid).join('|');
        pager.next.click();
        const nextExpectedCount = Math.min(pageSize, total - page * pageSize);
        if (!await waitForPageChange(beforeSignature, nextExpectedCount, page + 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        const explicitPage = Number(url().searchParams.get('pn') || url().searchParams.get('page') || 0);
        if (explicitPage > 0 && explicitPage !== page + 1) {
          await abort('BFC_PAGINATION_ORDER_INVALID');
          return;
        }
      }
      const captured = pages.reduce((sum,page) => sum + page.items.length, 0);
      if (captured !== total) {
        await abort('BFC_READBACK_TOTAL_MISMATCH');
        return;
      }
      await invoke('bfc_readback_submit', {
        ticket,
        payload: {
          kind: 'favorites',
          expected_pages: expected,
          reported_total: total,
          folder_id: folderId,
          folder_name: folderName,
          pages
        }
      });
      document.documentElement.dataset.browserosBfcReadback = 'ready';
    } catch (error) {
      try { await abort('BFC_BROWSER_SCRIPT_ERROR'); } catch {}
      document.documentElement.dataset.browserosBfcReadback = 'failed';
      document.documentElement.dataset.browserosBfcError = 'BFC_BROWSER_SCRIPT_ERROR';
    }
  })();`;
}
).test(text)
          || new RegExp('^(?:' + count + ')(?:播放|次|观看)?(?:[ \\t]+' + duration + ')
      };
      const cleanTitle = (value) => {
        const text = String(value || '').replace(/\s+/g, ' ').trim();
        if (!text || isMetric(text) || /^BV[0-9A-Za-z]{10}$/.test(text)) return '';
        return text.slice(0, 1000);
      };
      for (const anchor of container.querySelectorAll('a[href*="/video/BV"]')) {
        const href = String(anchor.getAttribute('href') || '');
        const bvid = href.match(/\\/video\\/(BV[0-9A-Za-z]{10})/)?.[1];
        if (!bvid || seen.has(bvid)) continue;
        const card = anchor.closest('[class*="fav"],[class*="video-card"],li') || anchor.parentElement || anchor;
        const titleNodes = [
          ...Array.from(card.querySelectorAll?.('[class*="title"],[class*="name"]') || []),
          anchor,
        ];
        const textLines = String(card.innerText || anchor.innerText || '').split('\\n').map(cleanTitle);
        const title = [
          ...titleNodes.flatMap((node) => [node.getAttribute?.('title'), node.getAttribute?.('aria-label'), node.innerText]),
          ...textLines,
        ].map(cleanTitle).find(Boolean) || '标题未获取';
        seen.add(bvid);
        items.push({bvid, title});
      }
      return items;
    };
    const isBlocked = () => blockedPattern.test(String(document.title || '') + '\\n' + String(document.body?.innerText || '').slice(0, 5000));
    let paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
    const waitForPageChange = async (beforeSignature, expectedCount, targetPage) => {
      let changedAt = 0;
      let lastSignature = '';
      const startedAt = Date.now();
      let diagnostic = null;
      paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
      delete document.documentElement.dataset.browserosBfcPaginationDiagnostic;
      for (let attempt = 0; attempt < 80; attempt++) {
        await new Promise((resolve) => setTimeout(resolve, 250));
        if (isCancelled() || isBlocked()) return false;
        const container = findFavoritesContainer();
        const items = container ? extractItems(container) : [];
        const signature = items.map((item) => item.bvid).join('|');
        const observedPage = observedPageNumber();
        const links = container ? Array.from(container.querySelectorAll('a[href*="/video/BV"]')) : [];
        // 只保留计数与页码，不记录网页正文、账号、URL、BV 或凭据。
        diagnostic = { targetPage, observedPage, expectedItems: expectedCount ?? null,
          uniqueItems: items.length, videoLinks: links.length,
          visibleVideoLinks: links.filter((node) => node.getClientRects?.().length > 0).length,
          containerFound: Boolean(container), contentChanged: Boolean(signature && signature !== beforeSignature),
          waitedMs: Date.now() - startedAt };
        if (signature && signature !== beforeSignature &&
            (observedPage === null || observedPage === targetPage) &&
            (expectedCount === undefined || items.length <= expectedCount)) {
          if (!changedAt || signature !== lastSignature) changedAt = Date.now();
          lastSignature = signature;
          if (Date.now() - changedAt >= (expectedCount !== undefined && items.length < expectedCount ? 3000 : 1200)) return true;
        } else {
          changedAt = 0;
        }
      }
      if (diagnostic) {
        document.documentElement.dataset.browserosBfcPaginationDiagnostic = JSON.stringify(diagnostic);
        paginationFailureCode = !diagnostic.containerFound ? 'BFC_FAVORITES_CONTAINER_UNRECOGNIZED'
          : !diagnostic.contentChanged || (diagnostic.observedPage !== null && diagnostic.observedPage !== targetPage)
            ? 'BFC_PAGINATION_NOT_ADVANCED'
            : expectedCount !== undefined && diagnostic.uniqueItems !== expectedCount
              ? 'BFC_PAGINATION_ITEM_COUNT_MISMATCH' : 'BFC_PAGINATION_UNSTABLE';
      }
      return false;
    };
    try {
      const currentUrl = url();
      const pathSegments = currentUrl.pathname.split('/').filter(Boolean);
      if (currentUrl.hostname !== 'space.bilibili.com' ||
          pathSegments[0] !== expectedMid || pathSegments[1] !== 'favlist') {
        await abort('BFC_FAVORITES_PAGE_REQUIRED');
        return;
      }
      if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
      // Native-only acknowledgement: eval dispatch alone does not mean the
      // Bilibili child WebView executed or had a working scoped Tauri bridge.
      await invoke('bfc_readback_submit', { ticket, payload: { kind: 'favorites_start' } });
      let container = null;
      for (let attempt = 0; attempt < 60; attempt++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        container = findFavoritesContainer();
        if (container) break;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }

      const total = reportedTotal();
      if (total === null || !Number.isSafeInteger(total) || total < 0 || total > 5000) {
        await abort('BFC_TOTAL_UNKNOWN');
        return;
      }
      const controls = pageControls();
      let current = pageNumber();
      if (current !== 1) {
        const first = controls.numeric.find((item) => item.value === 1)?.node;
        if (!first) { await abort('BFC_START_FROM_FIRST_PAGE'); return; }
        const beforeSignature = extractItems(container).map((item) => item.bvid).join('|');
        first.click();
        if (!await waitForPageChange(beforeSignature, undefined, 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
      }

      const firstItems = extractItems(container);
      const pageSize = firstItems.length;
      if (total > 0 && pageSize === 0) { await abort('BFC_FAVORITES_PAGE_EMPTY'); return; }
      const pagerText = String(pageControls().root?.innerText || '');
      const declaredPages = Number(pagerText.match(/共\\s*([0-9]+)\\s*页/)?.[1] || 0);
      const expected = declaredPages || (total === 0 ? 1 : Math.ceil(total / pageSize));
      if (expected < 1 || expected > 300) { await abort('BFC_PAGINATION_LIMIT'); return; }
      const folderId = url().searchParams.get('fid') || 'unknown';
      const titleNode = document.querySelector('[class*="fav"][class*="title"],[class*="fav-title"]');
      const folderName = String(titleNode?.innerText || document.title || '当前收藏夹').trim().slice(0,128);
      const pages = [];
      const seenAcrossPages = new Set();
      const submitPages = (kind) => invoke('bfc_readback_submit', { ticket, payload: {
        kind, expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName, pages
      }});
      let itemsCaptured = 0;
      for (let page = 1; page <= expected; page++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        if ((url().searchParams.get('fid') || 'unknown') !== folderId || reportedTotal() !== total) {
          await abort('BFC_COLLECTION_CHANGED'); return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
        const items = page === 1 ? firstItems : extractItems(container);
        const expectedCount = Math.max(0, Math.min(pageSize, total - (page - 1) * pageSize));
        if (items.length > expectedCount) { await abort('BFC_READBACK_PAGE_COUNT_MISMATCH'); return; }
        if (items.some((item) => seenAcrossPages.has(item.bvid))) {
          await abort('BFC_READBACK_CROSS_PAGE_DUPLICATE'); return;
        }
        items.forEach((item) => seenAcrossPages.add(item.bvid));
        pages.push({page, items});
        itemsCaptured += items.length;
        await invoke('bfc_readback_submit', {
          ticket,
          payload: { kind: 'progress', pages_captured: page, items_captured: itemsCaptured,
            expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName }
        });
        await submitPages('checkpoint');
        // 数量缺口或单轮三页预算触发停止，保留部分数据，不自动扫描全部收藏。
        if (items.length !== expectedCount || (page >= 3 && page < expected)) {
          await submitPages('partial'); return;
        }
        if (page === expected) break;
        const pager = pageControls();
        if (!pager.next || pager.next.hasAttribute('disabled') ||
            pager.next.getAttribute('aria-disabled') === 'true' ||
            /disabled/i.test(String(pager.next.className || ''))) {
          await abort('BFC_PAGINATION_INCOMPLETE');
          return;
        }
        const beforeSignature = items.map((item) => item.bvid).join('|');
        pager.next.click();
        const nextExpectedCount = Math.min(pageSize, total - page * pageSize);
        if (!await waitForPageChange(beforeSignature, nextExpectedCount, page + 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        const explicitPage = Number(url().searchParams.get('pn') || url().searchParams.get('page') || 0);
        if (explicitPage > 0 && explicitPage !== page + 1) {
          await abort('BFC_PAGINATION_ORDER_INVALID');
          return;
        }
      }
      const captured = pages.reduce((sum,page) => sum + page.items.length, 0);
      if (captured !== total) {
        await abort('BFC_READBACK_TOTAL_MISMATCH');
        return;
      }
      await invoke('bfc_readback_submit', {
        ticket,
        payload: {
          kind: 'favorites',
          expected_pages: expected,
          reported_total: total,
          folder_id: folderId,
          folder_name: folderName,
          pages
        }
      });
      document.documentElement.dataset.browserosBfcReadback = 'ready';
    } catch (error) {
      try { await abort('BFC_BROWSER_SCRIPT_ERROR'); } catch {}
      document.documentElement.dataset.browserosBfcReadback = 'failed';
      document.documentElement.dataset.browserosBfcError = 'BFC_BROWSER_SCRIPT_ERROR';
    }
  })();`;
}
).test(text)
          || new RegExp('^' + duration + '
      };
      const cleanTitle = (value) => {
        const text = String(value || '').replace(/\s+/g, ' ').trim();
        if (!text || isMetric(text) || /^BV[0-9A-Za-z]{10}$/.test(text)) return '';
        return text.slice(0, 1000);
      };
      for (const anchor of container.querySelectorAll('a[href*="/video/BV"]')) {
        const href = String(anchor.getAttribute('href') || '');
        const bvid = href.match(/\\/video\\/(BV[0-9A-Za-z]{10})/)?.[1];
        if (!bvid || seen.has(bvid)) continue;
        const card = anchor.closest('[class*="fav"],[class*="video-card"],li') || anchor.parentElement || anchor;
        const titleNodes = [
          ...Array.from(card.querySelectorAll?.('[class*="title"],[class*="name"]') || []),
          anchor,
        ];
        const textLines = String(card.innerText || anchor.innerText || '').split('\\n').map(cleanTitle);
        const title = [
          ...titleNodes.flatMap((node) => [node.getAttribute?.('title'), node.getAttribute?.('aria-label'), node.innerText]),
          ...textLines,
        ].map(cleanTitle).find(Boolean) || '标题未获取';
        seen.add(bvid);
        items.push({bvid, title});
      }
      return items;
    };
    const isBlocked = () => blockedPattern.test(String(document.title || '') + '\\n' + String(document.body?.innerText || '').slice(0, 5000));
    let paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
    const waitForPageChange = async (beforeSignature, expectedCount, targetPage) => {
      let changedAt = 0;
      let lastSignature = '';
      const startedAt = Date.now();
      let diagnostic = null;
      paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
      delete document.documentElement.dataset.browserosBfcPaginationDiagnostic;
      for (let attempt = 0; attempt < 80; attempt++) {
        await new Promise((resolve) => setTimeout(resolve, 250));
        if (isCancelled() || isBlocked()) return false;
        const container = findFavoritesContainer();
        const items = container ? extractItems(container) : [];
        const signature = items.map((item) => item.bvid).join('|');
        const observedPage = observedPageNumber();
        const links = container ? Array.from(container.querySelectorAll('a[href*="/video/BV"]')) : [];
        // 只保留计数与页码，不记录网页正文、账号、URL、BV 或凭据。
        diagnostic = { targetPage, observedPage, expectedItems: expectedCount ?? null,
          uniqueItems: items.length, videoLinks: links.length,
          visibleVideoLinks: links.filter((node) => node.getClientRects?.().length > 0).length,
          containerFound: Boolean(container), contentChanged: Boolean(signature && signature !== beforeSignature),
          waitedMs: Date.now() - startedAt };
        if (signature && signature !== beforeSignature &&
            (observedPage === null || observedPage === targetPage) &&
            (expectedCount === undefined || items.length <= expectedCount)) {
          if (!changedAt || signature !== lastSignature) changedAt = Date.now();
          lastSignature = signature;
          if (Date.now() - changedAt >= (expectedCount !== undefined && items.length < expectedCount ? 3000 : 1200)) return true;
        } else {
          changedAt = 0;
        }
      }
      if (diagnostic) {
        document.documentElement.dataset.browserosBfcPaginationDiagnostic = JSON.stringify(diagnostic);
        paginationFailureCode = !diagnostic.containerFound ? 'BFC_FAVORITES_CONTAINER_UNRECOGNIZED'
          : !diagnostic.contentChanged || (diagnostic.observedPage !== null && diagnostic.observedPage !== targetPage)
            ? 'BFC_PAGINATION_NOT_ADVANCED'
            : expectedCount !== undefined && diagnostic.uniqueItems !== expectedCount
              ? 'BFC_PAGINATION_ITEM_COUNT_MISMATCH' : 'BFC_PAGINATION_UNSTABLE';
      }
      return false;
    };
    try {
      const currentUrl = url();
      const pathSegments = currentUrl.pathname.split('/').filter(Boolean);
      if (currentUrl.hostname !== 'space.bilibili.com' ||
          pathSegments[0] !== expectedMid || pathSegments[1] !== 'favlist') {
        await abort('BFC_FAVORITES_PAGE_REQUIRED');
        return;
      }
      if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
      // Native-only acknowledgement: eval dispatch alone does not mean the
      // Bilibili child WebView executed or had a working scoped Tauri bridge.
      await invoke('bfc_readback_submit', { ticket, payload: { kind: 'favorites_start' } });
      let container = null;
      for (let attempt = 0; attempt < 60; attempt++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        container = findFavoritesContainer();
        if (container) break;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }

      const total = reportedTotal();
      if (total === null || !Number.isSafeInteger(total) || total < 0 || total > 5000) {
        await abort('BFC_TOTAL_UNKNOWN');
        return;
      }
      const controls = pageControls();
      let current = pageNumber();
      if (current !== 1) {
        const first = controls.numeric.find((item) => item.value === 1)?.node;
        if (!first) { await abort('BFC_START_FROM_FIRST_PAGE'); return; }
        const beforeSignature = extractItems(container).map((item) => item.bvid).join('|');
        first.click();
        if (!await waitForPageChange(beforeSignature, undefined, 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
      }

      const firstItems = extractItems(container);
      const pageSize = firstItems.length;
      if (total > 0 && pageSize === 0) { await abort('BFC_FAVORITES_PAGE_EMPTY'); return; }
      const pagerText = String(pageControls().root?.innerText || '');
      const declaredPages = Number(pagerText.match(/共\\s*([0-9]+)\\s*页/)?.[1] || 0);
      const expected = declaredPages || (total === 0 ? 1 : Math.ceil(total / pageSize));
      if (expected < 1 || expected > 300) { await abort('BFC_PAGINATION_LIMIT'); return; }
      const folderId = url().searchParams.get('fid') || 'unknown';
      const titleNode = document.querySelector('[class*="fav"][class*="title"],[class*="fav-title"]');
      const folderName = String(titleNode?.innerText || document.title || '当前收藏夹').trim().slice(0,128);
      const pages = [];
      const seenAcrossPages = new Set();
      const submitPages = (kind) => invoke('bfc_readback_submit', { ticket, payload: {
        kind, expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName, pages
      }});
      let itemsCaptured = 0;
      for (let page = 1; page <= expected; page++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        if ((url().searchParams.get('fid') || 'unknown') !== folderId || reportedTotal() !== total) {
          await abort('BFC_COLLECTION_CHANGED'); return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
        const items = page === 1 ? firstItems : extractItems(container);
        const expectedCount = Math.max(0, Math.min(pageSize, total - (page - 1) * pageSize));
        if (items.length > expectedCount) { await abort('BFC_READBACK_PAGE_COUNT_MISMATCH'); return; }
        if (items.some((item) => seenAcrossPages.has(item.bvid))) {
          await abort('BFC_READBACK_CROSS_PAGE_DUPLICATE'); return;
        }
        items.forEach((item) => seenAcrossPages.add(item.bvid));
        pages.push({page, items});
        itemsCaptured += items.length;
        await invoke('bfc_readback_submit', {
          ticket,
          payload: { kind: 'progress', pages_captured: page, items_captured: itemsCaptured,
            expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName }
        });
        await submitPages('checkpoint');
        // 数量缺口或单轮三页预算触发停止，保留部分数据，不自动扫描全部收藏。
        if (items.length !== expectedCount || (page >= 3 && page < expected)) {
          await submitPages('partial'); return;
        }
        if (page === expected) break;
        const pager = pageControls();
        if (!pager.next || pager.next.hasAttribute('disabled') ||
            pager.next.getAttribute('aria-disabled') === 'true' ||
            /disabled/i.test(String(pager.next.className || ''))) {
          await abort('BFC_PAGINATION_INCOMPLETE');
          return;
        }
        const beforeSignature = items.map((item) => item.bvid).join('|');
        pager.next.click();
        const nextExpectedCount = Math.min(pageSize, total - page * pageSize);
        if (!await waitForPageChange(beforeSignature, nextExpectedCount, page + 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        const explicitPage = Number(url().searchParams.get('pn') || url().searchParams.get('page') || 0);
        if (explicitPage > 0 && explicitPage !== page + 1) {
          await abort('BFC_PAGINATION_ORDER_INVALID');
          return;
        }
      }
      const captured = pages.reduce((sum,page) => sum + page.items.length, 0);
      if (captured !== total) {
        await abort('BFC_READBACK_TOTAL_MISMATCH');
        return;
      }
      await invoke('bfc_readback_submit', {
        ticket,
        payload: {
          kind: 'favorites',
          expected_pages: expected,
          reported_total: total,
          folder_id: folderId,
          folder_name: folderName,
          pages
        }
      });
      document.documentElement.dataset.browserosBfcReadback = 'ready';
    } catch (error) {
      try { await abort('BFC_BROWSER_SCRIPT_ERROR'); } catch {}
      document.documentElement.dataset.browserosBfcReadback = 'failed';
      document.documentElement.dataset.browserosBfcError = 'BFC_BROWSER_SCRIPT_ERROR';
    }
  })();`;
}
).test(text)
          || /^(?:播放|收藏|评论|弹幕|点赞)(?:数)?[ :：]*[0-9]+(?:[.][0-9]+)?[万亿千百]?$/.test(text);
      };
      const cleanTitle = (value) => {
        const text = String(value || '').replace(/\s+/g, ' ').trim();
        if (!text || isMetric(text) || /^BV[0-9A-Za-z]{10}$/.test(text)) return '';
        return text.slice(0, 1000);
      };
      for (const anchor of container.querySelectorAll('a[href*="/video/BV"]')) {
        const href = String(anchor.getAttribute('href') || '');
        const bvid = href.match(/\\/video\\/(BV[0-9A-Za-z]{10})/)?.[1];
        if (!bvid || seen.has(bvid)) continue;
        const card = anchor.closest('[class*="fav"],[class*="video-card"],li') || anchor.parentElement || anchor;
        const titleNodes = [
          ...Array.from(card.querySelectorAll?.('[class*="title"],[class*="name"]') || []),
          anchor,
        ];
        const textLines = String(card.innerText || anchor.innerText || '').split('\\n').map(cleanTitle);
        const title = [
          ...titleNodes.flatMap((node) => [node.getAttribute?.('title'), node.getAttribute?.('aria-label'), node.innerText]),
          ...textLines,
        ].map(cleanTitle).find(Boolean) || '标题未获取';
        seen.add(bvid);
        items.push({bvid, title});
      }
      return items;
    };
    const isBlocked = () => blockedPattern.test(String(document.title || '') + '\\n' + String(document.body?.innerText || '').slice(0, 5000));
    let paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
    const waitForPageChange = async (beforeSignature, expectedCount, targetPage) => {
      let changedAt = 0;
      let lastSignature = '';
      const startedAt = Date.now();
      let diagnostic = null;
      paginationFailureCode = 'BFC_PAGINATION_TIMEOUT';
      delete document.documentElement.dataset.browserosBfcPaginationDiagnostic;
      for (let attempt = 0; attempt < 80; attempt++) {
        await new Promise((resolve) => setTimeout(resolve, 250));
        if (isCancelled() || isBlocked()) return false;
        const container = findFavoritesContainer();
        const items = container ? extractItems(container) : [];
        const signature = items.map((item) => item.bvid).join('|');
        const observedPage = observedPageNumber();
        const links = container ? Array.from(container.querySelectorAll('a[href*="/video/BV"]')) : [];
        // 只保留计数与页码，不记录网页正文、账号、URL、BV 或凭据。
        diagnostic = { targetPage, observedPage, expectedItems: expectedCount ?? null,
          uniqueItems: items.length, videoLinks: links.length,
          visibleVideoLinks: links.filter((node) => node.getClientRects?.().length > 0).length,
          containerFound: Boolean(container), contentChanged: Boolean(signature && signature !== beforeSignature),
          waitedMs: Date.now() - startedAt };
        if (signature && signature !== beforeSignature &&
            (observedPage === null || observedPage === targetPage) &&
            (expectedCount === undefined || items.length <= expectedCount)) {
          if (!changedAt || signature !== lastSignature) changedAt = Date.now();
          lastSignature = signature;
          if (Date.now() - changedAt >= (expectedCount !== undefined && items.length < expectedCount ? 3000 : 1200)) return true;
        } else {
          changedAt = 0;
        }
      }
      if (diagnostic) {
        document.documentElement.dataset.browserosBfcPaginationDiagnostic = JSON.stringify(diagnostic);
        paginationFailureCode = !diagnostic.containerFound ? 'BFC_FAVORITES_CONTAINER_UNRECOGNIZED'
          : !diagnostic.contentChanged || (diagnostic.observedPage !== null && diagnostic.observedPage !== targetPage)
            ? 'BFC_PAGINATION_NOT_ADVANCED'
            : expectedCount !== undefined && diagnostic.uniqueItems !== expectedCount
              ? 'BFC_PAGINATION_ITEM_COUNT_MISMATCH' : 'BFC_PAGINATION_UNSTABLE';
      }
      return false;
    };
    try {
      const currentUrl = url();
      const pathSegments = currentUrl.pathname.split('/').filter(Boolean);
      if (currentUrl.hostname !== 'space.bilibili.com' ||
          pathSegments[0] !== expectedMid || pathSegments[1] !== 'favlist') {
        await abort('BFC_FAVORITES_PAGE_REQUIRED');
        return;
      }
      if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
      // Native-only acknowledgement: eval dispatch alone does not mean the
      // Bilibili child WebView executed or had a working scoped Tauri bridge.
      await invoke('bfc_readback_submit', { ticket, payload: { kind: 'favorites_start' } });
      let container = null;
      for (let attempt = 0; attempt < 60; attempt++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        container = findFavoritesContainer();
        if (container) break;
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }

      const total = reportedTotal();
      if (total === null || !Number.isSafeInteger(total) || total < 0 || total > 5000) {
        await abort('BFC_TOTAL_UNKNOWN');
        return;
      }
      const controls = pageControls();
      let current = pageNumber();
      if (current !== 1) {
        const first = controls.numeric.find((item) => item.value === 1)?.node;
        if (!first) { await abort('BFC_START_FROM_FIRST_PAGE'); return; }
        const beforeSignature = extractItems(container).map((item) => item.bvid).join('|');
        first.click();
        if (!await waitForPageChange(beforeSignature, undefined, 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
      }

      const firstItems = extractItems(container);
      const pageSize = firstItems.length;
      if (total > 0 && pageSize === 0) { await abort('BFC_FAVORITES_PAGE_EMPTY'); return; }
      const pagerText = String(pageControls().root?.innerText || '');
      const declaredPages = Number(pagerText.match(/共\\s*([0-9]+)\\s*页/)?.[1] || 0);
      const expected = declaredPages || (total === 0 ? 1 : Math.ceil(total / pageSize));
      if (expected < 1 || expected > 300) { await abort('BFC_PAGINATION_LIMIT'); return; }
      const folderId = url().searchParams.get('fid') || 'unknown';
      const titleNode = document.querySelector('[class*="fav"][class*="title"],[class*="fav-title"]');
      const folderName = String(titleNode?.innerText || document.title || '当前收藏夹').trim().slice(0,128);
      const pages = [];
      const seenAcrossPages = new Set();
      const submitPages = (kind) => invoke('bfc_readback_submit', { ticket, payload: {
        kind, expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName, pages
      }});
      let itemsCaptured = 0;
      for (let page = 1; page <= expected; page++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        if ((url().searchParams.get('fid') || 'unknown') !== folderId || reportedTotal() !== total) {
          await abort('BFC_COLLECTION_CHANGED'); return;
        }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
        const items = page === 1 ? firstItems : extractItems(container);
        const expectedCount = Math.max(0, Math.min(pageSize, total - (page - 1) * pageSize));
        if (items.length > expectedCount) { await abort('BFC_READBACK_PAGE_COUNT_MISMATCH'); return; }
        if (items.some((item) => seenAcrossPages.has(item.bvid))) {
          await abort('BFC_READBACK_CROSS_PAGE_DUPLICATE'); return;
        }
        items.forEach((item) => seenAcrossPages.add(item.bvid));
        pages.push({page, items});
        itemsCaptured += items.length;
        await invoke('bfc_readback_submit', {
          ticket,
          payload: { kind: 'progress', pages_captured: page, items_captured: itemsCaptured,
            expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName }
        });
        await submitPages('checkpoint');
        // 数量缺口或单轮三页预算触发停止，保留部分数据，不自动扫描全部收藏。
        if (items.length !== expectedCount || (page >= 3 && page < expected)) {
          await submitPages('partial'); return;
        }
        if (page === expected) break;
        const pager = pageControls();
        if (!pager.next || pager.next.hasAttribute('disabled') ||
            pager.next.getAttribute('aria-disabled') === 'true' ||
            /disabled/i.test(String(pager.next.className || ''))) {
          await abort('BFC_PAGINATION_INCOMPLETE');
          return;
        }
        const beforeSignature = items.map((item) => item.bvid).join('|');
        pager.next.click();
        const nextExpectedCount = Math.min(pageSize, total - page * pageSize);
        if (!await waitForPageChange(beforeSignature, nextExpectedCount, page + 1)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : paginationFailureCode);
          return;
        }
        const explicitPage = Number(url().searchParams.get('pn') || url().searchParams.get('page') || 0);
        if (explicitPage > 0 && explicitPage !== page + 1) {
          await abort('BFC_PAGINATION_ORDER_INVALID');
          return;
        }
      }
      const captured = pages.reduce((sum,page) => sum + page.items.length, 0);
      if (captured !== total) {
        await abort('BFC_READBACK_TOTAL_MISMATCH');
        return;
      }
      await invoke('bfc_readback_submit', {
        ticket,
        payload: {
          kind: 'favorites',
          expected_pages: expected,
          reported_total: total,
          folder_id: folderId,
          folder_name: folderName,
          pages
        }
      });
      document.documentElement.dataset.browserosBfcReadback = 'ready';
    } catch (error) {
      try { await abort('BFC_BROWSER_SCRIPT_ERROR'); } catch {}
      document.documentElement.dataset.browserosBfcReadback = 'failed';
      document.documentElement.dataset.browserosBfcError = 'BFC_BROWSER_SCRIPT_ERROR';
    }
  })();`;
}
