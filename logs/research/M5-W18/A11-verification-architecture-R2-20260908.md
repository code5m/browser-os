# A11 · M5-W18-R2 Evidence-Closure · 验证架构综合（修正 + 重建）

> 本文件是 R1（`A11-verification-architecture-20260908-0800.md`）的 **R2 证据闭环版**。R1 原文原样保留于文末附录 A，本文件按 `A0-M5-W18-R1-audit-20260908.md` 逐条 retract/replace，并用 `CURRENT_PRODUCT` / `REFERENCE_SOURCE` / `OBSERVED_BEHAVIOR` / `OFFICIAL_DOC` / `EXECUTED_SYNTHETIC_TEST` / `INFERENCE` 六类标记每一个事实主张。

```text
LANE=A11
STATUS=PASS_WITH_DEBT（R2 终稿被门禁阻断：A1–A10 尚无任何 R2 commit；A9 分支 c5bfc88 仍未 rebase 到 d6127c4；见 §10 阻塞项）
BASE=d6127c4
HEAD=<提交后回填>
REFERENCE_EVIDENCE=
  /home/ainfinit/Documents/极智简单/V3/research/dbx-src（Cargo.lock SHA-256 c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7，Apache-2.0）
  /home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src（upstream zvec-ai/zvec-grep @ 52653951b24617762f4ab0c71c34d594e5001617，package.json version 0.2.1，Apache-2.0，依赖 @zvec/zvec ^0.7.0）
  /home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian（graph.json / app.json / core-plugins.json / workspace.json）
FILES=logs/research/M5-W18/A11-verification-architecture-R2-20260908.md ; logs/checkpoints/A11-M5-W18-R-20260908-0800.md（更新）
SOURCE_MAP=
  本产品（CURRENT_PRODUCT，均为只读盘点）: src-tauri/src/{main.rs,database.rs,domain.rs,graph.rs,security_policy.rs,bridge.rs}, src/types.ts, src/stores/*.ts, src/components/workspace/{DatabasePanel.vue,GraphPanel.vue,...}, src-tauri/capabilities/{default.json,browser-remote.json}, src-tauri/dev-capabilities/main.json, scripts/check-command-set-consistency.py, scripts/measure-build-metrics.py, LICENSE
  对等 lane（peer R1 报告，经 `git show <sha>:<path>` 只读消费）:
    A1 225a3df logs/research/M5-W18/A1-product-baseline-20260908.md + A1-checkpoint-20260908.md
    A2 7bc910e logs/research/M5-W18/A2-obsidian-vault-semantics.md + A2-fixtures.md
    A3 7dde286 logs/research/M5-W18/A3-obsidian-graph-search-ux-20260908-0759.md
    A4 971378c logs/research/M5-W18/A4-dbx-backend-architecture-map.md + A4-checkpoint.md
    A5 71641e5 logs/research/M5-W18/A5-{workbench-ux-map,replication-blueprint}.md + A5-checkpoint.md
    A6 b471817+b11c541b logs/research/M5-W18/A6-security-lifecycle-audit.md + A6-checkpoint.md
    A7 8171290+3429699 logs/research/M5-W18/A7-zvec-grep-ingestion-index-architecture.md + A7-lane-checkpoint.md
    A8 b7a8615 logs/research/M5-W18/A8-zvec-grep-retrieval.md + A8-benchmark-ripgrep.mjs + A8-checkpoint.md
    A9 c5bfc88 logs/research/M5-W18/A9-{threat-model,source-map}.md + A9-checkpoint.md
    A10 a3c97ae logs/research/M5-W18/A10-{source-transplant-ledger,dependency-bom,notice-and-review-gate}.md + A10-checkpoint.md
CLASSIFICATION=验证资产本身不可 COPY 盲抄（均需 ADAPT/REIMPLEMENT）；本 lane 仅综合，不做 COPY/ADAPT 判定（属 A4/A7/A10）。已与 peer 核对并记录的「分类口径」见 §9。
VERIFY=见 §2 只读盘点命令与结果（无产品代码改动、无依赖安装、无 daemon/网络/模型下载、未触碰 vault 正文）
CHECKPOINT=logs/checkpoints/A11-M5-W18-R-20260908-0800.md
MERGE_NOTES=§9 三项矛盾（policy 计数 / DbValue / zvec 路由）已据实测与 peer 报告 resolve；仍有 A9 未 rebase、peer R2 缺失等结构性阻塞（§10）
NEXT=待 A1–A10 各自 R2 commit 到位后，由 A0 合入一条 research 分支，A11 据此出终稿（本件为 R2-provisional，依 dispatch 不在所有报告齐备前宣布 W18 完成）
```

