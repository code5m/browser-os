# A11 · M5-W18-R 验证架构综合（Verification Architecture Synthesis）

```text
LANE=A11
STATUS=PASS_WITH_DEBT（**provisional**：A1–A10 报告尚不可得，本综合以「当前产品 + pinned 参考源」独立取证，并逐条标注待补输入）
BASE=78d2cfb
HEAD=<lane research commit，提交后回填>
REFERENCE_EVIDENCE=
  /home/ainfinit/Documents/极智简单/V3/research/dbx-src（Cargo.lock SHA-256 c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7，Apache-2.0）
  /home/ainfinit/Documents/极智简单/V3/research/zvec-grep-src（upstream zvec-ai/zvec-grep @ 52653951b24617762f4ab0c71c34d594e5001617，Apache-2.0）
  /home/ainfinit/Documents/Knowledge-Base/secondBrain/.obsidian（graph.json / app.json / core-plugins.json / workspace.json / appearance.json）
  /home/ainfinit/Documents/极智简单/V3/dbx-study
  vault 规模：8253 个 *.md（仅作合成语料量级参考，**未读取/未复制正文，未改动 vault**）
FILES=logs/research/M5-W18/A11-verification-architecture-20260908-0800.md ; logs/checkpoints/A11-M5-W18-R-20260908-0800.md
SOURCE_MAP=dbx-src: crates/{dbx-core,dbx-cli,dbx-mcp,dbx-web}, apps/desktop, crates/dbx-core/tests, crates/dbx-mcp/tests, src-tauri/tests, tests, agents/drivers/{duckdb,tdengine}/tests ; zvec-grep-src: src/{authorization,cli,client,daemon,engine,mcp,observability,prompts,index.ts,index-progress.ts}, test/{authorization,change-set,config,config-cli,daemon-backend,daemon-cache,daemon-logger,embedding-runtime,index-coordinator,job-scheduler,mcp,mcp-contract,mcp-legacy-http,mcp-modern-http,mcp-request-state,install}.test.mjs + test/{e2e,integration,fixtures,helpers}, benchmarks/{browse-comp-plus,swe-qa-bench} ; 本产品: src-tauri/src/{graph.rs,database.rs,domain.rs,security_policy.rs,bridge.rs,main.rs}, scripts/*（47 门禁脚本）, src-tauri/permissions/*, src-tauri/dev-capabilities/main.json
CLASSIFICATION=本综合对「验证面」的 provisional 分类：COPY 0（验证脚本不可盲抄，均需适配）· ADAPT 9（zvec-grep authorization/embedding-runtime/index-coordinator/mcp-contract/daemon-* 五类 + dbx-core/db integration + dbx drivers tests）· REIMPLEMENT_FROM_BEHAVIOR 6（Obsidian graph.json 语义、ignore filters、orphan/unresolved、backlink/outgoing、local graph 交互、设置持久化）· DEFER 3（向量/嵌入基准、daemon/MCP 生命周期、模型下载）· REJECT 2（Node>=22 运行时整体引入、vault 正文直接作夹具）
VERIFY=见 §3 只读盘点命令与结果（无任何产品代码改动、无依赖安装、无 daemon/网络/模型下载、未触碰用户 vault）
CHECKPOINT=logs/checkpoints/A11-M5-W18-R-20260908-0800.md
MERGE_NOTES=强依赖 A1–A10；在 A0 集成其报告前本综合为 provisional。冲突点见 §11（尤其构建体积预算与运行时锁定）。
NEXT=待补输入：A1 差距清单、A2 vault 语义夹具、A3 图谱 UX 状态流、A4 dbx 后端符号表、A5 dbx UX 蓝图、A6 dbx 安全用例、A7 zvec 摄入架构、A8 检索基准、A9 信任边界、A10 来源账本/BOM。
```

---

## 1. 方法与边界

- 工作区：`/home/ainfinit/.codex/worktrees/m5-w18-a11/mvp-browser-os-v3`，分支 `codex/m5-w18-a11`，已 rebase 至 `origin/master` = `78d2cfb`（rebase 为 no-op）。
- **未编辑 canonical master 工作副本**（W18-R 硬要求）；未触碰 `src/`、`src-tauri/`、`tauri-browser-tabs/`、生产 `scripts/`、权限/能力、清单与构建配置。
- 未安装依赖、未下载模型、未启动 daemon、未连数据库、未做远程嵌入、未改用户 vault（仅读取 `.obsidian` 配置与统计 `.md` 数量）。
- 分类法遵循 dispatch L1383：`COPY` / `ADAPT` / `REIMPLEMENT_FROM_BEHAVIOR` / `DEFER` / `REJECT`。

