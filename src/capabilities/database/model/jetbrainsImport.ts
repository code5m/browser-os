/**
 * Manual JetBrains dataSources.xml preview. All XML parsing, URL validation,
 * entity rejection and driver semantics belong to the same IDEA importer
 * used for project-path scans; do not maintain a second DOM/JDBC parser.
 * The caller never stores the original file or the full JDBC URL.
 */
import { parseIdeaDataSourcesXml } from "../../../utils/ideaDataSources";

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

export function previewIdeaDataSources(xml: string): IdeaImportPreview {
  // Manual picker historically supports a larger file than project-path scans,
  // but uses the exact same strict parser with independent bounds.
  const parsed = parseIdeaDataSourcesXml(xml, {
    maxBytes: 2 * 1024 * 1024,
    maxSources: 200,
    strictMysql: true,
  });
  const supported = parsed.filter(source => source.kind === "mysql" && source.supported);
  const skipped = parsed.length - supported.length;
  return {
    candidates: supported.map(source => ({
      id: source.id,
      name: source.name,
      host: source.host,
      port: source.portText,
      database: source.database,
      username: source.username,
      warning: "仅导入连接地址；密码、SSH/证书和其他 JDBC 参数不自动导入",
    })),
    skipped,
    warnings: skipped ? ["部分数据源因驱动、凭据、证书或安全限制不兼容，已跳过"] : [],
  };
}
