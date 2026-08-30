# AI 模型切换与接手清单

> 文档角色：跨 Codex / Trae 的唯一接手入口；只记录当前执行指针、模型映射、交付证据和回写规则。
> 文档版本：V1.9。
> 更新时间：2026-08-30 15:38 CST。
> 当前状态：`CODEX_RUNNING`。
> 当前分支：`feature-M0-baseline`。
> 当前执行器：Codex 主任务；机械文档审计与独立脚本任务已委派 `gpt-5.6-luna / low`，最终裁决仍由主任务负责。
> 冲突裁决：WBS/验收以 `详细设计与实施计划.md` 为准，指标语义以冻结契约为准，本文只维护跨模型执行指针和交接证据。

---

## 1. 一眼看懂当前进度

| 项目 | 当前值 |
|------|--------|
| 已完成 WBS | `M0-0`（6 项 UNSTABLE 已裁决）、`M0-1 = PASS` |
| 已拒证据 | `ac0ecac` 三批候选：场景错误 + aggregate `UNSTABLE`，结论 `REJECTED`，不得复用 |
| 已落地修复 | `e8975d6`：保留 `about:` URL、关闭失败显式返回、单例扫描线程与插件状态清理 |
| 正式三批 | 源提交 `93a1ba6`；六批门禁逐批 PASS；raw aggregate `UNSTABLE`；manifest `20260830T153538+0800_93a1ba6_M0-0.c` |
| 当前硬风险 | tab RSS 三批持续正增长，必须由 M0-5 关闭；不属于 M0-0 PASS 伪装项 |
| 下一检查点 | `M0-2.a = NEXT`（资源所有权表 + 现状失败测试） |
| 下一任务路由 | `AI:DEEP / R:high` |
| 自动执行范围 | 仅 M0；按唯一关键路径逐点推进，每点独立验收和提交 |
| 必停门禁 | 见 §3「硬停止条件」；`M0-7.c` 必须等项目负责人确认 |
| 禁止启动 | M1~M5；当前检查点未提交前禁止夹带下一检查点 |
| 最近实现提交 | `e8975d6 fix(M0-0.b): close tab lifecycle leaks` |
| 最近门禁提交 | `73e9dfb fix(M0-1.c): validate versioned evidence safely` |
| 最近裁决提交 | `b9077d9 docs(M0-0.c): retain rejected formal baseline evidence` |
| 最新状态证据 | `logs/checkpoints/M0-0.c-20260830-1538.md` |
| 交接基线提交 | `504fcd7 docs(handoff): prepare Trae quota-window transfer` |
| 工作树要求 | 执行器开工前、每个提交后和交付时都必须干净 |

已经完成：

- 建立 `feature-M0-baseline` 分支。
- 完成 M0~M5 优先级、模型路由、原子检查点和验收门禁整理。
- 使用 `gpt-5.6-terra / medium` 独立审阅并完成 M0-0.a。
- 冻结 `logs/m0-baseline-contract-v1.md`：21 个 `REQUIRED_NOW` 指标、2 个延迟指标、环境指纹、固定场景、统计公式和证据目录。
- 将旧 `logs/baseline-2026-08-27.md` 降级为 `EXPLORATORY`，禁止当作正式性能基线。
- 修复原计划中的循环依赖，关键路径现为：
  `M0-0(完成，六项 UNSTABLE 已裁决) -> M0-1(PASS) -> M0-2.a(NEXT) -> ...`。
- 完成 M0-1.a（commit `1bd56b1`）：新增 `scripts/baseline-check.sh` 与 `scripts/fixtures/clippy-sample.json`；
  验收命令全过、`--self-test` 输出 `SELF_TEST_RESULT=ALL_PASS`；`.gitignore` 对 `logs/m0-baseline/` 开例外，
  原始 `.log` 证据随 run 目录入库可追溯（见 §5 证据保留说明）。
- 完成 M0-1.b（commit `506193b`）：新增 `scripts/verify-resources.sh`（资源验证驱动），
  覆盖 release 启动、进程树 RSS/FD 采样、tab/grid/terminal 资源循环、终端吞吐与孤儿进程检测驱动；
  正式模式在 ready/终端钩子缺失时输出完整 BLOCKED 证据并退出 1（不伪造 PASS）；
  `--self-test` 8 用例 ALL_PASS（统计计算/进程树/孤儿/BLOCKED 语义/SHA256SUMS）。
