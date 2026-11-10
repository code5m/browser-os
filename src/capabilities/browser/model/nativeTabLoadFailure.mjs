// Store only a bounded error classification, not native WebView response data.
const TLS_FAILURE = /(?:\bTLS\b|\bSSL\b|certificate|handshake|gnutls)/i;
const PLATFORM_LIMITED = /(?:\bHTTP\s*412\b|\bstatus\s*412\b)/i;
export const NATIVE_LOAD_FAILURE_TTL_MS = 120_000;

export function classifyNativeTabLoadFailure(status, message) {
  if (status !== 'load-failed' && status !== 'failed') return null;
  const reason = String(message || '').slice(0, 500);
  if (PLATFORM_LIMITED.test(reason)) return 'platform-limited';
  if (TLS_FAILURE.test(reason)) return 'tls';
  return 'load';
}

export function recentNativeTabLoadFailure(record, now = Date.now()) {
  if (!record || !Number.isFinite(record.at) || now < record.at ||
      now - record.at > NATIVE_LOAD_FAILURE_TTL_MS) return null;
  return ['tls', 'platform-limited', 'load'].includes(record.kind) ? record.kind : null;
}
