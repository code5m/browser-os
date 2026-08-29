# AI 模型切换与接手清单

> 文档角色：跨 Codex / Trae 的唯一接手入口；只记录当前执行指针、模型映射、交付证据和回写规则。
> 文档版本：V1.0。
> 更新时间：2026-08-29 08:03 CST。
> 当前状态：`HANDOFF_READY`。
> 当前分支：`feature-M0-baseline`。
> 当前执行器：Trae（额度窗口期间）；Codex 额度恢复后先审查再续做。
> 冲突裁决：WBS/验收以 `详细设计与实施计划.md` 为准，指标语义以冻结契约为准，本文只维护跨模型执行指针和交接证据。

---

## 1. 一眼看懂当前进度

| 项目 | 当前值 |
|------|--------|
| 已完成检查点 | `M0-0.a = PASS` |
| 下一检查点 | `M0-1.a = NEXT` |
| 下一任务路由 | `AI:BALANCED / R:high` |
| 禁止启动 | `M0-1.b` 及其后所有检查点 |
| 最近实现提交 | `7bad65c feat(M0-0.a): freeze baseline measurement contract` |
| 路线文档提交 | `c16ed27 docs(plan): route milestones by priority and model` |
| 工作树要求 | Trae 开工前、交付后都必须干净 |

已经完成：

- 建立 `feature-M0-baseline` 分支。
- 完成 M0~M5 优先级、模型路由、原子检查点和验收门禁整理。
- 使用 `gpt-5.6-terra / medium` 独立审阅并完成 M0-0.a。
- 冻结 `logs/m0-baseline-contract-v1.md`：21 个 `REQUIRED_NOW` 指标、2 个延迟指标、环境指纹、固定场景、统计公式和证据目录。
- 将旧 `logs/baseline-2026-08-27.md` 降级为 `EXPLORATORY`，禁止当作正式性能基线。
- 修复原计划中的循环依赖，关键路径现为：
  `M0-0.a(PASS) -> M0-1.a(NEXT) -> M0-1.b -> M0-1.c -> M0-0.b -> M0-0.c -> M0-2...`。

## 2. 模型映射

WBS 只使用供应商无关的路由标签。切换平台时只改本表，不批量改 50 个 WBS。

| 路由 | 任务类型 | Codex 当前映射 | Trae 映射 |
|------|----------|----------------|-----------|
| `AI:FAST` | 文档、格式化、机械修改、明确的小 UI | `gpt-5.6-luna` | `TRAE_FAST`：待补精确模型名 |
| `AI:BALANCED` | 边界清晰的多文件功能、测试工具、普通重构 | `gpt-5.6-terra` | `TRAE_BALANCED`：待补精确模型名 |
| `AI:DEEP` | 架构、安全、生命周期、并发、IPC、疑难故障 | `gpt-5.6-sol` | `TRAE_DEEP`：待补精确模型名 |

用户提供的微信临时图片在读取时已不存在，因此本文件不猜测 Trae 模型名称。重新附图或直接写出模型列表后，只填写上表三个 Trae 单元格并升级本文版本；任务上的 `AI:*` 标签不变。

Trae 选模规则：

1. `M0-1.a` 选 Trae 中具备仓库读写、Shell 执行和多文件理解能力的日常编码模型，映射到 `TRAE_BALANCED`。
2. 如果免费模型无法运行 Shell 或稳定处理多文件，只允许调研，不得勾选检查点或提交 PASS。
3. M0-2、M0-3、安全、进程树和并发任务必须使用 `TRAE_DEEP`；没有强模型时暂停，不能用轻量模型硬做。
4. 每个检查点在证据中记录 UI 显示的完整模型名、平台、推理档位和 `MODEL_DEVIATION`。全局映射缺失不妨碍记录实际执行模型。

## 3. Trae 当前唯一任务：M0-1.a

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
- 完成验证后，按 §4 回写状态文档和检查点证据。

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

只提交 M0-1.a：

```text
feat(M0-1.a): add baseline quality gate
```

检查点未通过时不得勾选。修复提交使用 `fix(M0-1.a-fix1): ...`，不得伪装成下一检查点。

## 4. 每个模型做完后必须回写什么

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

## 5. Codex 五小时后接手协议

Codex 恢复额度后先做审查，不直接继续写代码：

```bash
git status --short --branch
git log -5 --oneline --decorate
git show --stat --oneline HEAD
```

然后按顺序：

1. 阅读本文顶部当前指针与最新 `logs/checkpoints/` 证据。
2. 审查 Trae 提交是否只覆盖一个检查点，是否夹带 M0-1.b 或产品代码。
3. 复跑其验收命令，确认工作树干净、文档状态与证据一致。
4. M0-1.a PASS 才领取 M0-1.b；否则继续 `M0-1.a-fixN`。
5. 下一检查点重新按路由选模。M0-1.b/c 仍为 BALANCED；M0-2 开始切 DEEP。

任何模型都不得因为“额度快没了”提前勾选、压缩检查点或把多个 WBS 合进一个提交。

## 6. 可直接交给 Trae 的提示词

```text
你正在 feature-M0-baseline 分支接手 mvp-browser-os-v3。只执行 M0-1.a，不得开始 M0-1.b。

先阅读 AI-模型切换与接手清单.md、详细设计与实施计划.md §2.2、logs/m0-baseline-contract-v1.md。实现 scripts/baseline-check.sh：仓库根目录必须动态解析；支持 --help、--self-test、正式模式和非法参数非零退出；覆盖质量、构建、来源/环境指纹及证据目录初始化。不要修改 Rust/Vue 产品代码，不要采集或宣称正式性能基线。

执行文档规定的全部验收命令。全部通过后，回写详细计划、TODO、交接清单，并新增 logs/checkpoints/M0-1.a-<时间>.md，记录实际 Trae 模型名和验证结果。提交信息必须是：feat(M0-1.a): add baseline quality gate。若失败，保持 M0-1.a 未完成并记录阻塞。
```
