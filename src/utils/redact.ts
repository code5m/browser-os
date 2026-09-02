// M1-7：Git 写操作错误态的**前端二次脱敏**（防御性兜底）。
//
// 后端已在 `sync::scrub_sensitive_error` 做过一次脱敏（精确替换 token +
// 掩码 URL userinfo），并经 `sanitize_audit_text` 截断后才落审计/回前端。
// 这里再做一层，目标只有一个：即便后端哪天漏了，前端 toast / 错误态也绝不
// 把凭据原样打印到屏幕上。
//
// 覆盖范围（刻意保守，避免误伤 git OID 与文件路径）：
//   1. URL userinfo      : https://user:pass@host → https://***:***@host
//   2. 凭据查询参数       : ?access_token=… / &private_token=… / &password=…
//   3. 已知 token 前缀     : ghp_/gho_/ghu_/ghs_/github_pat_/glpat-
//   4. Authorization 头值 : Bearer <token> / Basic <token>
//   5. 疑似随机串         : ≥32 位且「小写+大写+数字」三者齐备（纯小写十六进制
//                          的 40 位 git OID 不匹配，不会被误伤）
// 最后统一截断到 300 字符，避免整段 diff / 堆栈刷屏。

const URL_USERINFO = /([a-z][a-z0-9+.-]*:\/\/)[^\s/:@]+:[^\s/@]+@/gi;
const CRED_PARAM =
  /([?&](?:access_token|refresh_token|private_token|token|password|secret|api_key)=)[^&\s"'<>]+/gi;
const TOKEN_PREFIX =
  /\b(?:gh[pous]_[A-Za-z0-9]{16,}|github_pat_[A-Za-z0-9_]{20,}|glpat-[A-Za-z0-9_-]{16,})\b/g;
const AUTH_VALUE = /\b(?:Bearer|Basic)\s+[A-Za-z0-9+/=._-]{8,}/gi;
const MASK = "***";
const MAX_LEN = 300;

function looksLikeRandomSecret(word: string): boolean {
  if (word.length < 32) return false;
  if (!/^[A-Za-z0-9+/=_-]+$/.test(word)) return false;
  return /[a-z]/.test(word) && /[A-Z]/.test(word) && /\d/.test(word);
}

function maskRandomSecrets(text: string): string {
  return text.replace(/[A-Za-z0-9+/=_-]{32,}/g, (word) =>
    looksLikeRandomSecret(word) ? MASK : word
  );
}

/** 把任意异常/错误串转成可安全展示的脱敏文本（不抛异常、绝不返回 undefined）。 */
export function redactSecrets(e: unknown): string {
  let text: string;
  if (typeof e === "string") text = e;
  else if (e && typeof e === "object") {
    const anyErr = e as { message?: unknown };
    text =
      typeof anyErr.message === "string" && anyErr.message.length > 0
        ? anyErr.message
        : String(e);
  } else text = String(e ?? "");

  let out = text.replace(URL_USERINFO, `$1${MASK}:${MASK}@`);
  out = out.replace(CRED_PARAM, `$1${MASK}`);
  out = out.replace(TOKEN_PREFIX, MASK);
  out = out.replace(AUTH_VALUE, (m) => `${m.split(/\s+/)[0]} ${MASK}`);
  out = maskRandomSecrets(out);
  return out.length > MAX_LEN ? `${out.slice(0, MAX_LEN)}…` : out;
}
