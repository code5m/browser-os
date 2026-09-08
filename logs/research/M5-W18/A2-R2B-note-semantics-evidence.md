# A2 · M5-W18-R2B 笔记语义与可复现行为证据

> Lane A2 是 **RESEARCH + DESIGN ONLY**。本卡（任务板 §3 "Lane A2" + `M5-W18-R2B-TASKS-20260908.md` §3）
> 在 R2 已交付成果上补充：① 把 fixture/harness 从易失的 `/tmp` 迁入 lane 研究目录并验证可复现；
> ② 为每个规则分别标注 **官方 / 真实观察 / 设计选择 / 兼容差异**；③ **撤回**"自写 resolver 20/20 即证明
> Obsidian 行为"的表述；④ 定义笔记资源 ID、来源位置、反链/出链、局部图谱输入、失效刷新、有界导入；
> ⑤ 给出笔记导航 + 图谱派生的实现卡（PROPOSED_NOT_AUTHORIZED）、序列化样例、反例与旧数据兼容。
> 不修改产品代码 / 其他 lane / 主矩阵，不读密钥、不记录私库正文。

## 证据标签

`CURRENT_PRODUCT` / `REFERENCE_SOURCE` / `OBSERVED_BEHAVIOR` / `OFFICIAL_DOC` / `EXECUTED_SYNTHETIC_TEST` /
`INFERENCE` / `DESIGN_DECISION` / `CONSUMED_PEERS`。每个事实标明其一，附精确 path/symbol/行 或 命令/结果。

## 0. 复用旧成果 & R2B 撤回/更正日志

- **复用**：R2 报告 `A2-obsidian-vault-semantics.md`（commit `bcdfc3b`）、R2 fixtures `A2-fixtures.md`、
  本地 `.obsidian` 聚合统计（见 R2 §4，无正文外泄）、产品 `graph.rs`/`domain.rs`/`main.rs`/`workspace.rs`/
  `bridge.rs` 现状（R2 §2）。
- **R2B 撤回（按 A0 audit #4 + 任务板 §3）**：
  1. R2 §5 把"自写 `resolve.py` 20/20"表述为规则证明 —— **撤回**。正确的表述：`EXECUTED_SYNTHETIC_TEST`
     只证明该 harness 自身规则的内部一致性（确定性），**不等同** Obsidian 行为。Obsidian 行为须以官方文档
     / 真实库观察独立证实后，W19 方可采纳。本报告中所有 `EXECUTED_SYNTHETIC_TEST` 仅是"我的实现按此规则自洽"，
     凡涉及"Obsidian 实际如此"的结论一律标 `INFERENCE`（官方原文本次未取到）或 `OBSERVED_BEHAVIOR`（真实库可测项）。
  2. R2 中 `[[Alpha]]`→`Alpha.md` 的"filename beats alias"在真实库可测（库内确有 `Alpha.md` 文件 + `A.md` 的
     `aliases:[Alpha]`），属 `OBSERVED_BEHAVIOR`；但"通用优先级在所有场景都成立"仍属 `INFERENCE`，待官方文档确认。
  3. 标题/块/embed（`#Heading`、`#^id`、`![[x]]`）在本库几乎为 0 出现 → 仅 `EXECUTED_SYNTHETIC_TEST` +
     `INFERENCE`，**不可**标 `OBSERVED_BEHAVIOR`。
- **R2 旧 6 条修正**仍保留（R2 §1）：产品已有 graph UI；图持久化为 JSON 快照非 SQLite；id 为 sha256 派生；
  正文不入图；rename/delete 与 heading/block 现已有可执行 fixture。

## 1. 消费的对等成果（CONSUMED_PEERS）

| Peer | SHA | 消费内容 | 用途 |
|---|---|---|---|
| A1 | `a54eed1` | `A1-product-baseline-R2-20260908.md`（其 U4：产品 graph snapshot 格式是否兼容 Obsidian `graph.json` 待 A2 比对） | 文档定位契约：笔记在 A1 工作台壳层中归入"知识/文档"工具窗口；A2 的笔记导航须对齐该定位，不另起导航范式。 |
| A1 / A0 | `WORKBENCH_BLUEPRINT-20260908.md`、`M5-W18-R2B-TASKS-20260908.md` | J1-J6、S0-S5、文档→工具窗口映射 | 笔记打开/返回/局部上下文的资源契约来源。 |
| A3 | `877b4f5` | `A3-obsidian-graph-search-ux-20260908-0759.md` | 下游消费者：本报告的 filter/backlink/group/orphan/unresolved 规则须可直接映射进 `GraphPanel`/`useGraphStore`。 |

