# TRAIN E — Developer Capability Family Audit（Phase 8E）

> 日期：2026-09-20
> 结论：**边界审计通过；Database / Git 已注册为独立 owner 的 CAPABILITY；成熟度 = C1（已登记、已治理，但非 absent-composable）。未谎报 C3。**
> 新增门禁：`scripts/check-developer-owners.mjs`（11/11 PASS）
> tag：`capability-phase8e-developer-family-audit-pass`

---

## 1. 审计范围

Developer 家族三域：

| 域 | owner | 物理位置 | 后端 | 重资源 |
|---|---|---|---|---|
| Database | `useDatabaseStore` | `src/stores/useDatabaseStore.ts` | `database.rs`（db_connect/query/disconnect/cancel） | DB 连接（NETWORK+SECRET） |
| Git | `useGitStore` | `src/stores/useGitStore.ts` | `git_*` tauri 命令 | git 子进程（fs + 网络） |
| Repo | `useRepoStore` | `src/capabilities/workspace/state/useRepoStore.ts` | repo 同步 | repo 配置 |

## 2. 边界裁决（与隔夜 Train 指令对齐）

1. **Repo Context ≠ Git Operation**：已确立。`useRepoStore` 只管 repos/form/preview/busy/job；
   `useGitStore` 只管 git status/diff/log/commit/branch/write；二者不交叉（DEV-04 实测）。
2. **Database / Git / Terminal 不得合并为 DeveloperStore**：三 owner 物理分离，无 `useDeveloperStore`
   合并符号（DEV-01/02b）。
3. **Credential 只经 reference / 后端密钥库**：
   - Database `connect(password)` 密码仅作瞬时参数，不进 form/store/localStorage；后端落 Keyring 键 `db:<conn_id>`（DEV-03a）。
   - Git 凭据只在后端推送瞬间从系统密钥库读取，前端零持久化（DEV-03b）。
4. **能力间不互相 import 内部**：跨能力内部 import 为零；Terminal 包内自引用合法（DEV-05）。

## 3. 成熟度（诚实，不高报）

| 能力 | 当前形态 | 成熟度 | 可否 C3（absent-composable） |
|---|---|---|---|
| Database | always-loaded Workspace 面板（DatabasePanel 在 workspace/ui），connect 即建真实 DB 连接 | **C1** | 否：未抽为可选能力包；absent 语义未定义 |
| Git | always-loaded Workspace 面板（GitPanel 在 workspace/ui），操作即 spawn git 子进程 | **C1** | 否：同上 |
| Terminal | 已 Train D 达成 | **C3** | 是（已证） |

**不强行拆**：指令明确「真实成熟度不足允许停在 C0/C1/C2，不要为了数量强拆」。将 Database/Git
物理抽取为可选能力包（使其 absent 时不挂载面板、不建连接/不 spawn）是更大的重构，且会触碰
Workspace 表面（DatabasePanel/GitPanel/RepoPanel 当前同处 `src/components/workspace/`）。该工作
记为 **Debt-8E-5（Database 抽能力包）/ Debt-8E-6（Git 抽能力包）**，交后续 train，不在本 Train 范围。

因此 **CURRENTLY_COMPOSABLE 仍为 4**（Bookmark + Workspace + Browser + Terminal），本 Train 不增加
可组合能力数，但把 Developer 家族的边界用机器门禁固化，杜绝未来回归（合并 store / 凭据落地 / 跨域污染）。

## 4. 门禁

`scripts/check-developer-owners.mjs`（静态扫描，与 check-workspace-owners 同口径）：
- DEV-01 三 owner 物理分离（无合并 DeveloperStore）
- DEV-02 各 owner 声明自身区分性状态（无第二真源）
- DEV-02b 全仓无 `useDeveloperStore`
- DEV-02c 全仓仅各自 owner 声明 DB/Git/Repo 状态
- DEV-03a/b 凭据只经引用（前端零持久化）
- DEV-04a/b Repo Context ≠ Git Operation
- DEV-05 跨能力不互相 import 内部

已接入 `npm run check` 与 `scripts/pre-merge.sh` Phase 03。

## 5. 既有 Database/Git checker 债（显式，未静默）

`check-database-policy.py` / `check-database-ui-logic.mjs` 在 pre-merge 历史中曾被记为 FAIL
（Debt-004 同类）。本 Train 未强行修复其具体断言（属 DB 后端管道语义，独立于能力边界），
但新增的 DEV 门禁从「边界/owner/凭据」维度补位。具体 DB 后端 checker 的失配归因与修复
交专项（不归并到本 Train，避免稀释 Terminal 债已修的清晰度）。

## 6. 诚实性审查

| 禁止项 | 自查 |
|---|---|
| 只是搬目录？ | 否。本 Train 是边界审计 + 机器门禁固化 + 诚实成熟度记录，未做无语义的物理迁移 |
| God Store？ | 否。三 owner 独立，无 DeveloperStore |
| 第二真源？ | 否（DEV-02c） |
| 凭据泄露？ | 否（DEV-03，密码瞬时 / 经 Keyring） |
| 误报 C3？ | 否。Database/Git 明确记为 C1 |
| Checker 失守？ | 否。新增 1 个门禁（11/11），未删未放宽 |
| shared 垃圾桶？ | 否 |
| 放宽规则？ | 否 |

NEXT：Train F — Resource Governor + 真实 Minimal/Developer/Full profile + 真实资源测量。