- 完成 M0-1.c（commit `2b476d3`）：固定 M0-1 两脚本的参数、退出码（0/1/2）、JSON schema
  （`scripts/schema/m0-summary.schema.json` + 共享校验器 `scripts/validate-summary.py`）、
  完整性哈希（SHA256SUMS）和日志格式（`scripts/GATE-CONTRACT.md`），并接入本地 pre-merge
  流程（`scripts/pre-merge.sh` + `.githooks/pre-merge-commit` 可选 hook，不代改 git 配置）；
  baseline `--self-test` 增至 7 用例、verify 增至 9 用例（各含 schema 校验用例），全部 ALL_PASS。
- 完成正式总控与后续门禁修复：`ac0ecac` 保证多批采集原子归档，`73e9dfb` 对全部证据校验 SHA 并按 V1.0/V1.1 选择 schema；最新完整 pre-merge 为 `ALL_PASS`。
- 拒绝 `ac0ecac` 的三批候选证据：tab 驱动把 `about:blank` 当搜索词打开百度，六项 aggregate 指标为 `UNSTABLE`；裁决见 `logs/checkpoints/M0-0.c-20260830-0913.md`。
- `e8975d6` 已修复上述 URL 和页签生命周期缺陷；短样本 smoke 显示 root/process FD 稳定，但 smoke 不能替代 M0-0.b/c 正式重跑。
- 修复后源提交 `b82eb55` 已完成 M0-0.b 单批正式采样：质量、资源与 aggregate 均 PASS；证据与结果见 `logs/checkpoints/M0-0.b-20260830-1452.md`。
- 源提交 `93a1ba6` 已完成 M0-0.c 三批正式采样：质量/资源门禁逐批 PASS，raw aggregate 的六项 UNSTABLE 已逐项归因并转交 M0-5/M3；证据与比较规则见 `logs/checkpoints/M0-0.c-20260830-1538.md`。

## 2. 模型映射

WBS 只使用供应商无关的路由标签。切换平台时只改本表，不批量改 50 个 WBS。

| 路由 | 任务类型 | Codex 当前映射 | Trae 映射 |
|------|----------|----------------|-----------|
| `AI:FAST` | 文档、格式化、机械修改、明确的小 UI | `gpt-5.6-luna` | `TRAE_FAST`：待补精确模型名 |
| `AI:BALANCED` | 边界清晰的多文件功能、测试工具、普通重构 | `gpt-5.6-terra` | `TRAE_BALANCED`：待补精确模型名 |
| `AI:DEEP` | 架构、安全、生命周期、并发、IPC、疑难故障 | `gpt-5.6-sol` | `TRAE_DEEP`：待补精确模型名 |

### 最低额度执行法

1. 先运行仓库脚本；格式、schema、哈希、固定夹具、构建和可枚举文档扫描以脚本结果为准，不让模型逐文件重复判断。
2. 能写成“输入、输出、允许文件、验收命令”的独立任务，优先交给 `gpt-5.6-luna / low` 或 Trae 免费模型；默认只读，写任务必须使用互不重叠的文件范围。
3. 低成本模型只提交候选 diff/报告，不能单独移动 `NEXT` 或宣称检查点 PASS；主执行器只审关键 diff、失败路径和脚本结果。
4. `AI:BALANCED` 负责边界明确的实现与集成；`AI:DEEP` 仅用于安全、生命周期、并发、IPC、未知根因、连续失败和最终放行。
5. 长时间 GUI/性能采样由脚本一次性无人值守执行；参数、commit 或环境未冻结前禁止反复试跑。smoke 只用于排错，不写入正式 PASS 分母。

用户提供的微信临时图片在读取时已不存在，因此本文件不猜测 Trae 模型名称。重新附图或直接写出模型列表后，只填写上表三个 Trae 单元格并升级本文版本；任务上的 `AI:*` 标签不变。

Trae 选模规则：

