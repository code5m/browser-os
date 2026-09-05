# M4-1 契约闭环收口（Lane A2 · Integration Fix Wave）

> 执行者：**Lane A2**（AI:DEEP / reasoning=high）
> 时间：2026-09-06（Integration Fix Wave）
> 触发：按指挥板 Integration Fix Wave 修复本 Lane 阻塞项（IF-3 / ④）
> 工作树：`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
> 基线：`origin/master`

---

## 0. 范围声明（与 `M4-1.d` §0 的关系）

`M4-1.d-20260905-2300.md` §0 第 21 行写「未做且不属于本 Lane：`scripts/check-database-policy.py`」。
但 Integration Fix Wave 阻塞项 **④** 明确指派：

> ④ `check-database-policy.py --self-test` 与 `--expect-pending`（**A2**：A7 落 DB 功能后
>   A2 未把 pending 提升为 ACTIVE，需 A2 照 A6 做法提升）

故本次**越界执行** pending→ACTIVE 提升，并据实记录于此，供 A0 仲裁脚本归属。

---

## 1. 阻塞项处理结果

### 1.1 IF-3：`check-tools-policy.py --self-test` 失配（已修复）

**根因**：坏样本 #10「ACL 顺序错误」使用「交换相邻两行」变异
（`"list_tools",\n    "list_artifact_images"` → `"list_artifact_images",\n    "list_tools"`）。
A4 在 `list_tools` 与 `list_artifact_images` 之间插入了 `task_*`×5 + `db_*`×3
（见 `src-tauri/permissions/default-commands.toml` 相对 `origin/master` 的 diff），
导致该 `replace` 失配为空操作，被变异防呆判为「漏检」→ `--self-test` FAIL。

**修复**（`scripts/check-tools-policy.py`，`run_self_test` 内坏样本 #10）：
改为**与中间内容无关**的变异——先摘出 `"list_tools",\n` 条目，再将其追加到锚点
`list_artifact_images` 之后。无论两者之间插多少命令，变异恒生效。

**验证**：
```text
python3 scripts/check-tools-policy.py --self-test
→ self-test OK: 好样本零违规 + 16 个坏样本全部检出（含变异防呆）
python3 scripts/check-tools-policy.py
→ tool manifest policy: all invariants hold
```

### 1.2 ④：`check-database-policy.py` pending→ACTIVE 提升（已完成）

原 8 个 PENDING 中 **7 个** 因 A3（`database.rs`）/ A4（`bridge.rs` 命令层）已落地而提升为
ACTIVE；**1 个** 因制品尚未实现而保留 PENDING。

| 码位 | 归属 | 提升依据（真实仓库复跑零违规） |
|---|---|---|
| `DB_ACL_ORDER` | A4 | ACL 中 `db_*` 在 `list_artifact_images` 之前 ✅ |
| `DB_CRED_IN_AUDIT` | A4 | `bridge.rs` 三个 `db.*` 审计 detail 仅含 `conn_id/rows/truncated`，无 `sql=`/`password` ✅ |
| `DB_MULTI_STATEMENT_FORBIDDEN` | A4 | `bridge.rs` 全文无 `execute_batch`/`simple_query`/`batch_execute` ✅（已 grep 确认） |
| `DB_CRED_NAMESPACE` | A3 | `database.rs` 含 `credential_key`/`DB_CRED_PREFIX` 的 `db:` 字面量（弱守，见 §3 D43 备注） |
| `DB_RESULT_LIMIT_MISSING` | A3 | `database.rs::DbQueryResult` 含 `truncated`/`field_truncated` ✅ |
| `DB_TIMEOUT_NOT_LAYERED` | A3 | `database.rs` 含 `timeout` 且 3 个分层常量均在 ✅ |
| `DB_PLATFORM_DEGRADE` | A3 | `database.rs` 有 `target_os` 且返回 `NotSupported` ✅ |
| `DB_PERSIST_NOT_ATOMIC` | A3 | **保留 PENDING**：连接配置落盘（`data_dir/db_connections.json` 原子写）未实现，`bridge.rs` 仅内存态 `DbConnectionRegistry`，无 `save_connections` 系列函数 → 制品缺失，按「存在才判」降级为 no-op |

**验证（提升后三态全绿）**：
```text
python3 scripts/check-database-policy.py                → ACTIVE=14，all invariants hold
python3 scripts/check-database-policy.py --expect-pending → NONE（1 个 pending 未实现）
python3 scripts/check-database-policy.py --self-test   → PASS：2 好样本零违规 + 15 坏样本全检出
```

> docstring 原第 33–39 行曾声称「4 个 A3 码位已由 pending 提升为 ACTIVE」，但 `ACTIVE_CODES`
> 元组并未更新——属文档/代码失配。本次已据实修正 docstring：7 提升、1 保留，并标注
> `DB_CRED_NAMESPACE` 当前为「弱守」（仅扫 `database.rs`；凭据 Keyring 调用实际在 `bridge.rs`）。

---

## 2. 契约一致性复核（M4-1 a/b/c/d ↔ 落地实现）

### 2.1 符合项（可解除 A3 / A4 阻塞）

| 契约点 | 落地证据 | 判定 |
|---|---|---|
| 依赖栈同步（不引 tokio 直接依赖） | `Cargo.toml` 无 tokio 进 `[dependencies]`；`database.rs` 用 `std::thread`/`Condvar` | ✅ |
| 错误码闭合 18 码 + `DB_` 前缀 | `domain.rs::DbErrorCode` 变体/`as_str`/映射一致（`DB_ERROR_CODE_CLOSED` ACTIVE 通过） | ✅ |
| 3 个 db 命令 + `check_invocation_source` | `bridge.rs:5959/5990/6047` 均首行 `check_invocation_source(...)` | ✅ |
| 凭据命名空间 `db:<conn_id>` | `bridge.rs:5974/6015/6059` 经 `crate::database::credential_key` → `db:` 前缀 | ✅ |
| 审计脱敏（detail 禁含 SQL/凭据） | `bridge.rs:5981/6035/6060` 仅记 `conn_id/rows/truncated/removed` | ✅ |
| 结果截断标记 | `database.rs::DbQueryResult` 含 `truncated`/`field_truncated` | ✅（但缺 `limit_hit`/`state`，见 D44） |
| 超时分层 30/600/5 | `database.rs` 三常量均在 | ✅（但类型/重定义见 D43） |
| 非 Linux 显式降级 | `database.rs` `target_os` 分支返 `NotSupported` | ✅ |

### 2.2 漂移项（**契约与实现不一致**，需 A0/A3/A4/A5 裁决）

#### D43 — `database.rs` 重定义 `domain.rs` 已冻结的 `DB_*` 常量（M4-1.c §1 违反）

- **证据**：`database.rs:39–59` 重定义 9 个常量
 （`DB_MAX_SQL_BYTES`/`DB_MAX_ROWS`/`DB_MAX_RESULT_BYTES`/`DB_MAX_TEXT_FIELD_BYTES`/
  `DB_DEFAULT_QUERY_TIMEOUT_SECS`/`DB_MAX_QUERY_TIMEOUT_SECS`/`DB_SOFT_TO_HARD_GRACE_SECS`/
  `DB_CANCEL_CHECK_EVERY_ROWS`/`DB_MAX_EXPORT_BYTES`），而 `domain.rs:1014–1047` 已是冻结单源。
- **额外风险**：超时三常量 `database.rs` 用 `u64`，`domain.rs` 用 `u32`
  （`domain.rs:1031/1035/1039`），类型不一致；后续改 `domain.rs` 不会自动传播 → 静默漂移。
- **归属/修复**：A3（`database.rs` 属 M4-2）。修复 = 删 `database.rs` 本地定义，
  `use crate::domain::{...}` 并 `Duration::from_secs(x as u64)`。
- **建议**：A4/A3 在 `check-database-policy.py` 增 `DB_LIMIT_CONST_REDEFINED` 码位
  （扫 `database*.rs` 中 `pub const DB_` 且与 `domain.rs` 同名即违），固化单源。

#### D44 — 结果 DTO / `DbValue` 双份定义且 serde 形态不一致（M4-1.c §2 违反，**含活 UI 缺陷**）

- **DTO 缺字段**：`database.rs:151–163` 的 `DbQueryResult` 字段为
  `columns/rows/row_count/truncated/field_truncated/elapsed_ms/query_id`，
  **缺** `domain.rs:1101` 冻结要求的 `limit_hit: Option<DbLimitKind>` 与 `state: DbQueryState`。
  → `db_query` 返回结构丢弃「超限原因」与「终止状态」，前端无法正确渲染截断原因/取消态。
- **枚举名漂移**：`database.rs:123–133` `DbValue` = `Null/Bool/I64/F64/Text/Binary{bytes}`；
  `domain.rs:1085` 冻结 = `Null/Bool/Int/Float/Text/BlobLen(u64)`（snake_case → `int/float/blob_len`）。
  `db_query` 实际序列化走 `database::DbValue`，故 wire 形态为 `{"i64":..}`/`{"binary":{"bytes":..}}`。
- **活 UI 缺陷**：`src/utils/dbUi.ts:242–262` 的 `decodeDbValue` 按**契约（domain）**解码
  `Int`/`Float`/`BlobLen` → 实际收到 `i64`/`f64`/`binary` 分支全部失配，
  二进制单元格落入 `JSON.stringify(value)` 兜底（显示 `{"binary":{"bytes":5}}` 而非 `<binary 5 B>`）。
- **归属/修复**：A3（`database::DbQueryResult`/`DbValue`）须对齐 `domain.rs` 冻结结构
  （补 `limit_hit`/`state`，枚举改名 `Int/Float/BlobLen`）；A5（`dbUi.ts`、未来 `src/types.ts`）
  当前按契约书写，待 A3 对齐后即正确——但 A5 在接线前须知悉此漂移，避免按 `database.rs` 现状写镜像。

#### D45 — `db_disconnect` 做凭据吊销，与 M4-1.b §3 / O-A2-1 / D28 默认相反

- **证据**：`bridge.rs:6059` `KeyringStore::delete_token(&crate::database::credential_key(&conn_id))`
  在断连时吊销凭据；而 M4-1.b §3「断连保留凭据；G-1 吊销要求降级为 D28 挂账」、O-A2-1 均默认保留。
- **性质**：实现侧对 D28 的**自行裁决**（走 A10 G-1 吊销路线），未经 A0 书面确认。
- **归属/裁决**：A0 二选一——(a) 接受吊销即关闭 D28；(b) 改回「断连保留」并显式落 `db_forget_connection`（D28 原案）。

> 以上三债务建议交 A0/A11 登记进 `logs/assist/M4-A11-debt-ledger-*.md`（编号从 **D43** 起，
> 因 A11 已占 D30–D42）。本 Lane 不擅自改债务台账。

---

## 3. 解除阻塞声明（Batch Dispatch Finish 条件）

| Lane | 状态 | 依据 |
|---|---|---|
| **A3（M4-2 连接池/查询）** | ✅ **解除阻塞**（实现符合契约），但有 **D43 / D44** 待修 | 依赖栈/错误码/截断/超时/降级全部符合；常量重定义与 DTO 漂移为后续清理项 |
| **A4（M4-2.s / M4-3 命令层）** | ✅ **解除阻塞** | 3 命令 + `check_invocation_source` + ACL 顺序 + 审计脱敏均符合；`DB_CRED_NAMESPACE` 弱守待收口 |
| **A5（M4-4 UI）** | ⏸ **仍受 F-3 约束** + 新增 D44 知悉项 | 命令名/DTO 待 A4 冻结；且须按 `domain.rs` 而非 `database.rs` 现状写 TS 镜像，知悉 D44 漂移 |

---

## 4. 元项：R-6「M4-1.c 矛盾状态」

债务台账（`M4-A11-debt-ledger`）将「M4-1.c 矛盾状态（R-6）」列为 A0 待裁。
但 `M4-1.c-20260905-2255.md` §9（第 133–145 行）已记录：
`STOPPED_EMPTY_ARTIFACT` 系 A0 读取竞态（0 字节瞬间）所致，契约正文已完整落盘，
标记段已在本版重写中移除。**物理标记段已无残留**（全文件仅 §9 的追溯记录）。
→ 建议 A0 **关闭 R-6**，无需再裁。

---

## 5. 验证证据（复跑命令）

```text
# IF-3
python3 scripts/check-tools-policy.py --self-test     # OK: 16 坏样本全检出
python3 scripts/check-tools-policy.py                 # all invariants hold

