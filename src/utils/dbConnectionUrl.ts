import type { DbKind, DbSslMode } from "./dbUi";

// Parse supported URIs into the established typed form. No caller may persist
// or display the raw URI. Reject credentials and unmodelled driver switches.
export interface SafeDatabaseUrl {
  kind: DbKind;
  host: string;
  portText: string;
  database: string;
  sslMode: DbSslMode;
  /** JDBC-only switches intentionally not forwarded to the native driver. */
  ignoredOptions?: string[];
}
const fail = (): never => { throw Error("连接 URL 无法安全导入；请分别填写连接地址和密码"); };
/** 地址栏也接受主机名或主机名:端口；库名等字段保持原值，不猜测凭据。 */
export function parseDatabaseHostAddress(raw: string): { host: string; portText: string } | null {
  const address = raw.trim();
  if (!/^[a-z0-9][a-z0-9.-]*(?::\d{1,5})?$/i.test(address) || address.length > 255) return null;
  const [host, portText = ""] = address.split(":");
  if (host.includes("..") || host.endsWith(".") || host.startsWith("-")) return null;
  if (portText && (Number(portText) < 1 || Number(portText) > 65535)) return null;
  return { host, portText };
}
export function parseDatabaseConnectionUrl(raw: string): SafeDatabaseUrl {
  if (!raw || raw.length > 4096 || /[\x00-\x1f\x7f]/.test(raw)) fail();
  const uri = raw.trim().replace(/^jdbc:/i, "");
  if (/^sqlite:/i.test(uri)) {
    const input = uri.slice(7);
    const path = input.startsWith("///") ? input.slice(2) : input;
    if (!path.startsWith("/") || path.startsWith("//") || /[?#;%@\\]|\.\.|\/\//.test(path)) fail();
    return { kind: "sqlite", host: "", portText: "", database: path, sslMode: "disable" };
  }
  const match = /^(mysql|postgres|postgresql):\/\//i.exec(uri);
  if (!match) fail();
  const kind: DbKind = match![1].toLowerCase() === "mysql" ? "mysql" : "postgres";
  let u: URL;
  try { u = new URL(uri); } catch { return fail(); }
  if (!u.hostname || u.username || u.password || u.hash || !u.pathname || u.pathname === "/"
    || /\/(?:.*\/)|%2f|%5c/i.test(u.pathname) || u.host.includes("@")
    || !/^[-.\w:\[\]]+$/.test(u.hostname)) fail();
  let sslMode: DbSslMode = "prefer";
  const pairs = [...u.searchParams];
  const seen = new Set<string>();
  const ignoredOptions: string[] = [];
  let explicitSsl: DbSslMode | null = null;
  for (const [rawKey, rawValue] of pairs) {
    const key = rawKey.toLowerCase();
    const value = rawValue.toLowerCase();
    if (seen.has(key)) fail();
    seen.add(key);
    if (key === "sslmode" || (kind === "mysql" && key === "usessl")) {
      let mode: DbSslMode;
      if (["require", "required", "verify-ca", "verify-full", "true"].includes(value)) mode = "require";
      else if (["disable", "disabled", "false"].includes(value)) mode = "disable";
      else if (value === "prefer" && key === "sslmode") mode = "prefer";
      else fail();
      if (explicitSsl && explicitSsl !== mode) fail();
      explicitSsl = mode;
      continue;
    }
    if (kind !== "mysql") fail();
    if (key === "useunicode" && /^(true|false)$/.test(value)
      || key === "characterencoding" && /^(utf8|utf-8|utf8mb4)$/.test(value)
      || key === "servertimezone" && /^[a-z][a-z0-9_+/-]{0,63}$/i.test(rawValue)
      || key === "allowmultiqueries" && /^(true|false)$/.test(value)
      || key === "rewritebatchedstatements" && /^(true|false)$/.test(value)) {
      ignoredOptions.push(rawKey);
      continue;
    }
    fail();
  }
  if (explicitSsl) sslMode = explicitSsl;
  let database: string;
  try { database = decodeURIComponent(u.pathname.slice(1)); } catch { return fail(); }
  if (!database || /[\/\\\x00-\x1f\x7f]/.test(database)) fail();
  return { kind, host:u.hostname, portText:u.port || (kind==="mysql"?"3306":"5432"), database, sslMode,
    ...(ignoredOptions.length ? { ignoredOptions } : {}) };
}
