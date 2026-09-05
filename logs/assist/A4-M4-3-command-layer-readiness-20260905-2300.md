# A4 · M4-3 数据库命令层 —— 开工前只读预研与阻塞说明

> Lane: `A4` — M4-3 database command layer（按 `PARALLEL_COMMAND_BOARD.md` 2026-09-05 **23:00** 版）
> 时间: 2026-09-05 23:00 CST
> Base: `3544c09`（`master`；**注意**：指挥板写 `Current mainline: master at f376346`，实测 HEAD 为 `3544c09`，领先 2 个提交 `e6e09cf` / `3544c09`，指挥板 BASE 字段滞后）
> 范围: **只读预研**。本轮零产品代码改动，零 `src/` `src-tauri/` `scripts/` 改动，不提交、不 push。
> 触发条款: 指挥板 §Dispatch Waves **Wave 2**「A4: Start after A2 delivers command DTOs and A3 delivers pool/safety interfaces」+
> 指挥板第 33 行「If the only blocker is an unfinished predecessor lane, do not edit product code. Produce a read-only assist note under `logs/assist/`」

---

## 1. 结论

**STATUS = BLOCKED**（前置未完成，非本 lane 缺陷）

A4 的两个前置 Lane **到本次核查时点均无任何交付**，因此本轮不写产品代码，只输出本预研笔记。

**同时必须上报一处 Lane 身份冲突**（见 §3），它会在 A3/A4 之间造成 `security_policy.rs` / `domain.rs` 撞车，需 A0 在本 lane 开工前裁决。

---

## 2. 阻塞核实（实测命令与输出）

### 2.1 A2（M4-1 契约）交付物核查 —— 全部为空

```bash
find src-tauri/src -maxdepth 1 -name 'database*' -print     # 无输出
grep -rn 'SupportedDb' src-tauri/src src/types.ts            # 无输出（.rs/.ts 中零命中）
ls scripts/check-database-policy.py                          # 不存在
ls logs/checkpoints/                                         # 无任何 M4-1.a/b/c/d 检查点
```

→ `SupportedDb` 枚举、连接配置 DTO、凭据命名空间、错误码枚举、`check-database-policy.py` 骨架**均未落地**。
→ 依据展开卡 F-3，`db_connect` / `db_query` / `db_disconnect` 目前**只是占位名**，最终签名以 M4-3.a 冻结为准，而 M4-3.a 又依赖 M4-1.b。**当前无法开始命名。**

### 2.2 A3（M4-2 pool/safety 接口）交付物核查 —— 全部为空

```bash
grep -n 'classify_sql_risk\|is_production_database' src-tauri/src/security_policy.rs   # 无输出
grep -n '^mod ' src-tauri/src/main.rs                                                  # 无 mod database;
```

→ 连接池抽象、内省/查询核心签名、SQL 风险分类与生产判定函数**均未落地**。A4 的命令层无处可调用。

### 2.3 工作树状态（不影响本 lane，但影响 A0 集成）

```
## master...origin/master
 M 详细设计与实施计划.md                                   ← A1 的展开回写，尚未提交
?? logs/assist/A10-M4-security-review-20260905-2240.md      ← A10 产出
?? logs/checkpoints/M4-20260905-2225.md                     ← A1 展开卡
```

脏文件**均不属于 A4**。按 `WORKSPACE_IDENTITY.md` 第 35 行，A4 不提交、不 rebase、不清理，交 A0 集成。

---

## 3. 【必须 A0 裁决】Lane 身份冲突

指挥板与 A1 展开卡对 A4 的定义**不一致**，且两者都在有效期内：

| 来源 | 对 A4 的定义 | 落点 |
|---|---|---|
| `PARALLEL_COMMAND_BOARD.md` **23:00 版**（ controller 最新，12 lanes） | **M4-3 database command layer** | `database*` / `bridge.rs` / `main.rs` / `default-commands.toml` / `domain.rs` / `src/bridge.ts` / `src/types.ts` / tests，Merge **4** |
| `logs/checkpoints/M4-20260905-2225.md`（A1，依据 22:35 版指挥板）裁决 **R-A1-1** | **M4-2.s** SQL 风险分类 + 生产判定 | `security_policy.rs`，**零 db 依赖**，Merge **3** |

指挥板第 31 行规定：「Stop and report the conflict **unless the controller has already corrected the lane in `PARALLEL_COMMAND_BOARD.md`**」。
23:00 版指挥板已把 A3 改为「M4-2 PoolKind and production safety policy（含 `security_policy.rs`）」、A4 改为「M4-3 database command layer」，**即 controller 已更正**，故本 lane 按 **A4 = M4-3 命令层** 执行，**不采用** A1 的 M4-2.s 归属。

