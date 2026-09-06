# M5-7 图模型与可追溯抽取（两阶段 + `graph.rs`）

> 子卡 ID：**M5-7** · 需求 #13（知识图谱）· `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A17**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L570（`M5-7 图模型与可追溯抽取`）
> 主预研：`logs/assist/M5-13.a-prework-20260902-1055.md`（两阶段抽取 + `source=Manual/Extract/Ai`）· `logs/assist/A9-M5-graph-store-contract-20260906-0700.md`（store 契约）
> 配套：`M5-8-graph-store-query.md`（存储与查询）· `M5-9-graph-ui-agent-consume.md`（UI 与消费）

---

## 0. 编号与锚定

- 批次任务号 `M5-7`；需求号 #13；WBS L570 一致。
- 依赖：M5-1 ✅（domain 落 core）+ A7 事件流 ✅ + M5-8 部分（GraphQuery DTO 共定）
- 关键决策交 A0 拍：① `source=ai` 智能抽取首期是否留 trait（**默认留**） ② `props` JSON 容错策略（**默认严格**）

---

## 1. GOAL

冻结图模型（`GraphNode` / `GraphEdge` / `GraphSchema`），实现 `graph.rs`（两阶段抽取器：① 静态扫描 HTML/脚本/文档 ② 实体识别与关系归一化），建立"可追溯"原则——每个节点/边带 `source` + `source_ref` + `extracted_at` + `extractor_version`，**派生索引层**不存主数据正文。

---

## 2. READ

1. `logs/assist/M5-13.a-prework-20260902-1055.md`（**全读**，图模型/两阶段抽取/AI 抽取 trait）
2. `logs/assist/A9-M5-graph-store-contract-20260906-0700.md`（store schema DDL + GraphNode/GraphEdge DTO）
3. `logs/assist/A9-M5-graph-scheduler-feed-20260906-0700.md`（scheduler feed 接缝）
4. `src-tauri/src/domain.rs`（契约根，**全读**——新类型加这里）
5. `src-tauri/src/workspace.rs`（路径解析）
6. `M5-1-core-workspace-split.md` §10（切片 0b 契约常量随迁）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/graph.rs` | **新增** | `graph_extractor` 两阶段 + schema 校验 + 增量写 |
| `src-tauri/src/graph_schema.rs` | **新增** | Schema 注册表（节点类型/关系类型/必填 props） |
| `src-tauri/src/graph_extractor_html.rs` | **新增** | 阶段① HTML 扫描（仅结构化标签 + link/script 资源） |
| `src-tauri/src/graph_extractor_doc.rs` | **新增** | 阶段① 文档扫描（md/txt/json，**禁**二进制） |
| `src-tauri/src/graph_normalize.rs` | **新增** | 阶段② 实体识别 + 关系归一 + 相似节点合并（首期同 hash 精确匹配） |
| `src-tauri/src/domain.rs` | 新增类型 | `GraphNode` / `GraphEdge` / `GraphSchema` / `GraphSource` / `ExtractResult` |
| `src-tauri/src/bridge.rs` | 修改 | `graph_extract` / `graph_extract_cancel` / `graph_schema_list/get` 命令（**全进 ACL**） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 `graph_extract*` / `graph_schema_*` 于 `list_artifact_images` 之前 |
| `src/types.ts` | 新增 | TS 镜像 `GraphNode`/`GraphEdge`/`GraphSchema` |
| `scripts/check-graph-policy.py` | **新增** | 挂 `pre-merge.sh`；自检 PASS |

---

## 4. 关键契约

### 4.1 图模型 schema

```rust
pub struct GraphNode {
    pub id: String,                    // hash(source_ref + kind + stable_props_hash)
    pub kind: String,                  // 节点类型（注册表必填）
    pub label: String,                 // 展示名
    pub props: serde_json::Value,      // 严格 schema 校验（首期严格，宽容版本做迁移期）
    pub source: GraphSource,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extractor_version: String,     // semver，触发再抽取判定
}
pub enum GraphSource {
    Manual,        // 用户手建（最高优先级，不被自动抽取覆盖）
    Extract,       // 阶段② 自动抽取
    Ai,            // 【首期留 trait】  LLM 抽取留接口，不实现
    Imported,      // 从其它来源导入
}
pub struct GraphEdge {
    pub id: String,
    pub from: String,                  // GraphNode.id
    pub to: String,                    // GraphNode.id
    pub kind: String,                  // 关系类型（注册表必填）
    pub props: serde_json::Value,
    pub source: GraphSource,
    pub created_at: DateTime<Utc>,
    pub weight: f64,                   // 0.0 ~ 1.0（首期 1.0 精确 / 0.0 否定）
}
```

**派生索引层原则**：`GraphNode.props` **不存主数据正文**（K7）；可重建；`source=Manual` 优先不被自动抽取覆盖。

### 4.2 两阶段抽取

**阶段① 候选发现**（无副作用，仅返回候选实体与关系）：

| 输入源 | 提取方法 | 风险 |
|---|---|---|
| `workspace/` 文档（md/txt/json） | 解析 frontmatter + 结构化字段 | Low |
| `workspace/` 文档（pdf/docx） | **首期不实现**（防二进制解析漏洞） | — |
| `script_runner` `runs.json` 的脚本输出 | 解析 YAML frontmatter + `## outputs:` 段 | Low |
| `tabs.db` 已存 tab（仅 URL/title/路径） | 提取链接 | Low |
| 浏览器访问历史（M4-5） | **首期不接入**（避免爬虫语义；可后续评估） | — |
| LLM 响应 | **首期留 trait，不实现** | — |

