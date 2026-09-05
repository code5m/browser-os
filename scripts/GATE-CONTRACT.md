# M0-1 门禁脚本统一契约（M0-1.c）

> 角色：固定 M0-1 两个门禁脚本（`baseline-check.sh`、`verify-resources.sh`）与本地
> pre-merge 流程的参数、退出码、JSON schema、完整性哈希和日志格式，消除两脚本之间的
> 漂移，并把脚本集合接入合并前检查。
> 状态：M0-1.c V2 落地，2026-08-30。
> 上层语义以 `logs/m0-baseline-contract-v1.md`（冻结契约 V1.0）为准；本文件只固定
> 「脚本自身如何被调用、如何输出、如何校验」，不改变任何指标语义。

## 1. 参数契约

两个门禁脚本共享同一参数面（`pre-merge.sh` 亦同）：

| 参数 | 行为 | 退出码 |
|------|------|--------|
| （无参数） | 正式模式：要求干净工作树，生成 `logs/m0-baseline/<run_id>/` 证据 | 0=通过，1=FAIL/BLOCKED |
| `--help` / `-h` | 打印用法到 stdout | 0 |
| `--self-test` | fixture 模式：用固定夹具验证脚本自身，不生成正式证据 | 0=ALL_PASS，1=FAIL |
| `--` | 结束选项解析（无其余参数） | 进入正式模式 |
| 未知选项 / 多余参数 | 打印错误与用法到 stderr | 2 |

仓库根目录一律由 `git rev-parse --show-toplevel` 动态解析，禁止硬编码绝对路径。

共享运行模式：

- `M0_RUN_MODE=formal`（默认）：必须使用冻结样本数，工作树必须干净；缩短样本直接退出 2。
- `M0_RUN_MODE=smoke`：允许缩短样本与脏工作树，但成功状态固定为 `EXPLORATORY`，永不输出 `PASS`。
- `M0_EVIDENCE_ROOT` 控制物理暂存根；`M0_EVIDENCE_LABEL_ROOT` 控制 summary 中的最终逻辑路径。
- `verify-resources.sh` 正式模式必须收到质量门禁产出的 `M0_EXPECTED_BINARY_SHA256` 并与当前 release 二进制完全一致。

## 2. 退出码契约

| 退出码 | 含义 | 适用 |
|--------|------|------|
| `0` | 本检查点覆盖指标全部 PASS | baseline / verify / pre-merge |
| `1` | 任一指标 FAIL；正式模式工作树非干净拒绝；钩子缺失时 BLOCKED；驱动自检 FAIL | baseline / verify / pre-merge |
| `2` | 非法参数 / 用法错误 | 全部 |

语义要点：

- 产品侧 ready/终端测量钩子尚未落地时，`verify-resources.sh` 正式模式必须输出完整
  `BLOCKED` 证据（`summary.json` 的 `status=BLOCKED`、blocked 列表 owner 指向
  M0-0.b）并退出 1；**禁止伪造 PASS**。
- `pre-merge.sh` 任一检查失败整体退出 1；非法参数退出 2。
- `collect-m0-baseline.sh` 是正式采集唯一入口：默认 1 批（M0-0.b），`--batches 3`（M0-0.c），`--smoke` 只暂存探索证据。

## 3. JSON schema 契约

- `summary.json` 是**机器判定源**，`summary.md` 仅人类阅读；两者不一致时整批 FAIL。
- 两脚本的 `summary.json` 必须满足固定 schema：
  `scripts/schema/m0-summary.schema.json`（JSON Schema draft-07 子集）。
- 共享校验器：`scripts/validate-summary.py`（python3 标准库，无外部依赖）。
  - 用法：`python3 scripts/validate-summary.py <schema.json> <summary.json>`
  - 输出 `VALID` 退出 0；输出 `INVALID` + 错误列表退出 1；缺参/坏参退出 2。
  - 自检：`python3 scripts/validate-summary.py --self-test`。