---

## 2. 待补输入账本（A1–A10 未到位）

`logs/research/M5-W18/` 为空 → 依 dispatch L1410，A11 继续独立分析并**逐条标注**待补输入，不以臆测替代：

| 依赖 | 我在本综合中的替代处理 | 到位后需回填处 |
|---|---|---|
| A1 当前产品差距清单 / 需求检查表 | 用 §3 自行的只读盘点替代（47 门禁 + graph/db 模块 + 容量常量） | §4 验收矩阵需按 A1 差距清单重排优先级 |
| A2 vault 语义与夹具 | 用 `.obsidian/graph.json`、`app.json` 的**可观测配置**替代（§5.1） | 合成 vault 夹具的链接/别名/frontmatter 细节 |
| A3 图谱 UX 状态流 | 用 `graph.json` 字段语义替代（§5.1、§6 GUI） | 组件/状态机蓝图与窄窗行为 |
| A4 dbx 后端符号表 | 用目录/测试布局替代（§3.2） | 逐符号移植清单与适配点 |
| A5 dbx UX 蓝图 | 未替代 → 标 DEFER | 结果网格/编辑器/执行栏用例 |
| A6 dbx 安全用例 | 用本产品既有 db 策略（ACTIVE=14）与容量常量替代（§6 安全） | 强制 W19 策略/测试用例 |
| A7 zvec 摄入架构 | 用 `src/` 目录与 `test/` 布局替代（§3.3） | 清单/索引格式与增量新鲜度断言 |
| A8 检索基准与采纳结论 | **不可替代理** → 向量/嵌入相关一律 DEFER（§11-3） | 路由选择（exact/BM25/vector/hybrid）与延迟预算 |
| A9 信任边界 | 用 dispatch 既有锁定 + `test/authorization.test.mjs` 存在性替代 | 威胁模型与「必须保持禁用」清单 |
| A10 来源账本/BOM | 未替代 → 标 DEFER | 许可证/NOTICE 义务与 no-blind-copy 闸门 |

---

## 3. 当前验证资产基线（只读盘点）

### 3.1 本产品
| 资产 | 实测 |
|---|---|
| 门禁脚本 | `scripts/` 共 47 个 `check-*`（`.py`/`.mjs`）+ `pre-merge.sh`、`baseline-check.sh`、`verify-resources.sh`、`measure-build-metrics.py`、`check-dev-startup.sh`、`check-command-set-consistency.py` |
| 命令一致性 | `check-command-set-consistency.py` **已存在**（B12-02 已闭环） |
| 启动门 | `check-dev-startup.sh` 已存在；debug 能力 `src-tauri/dev-capabilities/main.json`（`#[cfg(debug_assertions)]`，位于自动扫描目录之外） |
| 图/库模块 | `src-tauri/src/graph.rs`、`src-tauri/src/database.rs`（**无 search/vector 模块**） |
| 容量常量 | `GRAPH_MAX_DEPTH=4`、`GRAPH_QUERY_LIMIT=1000`、`GRAPH_MAX_NODES=5000`、`GRAPH_MAX_EDGES=20000`、`GRAPH_PROPS_MAX_BYTES=64KiB`、`DB_MAX_ROWS=1000`、`MAX_TEXT_FIELD_BYTES=64KiB` |
| 构建预算 | `TOTAL_BYTES_GROWTH_LIMIT_PCT=25.2`（W15 一次性上限） |

### 3.2 dbx 参考源
- crates：`dbx-core`、`dbx-cli`、`dbx-mcp`、`dbx-web`；应用：`apps/desktop`。
- 测试落点：`crates/dbx-core/tests`、`crates/dbx-mcp/tests`、`src-tauri/tests`、根 `tests/`、`agents/drivers/duckdb/tests`、`agents/drivers/tdengine/tests`、`packages/mcp-server/tests`。
- → 可 ADAPT 的验证面：连接注册表、驱动能力、schema 发现、查询执行/取消、历史、导出、SQL 风险分析、错误与关停。

