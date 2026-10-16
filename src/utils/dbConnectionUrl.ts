import type { DbKind, DbSslMode } from "./dbUi";

// Parse supported URIs into the established typed form. No caller may persist
// or display the raw URI. Reject credentials and unmodelled driver switches.
export interface SafeDatabaseUrl {
  kind: DbKind;
  host: string;
  portText: string;
  database: string;
  sslMode: DbSslMode;
}
const fail = (): never => { throw Error("连接 URL 无法安全导入；请分别填写连接地址和密码"); };
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
  if (pairs.length > 1 || pairs.some(([key]) => key.toLowerCase() !== "sslmode")) fail();
  if (pairs.length) {
    const value = pairs[0][1].toLowerCase();
    if (["require","required","verify-ca","verify-full"].includes(value)) sslMode="require";
    else if (value === "disable" || value === "disabled") sslMode="disable";
    else if (value !== "prefer") fail();
  }
  let database: string;
  try { database = decodeURIComponent(u.pathname.slice(1)); } catch { return fail(); }
  if (!database || /[\/\\\x00-\x1f\x7f]/.test(database)) fail();
  return { kind, host:u.hostname, portText:u.port || (kind==="mysql"?"3306":"5432"), database, sslMode };
}
