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
          if (Array.from(header?.querySelectorAll('a,button,[role="button"]') || [])
            .some((node) => /^(?:登\s*录|立即登录)$/.test(String(node.innerText || '').trim()) ||
              /passport\.bilibili\.com\/login/i.test(String(node.getAttribute('href') || ''))))
            loginControlSeen = true;
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
    const pageNumber = () => {
      const u = url();
      const fromUrl = Number(u.searchParams.get('pn') || u.searchParams.get('page') || 0);
      if (Number.isInteger(fromUrl) && fromUrl > 0) return fromUrl;
      const pager = Array.from(document.querySelectorAll('[class*="pager"],[class*="pagination"],[class*="pagenation"]'))
        .find((node) => node.getClientRects().length);
      const active = pager && pager.querySelector('[aria-current="page"],[class*="active"],[class*="current"]');
      const activeNumber = Number(String(active?.innerText || '').trim());
      return Number.isInteger(activeNumber) && activeNumber > 0 ? activeNumber : 1;
    };
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
        const match = String(folder.innerText || '').trim().match(/(?:收藏夹|收藏)\\s*[（(]?\\s*([\\d,]{1,8})\\s*(?:条|个|视频)?\\s*[）)]?\\s*$/);
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
      const text = String(document.body?.innerText || '').slice(0, 30000);
      const patterns = [
        /共\s*([\d,]+)\s*(?:个视频|条视频|个收藏|条收藏)/,
        /总计\s*([\d,]+)\s*(?:个视频|条视频|个收藏|条收藏)?/,
        /([\d,]+)\s*个视频/
      ];
      for (const pattern of patterns) {
        const match = text.match(pattern);
        if (match) {
          const value = Number(match[1].replaceAll(',', ''));
          if (Number.isSafeInteger(value) && value >= 0) return value;
        }
      }
      return null;
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
      for (const anchor of container.querySelectorAll('a[href*="/video/BV"]')) {
        const href = String(anchor.getAttribute('href') || '');
        const bvid = href.match(/\\/video\\/(BV[0-9A-Za-z]{10})/)?.[1];
        if (!bvid || seen.has(bvid)) continue;
        const card = anchor.closest('[class*="fav"],[class*="video-card"],li') || anchor.parentElement || anchor;
        const titleNode = card.querySelector('[title],[aria-label]');
        const title = String(anchor.getAttribute('title') || anchor.getAttribute('aria-label') ||
          titleNode?.getAttribute('title') || titleNode?.getAttribute('aria-label') ||
          card.innerText || anchor.innerText || '').trim().split('\\n')[0].slice(0, 1000);
        seen.add(bvid);
        items.push({bvid, title});
      }
      return items;
    };
    const isBlocked = () => blockedPattern.test(String(document.title || '') + '\\n' + String(document.body?.innerText || '').slice(0, 5000));
    const waitForPageChange = async (beforeSignature) => {
      let changedAt = 0;
      for (let attempt = 0; attempt < 80; attempt++) {
        await new Promise((resolve) => setTimeout(resolve, 250));
        if (isCancelled() || isBlocked()) return false;
        const container = findFavoritesContainer();
        const signature = container ? Array.from(container.querySelectorAll('a[href*="/video/BV"]'))
          .slice(0, 5).map((node) => node.getAttribute('href')).join('|') : '';
        if (signature && signature !== beforeSignature) {
          if (!changedAt) changedAt = Date.now();
          if (Date.now() - changedAt >= 1200) return true;
        } else {
          changedAt = 0;
        }
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
        const beforeSignature = Array.from(container.querySelectorAll('a[href*="/video/BV"]'))
          .slice(0, 5).map((node) => node.getAttribute('href')).join('|');
        first.click();
        if (!await waitForPageChange(beforeSignature)) { await abort('BFC_PAGINATION_TIMEOUT'); return; }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
      }

      const firstItems = extractItems(container);
      const pageSize = firstItems.length;
      if (total > 0 && pageSize === 0) { await abort('BFC_FAVORITES_PAGE_EMPTY'); return; }
      const expected = total === 0 ? 1 : Math.ceil(total / pageSize);
      if (expected < 1 || expected > 300) { await abort('BFC_PAGINATION_LIMIT'); return; }
      const folderId = url().searchParams.get('fid') || 'unknown';
      const titleNode = document.querySelector('[class*="fav"][class*="title"],[class*="fav-title"]');
      const folderName = String(titleNode?.innerText || document.title || '当前收藏夹').trim().slice(0,128);
      const pages = [];
      let itemsCaptured = 0;
      for (let page = 1; page <= expected; page++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
        const items = page === 1 ? firstItems : extractItems(container);
        const expectedCount = Math.max(0, Math.min(pageSize, total - (page - 1) * pageSize));
        if (items.length !== expectedCount) { await abort('BFC_READBACK_PAGE_COUNT_MISMATCH'); return; }
        pages.push({page, items});
        itemsCaptured += items.length;
        await invoke('bfc_readback_submit', {
          ticket,
          payload: { kind: 'progress', pages_captured: page, items_captured: itemsCaptured,
            expected_pages: expected, reported_total: total, folder_id: folderId, folder_name: folderName }
        });
        if (page === expected) break;
        const pager = pageControls();
        if (!pager.next || pager.next.hasAttribute('disabled') ||
            pager.next.getAttribute('aria-disabled') === 'true' ||
            /disabled/i.test(String(pager.next.className || ''))) {
          await abort('BFC_PAGINATION_INCOMPLETE');
          return;
        }
        const beforeSignature = Array.from(container.querySelectorAll('a[href*="/video/BV"]'))
          .slice(0, 5).map((node) => node.getAttribute('href')).join('|');
        pager.next.click();
        if (!await waitForPageChange(beforeSignature)) {
          if (!isCancelled()) await abort(isBlocked() ? 'BFC_BILIBILI_ACCESS_LIMITED' : 'BFC_PAGINATION_TIMEOUT');
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
