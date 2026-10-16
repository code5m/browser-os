import type { DbKind, DbSslMode } from "./dbUi";

/**
 * Converts an IDEA JDBC URL or a plain database URI to the existing typed
 * connection form. Never return or persist the supplied URI.
 * Passwords, user-info, fragments, tunneling and unknown driver switches are
 * rejected instead of silently ignored; enter credentials in the keyring form.
 */
export interface SafeDatabaseUrl {
  kind: DbKind;
  host: string;
  portText: string;
  database: string;
  sslMode: DbSslMode;
}

function databaseUrlError(): Error {
  // Do not interpolate the URI: it may contain secrets.
  return new Error("连接 URL 无法安全导入；请使用受支持的地址并在独立密码框输入凭据");
}

export function parseDatabaseConnectionUrl(raw: string): SafeDatabaseUrl {
  if (!raw || raw.length > 4096 || /[\x00-\x1f\x7f]/.test(raw)) throw databaseUrlError();
  const text = raw.trim();
  const input = text.toLowerCase().startsWith("jdbc:") ? text.slice(5) : text;
  const kind = /^(mysql):\/\//i.test(input) ? "mysql"
    : /^(postgres|postgresql):\/\//i.test(input) ? "postgres"
    : /^(sqlite):/i.test(input) ? "sqlite" : null;
  if (!kind) throw databaseUrlError();

  if (kind === "sqlite") {
    const pathText = input.slice("sqlite:".length);
    if (!pathText || pathText.includes("?") || pathText.includes("#") || pathText.includes("\\")
      || pathText.includes("%") || pathText.includes("..") || pathText.includes("@")
      || pathText.includes(";")) throw databaseUrlError();
    const path = pathText.startsWith("///") ? pathText.slice(2) : pathText;
    if (!path.startsWith("/") || path.startsWith("//") || path.includes("//"))
      throw databaseUrlError();
    return { kind, host: "", portText: "", database: path, sslMode: "disable" };
  }
  let url: URL;
  try { url = new URL(input); } catch { throw databaseUrlError(); }
  if (!url.hostname || url.username || url.password || url.hash || url.pathname === "/"
    || url.pathname === "" || url.pathname.slice(1).includes("/") || url.pathname.includes("%2f")
    || url.pathname.includes("%5c") || url.host.includes("@")) throw databaseUrlError();
  let sslMode: DbSslMode = "prefer";
  for (const [key, value] of url.searchParams.entries()) {
    if (!["sslmode", "sslMode"].includes(key) || url.searchParams.getAll(key).length !== 1)
      throw databaseUrlError();
    if (/^(require|required|verify-ca|verify-full)$/i.test(value)) sslMode = "require";
    else if (/^(prefer)$/i.test(value)) sslMode = "prefer";
    else if (/^(disable|disabled)$/i.test(value)) sslMode = "disable";
    else throw databaseUrlError();
  }
  let database: string;
  try { database = decodeURIComponent(url.pathname.slice(1)); } catch { throw databaseUrlError(); }
  if (!database || database.includes("/") || database.includes("\\") || /[\x00-\x1f\x7f]/.test(database))
    throw databaseUrlError();
  if (!/^[-.\w:\[\]]+$/.test(url.hostname)) throw databaseUrlError();
  return {kind, host:url.hostname, portText:url.port || (kind==="mysql"?"3306":"5432"), database, sslMode};
}
