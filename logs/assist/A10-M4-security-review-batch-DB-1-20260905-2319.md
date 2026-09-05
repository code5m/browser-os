# A10 · Batch DB-1 安全复核（A2 + A3 + A4）

> Lane: `A10` — M4 security review（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-05 23:19 CST
> Base: `85d2d7b`（`master`；`git fetch` + `git pull --ff-only` → 已经是最新的）
> 批次: **Batch DB-1**（board §Batch Implementation Dispatch → Lane A10 → Batch DB-1: A2+A3+A4 together）
> 范围: **只读复核**。本轮**未改任何产品代码、未改任何策略脚本**（board：A10「May edit policy scripts only if A0 explicitly assigns a fix」，A0 未指派）
> 前序: `logs/assist/A10-M4-security-review-20260905-2240.md`（G-1~G-12）、`logs/assist/A10-M4-security-recheck-A1A2A6-20260905-2345.md`（R-1~R-11）。本文件为增量，不替代二者。

---

## 1. 批次状态

| Lane | 本批产出 | 评审结论 |
|---|---|---|
| **A2**（M4-1 契约） | `domain.rs` 类型提案（前批已评）+ **本批新增上限常量** | PASS（常量已落代码，数值与 M4-1.c 一致） |
| **A3**（M4-2 池/驱动基础） | `Cargo.toml` / `Cargo.lock` 加入 `rusqlite`(bundled) / `mysql` / `postgres`；**`database.rs` 尚未创建** | **PASS_WITH_DEBT**：依赖正确，但模块未落地，包不自洽 |
| **A4**（M4-2.s 安全闸门） | `security_policy.rs` +896 行：SQL 风险分类器、生产判定、写闸门、测试 | **PASS**（G-4 / G-5 实质达标） |

**批次裁定：`PASS_WITH_DEBT`**。5 条发现（D1-1 ~ D1-5），其中 **1 条【中高】**。

---

## 2. 正向确认（红线达标项）

### 2.1 A4 分类器实质落实 G-4 / G-5 —— 本批最重要的安全资产

`classify_sql_risk`（`security_policy.rs:912`）的实现顺序**逐条对应**我上一轮的 G-4：

| G-4 要求 | 实现 | 位置 |
|---|---|---|
| ① 确定性剥离注释与字符串字面量 | `mask_literals_and_comments(sql)`，失败即 `Err` | `:927-930` |
| ② 注释/引号未闭合 → 拒绝（fail-closed） | `Err(_) ⇒ DB_SQL_PARSE_FAILED` | `:929` |
| ③ 整批处理，批中多于一条语句**整批拒绝** | `count > 1 ⇒ DB_MULTIPLE_STATEMENTS` | `:941-945` |
| ④ 不可解析即拒，不降级放行 | 未知动词 ⇒ `SqlRiskClass::Unknown` ⇒ `DB_SQL_PARSE_FAILED` | `:949-953` |
| ⑤ 不可用于「反向放行」 | `is_write_statement` 文档明写「fail-closed…调用方只能用它来**拒绝**，不能反过来用它放行」 | `:962-968` |

**G-5（生产判定 fail-closed）**：`is_production_database`（`:1012`）中 S5（库名空 / 主机不可解析）与 S2（名称命中生产词表）、S4（公网未加密）**一律返回 `Unknown`**（`:1022/1025/1029/1032/1035`）；写闸门 `:1084-1096` 中
- `Ddl | Admin ⇒ WriteDenied`
- `Production ⇒ WriteDenied`
- **`Unknown ⇒ ProductionUnknown`（拒绝）**
- `NonProduction` 仍需 `allow_write` 才放行，否则 `WriteDenied`

即「无信息 = 拒绝写」，与 M4-1.d §3 规则 4 一致。**G-5 实质达标。**

### 2.2 依赖栈符合冻结裁定

`Cargo.toml` 新增仅三项，且**精确等于** M4-1.a 裁定的同步栈：

```toml
rusqlite = { version = "0.40.2", features = ["bundled"] }
mysql = "28.0.2"
postgres = "0.19.14"
```

`tokio` / `sqlx` / `diesel` / JDBC / YAML / 代码生成 **均未引入**（与 `DB_UNDECLARED_RUNTIME`、`DB_JDBC_SIDECAR` 码位一致）。

### 2.3 上限常量已落代码（关闭我上一轮的 R-7）

`domain.rs:1014-1027`：

| 常量 | 值 | 与 M4-1.c §1 冻结值 |
|---|---|---|
| `DB_MAX_SQL_BYTES` | 64 KiB | ✅ 一致 |
| `DB_MAX_ROWS` | 1 000 | ✅ 一致 |
| `DB_MAX_RESULT_BYTES` | 4 MiB | ✅ 一致 |
| `DB_MAX_TEXT_FIELD_BYTES` | 64 KiB | ✅ 一致 |

上一轮 R-7「上限常量未落代码、当前零代码级约束」**已由 A3 关闭**。

---

## 3. 发现

### D1-1【中高】`DB_MAX_SQL_BYTES` 被重复定义，形成双真相源

**事实**：同一个安全上限常量在两处各定义一次——

```text
src-tauri/src/domain.rs:1014          pub const DB_MAX_SQL_BYTES: usize = 64 * 1024;
src-tauri/src/security_policy.rs:576  pub const DB_MAX_SQL_BYTES: usize = 64 * 1024;
```