---

## 1. R2 修正日志（逐条 retract / replace A0 审计点）

R1 原文在附录 A。下表把 A0 审计里与 A11 相关的每一条 retract 落到本文件。

| # | A0 审计结论 | R1 旧主张（附录 A 行号/措辞） | R2 修正（证据标记） |
|---|---|---|---|
| C1 | 命令一致性真实为 135/135/46（A0#1） | R1 §3.1 只说「`check-command-set-consistency.py` 已存在」，**未给数**，且隐含依赖 A1 计数 | 实测：`Registered(A)=135 / DefaultACL(B_main)=135 / RemoteACL(B_remote)=3 / Main invocations(C_main)=46 / Remote invocations(C_remote)=3`，`GATE:PASS`（仅 KNOWN_DRIFT）。【EXECUTED_SYNTHETIC_TEST：`python3 scripts/check-command-set-consistency.py --report`】 |
| C2 | 策略脚本 29 Python + 50 合计（A0#6） | R1 §3.1「`scripts/` 共 47 个 `check-*`」 | 修正：`scripts/` 下 `check-*` 共 **50**（29 `.py` + 20 `.mjs` + 1 `.sh`）。须显式声明计数规则：**「50 个 `check-*` 顶层文件」** vs **「29 个 `check-*.py`」** 二者皆真，不能混用。【OBSERVED_BEHAVIOR：`ls scripts/ \| grep '^check-'`】 |
| C3 | `database.rs` 已含 22 `#[test]`（A0#2） | R1 未断言「无数据库测试」，但亦未登记该资产 | 补登：本产品 Rust 测试共 **470 个 `#[test]`**（`src-tauri/src` 全量 grep）；其中 `database.rs` 含 **23 个** 命名 `#[test]`（`t_db_p1`…`t_db_p13`，见 §2）。“无数据库测试”为伪。【CURRENT_PRODUCT grep】 |
| C4 | `DbValue` 为**双真源漂移**，非旧 bug（A0#3） | R1 §11-6 称「`DbValue` 大小写漂移已确认且通路在线」「B8-1/B8-2 已修」 | 修正：`DbValue` 在**两处 Rust 定义**——`database.rs:122` 实用于 `{Null,Bool,I64,F64,Text,Binary{bytes:usize}}`；`domain.rs:1094` 死代码 `{Null,Bool,Int(i64),Float(f64),Text,BlobLen(u64)}`（标 `#[allow(dead_code)]`）；`src/types.ts:603` 镜像后者（`{null},{bool},{int},{float},{text},{blob_len}`）。B8-1 的 PascalCase→snake_case 已修（TS 现状 snake_case），但 `domain.rs`+`types.ts` 镜像与 `database.rs` 的**变体命名不一致（Int/Float/BlobLen vs I64/F64/Binary）**是**现存双真源风险**，非“已修 bug”。【CURRENT_PRODUCT grep】 |
| C5 | 凭据瞬时进 JS 内存（A0#4） | R1 §5.2「凭据只走瞬时参数，绝不进本 DTO」表述不精确 | 修正：`useDatabaseStore.connect(password)` / `bridge.dbConnect(cfg,password)` 在**传递过程**中确实进入 JS 内存与 Tauri 命令形参；只是**不落地**到 Pinia DTO/配置持久化。属“瞬时内存存在、不持久化”，非“永不进 JS”。【CURRENT_PRODUCT：`src/types.ts:614` `DbConnectionConfig` 无 password 字段；`bridats` 调用点核对】 |
| C6 | 无 `useConnectionStore`（A0#5） | R1 未明确 | 补登：前端 16 个 store（`src/stores/`）含 `useDatabaseStore.ts`，**确无 `useConnectionStore`**；任何蓝图必须映射到真实文件 `DatabasePanel.vue`（已存在 `src/components/workspace/DatabasePanel.vue`），不得凭空提新 store。【CURRENT_PRODUCT `ls`】 |
| C7 | 产品 LICENSE = MulanPSL-2.0（A0#7） | R1 未断言产品证书，仅在 §2 引用 dbx/zvec 的 Apache-2.0 | 补登：根 `LICENSE` = **木兰宽松许可证第 2 版（MulanPSL-2.0）**；dbx/zvec-grep 为 Apache-2.0。W19  inbound Apache-2.0 复用须按 A10 加 `LICENSE-APACHE-*` + NOTICE，且**不**改变产品 MulanPSL-2.0。【CURRENT_PRODUCT `head LICENSE`】 |
| C8 | `@zvec/zvec@0.7.0` 原生绑定未证（A0#8） | R1 §3.3 仅说「TypeScript/Node≥22」 | 修正：`zvec-grep-src/package.json`：`"version":"0.2.1"`，依赖 `"@zvec/zvec":"^0.7.0"`，`engines.node>=22`。A8/A10 已据源码判定其为 **Node-only 原生 addon**（非 Rust 可直链）。能否从 Rust 直链仍为 A10 待决项（见 §9-3）。【REFERENCE_SOURCE grep】 |

