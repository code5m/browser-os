# Lane A7 · M5-W4 graph core slice delta（在 A4/A5 数据契约之后准备图谱 core 切片）

> LANE=A7 · WAVE=M5-W4（SUPPORT DOCS ONLY）· 责任实现 Lane=**A17**（M5-7/8 卡指定）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W4（行 134-168）→ A7=**SUPPORT DOCS ONLY**；任务="Prepare graph core slice after A4/A5 data contracts; no graph runtime"；交付=**next-card delta**；范围 `logs/assist/A7-M5-W4-*.md` + `M5-7*/M5-8*`
> 衔接：W0 契约 / W1 delta / W1 checkpoint / W3 impl-card（`A7-M5-graph-core-W3-impl-card-20260906-1412.md`）/ W3 checkpoint
> 实测 BASE=`f7ad35a docs(M5): dispatch W4 agent memory and skill lanes`（`git pull --ff-only` 已同步 origin/master）

---

## 0. 本 delta 目的

W3 的 B1 阻塞（M5-1.b seam）在 W4 已**解除**（`core/seam.rs` 已集成）。W4 打开 A4（M5-3 A2A/agent_kv）与 A5（M5-4/5 Agent/Skill）两条产品代码 lane。本 delta 把"图谱 core 切片"重新钉死为 **A4/A5 数据契约之后的下一个可集成切片**：图谱的 Skill/Agent 节点、agent_kv/a2a 边、容量常量、政策码均与 A4/A5 实际（或即将）落地的契约精确对齐，并记录**新阻塞**（A4/A5 数据契约在途）。A7 不写产品代码（W4 硬停：仅 A4/A5 可动产品代码）。

---

## 1. mainline 锚点核验（grep 实证 @ `f7ad35a`）

| # | 断言 | 实测 | 结论 |
|---|---|---|---|
| W4-1 | M5-1.b seam 已集成（`ProgressSink`/`PathResolver`/`RootsProvider`） | `core/seam.rs` 存在（2902B）；`core/mod.rs` 暴露；`grep "pub trait ProgressSink\|PathResolver\|RootsProvider" core/seam.rs` = 3 命中 | ✅ **B1 解除** |
| W4-2 | M5-W2 常量集中 | `domain.rs` `MAX_TIMEOUT_SECS`/`HARD_GRACE_SECS`/`MAX_TEXT_FIELD_BYTES`/`DB_MAX_TEXT_FIELD_BYTES` 命中 | ✅ |
| W4-3 | A4 M5-3 数据契约已落（agent_memory.rs / AgentDef / agent_kv） | `grep -rniE "agent_kv\|AgentDef" src-tauri/src` = 0；无 `agent_memory.rs`/`a2a.rs` | ⏳ **W4 在途，未落** |
| W4-4 | A5 M5-4/5 数据契约已落（agent.rs/skills.rs / SkillDef） | `grep` 同上 = 0；无 `agent.rs`/`skills.rs`/`check-agent-*-policy.py` | ⏳ **W4 在途，未落** |
| W4-5 | 图谱产品代码 | `domain.rs` 无 `Graph*`；`bridge.rs` 0 条 `graph_` 命令；无 `graph*.rs`；无 `check-graph-policy.py` | ✅ 零（W4 docs-only 一致） |
| W4-6 | `agent_kv` 走 JSON 非 SQLite | A4 预研 F-A4-7：`data_dir/agent-kv.json` + `session::atomic_write`，明确"避免开第二个 DB 通道——M5-8 图谱已确定走 SQLite" | ✅ 图谱=唯一 SQLite 通道 |

---

## 2. 依赖闸门（重钉）

- **B1（M5-1.b seam）= RESOLVED**（W4-1）。图谱 core 纯模块现可注入 `PathResolver`/`RootsProvider`/`ProgressSink`（W3 impl-card §2.2 拆分表现在可执行）。
- **B-new（A4/A5 数据契约）= ⏳ 在途（W4 进行中）**。图谱 core 切片须等：
  - A5 的 `AgentDef`/`SkillDef` 落 `domain.rs` → 图谱 Skill/Agent 节点**按 id 引用**（不重定义）。
  - A4 的 `agent_kv` namespace/key 形态落 → 图谱 `memorizes`/`a2a_with` 边引用 `agent_id`。
  - A4 `check-agent-memory-policy.py` / A5 `check-agent-skill-policy.py` 落 → 图谱 `check-graph-policy.py` 与其同范式、同 `pending→default` 模式。