1. `M0-0.b/c` 选 Trae 中具备仓库读写、Shell 执行和多文件理解能力的日常编码模型，映射到 `TRAE_BALANCED`；正式 GUI 采样由既有脚本驱动。
2. 如果免费模型无法运行 Shell 或稳定处理多文件，只允许调研，不得勾选检查点或提交 PASS。
3. M0-2、M0-3、安全、进程树和并发任务必须使用 `TRAE_DEEP`；没有强模型时暂停，不能用轻量模型硬做。
4. 每个检查点在证据中记录 UI 显示的完整模型名、平台、推理档位和 `MODEL_DEVIATION`。全局映射缺失不妨碍记录实际执行模型。

## 3. Trae 在 M0 内连续自动执行

「一次只领取一个检查点」是变更和提交边界，不是要求每做完一点都停下等用户确认。Trae 可以在同一次会话中尽可能推进 M0，但不得一次领取、实现或提交多个检查点。

### 连续执行循环

1. 从本文顶部和 `详细设计与实施计划.md` §2.2 读取唯一 `NEXT`，本轮只领取该检查点。
2. 按 `AI:*` 路由选择 Trae 模型；开始后不在该检查点中途换模型。
3. 只修改当前检查点需要的文件，运行该点全部验收和 `git diff --check`。
4. PASS 后回写 §5 规定的状态、模型、命令、证据和 `NEXT`；FAIL 不得移动指针。
5. 使用 `<type>(<WBS.checkpoint>): <单一结果>` 独立提交。功能实现用 `feat`，重构用 `refactor`，缺陷修复用 `fix`，不得合并多个检查点。
6. 提交后运行 `git status --short`。工作树干净且未命中硬停止条件时，无需等待用户回复，立即回到第 1 步领取新 `NEXT`。

### 硬停止条件

命中任一条即停止写代码，保留当前指针并回写 `BLOCKED`、已试命令、证据与推荐下一步：

1. 同一检查点连续两次验收失败，或根因仍不明。
2. 检查点要求 `AI:DEEP`，但 Trae 没有等效强模型；不得用轻量模型硬做安全、生命周期、并发或 IPC。
3. 缺少 GUI/登录态、必要权限、外部环境或人工验收条件，且无法用固定夹具替代。
4. 需要修改冻结契约、扩大安全权限、删除用户数据，或需要产品/项目负责人裁决。
5. 检查点边界的工作树不干净，或发现来源不明、可能属于用户的并行改动。
6. 当前平台额度或时间耗尽。
7. 到达 `M0-7.c` 项目负责人确认门禁，或下一指针属于 M1~M5。Trae 可以准备 M0-7.c 材料，但不得代替负责人签字。

## 4. 已完成检查点记录：M0-1.a

> 状态：**已完成**（2026-08-29，commit `1bd56b1`；验收命令全过 + `--self-test` ALL_PASS）。以下为执行时的边界与要求，保留作记录。

本节的允许/禁止范围只约束 `M0-1.a` 这一轮。该检查点 PASS、证据回写、独立提交且工作树干净后，执行器应立即领取 `M0-1.b`，不再受本节「不实现 M0-1.b」的单点边界限制。

### 目标

新增 `scripts/baseline-check.sh`，把 M0-0.a 契约中的质量、构建、来源/环境指纹和证据目录初始化编码成可复跑门禁。

### 开工前

```bash
git switch feature-M0-baseline
git status --short --branch
git log -3 --oneline
```

必须看到分支为 `feature-M0-baseline` 且工作树干净。若不干净，先审查差异来源，不得 reset、checkout 或覆盖未知改动。

必读文件：

1. `AI-模型切换与接手清单.md`
2. `详细设计与实施计划.md` §2.2 的 M0-0/M0-1
3. `logs/m0-baseline-contract-v1.md`
4. `.gitignore`、`package.json`、`src-tauri/Cargo.toml`

### 允许改动

- 新增 `scripts/baseline-check.sh`。
- 可新增只服务该脚本的固定夹具或自测文件，放在 `scripts/fixtures/` 或 `scripts/tests/`。
- 完成验证后，按 §5 回写状态文档和检查点证据。

### 禁止改动