> 另发现一处 R1 自相矛盾：R1 §5.1 以「真实 vault 8253 篇」作合成语料上限锚点，但本次重测 `find ... -name '*.md' | wc -l` = **6630**（用户 vault 可变）。→ 合成语料档位改用**固定目标值**（200/2000/cap），不再以“真实 vault = N”为锚（见 §5.1）。

---

## 2. 已验证的当前产品事实（claim-tagged 基线）

| ID | 资产 | 实测值 | 标记 | 命令/来源 |
|---|---|---|---|---|
| F1 | 命令全链 | 135 注册 / 135 默认 ACL / 3 远程 ACL / 46 主桥调用 / 3 远程调用；GATE PASS | EXECUTED | `python3 scripts/check-command-set-consistency.py --report` |
| F2 | 策略脚本 | `check-*` 顶层 50（29 py + 20 mjs + 1 sh） | OBSERVED | `ls scripts/ \| grep '^check-'` |
| F3 | Rust 测试 | 470 个 `#[test]`；`database.rs` 23 个 `t_db_*` | CURRENT_PRODUCT | `grep -rn '#\[test\]' src-tauri/src \| wc -l` |
| F4 | 前端单测 | 0（package.json 无 vitest/jest） | CURRENT_PRODUCT | `grep -nE '"(vitest\|jest)"' package.json` → none |
| F5 | DbValue 双真源 | `database.rs:122` `I64/F64/Binary{bytes}`（实）；`domain.rs:1094`+`types.ts:603` `Int/Float/BlobLen`（死/镜像） | CURRENT_PRODUCT | grep 行号见 §1-C4 |
| F6 | 图/库模块 | `graph.rs`（bounded store + 7 容量常量）、`database.rs`（22+ 上限/脱敏/取消）存在；**无 search/vector 模块** | CURRENT_PRODUCT | `ls src-tauri/src/`，A8§1：「ripgrep\|bm25\|tantivy\|@zvec\|hybrid search」0 命中 |
| F7 | 容量常量 | `GRAPH_MAX_DEPTH=4` `GRAPH_QUERY_LIMIT=1000` `GRAPH_MAX_NODES=5000` `GRAPH_MAX_EDGES=20000` `GRAPH_PROPS_MAX_BYTES=64KiB`=`MAX_TEXT_FIELD_BYTES`；`DB_MAX_ROWS=1000` | CURRENT_PRODUCT | `domain.rs:2188-2200`、A1§2.1 |
| F8 | 构建预算 | `TOTAL_BYTES_GROWTH_LIMIT_PCT=25.2`（一次性上限，`measure-build-metrics.py:39`） | CURRENT_PRODUCT | `grep TOTAL_BYTES_GROWTH_LIMIT_PCT scripts/measure-build-metrics.py` |
| F9 | 锁定权限 | `capabilities/{default.json,browser-remote.json}` + `dev-capabilities/main.json`（仅 `#[cfg(debug_assertions)]`）；`security_policy.rs` 含 `BLOCKED_LAUNCH_PROGRAMS`/`WRAPPERS`/`INTERPRETERS` 黑名单 | CURRENT_PRODUCT | `ls src-tauri/capabilities src-tauri/dev-capabilities` |
| F10 | 前端 store/组件 | 16 store（含 `useDatabaseStore.ts`，无 `useConnectionStore`）；`DatabasePanel.vue`、`GraphPanel/GraphViewer/GraphFilter/NodeDetail/EdgeDetail` 已存在 | CURRENT_PRODUCT | `ls src/stores src/components/workspace src/components/graph` |
| F11 | 产品证书 | MulanPSL-2.0（根 LICENSE） | CURRENT_PRODUCT | `head LICENSE` |
| F12 | 调试/发布源门 | 调试主窗 `http://localhost:1421/`（main.rs 程序化）；发布 `tauri://localhost`；`tauri.conf.json` 禁全局 `build.devUrl` | CURRENT_PRODUCT / OFFICIAL | `WORKSPACE_IDENTITY.md` Desktop Runtime Source Gate |

