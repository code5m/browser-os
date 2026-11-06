const invokeLine = "const invoke = window.__TAURI__?.core?.invoke; if (typeof invoke !== 'function') throw new Error('BFC_BROWSER_BRIDGE_UNAVAILABLE');";
export function buildLoginProbeScript(ticket) {
  const encodedTicket = JSON.stringify(String(ticket));
  return `(async () => {
    ${invokeLine}
    const blockedPattern = /(?:^|\\b)412(?:\\b|$)|访问被拒绝|请求被拦截|安全验证|人机验证|访问过于频繁|操作频繁|风控验证/i;
    const ticket = ${encodedTicket};
    const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    let ready = false;
    for (let attempt = 0; attempt < 40; attempt++) {
      if (window.__browserosBfcCancelScan === ticket) return;
      if ((location.hostname === 'www.bilibili.com' || location.hostname === 'space.bilibili.com') &&
          document.readyState !== 'loading' && document.body) { ready = true; break; }
      await wait(250);
    }
    const header = document.querySelector('#bili-header-container, header, .bili-header__bar') || document;
    const text = String(document.title || '') + '\\n' + String(header.innerText || '').slice(0, 5000);
    const body = String(document.body?.innerText || '').slice(0, 5000);
    let login_status = 'unknown';
    let mid = null;
    if (!ready) {
      login_status = 'unknown';
    } else if (blockedPattern.test(text + '\\n' + body)) {
      login_status = 'access_limited';
    } else {
      const avatar = Array.from(header.querySelectorAll('a[href*="space.bilibili.com"]'))
        .find((node) => node.querySelector('img') || /avatar|header-entry-avatar/i.test(String(node.className || '')));
      const href = avatar?.getAttribute('href') || '';
      mid = href.match(/space\\.bilibili\\.com\\/(\\d{1,20})/)?.[1] || null;
      const loginControl = Array.from(header.querySelectorAll('a,button,[role="button"]'))
        .some((node) => /登录/.test(String(node.innerText || '').trim()) ||
          /passport\\.bilibili\\.com\\/login/i.test(String(node.getAttribute('href') || '')));
      if (mid) login_status = 'logged_in';
      else if (loginControl) login_status = 'logged_out';
    }
    try {
      await invoke('bfc_readback_submit', {
        ticket,
        payload: { kind: 'login', login_status, mid }
      });
      document.documentElement.dataset.browserosBfcLoginProbe = login_status;
    } catch (error) {
      try {
        await invoke('bfc_readback_submit', {
          ticket,
          payload: { kind: 'abort', message_code: 'BFC_BROWSER_SCRIPT_ERROR' }
        });
      } catch {}
      document.documentElement.dataset.browserosBfcLoginProbe = 'bridge_error';
      document.documentElement.dataset.browserosBfcError = String(error).slice(0, 120);
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
      catch (error) { document.documentElement.dataset.browserosBfcError = String(error).slice(0, 120); }
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
      const text = String(document.body?.innerText || '').slice(0, 30000);
      const patterns = [
        /共\\s*([\\d,]+)\\s*(?:个视频|条视频|个收藏|条收藏)/,
        /总计\\s*([\\d,]+)\\s*(?:个视频|条视频|个收藏|条收藏)?/,
        /([\\d,]+)\\s*个视频/
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
      if (expected < 1 || expected > 200) { await abort('BFC_PAGINATION_LIMIT'); return; }
      const pages = [];
      for (let page = 1; page <= expected; page++) {
        if (isCancelled()) return;
        if (isBlocked()) { await abort('BFC_BILIBILI_ACCESS_LIMITED'); return; }
        container = findFavoritesContainer();
        if (!container) { await abort('BFC_FAVORITES_CONTAINER_UNRECOGNIZED'); return; }
        const items = page === 1 ? firstItems : extractItems(container);
        const expectedCount = Math.max(0, Math.min(pageSize, total - (page - 1) * pageSize));
        if (items.length !== expectedCount) { await abort('BFC_READBACK_PAGE_COUNT_MISMATCH'); return; }
        pages.push({page, items});
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
      const folderId = url().searchParams.get('fid') || 'unknown';
      const titleNode = document.querySelector('[class*="fav"][class*="title"],[class*="fav-title"]');
      const folderName = String(titleNode?.innerText || document.title || '当前收藏夹').trim().slice(0,128);
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
      document.documentElement.dataset.browserosBfcError = String(error).slice(0, 160);
    }
  })();`;
}