**但三份主文档尚未同步**（A1 的回写仍写 A4=M4-2.s）：

- `详细设计与实施计划.md:45` — 「Lane A2=M4-1 / **A4=M4-2.s** / A3=M4-2,M4-3 / …」
- `详细设计与实施计划.md:456` — 「**子卡 M4-2.s（Lane A4，merge 3，落 `security_policy.rs` 且零 db 依赖）**」

**风险**：A3 与 A4 若各按一套编号施工，会在 `security_policy.rs`（A3 写分类器 vs A4 按旧裁决也写分类器）与 `domain.rs` 上直接撞车。展开卡 O-A1-7 已预警过该编号冲突，请 A0 一并回改展开卡与三份主文档。

### 3.1 交叉印证：A3 独立发现同一冲突，且倾向相反裁决

`logs/assist/A3-M4-2-prework-20260905-2305.md`（A3，同 Wave 2 只读预研）§3 独立定位到同一冲突，并提出：

- **死锁论证**（其第 79 行）：指挥板写「A4 等 A3 的 pool/safety 接口」，展开卡写「M4-2.a 的查询路径必须过 M4-2.s 分类器」（即 A3 等 A4）。**若两者都照办 → A3 与 A4 互相等待。**
- **A3 的倾向**：采纳**展开卡口径**（`M4-2.s` 归 A4，A3 承接 `M4-2.a~d` + `M4-3.a~d`），与本 lane 依据「指挥板已被 controller 更正」而采纳的**指挥板口径相反**。

**本 lane 对死锁论证的澄清**：两套口径各自**内部自洽**，死锁只发生在**混用**时——

| 口径 | A4 做什么 | A3 做什么 | 依赖方向 | 是否自洽 |
|---|---|---|---|---|
| 指挥板 23:00 | M4-3 命令层 | M4-2 含连接池 **+ SQL 分类器 + fail-closed** | A4 ← A3 | 自洽（A3 merge 3 → A4 merge 4 串行） |
| A1 展开卡 | M4-2.s 分类器（零 db 依赖，先合入） | M4-2.a~d + M4-3.a~d | A3 ← A4 | 自洽（A4 merge 3 → A3 merge 4 串行） |

因此**关键不是谁对，而是必须二选一并全局同步**。在 A0 明确裁决前，A3 与 A4 都不得动 `security_policy.rs` / `domain.rs` / `main.rs`。**本 lane 维持按指挥板 23:00 口径（controller 最新且已更正），但接受 A0 改采展开卡口径**——两种口径下 A4 均处 Wave 2 阻塞态，本轮交付不受影响。

---

## 4. 只读预研：M4-3 命令层落地码位（开工时直接照此施工）

### 4.1 后端命令范式（`src-tauri/src/bridge.rs`）

```rust
// bridge.rs:2977
pub fn run_command(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
    values: HashMap<String, String>,
) -> Result<RunSnapshot, String> {
    check_invocation_source(&webview, "run_command", None, &app)?;   // ① 第一行必是来源校验
    check_id(&id, "命令片段 id")?;                                    // ② 领域校验
    ...                                                               // ③ 调核心层
    // 失败路径：审计只记 id + 稳定错误码，不记参数值（:3003-3008）
    workspace::log_audit(&app, "cmd.validate.reject",
        format!("id={} error_code={}", snippet.id, inner.code()));
    // 成功路径：:3015 workspace::log_audit(&app, "cmd.run.start", ...)
}
```

db 三命令须逐条对齐：① `check_invocation_source(&webview, "<name>", None, &app)?` 置首行；② 失败与成功双路径审计；③ **审计 detail 只含 `op/conn_id/风险等级/行数`，禁含 SQL 正文、参数值、凭据、连接串**（F5 + A10 G-3）。

### 4.2 命令注册（`src-tauri/src/main.rs`）

- 主窗 handler 列表：`main.rs:1299` `.invoke_handler(tauri::generate_handler![` … `:1406` `bridge::script_runs_list,` … `:1407` `])`。
- handler 列表**无顺序约束**，新命令追加即可（建议集中追加在 `script_runs_list` 之后，与既有「按卡追加」习惯一致）。
- `mod` 声明在 `main.rs:3-19`，**`mod database;` 归 A3（M4-2.a）**，A4 不得抢先添加，否则与 A3 的 `main.rs` 改动冲突。

### 4.3 ACL 登记（`src-tauri/permissions/default-commands.toml`）