- 其余 B2–B5（domain 类型、AGRAPH_1 脚本、ACL、shutdown 序）仍由 A17 实施期自闭环（同 W3）。

---

## 3. 图谱 core 切片与 A4/A5 数据契约的精确对齐（next-card delta 核心）

### 3.1 节点类型注册表引用 A5（不重定义）

`GraphNode.kind ∈ {File, Dir, Tab, Script, Skill, Agent, Tag, Topic}`。
- **Skill / Agent 节点是派生引用**：`GraphNode.id = sha256("agent:"+AgentDef.id)` / `sha256("skill:"+SkillDef.id)`；`props` 仅存稳定摘要（name/enabled/来源），**不**复制 A5 的完整 `SkillDef`/`AgentDef`（A5 为单源）。
- 新增边完整性校验：图谱落库/查询时，Skill/Agent 节点须能反查到 A5 的 `SkillDef.id`/`AgentDef.id` 存在（**AGRAPH_10**）。

### 3.2 新增边类型引用 A4（a2a / agent_kv）

- `uses`（Agent→Skill）：Agent 调用某 Skill。
- `a2a_with`（Agent→Agent）：A4 A2A 双向关系（对应 A4 协议信封；图谱仅存关系边，不存消息体）。
- `memorizes`（Agent→Topic）：映射 A4 `agent_kv` 的 `namespace` 语义（`memory`/`facts`/`prefs`/`session`/`draft`）；**图谱不存 agent_kv 值**（agent_kv 是 JSON 草稿纸，F-A4-7），只存"某 Agent 在哪些 namespace 有记忆"的边。
- 禁止图谱写 agent_kv 值或越权读 `workspace/` 成果库（与 F-A4-7 隔离一致）。

### 3.3 存储通道分工（无冲突）

- 图谱 `graph_store`：复用 `database.rs` 的 SQLite **单连接**（W3 impl-card §2.4；M5-8 §4.1）。图谱 = **唯一 SQLite 通道**。
- `agent_kv`：JSON 文件（`data_dir/agent-kv.json` + `session::atomic_write`），**不走 SQLite**（F-A4-7）。两者通道隔离，无"第二 DB 通道"冲突。
- 维护任务（vacuum/backup/fts-rebuild）复用 `script_runner`（`TaskKind::{Script,Command}`，`enabled=false`），禁新增 `TaskKind`。

### 3.4 容量常量对齐（同款不变量）

| 图谱常量 | 值 | 对齐来源 |
|---|---|---|
| `GRAPH_PROPS_MAX_BYTES` | `= MAX_TEXT_FIELD_BYTES`（64KiB） | 同 `AGENT_KV_MAX_VALUE_BYTES == DB_MAX_TEXT_FIELD_BYTES`（A4 F-A4-8 / §4.5） |
| `GRAPH_LABEL_MAX_BYTES` | 256 | 与 `AGENT_KV_MAX_KEY_BYTES=256` 同量级（A4 §4.5） |
| `GRAPH_MAX_DEPTH` | 4 | 防爆栈 |
| `GRAPH_QUERY_LIMIT` | 1000 | 防响应爆 |
| （未来）`GRAPH_MAX_NODES` | 参照 `AGENT_KV_MAX_ENTRIES=5000` 量级 | 容量封顶，防图爆炸 |

单测守 `GRAPH_PROPS_MAX_BYTES == MAX_TEXT_FIELD_BYTES`（同 A4 守 `AGENT_KV_MAX_VALUE_BYTES == DB_MAX_TEXT_FIELD_BYTES`）。

### 3.5 政策脚本对齐（AGRAPH_1 镜像 A4/A5）