### 3.3 zvec-grep 参考源
- **TypeScript / Node ≥22**（`package.json`：`"type":"module"`、`engines.node>=22`、Apache-2.0、v0.2.1）。
- `src/`：`authorization`、`cli`、`client`、`daemon`、`engine`、`mcp`、`observability`、`prompts`、`index.ts`、`index-progress.ts`。
- `test/`：`authorization`、`change-set`、`config`、`config-cli`、`daemon-backend`、`daemon-cache`、`daemon-logger`、`embedding-runtime`、`index-coordinator`、`job-scheduler`、`mcp`、`mcp-contract`、`mcp-legacy-http`、`mcp-modern-http`、`mcp-request-state`、`install` 等 + `e2e/`、`integration/`、`fixtures/`、`helpers/`。
- `benchmarks/`：`browse-comp-plus`、`swe-qa-bench`。
- → 可 ADAPT 的验证面：workspace 授权、增量/新鲜度、索引协调、嵌入运行时不可用降级、MCP 契约；**benchmark 需 A8 结论后才可启用**。

### 3.4 Obsidian 可观测配置（行为规格来源）
- `graph.json`：`showOrphans=true`、`hideUnresolved=false`、`showTags=false`、`showAttachments=false`、`collapse-filter=false`、`showArrow=false`、`textFadeMultiplier=0`、`nodeSizeMultiplier=1`、`lineSizeMultiplier=1`、`centerStrength=0.5187`、`repelStrength=10`、`linkStrength=1`、`linkDistance=250`、`scale=0.667`、`colorGroups=[]`、`search` 含 `-(path:... AND ext:...)` 过滤式。
- `app.json`：`userIgnoreFilters` = `phantom-wiki/**` 下 `*.py/*.pyc/*.sh/*.pkl/*.json/.gitignore/requirements.txt/__pycache__/.venv`。
- `core-plugins.json`：`graph`、`backlink`、`outgoing-link`、`tag-pane`、`global-search`、`properties`、`page-preview` 启用；`footnotes` 关闭。
- → 这些是**行为复制的验收锚点**（orphan/unresolved/tag/attachment/过滤/力导参数），非代码来源。

---

## 4. 目标验收矩阵（Target Acceptance Matrix）

> 每条给出「可判定的通过条件」与「判据来源」；凡依赖 A1–A10 者标注 `[待补]`。

| # | 能力 | 通过条件（可判定） | 判据来源 |
|---|---|---|---|
| V-1 | 图谱查询边界 | depth≤`GRAPH_MAX_DEPTH`、limit≤`GRAPH_QUERY_LIMIT`、节点≤5000、边≤20000、props≤64KiB；超限**静默截断并显式 `truncated`**，不报错 | `domain.rs:2188-2201` + `check-graph-policy.py` |
| V-2 | 图谱语义 | orphan/unresolved/tag/attachment 四类节点与过滤式结果对齐 `graph.json` 可观测语义 | `.obsidian/graph.json` + A2/A3 `[待补]` |
| V-3 | 数据库查询 | 行数≤`DB_MAX_ROWS`、单字段≤64KiB、`truncated`/`field_truncated`/`limit_hit` 显式 | `domain.rs:1019-1034` + `check-database-policy.py` |
| V-4 | 数据库 DTO 契约 | `DbValue`/`SkillDef`/`AgentDef` 的 Rust↔TS 序列化大小写**逐字段一致**（B8-1/B8-2 不得复燃） | BUG-HUNT B8-1/B8-2 + A7/A1 |
| V-5 | 检索路由 | exact / BM25 / vector / hybrid 路由可分离、可单独禁用；模型不可用时**明确降级**而非静默错 | zvec `src/engine` + A8 `[待补]` |
| V-6 | 工作区授权 | 未授权路径零读取；忽略规则生效；索引不越界 | zvec `test/authorization.test.mjs` + A7/A9 |
| V-7 | 命令一致性 | 每个新命令 = Rust 实现 + `check_invocation_source` + `generate_handler!` + ACL + `bridge.ts`/`types.ts` + 策略/测试**同包落地** | `check-command-set-consistency.py` |
| V-8 | 启动/来源门 | debug 与 release 来源不混；`dev-capabilities/main.json` 不进 release；`tauri.conf.json` 不得恢复全局 `build.devUrl` | dispatch L1369-1376（non-repeat gate） |
| V-9 | 构建预算 | `total_bytes_pct` ≤ 25.2；cargo 告警不增 | `measure-build-metrics.py` |
| V-10 | 隐私/脱敏 | 新 UI 与错误面零敏感回显（URL query/凭据/本地路径）；无新持久化敏感面 | `check-*-policy.py` + A6/A9 |

---

## 5. 合成语料规格（Synthetic Corpora）