- 两脚本 `--self-test` 均含 schema 校验用例（对 fixture run 的 summary.json 调
  `validate-summary.py`），保证 schema 与脚本输出同步演进。
- 新增顶层字段必须同步进 schema（`additionalProperties: true` 允许扩展，但
  `required` 字段只增不减，增删必须升级 `SCRIPT_VERSION`）。

## 4. 完整性哈希契约

- 每个正式 run 的 `SHA256SUMS` 覆盖该 run 目录内**除自身外**的全部证据文件；
  校验方式：`(cd logs/m0-baseline/<run_id> && sha256sum -c SHA256SUMS)`。
- 两脚本 `--self-test` 均含 SHA256SUMS 校验用例。
- 原始命令输出按契约 §8 保存为 `raw/<metric-id>_r<NN>.stdout.log` / `.stderr.log`，
  不得静默丢失；`summary/environment/scenario/measurements/SHA256SUMS` 必须入库。
- 多批次先写仓库外暂存目录；每批 schema、SHA、commit、环境指纹和二进制哈希均通过后，才一次性归档到 `logs/m0-baseline/`。

## 5. 日志格式契约

- 正式模式：人类可读进度行统一前缀 `[<tag>] `，`tag` = `M0-1.a` / `M0-1.b` /
  `pre-merge`；错误行追加到 stderr，允许 `error:` / `FAIL:` 前缀。
- `--self-test`：首行 `[self-test] fixture mode: ...` 声明非正式；逐用例输出
  `PASS: <描述>` 或 `FAIL: <描述>`；结尾一行固定 `SELF_TEST_RESULT=ALL_PASS` /
  `SELF_TEST_RESULT=FAIL`。
- 机器可读结果只进入 run 目录的 `summary.json` 等 JSON 文件，stdout 的人类可读
  行不做机器解析。

## 6. 本地 pre-merge 流程

入口：`scripts/pre-merge.sh`。检查项：

1. `bash -n` 两个门禁脚本和总控脚本；
2. `--help` 各退出 0；
3. `--self-test` 两脚本全部用例 ALL_PASS（fixture，不生成正式证据、不触碰用户数据）；
4. 非法参数各退出非 0；
5. `validate-summary.py --self-test`；
6. 总控/聚合器自检；
7. 主工程与插件 `cargo fmt --check`、`npm run build`、`cargo check --locked`；
8. 工作树、暂存区及当前分支相对主分支 merge-base 的 `git diff --check`。

接入方式（不修改 git 配置）：

- 手动：合并/提交前运行 `scripts/pre-merge.sh`；输出
  `PRE_MERGE_RESULT=ALL_PASS` 即通过。
- hook：`cp .githooks/pre-merge-commit .git/hooks/ && chmod +x .git/hooks/pre-merge-commit`
  （或由用户自行 `git config core.hooksPath .githooks`，本流程不代改 git 配置）。
- `pre-merge.sh --self-test` 校验所需工具与文件存在。

## 7. 版本跟踪

| 脚本 | SCRIPT_VERSION | 本契约变更 |
|------|----------------|------------|
| `baseline-check.sh` | `M0-1.a-4` | formal/smoke 隔离、外部暂存、V1.1 指纹与 UNSTABLE 判定 |
| `verify-resources.sh` | `M0-1.b-3` | 哈希交接、四阶段循环、精确终端校验、单调时钟与 idle 聚合 |
| `validate-summary.py` | `M0-1.c-2` | 可选字段递归校验与跨字段语义门禁 |
| `collect-m0-baseline.sh` | `M0-0-collector-1` | 单批/三批原子采集与归档 |
| `aggregate-m0-baseline.py` | `M0-0-aggregate-1` | 同 commit/环境三批聚合与波动率裁决 |
| `pre-merge.sh` | `M0-1.c-2` | 真实 fmt/build/check 与分支累计 diff 门禁 |