**阶段② 实体识别与归一**：

- 节点类型注册表（首期：File / Dir / Tab / Script / Skill / Agent / Tag / Topic）
- 关系类型注册表（首期：`in_dir` / `references` / `related_to` / `tagged_with`）
- 同 hash 精确匹配合并（**首期**；相似度算法留债）
- 边权重：精确 1.0；否定 0.0

### 4.3 可追溯性

- 每个 `GraphNode` / `GraphEdge` 必带 `source` + `extractor_version` + `created_at` + `updated_at`
- `props` 不含原文（K7）；原文路径存 `source_ref`（仅引用，不复制）
- 重复抽取：仅当 `extractor_version` 变化才再识别；否则跳过

### 4.4 取消与收口

- `graph_extract` 是长任务（可能数分钟），必须支持取消
- 复用 A6/A7 既有 `TaskKind::{Script, Command}`，**禁**新增 `TaskKind::GraphExtract`
- 落成 `Script`（`ScriptExec` 段）—— 走 `script_runner` 通道
- 取消复用 `script_runner.cancel`
- **新任务默认 `enabled=false`**（R-A6-1）

### 4.5 审计契约

- `graph_extract` 写 `audit.json`（`action=graph_extract`）
- `detail` 含 `scope` / `extractor_version` / `nodes_added` / `edges_added` / `nodes_merged` / `duration_ms` / `result`
- 禁记原文/响应正文（K3 + 体量防爆）

---

## 5. FORBID

- **不**让 `GraphNode.props` 存主数据正文（K7，可重建原则）
- **不**让 `source=Manual` 节点被自动抽取覆盖
- **不**为抽取新增 `TaskKind::GraphExtract`（A6 冻结）
- **不**让首期实现 `source=ai` 抽取（仅留 trait）
- **不**让二进制文档（pdf/docx）被解析（防漏洞）
- **不**破 K1（ACL 末条恒为 `list_artifact_images`）/K3/K5
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
grep -rniE "GraphNode|GraphEdge|graph_extractor" src-tauri/src | head

# B. 政策脚本自检
python3 scripts/check-graph-policy.py --self-test

# C. 反向用例
# N1: props 含主数据正文（> 1 KB）→ 拒
# N2: 重复抽取同 hash → 跳过
# N3: 抽取期间取消 → 终止，无半成品
# N4: source=Manual 节点被覆盖 → 阻断
# N5: extractor_version 变更 → 触发再抽取
# N6: 二进制文件被解析（pdf/docx）→ 拒

# D. 性能基线
# 1000 文件/100 MB workspace 抽取 < 60s
# 10000 节点查询 depth=2 < 500ms

# E. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# F. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | `GraphNode` / `GraphEdge` / `GraphSource` 落 `domain.rs` | `grep` 命中 |
| 2 | 两阶段抽取器落地 | `grep graph_extractor` 命中 |
| 3 | `props` 不存主数据正文（K7） | 单测 N1 |
| 4 | 重复抽取同 hash 跳过 | 单测 N2 |
| 5 | 取消生效无半成品 | 单测 N3 |
| 6 | `source=Manual` 不被覆盖 | 单测 N4 |
| 7 | `extractor_version` 变更触发再抽取 | 单测 N5 |
| 8 | 二进制文档解析被拒 | 单测 N6 |
| 9 | 性能基线 1000 文件/100 MB < 60s | 命令 D |
| 10 | ACL 末条仍为 `list_artifact_images` | 命令 B 0 违规 |
| 11 | `cargo test` 全绿 | 命令 E |
| 12 | `pre-merge.sh` ALL_PASS | 命令 F |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| `props` 存原文 | 阻断（K7） |
| `source=Manual` 被覆盖 | 阻断（用户数据红线） |
| 抽取无取消 | 阻断（A6 范式） |
| 二进制解析失败引发漏洞 | 立即修复并重新评估 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L570 `[ ]` → `[x]`
2. `后续需求TODO.md` §13 状态 `PARTIAL`（留 `M5-8/9`）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-8`
4. `logs/checkpoints/M5-7.a-2026MMDD-HHMM.md`
5. `M5-14-debt-ledger.md` 增项：`source=ai` trait / 相似度算法 / 二进制文档

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A17 实施填
- **NEXT**：M5-8（存储与查询）

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