### 5.1 合成 vault（图谱/检索共用）
- 规模档位：**小 200 / 中 2 000 / 大 8 253**（对齐真实 vault `.md` 数量作上限压力档）；全部为**生成内容，不复制用户 vault 正文**（避免隐私与许可问题 → REJECT「vault 正文直接作夹具」）。
- 必备形态：wikilink（`[[...]]`/`[[...\|别名]]`/`[[...#块]]`）、frontmatter（含 tags/aliases）、标签（`#tag` 与嵌套）、附件引用、孤立笔记、悬空链接（unresolved）、忽略路径（对齐 `userIgnoreFilters` 语义）。
- 变更场景：重命名、删除、批量移动 → 驱动图谱/索引的重命名传播与陈旧检测。

### 5.2 合成数据库语料
- 连接：本地文件型（如 sqlite/duckdb 类）与内存型各一；**不接真实/生产库**（A10 dispatch 明令禁止数据库连接）。
- 数据：宽表/长表、NULL、二进制列（只回长度）、超长文本（触发 `field_truncated`）、超大结果集（触发 `DB_MAX_ROWS`）。
- 语句：只读、写操作（触发确认闸门）、风险 SQL（风险分析/拦截）、超时与取消竞态。

### 5.3 合成检索语料
- 中英混排 + 代码片段 + 长文档；包含需分块的超长文件。
- 索引态：全新、增量更新、陈旧（stale）、损坏（触发重建）—— 对齐 `test/index-coordinator.test.mjs` 语义。
- 模型态：本地可用 / **不可用**（必须走 A8 的降级结论）。

---

## 6. 测试架构（按层）

| 层 | 内容 | 复用来源（分类） |
|---|---|---|
| **单元** | 图遍历边界与截断；`DbValue` 序列化；忽略规则匹配；分块/抽取；路由选择；DSN/查询脱敏 | dbx `crates/dbx-core/tests`（ADAPT）、zvec `test/{authorization,index-coordinator,embedding-runtime}`（ADAPT） |
| **集成** | 命令注册→来源校验→ACL→bridge/types 全链；查询执行与取消；增量索引与 watcher 对账；历史/导出落盘与恢复 | dbx `src-tauri/tests`、`tests/`（ADAPT）；zvec `test/integration`（ADAPT） |
| **安全** | 未授权工作区读取；路径穿越/符号链接；写确认闸门；超时/取消竞态；池/会话清理；日志与错误脱敏；**远程嵌入与数据外发必须默认关闭** | zvec `test/authorization.test.mjs`、`test/mcp-contract.test.mjs`（ADAPT）；A6/A9 `[待补]` |
| **性能** | 三档语料的 p50/p95 延迟、内存峰值、索引体积；与 A8 基准对齐后方可设阈值 | `benchmarks/{browse-comp-plus,swe-qa-bench}`（**DEFER**，待 A8） |
| **GUI** | 空/载入/错误/截断/容量告警五态确定性；键盘可达与可访问名；窄窗；**无敏感文案** | Obsidian 行为（REIMPLEMENT_FROM_BEHAVIOR）+ 既有 `check-*-ui-logic.mjs` 范式 |
| **Debug/Release** | debug：`check-dev-startup.sh` + 真机 IPC 至少一条；release：bundled `tauri://localhost`、无 Vite 依赖、`dev-capabilities` 不入围 | dispatch L1376 强制门 + `WORKSPACE_IDENTITY.md` Desktop Runtime Source Gate |

---

## 7. Debug / Release 双轨矩阵

| 项 | Debug | Release |
|---|---|---|
| 主窗口来源 | `http://localhost:1421/`（程序化加载） | bundled `tauri://localhost` |
| 能力 | `src-tauri/dev-capabilities/main.json`，仅 `#[cfg(debug_assertions)]` | 自动扫描 `src-tauri/capabilities/`，**dev 能力不得入围** |
| 配置红线 | 不得恢复全局 `build.devUrl` | 不得产出依赖 Vite 的产物 |
| 验收 | `check-dev-startup.sh` 通过 + 真机跑通 ≥1 条 IPC | 构建产物独立运行 + `pre-merge.sh` ALL_PASS |
| 禁止 | 「浏览器预览」不得称为原生客户端验收 | 不得用放宽 `remote.urls` 修一屏问题 |

---

## 8. 迁移 / 回滚与容量闸门

- **持久化格式**必须带 `schema_version`；加载失败走「`.corrupt` 备份 + 告警」，**禁止** `unwrap_or_default()` 静默清空（对齐 B4-1/P0 教训）。
- **回滚**：每片实现Slice 前置「格式兼容 + 数据可读」测试；新索引/缓存可整体删除重建且不影响既有数据。
- **容量闸门**：图、库、检索三面各自的 rows/nodes/bytes 上限必须以**单一常量真源**表达并有单测守护（沿用 `GRAPH_PROPS_MAX_BYTES == MAX_TEXT_FIELD_BYTES` 的同量级约束范式）。