# ④
python3 scripts/check-database-policy.py              # ACTIVE=14, all invariants hold
python3 scripts/check-database-policy.py --expect-pending  # NONE（1 pending）
python3 scripts/check-database-policy.py --self-test  # PASS: 15 坏样本全检出

# 漂移取证
grep -n "pub const DB_" src-tauri/src/database.rs     # D43：39–59 重定义
grep -n "pub struct DbQueryResult" src-tauri/src/database.rs  # D44a：151，缺 limit_hit/state
grep -n "pub enum DbValue" -A8 src-tauri/src/database.rs      # D44b：123，I64/F64/Binary
grep -n "delete_token" src-tauri/src/bridge.rs        # D45：6059 断连吊销
sed -n '242,262p' src/utils/dbUi.ts                   # D44c：按 domain 解码 Int/Float/BlobLen
```

---

## 6. 交付物清单

| 文件 | 性质 | 说明 |
|---|---|---|
| `scripts/check-tools-policy.py` | 修改（tracked） | IF-3 修复，坏样本 #10 改为锚点无关变异 |
| `scripts/check-database-policy.py` | 修改（untracked，与 A4 共拥） | ④ 提升 7/8 pending→ACTIVE，docstring 据实修正 |
| `logs/checkpoints/A2-M4-1-contract-closure-20260906-1010.md` | 新增 | 本文件 |
| `logs/assist/A2-M4-1-drift-findings-20260906-1010.md` | 新增 | D43/D44/D45 给 A3/A4/A5 的修复建议 |
| `Lane-A2-M4-1-closure-20260906-1010.patch` | 新增 | 仅含本 Lane hunk（check-tools-policy.py + 两份 md）；`check-database-policy.py` 因共拥文件不进 patch，已在树中，由 A0 与 A4 调和 |

> 本 Lane 仅 `git diff` 自身 hunk，不触碰他 Lane 文件；不 push（仅 A0 可 push master）。

---

## 7. 协调须知：IF-3 与 A1 重叠交付（必读）

`check-tools-policy.py` 是 M2-7/M2-8 工具契约夹具，pre-merge ③ 标注为「**A1/A2 工具策略**」，
故 A1、A2 均有权修。本次出现**双 Lane 等价交付**：

- **A2（本 Lane）**：已在工作树中直接改 `check-tools-policy.py` 坏样本 #10
  （锚点无关变异，`scripts/check-tools-policy.py:368–388`），`--self-test` 已实测全绿。
- **A1（独立补丁）**：`logs/checkpoints/A1-IF3-tools-policy-fixture-fix-20260906-0658.patch`
  （未 apply，仅落盘为文件），修复逻辑与 A2 **完全等价**（仅字面量写法略有差异）。

**两版本功能等价、结果一致**，但作用于同一 `@@ -366,15` 区块 → **A0 只能取其一，切勿叠加**：

- ✅ 推荐：**保留工作树现有 A2 版本**（已绿），将 A1 的 `.patch` 视为**冗余**，
  集成时不 apply 它（或保留为历史证据即可）。
- ⚠️ 若 A0 偏好 A1 的规范版本：须先 `git checkout scripts/check-tools-policy.py` 还原，
  再 `git apply A1-IF3-tools-policy-fixture-fix-20260906-0658.patch`；此时本 Lane 的
  `Lane-A2-M4-1-closure-20260906-1010.patch` 中 **check-tools-policy.py 的 hunk 须跳过**，
  否则两个补丁在同一文件冲突。

> 无论取哪版，IF-3 物理已修复（`--self-test` 16 坏样本全检出）。冲突风险只在于「双补丁叠加」，
> 不在功能。请 A0 在集成说明中显式标注采用了哪一份。

## 8. 集成前 A0 待办（汇总）

1. **pre-merge ③（IF-3）**：取 A1 或 A2 任一 IF-3 修复（见 §7），勿叠加。
2. **pre-merge ④（DB pending）**：本 Lane 已在树中把 `check-database-policy.py` 的
   7 个 pending 提升为 ACTIVE、`--expect-pending` 返回 NONE；A0 提交时连同
   `?? scripts/check-database-policy.py` 一并纳入（该文件为 A4 交付、A2 提升，共拥）。
3. **债务 D43/D44/D45**：交 A11 登记进 `M4-A11-debt-ledger`（编号从 D43 起），
   并指派 A3（D43/D44）、A0（D45/D28 裁决）。
4. **R-6（M4-1.c 矛盾状态）**：据 §4 建议关闭，无需再裁。
5. **未触碰**的非本 Lane 阻塞项：① cargo fmt、② build metrics（均属 A7/A0），不在本 Lane 范围。