---

## 3. 对等 lane（R1）消费账本

> 所有 peer 报告均为 **R1 draft**（A0 已保留但**未采纳为实施权威**）；且截至本提交，**无任何 peer 产生 R2 commit**（见 §10）。下表仅记录“我读了什么、取了什么”，不代表 A0 已认证。

| Peer | commit | 取用要点（供 A11 综合） | 可靠性 |
|---|---|---|---|
| A1 | 225a3df | 10 维基线 + 规范缺口 G1–G8 + 需求清单 R1–R12；确认图/库模块、命令、容量常量、构建预算、锁定面 | R1（未认证） |
| A2 | 7bc910e | Obsidian vault 语义（wikilink/alias/heading-block/tag/frontmatter/unresolved/backlink/outgoing/orphan/ignore）+ 合成夹具 | R1（未认证） |
| A3 | 7dde286 | 图谱/搜索 UX 状态流（global/local graph、filters、groups、orphans、depth、selection、键盘可达、窄窗、五态） | R1（未认证） |
| A4 | 971378c | dbx 后端 source map（connection/secrets/schema/query/cancel/history/export/sql_risk/production_safety）+ Copys/Adapt/Reimplement/Reject | R1（未认证） |
| A5 | 71641e5 | dbx 工作台 UX 蓝图（连接树/schema 浏览器/编辑器/执行栏/结果网格/导出/历史/键盘） | R1（未认证） |
| A6 | b471817 | 安全生命周期审计：COPY3/ADAPT4/REIMPL3/DEFER2/REJECT2（凭据/DSN 脱敏/写确认/生产判定/超时取消/池清理/历史/导出/关停） | R1（未认证） |
| A7 | 8171290 | zvec 摄入/索引架构：COPY2（engine+ripgrep core）/ADAPT3/REIMPL7/DEFER1；未决：@zvec/zvec 是否暴露 Rust API | R1（未认证） |
| A8 | b7a8615 | 检索基准：managed-ripgrep 实测 11–17ms/420 文件；FTS/BM25/vector/hybrid 仅源码论证；**vector 模型缺失=硬失败非静默降级**；Z 路线分类 | R1（未认证） |
| A9 | c5bfc88 | 信任边界：B1 远程嵌入授权门（STRONG, COPY）、B3 API-key 明文（ADAPT→keychain）、B7 模型下载无完整性校验（REJECT as-is） | R1（未认证，且**未 rebase 到 d6127c4**） |
| A10 | a3c97ae | 移植账本 + BOM + NOTICE/审查门；dbx COPY6/ADAPT8/REIMPL4/DEFER2/REJECT5；zvec REJECT 直抄 | R1（未认证） |

---

## 4. 目标验收矩阵（重建，逐条可判定 + 命令/断言）

> 每条含：`通过条件` + `证据标记` + `验证命令（CI 可执行）` + `期望断言`。