`scripts/check-graph-policy.py`（W3 impl-card §3 八码）补充与 A4/A5 同源的两条：
- **AGRAPH_9**（隐私）：`GraphNode.props` / 审计 `detail` 不得含凭据特征（递归字符串叶子扫描），镜像 A4 `AGENT_KV_SECRET_SUSPECTED`。
- **AGRAPH_10**（id 完整性）：Skill/Agent 节点 id 须可反查 A5 `SkillDef.id`/`AgentDef.id`；缺失即拒。
- 其余 `AGRAPH_1..8` 沿用 W3（core 无 tauri/AppHandle/第二路径、props≤64KiB、depth≤4、limit≤1000、ACL 序、`TaskKind` 冻结、shutdown 序、`source=Manual` 不覆盖）。
- `pending→default`：W4 期全 `PENDING`；A17 实现后转 `DEFAULT`（同 `check-agent-memory-policy.py` / `check-agent-skill-policy.py` 模式）。

### 3.6 core 纯函数现在可注入 seam（B1 解除后的落地）

W3 impl-card §2.2 拆分表现在可执行：
- `graph_extract_html.rs` 取 `&dyn RootsProvider`（仅扫 `allowed_roots`）—— 直接复用 `core/seam.rs::RootsProvider`（已集成）。
- `graph.rs`（bin 编排）接 `&dyn ProgressSink`（取消/进度，复用 `script_runner` 通道）—— 直接复用 `core/seam.rs::ProgressSink`。
- `graph_store.rs` 取 `&dyn PathResolver`（`base_dir` 解析 `graph.db` 路径）—— 复用 `PathResolver`。
- 以上均在 `check-core-boundary.py` `COREBOUND_*` 守门范围内（core 模块零 `tauri`/`AppHandle`/`crate::bridge`）。

---

## 4. next-card 放置（图谱 core 切片 = M5-7.a / M5-8.a）

- 责任 Lane 仍为 **A17**（M5-7/8 卡指定）。
- 切片顺序（相对 A4/A5）：A4 落 `agent_kv` + A5 落 `AgentDef`/`SkillDef` → A0 签 `M5-7.a`（模型/抽取，含 Skill/Agent 引用 + AGRAPH_10）→ `M5-8.a`（存储/查询，复用 `database.rs` 单连接 + AGRAPH_1..10）→ A8 接 `M5-9`（UI）。
- 本 delta **不写 M5-7/8 卡正文**（避免与 A1 冲突）；作为 A7 自有 next-card delta 输入 A17。

---

## 5. 自测（A7 docs 范围，grep 实证）

| # | 项 | 结果 |
|---|---|---|
| T1 | `check-core-boundary.py --self-test` | PASS |
| T2 | `core/seam.rs` 三 trait 命中 | 3（ProgressSink/PathResolver/RootsProvider） |
| T3 | `domain.rs` `Graph*` = 0 | ✅ 待 A17 |
| T4 | `bridge.rs` `graph_` = 0 | ✅ |
| T5 | A4/A5 产品码 absent | ✅（W4 在途） |
| T6 | M5-W2 常量命中 | ✅ |
| T7 | `git diff --check` | 干净 |

---

## 6. 输出模板回填

```text
LANE=A7
STATUS=PASS_WITH_DEBT（债务=B-new 在途，非 A7 自身）
WAVE=M5-W4 (SUPPORT DOCS ONLY)
BASE=f7ad35a
HEAD=docs only（A7-M5-W4-graph-core-delta-20260906-1455.md + A7-M5-W4-checkpoint-20260906-1455.md）
FILES=logs/assist/A7-M5-W4-graph-core-delta-20260906-1455.md, logs/assist/A7-M5-W4-checkpoint-20260906-1455.md
VERIFY=T1-T7 全 PASS/命中；零产品代码
CHECKPOINT=logs/assist/A7-M5-W4-checkpoint-20260906-1455.md
MERGE_NOTES=W4 docs-only；B1(M5-1.b seam)已解除（core/seam.rs 集成）；新阻塞 B-new=A4/A5 数据契约（W4 在途，产品码未落）；图谱与 agent_kv 通道隔离（图谱=唯一SQLite，agent_kv=JSON，F-A4-7）；Skill/Agent 节点按 id 引用 A5，memorizes/a2a_with 边引用 A4；AGRAPH_1 增补 AGRAPH_9/10 镜像 A4/A5；未改 M5-7/8 卡（避免与 A1 冲突）；零产品代码、未 push
NEXT=A4 落 agent_kv + A5 落 AgentDef/SkillDef → A0 签 M5-7.a/M5-8.a 交 A17（图谱 core 切片）
```
