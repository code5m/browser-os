import type { CapabilityDefinition } from "../../capability/types"

// Database 能力 Manifest（Capability Library Expansion v1 — 从 Workspace 内嵌面板升格为独立 Capability）
// 语义 owner = useDatabaseStore（Semantic Registry 已冻结）。
// 物理：面板经通用 Contribution Registry 的 WORKBENCH_MAIN 槽贡献（view='db'），
// MainArea 只按槽 + view 渲染、不 import 本能力内部（C3 关键，且避免 workspace→database 反向依赖环）。
//
// 资源法（§18）核心区分（绝不可合并成单一 bool）：
//   - 保存的连接配置（connections/configs）        → disk 持久化，非资源
//   - 凭据引用（credential reference, keyring key）  → 仅 keyring id，前端永不持有明文
//   - 连接可用性 / 活跃态（connected flag）          → 前端标志，非资源对象
//   - 实际连接资源（DbPool）                          → 仅 Rust 在 connect/query 时瞬态创建、即弃（不驻留）
// Database absent → 面板不挂载 → 无 connect 调用 → 无 keyring 读、无 DbPool、无 watcher（§18 满足）。
export const databaseManifest: CapabilityDefinition = {
  id: "database",
  name: "数据库",
  category: "CAPABILITY",
  provides: [
    "database.connect",
    "database.query",
    "database.cancel",
  ],
  dependsOn: ["credential", "bridge"],
  optionalDependencies: [],
  lifecycle: {
    supported: ["ACTIVE", "SUSPENDED"],
    default: "ACTIVE",
    activatable: true,
    resident: false,
  },
  resources: {
    class: ["NETWORK", "SECRET"],
    suspendable: true,
    destroyable: true,
  },
  permissions: ["network.connect", "keyring.access"],
  persistence: {
    scope: "disk",
    sensitive: true,
  },
  entrypoint: "index.ts",
  semanticOwner: "useDatabaseStore",
  governanceStatus: "GOVERNED",
  status: "COMPATIBILITY_WRAPPED",
  // Building Block Contract v1（§9）：真实 resource ownership 由 scripts/measure-resources.mjs 测量支撑。
  v1: {
    id: "database",
    version: "1.0.0",
    displayName: "数据库",
    description:
      "连接数据库（凭据经 OS keyring，前端零明文）、执行只读/写查询、取消查询。连接资源由后端瞬态创建即弃，不驻留全局。",
    maturity: "C3",
    maturityEvidence: ["scripts/check-developer-owners.mjs", "scripts/measure-resources.mjs", "scripts/check-hot-plug-acceptance.mjs"],
    dependencies: ["credential", "bridge"],
    optionalDependencies: [],
    conflicts: [],
    provides: ["database.connect", "database.query", "database.cancel"],
    requires: [],
    contributions: [{ id: "database.main.panel", slot: "workbench-main", type: "surface" }],
    permissions: ["network.connect", "keyring.access"],
    resources: [
      // 实际连接资源（DbPool）由 Rust 在 connect/query 时瞬态创建即弃；前端不持有长驻连接对象。
      { kind: "NETWORK", ownership: "owned", evidence: "src-tauri/src/database.rs db_connect/db_query" },
    ],
    persistenceScope: "disk",
    persistenceSensitive: true,
    activationPolicy: "auto",
    // §18：停用须确保进行中的查询已取消、瞬态连接已释放（Rust 每查询重连、无池，天然释放）。
    deactivationPolicy: "graceful",
    installPolicy: "static",
    uninstallPolicy: "static",
    hotPlug: {
      level: "HP2",
      enable: true,
      disable: true,
      register: true,
      unregister: true,
      install: false,
      uninstall: false,
      limitationReason:
        "HP2：后端连接资源按请求瞬态创建；暂停/停用会取消全部已知 query_id 并失效 schema generation，统一 Harness 验证贡献摘除、重复启停与 fresh Runtime。",
    },
    publicContract: [{ name: "publicApi", locator: "src/capabilities/database/public.ts" }],
    entrypoint: "src/capabilities/database/index.ts",
    semanticOwner: "useDatabaseStore",
  },
}
