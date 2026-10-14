/**
 * JetBrains IDEA/DataGrip dataSources.xml importer.
 * Pure browser-side preview: never sends or stores the source XML, passwords,
 * credentials, SSH details, or whole JDBC URLs. Import only supported MySQL
 * address fields after an explicit selection and confirmation.
 */
export interface IdeaMysqlCandidate {
  id: string;
  name: string;
  host: string;
  port: string;
  database: string;
  username: string;
  warning: string;
}
export interface IdeaImportPreview {
  candidates: IdeaMysqlCandidate[];
  skipped: number;
  warnings: string[];
}

function parseJdbcMysql(value: string): { host: string; port: string; database: string } | null {
  if (!/^jdbc:mysql:\/\//i.test(value)) return null;
  try {
    // Browser URL parser handles IPv6 and percent-escaped database names.
    const url = new URL(value.slice(5));
    if (url.protocol !== "mysql:" || url.username || url.password || !url.hostname) return null;
    const database = decodeURIComponent(url.pathname.slice(1));
    if (!database || database.includes("/") || database.includes("\\") || database.includes("\0")) return null;
    if (url.hostname.length > 253 || database.length > 128) return null;
    if (url.searchParams.has("password") || url.searchParams.has("user")) return null;
    return { host: url.hostname.replace(/^\[|\]$/g, ""), port: url.port || "3306", database };
  } catch { return null; }
}

export function previewIdeaDataSources(xml: string): IdeaImportPreview {
  if (xml.length > 2 * 1024 * 1024) throw new Error("IDEA 配置文件超过 2 MiB 限制");
  if (/<\s*!\s*(?:DOCTYPE|ENTITY)\b/i.test(xml)) throw new Error("IDEA 配置包含 DTD/实体，已拒绝解析");
  const doc = new DOMParser().parseFromString(xml, "application/xml");
  if (doc.querySelector("parsererror")) throw new Error("IDEA 配置 XML 格式无效");
  const nodes = Array.from(doc.getElementsByTagName("data-source"));
  const candidates: IdeaMysqlCandidate[] = [];
  const warnings: string[] = [];
  let skipped = 0;
  for (const node of nodes.slice(0, 200)) {
    const get = (tag: string) => node.getElementsByTagName(tag)[0]?.textContent?.trim() || "";
    const jdbc = get("jdbc-url");
    const driver = get("driver-ref").toLowerCase();
    const parsed = parseJdbcMysql(jdbc);
    // Reject MySQL-like names with unrecognized driver/URL or advanced topologies.
    if (!parsed || (driver && !driver.includes("mysql"))) { skipped++; continue; }
    const advanced = Array.from(node.getElementsByTagName("*")).some(el =>
      /ssh|tunnel|proxy|cloud|secret|ssl-cert|client-cert|password|credential/i.test(el.tagName)
    );
    // JetBrains can put advanced settings in attributes as well.
    const serialized = new XMLSerializer().serializeToString(node);
    const advancedAttribute = /\b(?:sshConfig|sshTunnel|proxyConfig|sslCert|sslKey)\s*=/i.test(serialized);
    if (advanced || advancedAttribute) {
      skipped++;
      warnings.push("检测到需要特殊处理的隧道、代理、证书或凭据配置；未导入该连接");
      continue;
    }
    const name = (node.getAttribute("name") || parsed.database).slice(0, 128);
    candidates.push({
      id: node.getAttribute("uuid") || String(candidates.length),
      name, host: parsed.host, port: parsed.port, database: parsed.database,
      username: get("user-name").slice(0, 128),
      warning: "仅导入连接地址；密码、SSH/证书和其他 JDBC 参数不自动导入",
    });
  }
  if (nodes.length > 200) warnings.push("只检查前 200 个数据源");
  return { candidates, skipped: skipped + Math.max(0, nodes.length - 200), warnings };
}