- 不实现 `scripts/verify-resources.sh`，那属于 M0-1.b。
- 不修改 Rust/Vue 产品代码，不补 ready/终端测量钩子，那属于 M0-0.b。
- 不采集或宣称正式 release 性能基线，那属于 M0-0.b/c。
- 不处理 capability、资源泄漏、依赖清理或 GUI 回归。
- 不修改冻结契约的指标语义；若发现必须修改，停止并记录 `CONTRACT_CHANGE_REQUIRED`，不得静默改口径。

### 实现要求

- Bash 使用 `set -euo pipefail`，仓库根目录通过 `git rev-parse --show-toplevel` 获取，禁止硬编码 `/home/...`。
- 至少支持 `--help`、`--self-test`、正式运行模式和非法参数非零退出。
- 候选基线要求 clean commit；自测模式允许在开发工作树运行，但必须清晰标记为 fixture，不能生成正式 PASS。
- 初始化契约规定的 run 目录，生成来源/环境字段、命令原始输出、`summary.json` 和 `summary.md`。
- 覆盖 fmt、结构化 clippy warning 分组、前端构建/体积、release 二进制信息；不得把重复的人类可读 warning 行直接相加。
- 不自动安装依赖、不访问公网、不删除用户数据；生成物只进入本 run 的隔离目录。
- 当前 `.gitignore` 会忽略 `*.log`。M0-1.a 必须明确证据保留策略：至少提交 summary、环境、场景、结构化 measurements 和哈希；若要求原始 `.log` 入库，先提出契约版本变更，不能让它们静默丢失。
- 产品侧 ready/终端钩子尚未实现时，相关正式指标返回明确 `BLOCKED` 和非零退出；`--self-test` 使用固定夹具验证脚本本身。

### 验收命令

```bash
bash -n scripts/baseline-check.sh
bash scripts/baseline-check.sh --help
bash scripts/baseline-check.sh --self-test
bash scripts/baseline-check.sh --definitely-invalid
git diff --check
```

最后一条非法参数命令必须非零退出。若新增独立自测脚本，也必须执行并在证据中记录。

### 提交

本轮只提交 M0-1.a：

```text
feat(M0-1.a): add baseline quality gate
```

检查点未通过时不得勾选。修复提交使用 `fix(M0-1.a-fix1): ...`，不得伪装成下一检查点。PASS 并完成上述提交后，按 §3 继续自动执行。

## 5. 每个检查点做完后必须回写什么

### 必改文档

1. `详细设计与实施计划.md`
   - 升级文档版本、日期时间、实际模型。
   - 只有验收全部通过才把对应 `[ ]` 改为 `[x]`。
   - 更新下一检查点和任何真实阻塞，不修改未开始里程碑状态。
2. `后续需求TODO.md`
   - 更新“当前关键路径”和队列表状态。
   - 标出 `PASS/NEXT/BLOCKED`，不得只写“已开发”。
3. `AI-模型切换与接手清单.md`
   - 更新当前执行器、实际模型、最近提交、验证命令与结果。
   - 将“下一检查点”移动一格；失败则保持原检查点并写 fix 编号。
4. `logs/checkpoints/<WBS>-<YYYYMMDD-HHMM>.md`
   - 记录 commit、模型、变更文件、命令、期望/实际、证据路径和结论。

### 条件修改

- `logs/m0-baseline-contract-v1.md`：只有指标/场景/公式确需变化时修改，并升级契约版本；普通实现不得改。
- `logs/baseline-2026-08-27.md`：历史记录，原则上不再修改。
- README/使用指南：只有用户可见行为已经实现并验收后才更新。
- 源码架构文档：仅在对应架构检查点真正落地时更新。

### 回写模板

```text
CHECKPOINT=M0-1.a
STATUS=PASS|FAIL|BLOCKED
EXECUTOR=Trae|Codex
MODEL=<界面完整模型名>
ROUTE=AI:BALANCED
MODEL_DEVIATION=none|<原因>
COMMIT=<sha>
VERIFY=<命令及退出码>
NEXT=M0-1.b|M0-1.a-fix1
```

### M0-1.a 回写记录（2026-08-29，Codex / CodeBuddy 主机）

```text
CHECKPOINT=M0-1.a
STATUS=PASS
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=1bd56b1
VERIFY=bash -n(0); --help(0); --self-test(0, ALL_PASS); --definitely-invalid(2); git diff --check(0)
NEXT=M0-1.b
```