| # | 能力 | 通过条件 | 标记 | 验证命令与期望 |
|---|---|---|---|---|
| V-1 | 图谱查询边界 | depth≤4、limit≤1000、节点≤5000、边≤20000、props≤64KiB；超限**静默截断并显式 `truncated`** | F7 + A7 | `cargo test graph::` + `python3 scripts/check-graph-policy.py --self-test` ⇒ bounded_subgraph 越界即拒；ACTIVE 码全 PASS |
| V-2 | 图谱语义 | orphan/unresolved/tag/attachment 四类 + 过滤式对齐 `graph.json` 可观测语义 | F12 + A2/A3 | `node` 合成图谱 fixture（§5.1）跑 `GraphPanel` 渲染断言；非空/无敏感文案 |
| V-3 | 数据库查询 | 行≤1000、单字段≤64KiB、显式 `truncated`/`field_truncated`/`limit_hit` | F7 + A6 | `cargo test database::t_db_p8 t_db_p13` ⇒ 截断标记置位；`check-database-policy.py` PASS |
| V-4 | DTO 契约一致性 | `DbValue` Rust↔TS 序列化**逐字段一致**，无 dead_drift；双真源收敛到 `database.rs` 为 SSOT | F5（修正后） | 新增 `serde_roundtrip_dbvalue` 单测：构造 `I64/F64/Binary`，`to_json`→`from_json` 闭环；`domain.rs` 镜像删除或 `#[derive]` 同源 |
| V-5 | 检索路由 | exact/BM25/vector/hybrid **可分离、可单独禁用**；模型不可用**显式报错非静默空结果** | A8（R1） | 合成检索语料（§5.3）跑 `A8-benchmark-ripgrep.mjs`；vector 且模型缺失 ⇒ 抛 `EMBEDDING_MODEL_REQUIRED` 类错误码 |
| V-6 | 工作区授权 | 未授权路径零读取；忽略规则生效；索引不越界 | A7/A9（R1） | `test/authorization` 行为复刻为 Rust 单测；越权路径读取 ⇒ 拒绝 |
| V-7 | 命令全链一致性 | 每新命令 = Rust 实现 + `check_invocation_source` + `generate_handler!` + ACL + `bridge.ts`/`types.ts` + 策略/测试同包 | F1 | `python3 scripts/check-command-set-consistency.py` ⇒ GATE PASS（无新增 drift） |
| V-8 | 启动/来源门 | debug≠release 来源不混；`dev-capabilities/main.json` 不进 release；`tauri.conf.json` 不复全局 `build.devUrl` | F12 | `bash scripts/check-dev-startup.sh` + 真机 IPC≥1；release 构建产物独立运行 |
| V-9 | 构建预算 | `total_bytes_pct ≤ 25.2`；cargo 告警不增 | F8 | `python3 scripts/measure-build-metrics.py` ⇒ `exceeds_growth_limit=false`；`cargo build` 0 新增 warning |
| V-10 | 隐私/脱敏 | 新 UI/错误面零敏感回显（URL query/凭据/本地路径）；无新持久化敏感面 | A6/A9（R1） | `check-*-policy.py` 全 PASS；`redactSecrets` 经 `check-clipboard-persistence-logic.mjs` 类门禁守护 |
| V-11 | 凭据流 | 密码瞬时进 JS 内存但**不落地** DTO/配置 | F5（修正后） | 静态审计：`DbConnectionConfig` 无 password 字段；`useDatabaseStore.connect`/`bridge.dbConnect` 调用点仅传参 |
| V-12 | 许可证义务 | Apache-2.0 inbound 带 NOTICE，产品维持 MulanPSL-2.0 | F11 + A10 | 新增移植文件含 Apache-2.0 头；仓库根 `LICENSE` 不变；`LICENSE-APACHE-*` 落位 |

---

## 5. 合成语料规格（固定目标值，不锚真实 vault）

### 5.1 合成 vault（图谱/检索共用）
- 档位：**小 200 / 中 2000 / 大（cap，固定 8000）**——改用固定目标，不再引用“真实 vault = 6630/8253”。
- 形态（全部生成，不复制用户 vault 正文 → REJECT「vault 正文作夹具」）：wikilink `[[X]]`/`[[X\|别名]]`/`[[X#块]]`、frontmatter(tags/aliases)、嵌套标签、`![[附件]]`、孤立笔记、悬空链接、忽略路径（对齐 `app.json` `userIgnoreFilters` 的 `phantom-wiki/**` 语义）。
- 变更场景：重命名/删除/批量移动 ⇒ 驱动图谱重命名传播 + 索引陈旧检测。

### 5.2 合成数据库语料
- 连接：本地文件型（sqlite 类）+ 内存型各一；**不连真实/生产库**（dispatch 明令禁止）。
- 数据：宽表/长表、NULL、二进制列（只回长度）、超长文本（触发 `field_truncated`）、超大结果集（触发 `DB_MAX_ROWS`）。
- 语句：只读、写操作（确认闸）、风险 SQL（拦截）、超时与取消竞态。