注：A1 U4 要求 A2 比对"产品 graph snapshot 格式 vs Obsidian `graph.json`"。结论见 §6（二者**不兼容**：产品
`graph.json` 是 `GraphStore` 节点/边快照；Obsidian `graph.json` 是**视图配置**，非节点数据 → 切勿把整份
Obsidian 配置当图谱数据导入）。

## 2. 逐规则证据表（官方 / 真实观察 / 设计选择 / 兼容差异）

> `OFFICIAL_DOC` 列：本次 `web_fetch https://help.obsidian.md` 仅返回 SPA 外壳（JS 渲染），**未取得规则原文**。
> 故官方列统一为"未取到原文，标 `INFERENCE`/待 W19 对照官方文档"。

| 规则 | 官方(OFFICIAL_DOC) | 真实观察(OBSERVED_BEHAVIOR) | 设计选择(DESIGN_DECISION) | 兼容差异 / 风险 |
|---|---|---|---|---|
| 文件名 / alias 解析 | 未取到原文（`INFERENCE`：basename→alias） | 库内 `Alpha.md` 存在且 `A.md` 含 `aliases:[Alpha]`；`[[Alpha]]` 命中文件（非 A 自链） | 解析序：exact path → basename → alias | 若库无同名文件则 alias 生效；优先级须官方确认。 |
| 同名路径消歧 | 未取到原文 | `find` 得 840 个重名 basename（最大 48 文件同基名） | 最短 vault 相对路径胜；同长按字典序 | 真实库大量重名 → 消歧为**必需**，非可选。 |
| 大小写 | 未取到原文（`INFERENCE`：多数文件系统大小写不敏感） | `[[ALPHA]]` 在合成库命中 `Alpha.md` | 统一 NFC + casefold 归一后匹配；保留原 basename 作 label | 跨平台大小写敏感 FS 可能行为不同 → `INFERENCE`。 |
| 标题 `#H` / 块 `#^id` | 未取到原文（`INFERENCE`） | 全库仅 11 个 `#` wikilink、1 行 `#^id`、0 embed → **不可**以 `OBSERVED_BEHAVIOR` 验证 | 锚点匹配目标笔记标题文本归一；缺锚点不算 unresolved | 仅合成 fixture + `INFERENCE`；W19 需对照官方锚点规范。 |
| 附件 / embed `![[x]]` | 未取到原文（`INFERENCE`：transclusion） | 7,888 个附件文件；0 `![[` embed | `![[x]]` **仅**作 embed，绝不可同时计为 broken wikilink（`EXECUTED_SYNTHETIC_TEST` 证明双重计数 bug） | 附件是非 `.md` 目标，非"缺失笔记"。 |
| 忽略规则 | 未取到原文 | `.obsidian/app.json` `userIgnoreFilters`（glob `phantom-wiki/**/*.{py,...}`）；`graph.json` `search`（布尔查询） | 两层：① glob 预扫描（解析前）② 视图期查询求值 | 该 `search` 当前指向 `phantom-wiki/**`（0 `.md`）→ 陈旧配置，仅作兼容样例。 |
| 重命名 / 删除 | 未取到原文（`INFERENCE`：链接改写 + 级联 unresolved） | 不可直接观察（写路径，R2 已 DEFER） | 重命名改 id（id=路径派生）→ 旧 id 边变 unresolved，须事务内回填；删除级联 inbound 为 unresolved | **写路径 DEFER**，仅读派生已闭环；解析须在重命名后保持稳定（§4）。 |

## 3. 笔记资源 ID / 来源位置 / 反链出链 定义

- **资源 ID**：`id = sha256_hex(format!("note:{}", vault_rel_nomd))`（`CURRENT_PRODUCT`：`domain.rs:2082`
  现有方案为 `sha256("kind:path")`；`graph.rs:126` `validate_id` 仅对 Skill/Agent 要求 64-hex）。笔记 id 用
  `note:` 前缀 + 相对路径（去 `.md`），≤512 字节，过 `validate_id`。附件：`sha256("attachment:"+rel)`。