证据保留策略（M0-1.a 落地，对应实现要求「明确证据保留策略」）：`.gitignore` 已对 `logs/m0-baseline/` 开例外，
`raw/` 原始命令输出（含 `*.log`）随 run 目录入库可追溯；`summary/environment/scenario/measurements/SHA256SUMS`
全部提交；若未来需要调整原始 `.log` 入库口径，先提契约版本变更。

### M0-1.a-fix1 修复记录（2026-08-29，Codex / CodeBuddy 主机，commit `3a7bbac`）

发现并修复正式模式缺陷：前端构建失败（如依赖未装、超时）时 `run_frontend_build` 不设置
`DIST_TOTAL_BYTES/LARGEST_JS_*`，`write_measurements_frontend` 在 `set -u` 下 unbound 崩溃，
无法生成完整 FAIL 证据（违反「门禁失败返回非 0 且输出完整证据」语义，M0-0.b 会踩坑）。

修复内容：
1. `run_frontend_build` 开头初始化 dist 指标默认值，任何失败路径均不 unbound。
2. `summary.json` 中 dist_total_bytes / largest_js_* 状态与 `FRONTEND_STATUS` 联动（缺原始证据即 FAIL）。
3. `--self-test` 新增用例 6（`BS_SELF_TEST_FRONTEND_FAIL=1` fixture）：断言前端失败时退出非 0、
   summary status=FAIL、证据完整、不崩溃；用例 3 的退出码捕获改为 `|| code=$?` 防御 errexit。
4. 脚本版本 `M0-1.a-1` → `M0-1.a-2`。

回写模板（不移动指针，NEXT 仍为 M0-1.b）：
```text
CHECKPOINT=M0-1.a-fix1
STATUS=PASS（原 M0-1.a 维持 PASS，未改变检查点结论）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
COMMIT=3a7bbac
VERIFY=bash -n(0); --help(0); --self-test(0, ALL_PASS, 6 用例); --definitely-invalid(2); git diff --check(0)
NEXT=M0-1.b
```

### M0-1.b 回写记录（2026-08-29，CodeBuddy / Codex 主机，commit `506193b`）

```text
CHECKPOINT=M0-1.b
STATUS=PASS（驱动能力与 BLOCKED 语义验收通过；产品 ready/终端钩子未落地，正式基线归 M0-0.b）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=506193b
VERIFY=bash -n(0); --help(0); --self-test(0, ALL_PASS, 8 用例); --definitely-invalid(2); git diff --check(0)
NEXT=M0-1.c
```

实现要点（对应 §7 提示词）：
- 新增 `scripts/verify-resources.sh`（1139 行）：`set -euo pipefail`、根目录 `git rev-parse --show-toplevel` 动态解析；
  `--help` / `--self-test` / 非法参数非零退出（2）/ 正式模式要求 clean commit。
- 覆盖契约 §5/§6 资源指标驱动：release 启动（§6.1）、进程树枚举（`/proc/<pid>/stat` starttime 防 PID 复用）、
  RSS（VmRSS）与 FD（`/proc/<pid>/fd` 可访问条目）采样、idle 采样编排（每 5 秒 × 60 秒）、
  tab/grid/terminal 循环（5 预热 + 20 正式，OLS 斜率/FD delta/孤儿进程检测）、终端吞吐（10 MiB begin/end 标记）。
- 统计与契约 §7 一致：median/min/max/波动率、p95 nearest-rank（<20 样本不伪报）、OLS 斜率。
- 正式模式钩子缺失：`driver_smoke`（真实 /proc 冒烟）后输出完整 BLOCKED 证据
  （environment/scenario/summary.json|md/SHA256SUMS/measurements，`summary status=BLOCKED`）并退出 1；
  10 个资源指标 owner=M0-0.b（补钩子的 WBS），DEFERRED(M2-4/M4-3) 不计入判定。
- 正式 BLOCKED run 证据：`logs/m0-baseline/20260829T143410+0800_506193b_release_x11/`（已入库）。
- 未实现 M0-1.c（参数/退出码/schema/哈希/日志格式统一与 pre-merge 接入）；未修改冻结契约与产品代码。