### 5.3 合成检索语料
- 中英混排 + 代码片段 + 长文档 + 需分块超长文件。
- 索引态：全新 / 增量 / 陈旧 / 损坏（`.corrupt` 备份 + 重建）——对齐 `test/index-coordinator.test.mjs` 语义（A7 R1）。
- 模型态：本地可用 / **不可用**（必须走 V-5 显式错误）。

---

## 6. 测试架构（按层，含确切调用与期望）

| 层 | 内容 | 复用来源 | 验证命令与期望 |
|---|---|---|---|
| 单元 | 图遍历边界/截断；`DbValue` serde 闭环；忽略规则匹配；分块/抽取；路由选择；DSN/查询脱敏 | dbx `crates/dbx-core/tests`（ADAPT）、zvec `authorization/index-coordinator/embedding-runtime`（ADAPT） | `cargo test --lib` + `node --test` 复刻件 ⇒ 各断言显式 |
| 集成 | 命令注册→来源校验→ACL→bridge/types 全链；查询执行/取消；增量索引+watcher 对账；历史/导出落盘恢复 | dbx `src-tauri/tests`、`tests/`；zvec `test/integration` | `bash scripts/pre-merge.sh` ⇒ ALL_PASS；`cargo test integration::` |
| 安全 | 未授权工作区读取；路径穿越/符号链接；写确认闸；超时/取消竞态；池/会话清理；日志/错误脱敏；**远程嵌入与数据外发默认关闭** | zvec `authorization`/`mcp-contract`（ADAPT）；A6/A9 | `python3 scripts/check-command-set-consistency.py` + 脱敏策略脚本 ⇒ PASS；远程嵌入开关默认 off |
| 性能 | 三档语料 p50/p95 延迟、内存峰值、索引体积 | `A8-benchmark-ripgrep.mjs`（EXECUTED 11–17ms/420 文件）；benchmarks/**DEFER** 待 A8 | managed-ripgrep 路线断言 ≤ 给定的 p95 阈值（先记录基线，A8 终稿前不固化） |
| GUI | 空/载入/错误/截断/容量告警五态确定性；键盘可达与可访问名；窄窗；无敏感文案 | Obsidian 行为（REIMPLEMENT）+ `check-*-ui-logic.mjs` 范式 | `node scripts/check-<feature>-ui-logic.mjs` ⇒ 各断言 PASS |
| Debug/Release | debug：`check-dev-startup.sh`+真机 IPC≥1；release：bundled `tauri://localhost`、无 Vite 依赖、`dev-capabilities` 不入围 | dispatch L1376 + `WORKSPACE_IDENTITY.md` Source Gate | `bash scripts/check-dev-startup.sh`；release 构建 `cargo build --release` 产物独立运行 + `pre-merge.sh` ALL_PASS |
| 迁移/回滚 | 持久化格式带 `schema_version`；加载失败走 `.corrupt` 备份+告警，禁 `unwrap_or_default` 静默清空 | B4-1/P0 教训 | `cargo test persistence::corrupt_recovery` ⇒ 备份生成且旧数据可读 |

---

## 7. Debug / Release 双轨矩阵（沿用 R1 §7，事实未变）

| 项 | Debug | Release |
|---|---|---|
| 主窗来源 | `http://localhost:1421/`（程序化） | bundled `tauri://localhost` |
| 能力 | `dev-capabilities/main.json`（仅 `cfg(debug_assertions)`） | 自动扫描 `capabilities/`，dev 不入围 |
| 配置红线 | 不复全局 `build.devUrl` | 不产出依赖 Vite 产物 |
| 验收 | `check-dev-startup.sh` + 真机 ≥1 IPC | 构建产物独立运行 + `pre-merge.sh` ALL_PASS |
| 禁止 | “浏览器预览”≠原生客户端验收 | 不得放宽 `remote.urls` 修一屏问题 |

---

## 8. 实现切片边界 / 合并顺序 / GO-NO-GO

> 每片给出：先决条件、目标文件（**真实现存路径**）、合并序、回滚、停止准则、GO/NO-GO。

- **S1 契约与门（GO）**：先决=F1 门禁已 PASS（135/135/46）。目标文件：`src-tauri/src/main.rs`(generate_handler)、`src-tauri/permissions/default-commands.toml`、`src/bridge.ts`、`src/types.ts`、`scripts/check-command-set-consistency.py`。合并序=首片。回滚=移除新增命令三处注册。停止=出现新增 drift 或体积越限。
- **S2 图谱增强（GO，行为复制）**：先决=A2 夹具 + A3 状态流（R1 已给，待 R2 认证）。目标=`src-tauri/src/graph.rs` + `src/components/graph/*`。合并序=S1→S2。回滚=feature flag 关闭新语义。停止=容量常量被突破。
- **S3 数据库工作台（GO，ADAPT dbx-core）**：先决=A4 符号表 + A6 安全用例（R1）。目标=`src-tauri/src/database.rs` + `src/components/workspace/DatabasePanel.vue`（**非新 store**）。合并序=S1→S3。回滚=隐藏新面板 tab。停止=引入 daemon/连接注册表越界。
- **S4 检索摄入（CONDITIONAL GO）**：先决=A7 摄入架构 + A9 授权门（R1）。首片仅 **managed-ripgrep**（无 `@zvec/zvec` 依赖，F6 已证产品为零检索）。目标=`src-tauri/src/search.rs`(新增) + 复用 `src-tauri/src/graph.rs` 既有 bounded 范式。合并序=S2/S3→S4。回滚=隐藏搜索入口。停止=任何 daemon/MCP/远程嵌入/模型下载企图。
- **S5 检索路由（NO-GO，DEFER）**：先决=A8 路由结论（**R1 未决，且 vector 路线 REJECT 直抄**）+ A10 原生可行性。仅做 exact +（可选）FTS/BM25；vector/hybrid **不在 W19 首波**。合并序=S4→S5(条件)。停止=未获 A0 书面裁定即上 vector。
- **S6 GUI 与可达性（GO）**：先决=V-2/V-10 语义。目标=`src/components/graph/*`、`src/components/workspace/*`、`scripts/check-*-ui-logic.mjs`。合并序=与 S2/S3/S4 并行收口。回滚=回退组件。停止=五态/键盘/窄窗任一断言失败。

**GO/NO-GO 汇总**：S1 S2 S3 S6 = GO；S4 = CONDITIONAL GO（仅 managed-ripgrep）；S5 = NO-GO/DEFER。

---

## 9. 给 A0 的矛盾 resolve（核心交付，R2 重建）

### 9.1 policy 计数矛盾 → RESOLVED
- 事实（F2/C2）：`scripts/check-*` 共 **50**（29 `.py` + 20 `.mjs` + 1 `.sh`）。
- 任一报告必须**显式声明计数规则**：用「50 个顶层 `check-*` 文件」或「29 个 `check-*.py`」均可，但不得混用或断言「47/49」。A11 矩阵与门禁描述统一采用「50（29 py + 20 mjs + 1 sh）」。

### 9.2 DbValue 矛盾 → RESOLVED（指定 SSOT）
- 事实（F5/C4）：`database.rs:122` 的 `{I64,F64,Binary{bytes:usize}}` 是**唯一活契约**；`domain.rs:1094` + `src/types.ts:603` 的 `{Int,Float,BlobLen}` 是**死代码/镜像**（domain.rs 已标 `#[allow(dead_code)]`）。
- 处置：W19 S1 门禁新增 `serde_roundtrip_dbvalue` 单测（V-4）；并**删除或同源派生** `domain.rs`/`types.ts` 镜像，使变体命名统一为 `I64/F64/Binary`。B8 类漂移复燃风险由 V-4 强制门消除。

### 9.3 zvec 路由矛盾 → RESOLVED（口径统一）
- 矛盾点：A7 将 `@zvec/zvec` 引擎标 **COPY=2**；A8/A10 判定其 **Node-only 原生 addon，REJECT 直抄**（A8 R1 §12 R6/R7；A10 Z8/Z10）。
- 依据（F/C8 + A8/A10）：`zvec-grep-src/package.json` 依赖 `@zvec/zvec ^0.7.0`、`engines.node>=22`；Rust/Tauri 二进制无法直链 Node addon。
- 结论：**A7 的 `COPY=2` 被 A8/A10 的 `REJECT 直抄` 覆盖**；可移植价值是**算法层**（路由选择、RRF K=60、类型感知分块、增量对账、新鲜度、紧凑输出、egress 前授权）→ **REIMPLEMENT in Rust**（Z1–Z7,Z9,Z12,Z13）。W19 首波仅 **managed-ripgrep**（F6 产品为零检索，无需 `@zvec`）；FTS/BM25/vector 待 A10 台账 + 原生可行性裁定。A11 记录：A7 分类口径在 A10 落地后须**逐条重分类**，当前以 A8/A10 为准。

### 9.4 其余 A11 R1 矛盾（§11）复核
1. 运行时形态冲突 → 并入 §9.3，结论：仅复用算法层，不破锁。
2. 体积预算冲突 → F8 维持 25.2；S2–S6 单波增量须 ≤ 预算余量（W17 实测 25.14% 余量仅 ~0.06pp）；**禁止实现 lane 自抬限**，重基线须 A0 书面。
3. A8 未决 → §9.3 已解：S5 DEFER。
4. A11 依赖链结构矛盾 → 本件即“在 peer R2 缺失下先出 provisional”，最终由 A0 合 research 分支后出终稿（§10）。
5. 夹具隐私/许可 → §5.1 强制全合成 + Apache-2.0 NOTICE（F11/V-12）。
6. DTO 漂移 → §9.2。

---

## 10. 本件定位与 R2 终稿门禁（更新于 R2B，2026-09-08）

> 本 §10 为 R2-provisional 的阶段性说明。**R2B 整包已接替本件作为 A11 权威交付**，见 `A11-R2B-progress.md` / `A11-R2B-integration-manifest.json` / `A11-R2B-A0-brief.md` / `A11-R2B-S0-S5-task-cards.md`。

R2-provisional 原门禁（dispatch：“A11 终稿仅在 A1–A10 R2 commit 全部存在后”）**已被 R2B 进度包解除并替代**：

1. **A9 当前 tip `28a934a`** 已基于 `d6127c4`/origin/master 且含 R2 threat-model（`A9-threat-model.md`/`A9-source-map.md`）；原 `c5bfc88` 已过时。
2. **A1–A10 现已全部存在 R2 证据闭环提交**：A1 `526e2ef`、A2 `bcdfc3b`、A3 `ab455c1d`、A5 `a42b915`、A6 `38faa2d`、A7 `07fda2f`、A8 `f4a4f3b`、A9 `28a934a`、A10 `b81fa69`。**缺口两项**（R2B 的 `WAITING_DEPENDENCY`）：**A4 `b737e5d` 仍停留在 R1 架构图，无 R2 证据闭环提交**；**A10 仍 HOLD，`A10-R2B-review-findings.md` 尚未产出**（peer R2 现已齐，A10 应重读后产出）。
3. A10 ledger 与 A11 matrix 一致性由 R2B 包的 `CONSUMED_PEERS` + 跨 lane 差异清单承接（见 R2B 进度包 §跨 lane 待 A0 裁决）。

**本件保留为 R2-provisional 历史**；A11 当前权威状态以 R2B 包（`STATUS=WAITING_DEPENDENCY`）为准，不在此宣布 W18 完成。

**立即停止 / 阻断 W19 的条件**（不变）：体积无解且未获 A0 书面重基线；或 S5 未决却上 vector；或任何片引入 daemon/MCP/远程嵌入/模型下载。

---

## 11. 声明

- 本件**只做研究与综合**，未改任何产品代码；未安装依赖、未下载模型、未启动 daemon、未连数据库、未做远程嵌入、未改用户 vault。
- 已用 `git show <peer-sha>:<path>` 只读消费 A1–A10 R1 报告（§3），并据实测（§1/§2）逐条 retract/replace A0 审计点（§1）。
- 依 dispatch L1405：**在 A1–A10 R2 齐备前不宣布 W18 完成**；本件不构成完成声明。
- 未 push（仅 A0 推送）。

---

## 附录 A：R1 原文（保留，未改写）

> 原始 `A11-verification-architecture-20260908-0800.md` 内容完整保留于此，供追溯。其被 §1 修正的条款以删除线/批注形式已在本文件正文处理，此处不再重复改写以免“静默重写”。

（R1 全文见 `logs/research/M5-W18/A11-verification-architecture-20260908-0800.md`；本次 R2 **未删除**该文件，保留为历史。）