- 末条锚点 = `"list_artifact_images"`（**第 110 行**）。新条目**必须插在其之前**，保持它仍是末条——`check-image-policy.py` 等既有夹具按「末行无逗号」定位，M2-1 / M2-2.b / M2-3.b 三度踩坑。
- **禁止**写入 `src-tauri/permissions/remote-collect.toml`（当前仅 3 条：`report_resources` / `report_title` / `report_grid_load_failed`，供外部页面子 webview 用）。db 命令一旦进 remote 集 = 外部页面可连库，A10 G-10 列为严重项。

### 4.4 前端封装范式

- `src/bridge.ts:284` `runCommand: (id, values) => invoke<RunSnapshot>("run_command", { id, values })` —— 组件不得直接 `invoke`，须经 `bridge.ts`。
- `src/types.ts` 为 TS DTO 镜像，随 `domain.rs` 同步（A2 已在 M4-1.b 落 TS 镜像，A4 只补命令出入参类型）。
- **口令只允许组件局部瞬态存在**，禁止进入任何持久化 store（A10 G-12）。

### 4.5 凭据接入（`src-tauri/src/keyring_store.rs`）

- `SERVICE = "com.jizhijiandan.mvp"`，`save_token` / `get_token` / `delete_token(repo_id)`，**按精确字符串取键**。
- `delete_token` 仍标 `#[allow(dead_code)]`（`:22`），**零调用者、零测试**。
- 展开卡 F-7 定：db 键格式 `db:<conn_id>`，须加单测证明 `db:<x>` 与 `<x>` 不碰撞（否则与 git 的 `repo_id` 静默互串）。

### 4.6 门禁夹具接入点

- `scripts/pre-merge.sh:331-335` 终端夹具接法（**默认模式零违规**的范式，db 夹具照此）：
  `check-terminal-policy.py --self-test` → `check-terminal-policy.py`（默认）。
- `scripts/pre-merge.sh:216-222` 安全夹具接法（`--self-test` + `--expect-current-gaps`，**默认模式按设计 EXIT=1**，不可照抄）。
- `scripts/pre-merge.sh:390-460` 为「脚本存在性 + self-test」断言块，新夹具须同步登记。
- `scripts/check-security-policy.py:79` `detect_gaps` 现有 `SEC-01..SEC-09`；`SEC-08`（`:105`）守护「remote 集不得开放副作用命令」，A10 G-10 要求 M4 补 `SEC-10+` 覆盖 8 条新命令。

---

## 5. 风险登记（A4 开工前须逐条确认）

| 编号 | 风险 | 归属 |
|---|---|---|
| **R-A4-1**【高】 | §3 的 Lane 身份冲突未回改三份主文档 → A3/A4 在 `security_policy.rs` / `domain.rs` 撞车。**A3 已在 `A3-M4-2-prework-20260905-2305.md` 独立复现并倾向相反裁决**，上升为全局阻塞项 | **A0 紧急裁决**（见 §3.1 两套口径对照表） |
| **R-A4-2**【高】 | 命令名未冻结（A2 未交付，`db_*` 仍为占位名）。A4 **不得自行命名**；若与 M4-1.b 不一致，按展开卡 M4-3.a `FAIL_ACTION` 走 `STATUS=BLOCKED` 交 A2，不得改名 | A2 → A4 |
| **R-A4-3**【中】 | **ACL 计数基准失准**：A1 展开卡记「ACL 108 条」，实测 `grep -c '^    "'` = **106**。A4 新增 3 条后应为 **109**。若夹具按 108 断言会误判 | A0 确认以实测为准 |
| **R-A4-4**【中高】 | **`tools::open_tool` 已注册（`main.rs:1402`）但两个 ACL 文件均无该条目** —— 说明「注册 ↔ 入 ACL 双向一致」当前**无夹具守护**。A4 的 `DB_CMD_NOT_REGISTERED` / `DB_ACL_ORDER` 应覆盖双向，否则同类缺口会在 db 命令上重演。**是否顺手补 `open_tool` 属 A3/A0 范围，A4 不代改** | A4 夹具覆盖；修缺口交 A0 |
| **R-A4-5**【高】 | **口径直接冲突**：展开卡 F-7（`M4-20260905-2225.md:59`）定「`db_disconnect` **默认不删凭据**，删除仅由显式『忘记连接』入口触发」；A10 评审 G-1（`A10-...md:54`）要求「`db_disconnect` **必须吊销凭据**并移除 `#[allow(dead_code)]`」。二者相反，须先裁决 | **A0 裁决** |
| **R-A4-6**【高】 | `log_audit` 内部无脱敏，仅 git 写路径调 `sanitize_audit_text`。SQL 原文可含 `CREATE USER ... IDENTIFIED BY 'x'` / `SET PASSWORD =` / `COPY ... WITH PASSWORD` —— **记 SQL = 记凭据**（git 路径不存在的泄漏面）。db 审计 detail 只记语句类别 + 参数摘要 + 影响行数 + conn_id | A4 实现 + A10 复核 |
| **R-A4-7**【中高】 | 行数/字节上限须在**取数循环内**生效（A10 G-6：物化后再截断等于没截断），而取数循环在 A3 的 `database*` 内。A4 只能在命令层做**二次兜底截断 + `truncated` 标记透传**，**不能替代核心层上限**，须与 A3 书面划界 | A3 主 / A4 兜底 |
| **R-A4-8**【中】 | 写闸门须在**命令层独立兜底**：「未开 `allow_write` 的写」与「开了但未二次确认的写」两条反向用例均须被拒（F1），不得依赖前端不发 invoke | A4 |
| **R-A4-9**【中】 | `main.rs` / `bridge.rs` / `default-commands.toml` / `domain.rs` 均为指挥板列明的高冲突文件；A3(merge 3) 早于 A4(merge 4)，A4 的 `domain.rs` 增量（错误码枚举、conn_id 类型）须待 A3 landed 后 rebase | A0 按序集成 |
| **R-A4-10**【低】 | 工作树当前脏（`详细设计与实施计划.md` + 2 个未跟踪日志），均属 A1/A10。A4 不提交、不 rebase、不 push | A0 |
| **R-A4-11**【低】 | 指挥板 `Current mainline` 写 `f376346`，实测 HEAD `3544c09`，BASE 字段滞后，A0 集成时以实测 HEAD 为准 | A0 |