- **来源位置**：vault 根由 A9 工作区授权提供（`workspace::notes_dir` 现状为 `~/Documents/极智笔记`，与 Obsidian
  库不同根 → A9 须给"vault 根"授权路径，非 dialog 插件，因 `capabilities/default.json` 无 fs dialog）。
- **出链**：每笔记解析 `[[..]]`/`![[..]]`/`#tag`/`#H` → `GraphEdge`（kind=References/Embed/TaggedWith/InDir）。
- **反链**：查询期对 `GraphEdge` 取逆（`EXECUTED_SYNTHETIC_TEST` backlink 断言：A 的 inbound = 8 个，
  排除 `sub/Note` 的 `[[Alpha]]`→`Alpha`）。
- **局部图谱输入**：`GraphQueryRequest`（domain.rs:2143）已含 `depth` + `filters`；局部图 = focus 节点 + 方向
  （出/入）+ `depth` 跳数 + tag/folder/link-type 过滤 + `hideUnresolved`/`showOrphans`。无需新增查询契约，
  仅需派生层填满 `GraphState`（R2 §6）。
- **失效刷新**：watcher-free、显式触发（遵循 A7/A8 的"无守护、显式触发"约束）。导入时记录每文件 `mtime`；
  提供命令 `graph_import_vault` 在"库 mtime 变化"或用户手动触发时重派生并替换 `GraphState`。**不**引入后台
  watcher（A7 拥有 watcher 设计，A2 不越界）。
- **有界导入**：`GRAPH_MAX_NODES=5000`（`graph.rs` 常量，R2 §2）。真实库 8,370 笔记 > 上限 → 两种设计选择：
  (a) A1 容量预算决策抬限（= 改一个 const，但 `graph.json` 与内存增大）；(b) 仅导入用户授权的子树（vault 根
  下子目录）。属 `DESIGN_DECISION`，阻塞项交 A1（同 R2 B1）。

## 4. 序列化样例 / 反例 / 旧数据兼容

**正常 `GraphNode` 序列化（对齐 `domain.rs:2082`）**
```json
{ "id":"a1b2...fe", "label":"Alpha", "kind":"Note",
  "props":{ "aliases":["Alpha"], "tags":["proj","active"], "rel_path":"Alpha.md", "mtime":1694000000 },
  "edges":[ {"id":"e1","source":"a1b2...fe","target":"c3d4...","kind":"References","unresolved":false} ] }
```
**反例（broken link → placeholder，绝不崩溃）**
```json
{ "id":"sha256(note:Missing Note)", "label":"Missing Note", "kind":"Note", "props":{"exists":false},
  "edges":[ {"source":"A","target":"sha256(note:Missing Note)","kind":"References","unresolved":true} ] }
```
**旧数据兼容（DESIGN_DECISION）**：产品 `load_snapshot`（graph.rs:395）读的是 `GraphStore` 节点/边快照；
Obsidian `graph.json` 是**视图配置**（filters/layout/colorGroups），二者 schema 不同、**不可互换**。W19 须：
① 自扫描 vault 派生我们自己的 `graph.json`（节点/边）；② 仅**消费** Obsidian `graph.json` 的过滤器/布局字段
作为 UI 默认（如 `showOrphans`/`hideUnresolved`/`colorGroups`/`search`），**绝不**把整份配置当图谱数据导入。
避免任务板 §3 警告的"把整个 Obsidian 配置当成图谱数据"。

## 5. 笔记导航 + 图谱派生 实现卡（PROPOSED_NOT_AUTHORIZED）

> 以下为可直接交 W19 的候选卡；标注 `PROPOSED_NOT_AUTHORIZED`（R2B `W19=CLOSED`）。

- **范围**：只读 vault 派生 importer（`src-tauri/src/vault_graph.rs`），复用 `graph.rs` 守卫；加 `Note`/`Attachment`
  `GraphNodeKind` + `Embed` `GraphEdgeKind`（domain.rs/types.ts/graphUi.ts/ACL/policy）；`graph_import_vault`
  命令（A9 授权 + intent token，仿 `bridge::save_note`）；可选 `save_snapshot`（逆向 `load_snapshot`）。
