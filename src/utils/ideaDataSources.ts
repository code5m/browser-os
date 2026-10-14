/** Strict, read-only import of JetBrains .idea/dataSources.xml metadata.
 * Never reads dataSources.local.xml, passwords, KeePass, IDEA Keychain or DSN secrets.
 * The caller must explicitly choose an allowed project directory.
 */
export interface IdeaSource {
  id: string;
  name: string;
  kind: "mysql" | "sqlite" | "unsupported";
  host: string;
  portText: string;
  database: string;
  username: string;
  supported: boolean;
  note: string;
}
const MAX_XML_BYTES = 512 * 1024;
const MAX_SOURCES = 128;
function decoded(value: string): string {
  return value.replace(/&(#x[0-9a-f]+|#[0-9]+|amp|lt|gt|apos|quot);/gi, (_m, code: string) => {
    const simple: Record<string,string> = {amp:"&",lt:"<",gt:">",apos:"'",quot:'"'};
    if (simple[code]) return simple[code];
    const n = code.toLowerCase().startsWith("#x") ? parseInt(code.slice(2),16) : parseInt(code.slice(1),10);
    return Number.isFinite(n) && n > 0 && n <= 0x10ffff && !(n >= 0xd800 && n <= 0xdfff)
      ? String.fromCodePoint(n) : "";
  });
}
function field(block: string, tag: string): string {
  const match = new RegExp("<" + tag + "(?:\\s[^>]*)?>([\\s\\S]*?)<\\/" + tag + "\\s*>", "i").exec(block);
  return decoded(match?.[1] ?? "").trim();
}
function attr(attributes: string, name: string): string {
  const match = new RegExp("(?:^|\\s)" + name + "\\s*=\\s*([\\\"'])([\\s\\S]*?)\\1","i").exec(attributes);
  return decoded(match?.[2] ?? "").trim();
}
function safeDisplay(value: string, max=128): string {
  return value.replace(/[\\u0000-\\u001f\\u007f]/g,"").slice(0,max);
}
export function ideaProjectDataSourcesPath(project: string): string {
  const clean = project.trim().replace(/\\\\/g,"/");
  if (!clean || clean.length > 4096 || clean.includes("\\0") || !clean.startsWith("/")) {
    throw new Error("请选择本地项目的绝对目录路径");
  }
  const path = clean.replace(/\\/+$/, "").replace(/\\/+$/, "");
  if (path.endsWith("/.idea/dataSources.xml")) return path;
  if (path.endsWith("/.idea")) return path + "/dataSources.xml";
  if (path.endsWith(".xml")) throw new Error("只允许读取项目 .idea/dataSources.xml，禁止读取 dataSources.local.xml");
  return path + "/.idea/dataSources.xml";
}
export function parseIdeaDataSourcesXml(xml: string): IdeaSource[] {
  if (new TextEncoder().encode(xml).length > MAX_XML_BYTES) throw new Error("IDEA 配置超过 512KiB 安全上限");
  if (/<!DOCTYPE|<!ENTITY|<script\b|<\?xml-stylesheet/i.test(xml)) throw new Error("禁止 DTD、外部实体或脚本");
  if (!xml.includes("<project") && !xml.includes("<component")) throw new Error("不是 IDEA 项目数据源配置");
  const matches = [...xml.matchAll(/<data-source\b([^>]*)>([\s\S]*?)<\/data-source\s*>/gi)];
  if (matches.length > MAX_SOURCES) throw new Error("IDEA 连接超过 128 条上限");
  return matches.map((match,index) => {
    const attributes=match[1], body=match[2];
    const jdbc=field(body,"jdbc-url");
    const driver=field(body,"driver-ref") || field(body,"jdbc-driver");
    const name=safeDisplay(attr(attributes,"name") || "IDEA 数据源 "+(index+1));
    const id=safeDisplay(attr(attributes,"uuid") || name);
    const user=safeDisplay(field(body,"user-name"));
    const base: IdeaSource = {id,name,kind:"unsupported",host:"",portText:"",database:"",username:user,supported:false,note:""};
    if (/ssh-tunnel|ssh-config|tunnel-config|url-only/i.test(body)) return {...base,note:"需要 SSH/自定义隧道或专用连接方式，暂不自动导入"};
    if (/^jdbc:mysql:\/\//i.test(jdbc)) {
      try {
        const url = new URL(jdbc.slice(5));
        if (url.username || url.password || url.hash) return {...base,note:"连接串包含敏感账户信息或片段，拒绝自动导入"};
        const host=url.hostname;
        const database=decodeURIComponent(url.pathname.replace(/^\//,""));
        if (!host || !database || database.includes("/") || database.length>255 || /[\u0000-\u001f]/.test(database)) {
          return {...base,note:"MySQL 地址或数据库名不完整"};
        }
        const sslSetting=url.searchParams.get("sslMode")||"";
        const tlsNotice=/DISABLED|useSSL=false/i.test(sslSetting+" "+url.search) ? "；IDEA 的非 TLS 选项未自动继承" : "";
        return {...base,kind:"mysql",host,portText:url.port||"3306",database,supported:true,
          note:"导入连接元数据，不导入密码；默认采用 BrowserOS 安全 TLS 配置"+tlsNotice};
      } catch { return {...base,note:"无法解析 MySQL JDBC URL"}; }
    }
    if (/^jdbc:sqlite:/i.test(jdbc)) {
      const filepath=jdbc.slice("jdbc:sqlite:".length);
      if (!filepath.startsWith("/") || filepath.length>4096 || /[\u0000-\u001f]/.test(filepath) || filepath.split("/").includes("..")) {
        return {...base,note:"SQLite 必须使用有效的本地绝对文件路径"};
      }
      return {...base,kind:"sqlite",database:filepath,supported:true,note:"SQLite 文件仍需位于 BrowserOS 授权目录"};
    }
    return {...base,note: driver ? "尚未支持 IDEA 驱动："+safeDisplay(driver,90) : "不支持或缺失 JDBC 驱动/URL"};
  });
}