---

## 6. 解除阻塞条件（全部满足才可动产品代码）

1. **A2 交付到账**：`logs/checkpoints/M4-1.d-<ts>.md` 存在且含 M4-1.a/b/c 冻结结论；`src-tauri/src/domain.rs` 含 `SupportedDb` 与连接配置 DTO（**结构性无 password 字段**）；`src/types.ts` 有 TS 镜像；`scripts/check-database-policy.py` 存在且 `--self-test` 通过。
2. **A3 交付到账**：`src-tauri/src/database*.rs` 与 `main.rs` 的 `mod database;` 存在；`security_policy.rs` 含 `classify_sql_risk` / `is_production_database`；pool 三接口（连接/断开/查询）的**函数名与错误类型签名稳定**。
3. **命令名一致**：A2 冻结名 == A4 实现名；不一致 → `STATUS=BLOCKED` 交 A2，A4 不改。
4. **A0 已裁决**：R-A4-1（Lane 身份与三份主文档回改）、R-A4-5（`db_disconnect` 是否吊销凭据）、R-A4-3（ACL 计数基准）。

---

## 7. 开工前自检清单（解除阻塞后第一条命令起）

```bash
cat .workspace-identity                 # WORKSPACE_ID=BACKV3_MAIN
pwd                                     # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --short --branch             # master；脏文件须能明确归属于 A4
git log --oneline -12                   # 确认已含 A2 + A3 落地提交
# 现场复核（勿沿用文档旧数）
grep -c '^    "' src-tauri/permissions/default-commands.toml   # 实测 106，+3 后 = 109
grep -n 'list_artifact_images' src-tauri/permissions/default-commands.toml  # 末条锚点行号
grep -n '^mod ' src-tauri/src/main.rs                          # 确认 mod database; 已由 A3 添加
grep -c 'pm_log' scripts/pre-merge.sh                          # 现场复核项号，勿照抄「第 22 项起」
```

---

## 8. 本次未覆盖（声明，避免误读为已完成）

- **未写任何产品代码**：`src/` `src-tauri/` `scripts/` `package*.json` 一律零改动。
- **未运行** `cargo test` / `npm run build` / `bash scripts/pre-merge.sh`：本 lane 零产品代码改动，沿用 A10 第 199 行口径，基线以既有记录为准（M3.c 交付时 `cargo test` 232/232）。
- **未评审 A2/A3/A7 实现**：无任何对象可裁。
- **未修改** `PARALLEL_COMMAND_BOARD.md`（A0 协调文件，避免并发写冲突）、未修改三份主文档（A1 范围，且其改动尚未提交）、未触碰 `logs/assist/assist-index-*.md`（非本 lane 文件）。
- 本笔记为**开工前预研**，**不构成**对 M4-3 实现的安全裁定；实现后应另由 A10 做实现后复审（A10 第 208 行建议）。