### M0-1.c 回写记录（2026-08-29，CodeBuddy / Codex 主机，commit `2b476d3`）

```text
CHECKPOINT=M0-1.c
STATUS=PASS（参数/退出码/schema/哈希/日志格式已固定并接入 pre-merge；产品 ready/终端钩子未落地，正式基线归 M0-0.b）
EXECUTOR=CodeBuddy（Codex 主机）
MODEL=界面未显示完整模型名；按 Codex 映射 AI:BALANCED 对应 gpt-5.6-terra
ROUTE=AI:BALANCED
MODEL_DEVIATION=UI 未显示完整模型名，推理档位未能精确记录
COMMIT=2b476d3
VERIFY=bash -n(0×3); --help(0×3); --self-test(0, baseline 7 用例 / verify 9 用例 ALL_PASS); --definitely-invalid(2×3); validate-summary.py --self-test(0); pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS); git diff --check(0)
NEXT=M0-0.b
```

实现要点（对应 §7 提示词）：
- 新增 `scripts/schema/m0-summary.schema.json`（draft-07）与 `scripts/validate-summary.py`
  （python3 标准库共享校验器，支持 `--self-test` 与 `<schema> <summary>` 双模式）；
  两脚本 `--self-test` 各加 schema 校验用例（baseline 6→7、verify 8→9），脚本版本升至
  `M0-1.a-3` / `M0-1.b-2`；既有 BLOCKED run 的 summary.json 经校验 VALID。
- 新增 `scripts/GATE-CONTRACT.md`：固定参数面（`--help/-h`、`--self-test`、`--`、未知选项/多余
  参数）、退出码（0=PASS / 1=FAIL|BLOCKED / 2=用法错误）、schema、SHA256SUMS 与日志格式
  （正式 `[<tag>] ` 前缀、self-test `PASS:/FAIL:` + `SELF_TEST_RESULT=` 尾行）。
- 新增 `scripts/pre-merge.sh`：合并前门禁入口（bash -n ×2、--help ×2、--self-test ×2、
  非法参数 ×2、schema 自检、git diff --check），输出 `PRE_MERGE_RESULT=ALL_PASS`；
  支持 `--help` / `--self-test` / 非法参数 2。`.githooks/pre-merge-commit` 为可选 hook
  模板（cp 到 .git/hooks 或由用户自行 core.hooksPath，本流程不代改 git 配置）。
- 主仓库正式 run 证据：`logs/m0-baseline/20260829T153210+0800_2b476d3_release_x11/`
  （status=BLOCKED，11 项 blocked，driver=PASS，schema VALID，SHA256SUMS 通过）。
- 未实现 M0-0.b（产品 ready/终端钩子与正式基线采集）；未修改冻结契约与产品代码。

### M0-0.b/c 被拒候选与修复记录（2026-08-30）

```text
CHECKPOINT=M0-0.b/M0-0.c candidate
STATUS=REJECTED（不得移动指针）
SOURCE_COMMIT=ac0ecac
REASON=about:blank 被改写为百度搜索；aggregate 六项指标 UNSTABLE
FIX_COMMIT=e8975d6
DECISION_COMMIT=b9077d9
LATEST_GATE_COMMIT=73e9dfb
VERIFY=scripts/pre-merge.sh(0, PRE_MERGE_RESULT=ALL_PASS)
NEXT=M0-0.b RE-RUN
```

被拒原始证据保留在 `logs/m0-baseline/` 与 `logs/m0-baseline/manifests/`，只用于诊断；裁决见 `logs/checkpoints/M0-0.c-20260830-0913.md`。`e8975d6` 后的短样本 smoke 只证明 URL/FD/线程修复方向正确，不能替代契约规定的一批正式 M0-0.b 和三批正式 M0-0.c。

### M0-0.b 修复后正式 PASS（2026-08-30，源提交 `b82eb55`）

```text
CHECKPOINT=M0-0.b
STATUS=PASS
EXECUTOR=Codex 主任务；采样由既有脚本无人值守执行
ROUTE=AI:BALANCED
SOURCE_COMMIT=b82eb55a0fd5d11f572753b9c17e929c47c787d0
VERIFY=scripts/collect-m0-baseline.sh(0, quality PASS, resources PASS, aggregate PASS)
MANIFEST=logs/m0-baseline/manifests/20260830T145117+0800_b82eb55_M0-0.b/
NEXT=M0-0.c
```