---

## 9. 实现切片边界与合并顺序

建议 W19 切片（每片独立可验收、可回滚）：
1. **S1 契约与门**：新命令全链路骨架（实现+来源校验+注册+ACL+bridge/types+`check-command-set-consistency.py`）→ 先立门再填肉。
2. **S2 图谱增强**：orphan/unresolved/tag/attachment + 过滤式（行为复制）。
3. **S3 数据库工作bench**：连接/查询/取消/结果/导出（ADAPT dbx-core）。
4. **S4 检索摄入**：工作区授权 + 忽略规则 + 增量索引（ADAPT zvec，禁 daemon/MCP）。
5. **S5 检索路由**：先 exact + BM25；**vector/hybrid 待 A8 结论**（DEFER）。
6. **S6 GUI 与可达性**：五态 + 键盘/窄窗 + 无敏感文案。

**合并顺序**：`S1 → S2/S3 → S4 → S6 → S5(条件性)`；每片合并前必须 `check-command-set-consistency.py` + `check-dev-startup.sh` + focused tests + `npm run build` + `pre-merge.sh` 全绿（dispatch L1376）。

---

## 10. 停止准则（Stop Criteria）

**W18-R 完成当且仅当**：
1. A1–A10 全部报告存在且互不矛盾（A11 本件为 provisional，末位收口）；
2. §4 验收矩阵每条均有可判定判据与对应测试用例；
3. §5 三套合成语料规格被 A2/A4/A7 确认可生成；
4. A10 来源账本/BOM 与许可证义务完整；
5. §11 全部矛盾项由 A0 逐条裁定。

**立即停止 / 阻断 W19 的条件**：体积预算无解且未获 A0 书面重基线与理由；或 A8 未给路由结论却要上 vector；或任何片试图引入 daemon/MCP/远程嵌入/模型下载。

---

## 11. 给 A0 的矛盾与风险（核心交付）

1. **运行时形态冲突（高）**：zvec-grep 为 **TypeScript/Node ≥22**，本产品为 Tauri2+Rust。其多半差异化能力（daemon、MCP、远程嵌入）恰在本产品**锁定清单**内（禁 daemon/网络/模型下载/MCP 暴露）。→ 需 A0 裁定：是「仅复用其**行为与测试语义**（REIMPLEMENT/ADAPT）」，还是为引入 Node 运行时**破锁**？后者与「无新依赖、体积预算」直接冲突。
2. **体积预算冲突（高）**：当前 `total_bytes_pct` 已贴 25.2 上限；dbx（多 crate + duckdb/tdengine 驱动原生依赖）与向量/嵌入能力叠加几乎必然越限。→ 需 A0 在「缩预算/切片取舍」与「重基线（须书面理由，board 已要求记录基线决策）」间裁定，不得由实现 lane 自行抬限。
3. **A8 未决导致 S5 不可排期（中）**：exact/BM25/vector/hybrid 路由与延迟预算缺证据。→ 建议 W19 先只做 S4（摄入+精确/BM25），vector 另案。
4. **A11 依赖链结构性矛盾（中）**：A11 须消费 A1–A10，但各 lane 写入**各自 worktree**且仅 A0 集成 → 在 A0 集成前 A11 无法真正消费。→ 建议 A0 在收口轮把 A1–A10 报告合入一条 research 分支后再让 A11 出一版**终稿**。
5. **夹具隐私/许可（中）**：真实 vault 8253 篇正文不可直接作夹具（用户数据 + Obsidian 非开源 donor）。→ 强制全合成（§5.1），仅「配置语义」可参考。
6. **B8 类契约漂移复燃风险（中）**：`DbValue` 大小写漂移已确认且通路在线；新增大量 DTO 会放大该风险。→ 建议把「Rust↔TS 序列化一致性 fixture」提升为 S1 的强制门（对齐 A7 W19 任务）。

---

## 12. 声明

- 本件**只做研究与综合**，未改任何产品代码；未安装依赖、未下载模型、未启动 daemon、未连数据库、未做远程嵌入、未改用户 vault。
- 未消费 A1–A10 报告（其目录为空），全部结论标注为 provisional 并逐条给出待补输入（§2）。
- 依 dispatch L1405：**在所有报告齐备前不宣布 W18 完成**；本件不构成完成声明。
- 未 push（仅 A0 推送）。