- **前置**：A9 给出 vault 根授权路径；A1 决定 5000 上限抬升或子树范围；A10 确认 markdown/YAML/glob 解析依赖。
- **步骤**：glob 预扫描 → 扫 `.md`/附件 → 析 frontmatter/inline → 建 resolve 索引（path/basename/alias，
  NFC+casefold）→ 解析每条 link/embed/tag → 生成 `GraphNode`/`GraphEdge` → 跑 `graph_props_contain_secret` +
  容量检查 → 替换 `GraphState` →（可选）写 `graph.json`。
- **测试**：移植 §F-harness / 本 `A2-R2B-resolve.py` 为 Rust `#[test]`；A11 矩阵含 10 文件合成库 + 8,370 真实
  库容量/截断用例。
- **完成条件**：`graphQuery` 返回与合成 fixture 一致；`truncated=true` 在 >5000；私密 `props` 永不出现在错误/
  输出（K7 双门）；无第二执行路径（`GRAPH_NO_SECOND_PATH`）。
- **回滚**：删 `Note`/`Attachment`/`Embed` 枚举 + 命令 + ACL 即可；旧 `graph.json` 仍有效（旧 kind 不受影响）。
- **硬停**：不新增 fs dialog；不回显 `props`/路径（K7）；不写第二执行路径；导入只读，rename/delete 链接改写
  DEFER；每新命令须 source-check + handler + ACL + 类型 bridge + policy 测试齐备。

## 6. 复用 A1 U4 的比对结论（产品 graph snapshot vs Obsidian graph.json）

- `CURRENT_PRODUCT`：`graph.rs:395 load_snapshot` 读取 `data_dir/graph.json` → `GraphState`（节点/边/统计）。
- `OBSERVED_BEHAVIOR`：库内 `.obsidian/graph.json` 内容 = `{}` 默认骨架，且字段均为视图配置
  （`showOrphans`/`hideUnresolved`/`colorGroups`/`search`/`type`/`depth` 等），**无**节点/边数组。
- **结论**：两者 schema 不同，**不可**用 Obsidian `graph.json` 回填产品 `GraphState`；产品只**借用**其视图
  过滤字段作 UI 默认值。已在 §4 列为旧数据兼容硬规则。

## 7. 验证（VERIFY）

- `EXECUTED_SYNTHETIC_TEST`：`python3 A2-R2B-make_vault.py` → `python3 A2-R2B-resolve.py <dir>` → **19/19 PASS**
  （见 `A2-R2B-run-20260908.out`）。harness 已迁出 `/tmp`，存于本 lane 目录，**可复现、无外部依赖**。
- `OBSERVED_BEHAVIOR`：见 R2 §4 聚合统计（仅计数，无正文）。
- 未运行项：`cargo test`/Web 构建/GUI（研究边界，非本卡授权）；官方文档对照（本次未取得原文，标 `INFERENCE`）。

## 8. 分类（R2B 增补）

沿用 R2：COPY=0, ADAPT=2, REIMPLEMENT_FROM_BEHAVIOR=10, DEFER=1, REJECT=Obsidian IP。R2B 无新分类项；
新增的"笔记资源 ID/刷新/有界导入"均为 `DESIGN_DECISION`，落地时归 `ADAPT`（枚举/命令扩展）或 `REIMPLEMENT`
（派生逻辑）。

## 9. 状态 & 阻塞

- **STATUS = PASS_WITH_DEBT**（满足 R2B 退出门：无会迫使 W19 重探架构的未解项；唯一阻塞 B1=容量预算=A1 决策）。
- 阻塞：**B1** 8,370 笔记 > `GRAPH_MAX_NODES`=5000（A1 容量决策）；**B2** 标题/块/embed 仅 `INFERENCE`+合成
  fixture（W19 对照官方文档）；**B3** 笔记导航须对齐 A1 文档定位契约（已消费 `a54eed1`，无冲突）。

## 10. 输出文件

- `logs/research/M5-W18/A2-R2B-note-semantics-evidence.md`（本卡主报告）
- `logs/research/M5-W18/A2-R2B-make_vault.py`（自含 fixture 生成，无 `/tmp` 依赖）
- `logs/research/M5-W18/A2-R2B-resolve.py`（自含派生 harness，19/19 PASS）
- `logs/research/M5-W18/A2-R2B-run-20260908.out`（执行结果）
- `logs/research/M5-W18/A2-obsidian-vault-semantics.md`（R2，已补 R2B 撤回注记）
- `logs/checkpoints/A2-M5-W18-R2B-20260908.md`