完整指标、run ID、二进制哈希和证据路径见 `logs/checkpoints/M0-0.b-20260830-1452.md`。

### M0-0.c 三批采集与 UNSTABLE 裁决（2026-08-30，源提交 `93a1ba6`）

```text
CHECKPOINT=M0-0.c
STATUS=COMPLETE_WITH_ADJUDICATION
RAW_AGGREGATE_STATUS=UNSTABLE（不得改写）
EXECUTOR=Codex 主任务；机械数据归因=gpt-5.6-luna/low
ROUTE=AI:BALANCED
SOURCE_COMMIT=93a1ba63983fba0e5e62ec623c225aa4c13f0bdd
VERIFY=三批 quality PASS + 三批 resources PASS；commit/environment/binary comparable
MANIFEST=logs/m0-baseline/manifests/20260830T153538+0800_93a1ba6_M0-0.c/
RISK_OWNER=tab RSS/FD -> M0-5 BLOCKER；terminal frame gap -> M3-1
NEXT=M0-2.a
```

六项指标仍使用 UNSTABLE 区间，不使用伪精确 median；逐项证据、比较规则和 owner 见 `logs/checkpoints/M0-0.c-20260830-1538.md`。

## 6. 跨模型接手审查协议

Trae 或后续执行器领取当前 `NEXT` 前，必须先做审查，不直接继续写代码：

```bash
git status --short --branch
git log --reverse --oneline --decorate 504fcd7..HEAD
```

然后按顺序：

1. 阅读本文顶部当前指针与最新 `logs/checkpoints/` 证据。
2. 从 `504fcd7` 之后按提交顺序逐个运行 `git show --stat --oneline <sha>`，确认每个提交只覆盖一个检查点。
3. 按各自证据复跑验收命令，确认工作树干净、文档状态、提交顺序和 `NEXT` 一致。
4. 若最新检查点 PASS，从文档当前 `NEXT` 续做；若 FAIL/BLOCKED，留在同一检查点修复或先解除阻塞。
5. 下一检查点重新按路由选模；检查执行器是否用轻量模型越权执行了 `AI:DEEP` 任务。

任何模型都不得因为“额度快没了”提前勾选、压缩检查点或把多个 WBS 合进一个提交。

## 7. 可直接交给 Trae 的连续执行提示词

```text
你正在 feature-M0-baseline 分支接手 mvp-browser-os-v3。目标是在额度和环境允许时尽可能完成 M0，但必须严格按检查点顺序逐个执行；不得开始 M1~M5，不得把多个检查点合成一个提交。

先阅读 AI-模型切换与接手清单.md、详细设计与实施计划.md §2.2、logs/m0-baseline-contract-v1.md。每轮只从文档领取唯一 NEXT，按 AI:* 路由选择 Trae 模型，完成实现、全部验收、四处回写和独立提交。提交后确认工作树干净；未命中硬停止条件时，不用等待用户回复，立即领取下一个 NEXT。

第一轮是 M0-2.a，必须使用 TRAE_DEEP 等效强模型：先确认 M0-0.c 的原始 UNSTABLE manifest、裁决 checkpoint 和全部 SHA 已提交且工作树干净；然后绘制 main window、tab 子 WebView、grid 子进程、PTY、resource scanner/后台线程的资源所有权表，标明创建者、唯一 owner、关闭入口、幂等要求和失败传播。补现状失败测试或最小可复跑失败夹具，证明重复退出、半初始化退出、单资源清理失败至少一个路径尚未由统一核心收口。该检查点不实现 ShutdownCoordinator，不迁移调用方，不改冻结基线；结束时新增 checkpoint、跑相关测试与 scripts/pre-merge.sh，并独立提交 docs/test(M0-2.a)。

同一检查点连续两次失败、缺少 DEEP 等效模型、缺少 GUI/权限/环境、需要改冻结契约或人工裁决、发现不明改动、额度耗尽或到达 M0-7.c 时必须停止，回写 BLOCKED 和证据；不得伪造 PASS。
```