**现状**：两处数值**当前一致**（均 64 KiB，且与 M4-1.c 冻结值相符），故**尚无实际绕过**。
**风险**：这是一个安全上限的两个独立真相源，无任何机制强制同步。若将来只改一处（例如把 `domain.rs` 调到 128 KiB 而 `security_policy.rs` 仍是 64 KiB），则**分类器按 64 KiB 拒绝、调用方按 128 KiB 放行**，或反之——产生「分类与执行不一致」的真实绕过窗。同族问题在本项目有过教训（M2-2.b「通道/边界/命令三处同步」）。
**建议（A3/A4 协商，A0 指定其一改）**：`security_policy.rs` 改为 `use crate::domain::DB_MAX_SQL_BYTES;`（或由 `domain.rs` 反向引用），**只保留一处定义**。
**建议加码位**：`DB_LIMIT_CONSTANT_DUPLICATED` —— 扫描 `src-tauri/src/*.rs`，同名 `pub const DB_*` 出现多于一次即报。

### D1-2【中】A3 的 `database.rs` 尚未落地，本批「代码包」不自洽

**事实**：board §Batch Implementation Dispatch → Lane A3 要求「Add `src-tauri/src/database.rs` or `src-tauri/src/database/`」并包含连接校验、凭据键助手 `db:<conn_id>`、`DbPool`、连接/断开生命周期、SQLite 路径根校验、结果 DTO 与取消/超时钩子。
**实测**：`ls src-tauri/src/database*.rs` → **不存在**；本批 A3 的改动**只有** `Cargo.toml` 与 `Cargo.lock`。
**影响**：依赖已就位但模块缺失 → 编译期会引入未使用的依赖；A4 的命令层无内部 API 可调用。**这本身不违反安全红线**（未落地即无风险面），但使 Batch DB-1 无法判定为完整交付。
**建议**：A3 补齐 `database.rs` 后再请 A10 复核该模块（凭据键命名空间、路径根校验、结果截断、取消/超时为本批复核重点）。

### D1-3【中】二进制体积 +14.6%，逼近 15% 门禁上限（跨批次风险，详见 SCHED-1 的 S1-4）

`measure-build-metrics.py --compare … --skip-build` 输出 `total_bytes_pct: 14.6`，未超 15% 上限（`exceeds_growth_limit: false`），但**余量仅 0.4 个百分点**。
**风险**：M4-2 尚未接线（无 `database.rs`、无命令）就已耗尽体积预算；A4/A5 后续必然再增，届时 `build metrics` 会**硬失败**且难以回旋（要么抬门槛、要么砍依赖）。
**建议（A0）**：提前决策——是上调门禁阈值并说明理由，还是改用 `rusqlite` 非 bundled（依赖系统 sqlite）以省体积。不要等到红灯才处理。

### D1-4【低】`M4-1.c` 仍为 STOPPED，但常量已被下游按文档值落地

board 第 112 行确认「`M4-1.c` remains STOPPED and must be completed」；而 A3 已按 M4-1.c §1 的数值把常量写进 `domain.rs`。
**评估**：**数值本身正确且可用**，不构成安全问题；但「契约卡 STOPPED、实现已按其落地」的状态应尽快由 A2 收口（board §Lane A2 已派工），否则后续若 M4-1.c 被改写，代码与契约会漂移。
**继承上一轮 R-6**：`M4-1.c-20260905-2255.md` 仍是「完整正文 + 尾部 STOPPED 裁定」并存的矛盾产物。

### D1-5【低】本批无新增命令，source check / ACL 面尚未产生

`git diff src-tauri/permissions/default-commands.toml` → **空**；`main.rs` 中无 `db_*` 注册。
**评估**：故本批**不涉及** G-10 / R-9（ACL 插位、来源校验）。这是**中性事实而非缺陷**——A4 的派工也明确要求「Do not implement bridge commands until A3 pool boundary exists」。
**待办**：A4 落地命令时须一次性满足 `DB_ACL_ORDER`（插在 `list_artifact_images` 之前）、`check_invocation_source` 置首行、审计脱敏三条。

---

## 4. 红线对照（Batch DB-1）

| 红线 | 结论 | 依据 |
|---|---|---|
| 凭据 | ✅ 本批无凭据面 | 无命令、无 `database.rs`；`DbConnectionConfig` 结构性无凭据字段（前批 T-db-c3） |
| 写权限 | ✅ 达标 | 写闸门 `:1084-1096`，`Unknown ⇒ ProductionUnknown ⇒ 拒绝` |
| 结果上限/取消 | ⚠️ 常量已落、实现未至 | 常量正确（`:1014-1027`）；取数循环/截断/取消尚无 `database.rs` 承载 |
| 隐私 | ✅ 无违规 | 分类器不落盘、不记录 SQL 正文；审计脱敏待命令层落地时复核 |
| source check / ACL | ➖ 本批不适用 | 无新增命令 |

---

## 5. 声明

- 本轮**未修改任何产品代码或策略脚本**（board：A10 仅在 A0 明确指派时方可改策略脚本；D1-1 的修复需 A0 指派 A3 或 A4 之一执行，我未擅改）。
- 未复跑 `cargo test` / `npm run build` 全量：`pre-merge.sh` 实跑 **EXIT=1**（唯一 FAIL = build metrics，见 SCHED-1 §S1-4），已作为证据记录；本轮仅执行 `cargo check` 取样警告分布。
- 结论均基于源码实证（`security_policy.rs:912-1096`、`domain.rs:1014-1027`、`Cargo.toml` diff），非文档互证。
