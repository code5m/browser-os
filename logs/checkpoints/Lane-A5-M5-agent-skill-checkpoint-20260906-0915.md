# Lane A5 · M5-W1 Checkpoint（Agent/Skill runtime+commands 支持文档整合）

> LANE=A5　WAVE=M5-W1 Implementation Dispatch　STATUS=PASS_WITH_DOCS
> BASE=854bc40（`master` 已合 M5-1.a core boundary gate；领先 `origin/master` 4）
> ROLE（按 `PARALLEL_COMMAND_BOARD.md` §M5-W1）：**A5 = SUPPORT DOCS ONLY**；仅 A2 可动产品代码
> DELIVERABLE=本 checkpoint（整合 W0+W1 文档 + 自测证据 + 实施闸门建议）；**无产品代码、无 patch、未 push**
> 配套文档：`logs/assist/A5-M5-agent-skill-20260906-0800.md`（W0）+ `logs/assist/A5-M5-agent-skill-W1-delta-20260906-0900.md`（W1 增量）

---

## 0. 摘要（给 A0 / A16 / A2 / A3）

A5 在 M5-W1 的职责是**支持文档**，不是实现者。M5-4/M5-5 的实现责任 Lane 候选为 **A16**（见 `logs/checkpoints/M5-20260906/M5-4-agent-skill-runtime.md` / `M5-5-agent-skill-commands.md` 顶部）。本 checkpoint 交付：

1. 整合 A5 的 W0+W1 文档为可集成切片（docs 已由 A0 在 `404f514 docs(M5): integrate prework...` 拣入，master 领先 4）。
2. **自测**：用 grep 实证 A5 红线锚点真实成立（见 §2），证明 A5 文档零产品代码、且 AGSK_1「Skill 复用 M2-4」与 A2 `check-core-boundary.py` 同源。
3. **实施闸门建议**：明确 M5-4/5 产品代码何时可由 A0 签发给 A16（依赖 M5-1 + M5-2）。
4. **阻塞旗标**：指挥板当前 `NEXT` 仍为 M5-W1，且**无 M5-W2 段落**；若 A0 拟让 A5（而非 A16）在 W2 写产品代码，须先补 M5-W2 段落明确授权，A5 不越权预判。

---

## 1. 启动门禁 / 工作树

| 项 | 实测 | 判定 |
|---|---|---|
| `.workspace-identity` | `WORKSPACE_ID=BACKV3_MAIN` / `EXPECTED_BRANCH=master` | ✓ |
| 分支 | `master`，领先 `origin/master` 4 | ✓ |
| `git status --short` | 仅 1 个未跟踪：`logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`（**A2 的产物，非 A5**） | ✓ 干净 |
| `git diff --name-only` | 空（A5 范围外无产品代码改动） | ✓ |
| `git diff --check` | 空（无空白错误） | ✓ |
| 现有 A5 文档 | 两份均存在且已提交（W0 24491 B / W1 增量 12982 B） | ✓ 已集成 |

> **「整理补丁」说明**：A5 在 W0/W1 全程 docs-only，从未产生产品代码补丁；A0 已直接拣入 assist 文档（commit `404f514`）。工作树无 A5 历史 patch 需 rebase。本 checkpoint 亦为 docs，留待 A0 拣入（与 A9/A11 checkpoint 同口径，未跟踪）。

---

## 2. 自测（grep 实证，非 cargo 跑测——docs-only 无代码可编译）

| # | 断言 | 命令 | 结果 |
|---|---|---|---|
| S1 | A2 core 边界门已落地且守「无 tauri/AppHandle/crate::bridge/第二执行路径」 | `grep -nE "second execution path\|AppHandle\|crate::bridge\|tauri" scripts/check-core-boundary.py` | ✅ 命中 `CORE_TAURI_IMPORT`/`CORE_APP_HANDLE`/`CORE_BRIDGE_REF`（行 21/22/25） |
| S2 | M2-4 执行锚点真实存在 | `grep -nE "pub fn start_run\|pub fn start_command" src-tauri/src/script_runner.rs` | ✅ `843` start_run / `882` start_command |
| S3 | ACL 末条恒为 `list_artifact_images`（K1） | `grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml \| tail -3` | ✅ 行 118 为末条 |
| S4 | A5 文档零产品代码 | `grep -nE "^\s*(pub )?fn \|impl \|use crate\|std::process" logs/assist/A5-M5-agent-skill-*.md` | ✅ 仅命中文档内**对禁项的描述性引用**（如「严禁 `use crate::bridge`」「`std::process::Command` 必须为 0」），无真实 Rust 代码 |
| S5 | A1 M5-4/5 展开卡存在 | `ls logs/checkpoints/M5-20260906/M5-4-agent-skill-runtime.md M5-5-agent-skill-commands.md` | ✅ 两份均在 |
| S6 | AGSK_1 与 A2 门同源 | 比对 W1-delta §2.2 与 `check-core-boundary.py` 失败条件 | ✅ 均指向「no second execution path / crate::bridge」 |

> 结论：A5 的 9 条 `AGSK_*` 红线（W0 §6）中，可在 docs 阶段静态验证的（AGSK_1 执行复用、AGSK_2 命令+ACL、AGSK_3 禁 Inline、AGSK_7 安装默认禁用、K1 末条）均已被现状佐证；运行期断言（AGSK_4 凭据脱敏、AGSK_5 运行明细落 skill-runs.json、AGSK_6 容量、AGSK_8 双阶段确认、AGSK_9 pre-merge 接入）待 A16 实施期由 `check-agent-skill-policy.py` + `cargo test` 守门。

---

## 3. A5 红线（最高优先级：Skill 复用 M2-4）

指挥板对 A5 的硬性要求原话：**keep Skill execution reuse of M2-4 explicit**。本 checkpoint 重申并锁定：

- `skill_runtime::run_skill` / `agent_runtime` 调工具 **必须** 经 `script_runner::start_run` / `start_command`（锚点 §2 S2）。
- **禁止** `std::process::Command`、`sh -c`、`tokio::spawn` stdio（第二执行路径，P0）。
- `SkillExec` 仅 `ScriptRef`/`CommandRef`/`Sequence`（A1 卡命名），**禁** `InlineScript`/`RawShell`（K6）。
- 该红线被两层守门：A2 `check-core-boundary.py`（core 内不得建第二执行路径）+ A5 预留 `scripts/check-agent-skill-policy.py`（W0 §11.1，实施期补）。

---

## 4. 实施闸门（A0 何时可签 M5-4.a / M5-5.a）

A5 不实现；实施责任候选 **A16**（A0 签发时定）。解锁需满足：

1. **A2 M5-1 core boundary 落地**（✅ M5-1.a 已合 `854bc40`；🔄 M5-1.b 进行中 `M5-1.b-seam-trait-injection-and-b-extract.md` 未跟踪）—— 提供 runtime 模块落点。
2. **A3 M5-2 `capability.rs` 落地**（⏳ 待 M5-1 后）—— 提供能力白名单单源（`SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`）。
3. A1 M5-4/5 卡已展开（✅ 存在）。
4. A0 签发首张实现 dispatch 并指定 Lane（候选 A16）。

> A5 文档工作**不阻塞**上述 1/2/3；A5 在 W1 仅持续提供本 checkpoint 与后续 review 支持。

---

## 5. 阻塞旗标（给 A0 决策）

- 指挥板 `Current NEXT`（行 7）仍为 **M5-W1 core boundary implementation**；**全板无 M5-W2 段落**。
- 用户提示提及「M5-W1/M5-W2 职责」，但 W2 尚未写入指挥板。A5 严格按板执行：**W1 = SUPPORT DOCS ONLY，不写产品代码**。
- 若 A0 意在 W2 将 M5-4/5 实现交予 **A5**（而非 A16），请于指挥板补 `## M5-W2 Implementation Dispatch` 并显式将 A5 行改为 `START PRODUCT CODE` + 允许范围（`src-tauri/src/{agent_runtime,skill_runtime,skill_parser}.rs`、`bridge.rs`、ACL、`scripts/check-agent-skill-policy.py`）。A5 在获得该授权前**不预判写码**。
- 当前最稳路径：A0 走既有 A1 卡建议（A16 实现），A5 转为 W2 的**安全 review / 策略脚本实现**角色（与 A10 协同），避免 Lane 职责漂移。

---

## 6. 输出模板回填

```text
LANE=A5
STATUS=PASS_WITH_DOCS
WAVE=M5-W1 (SUPPORT DOCS ONLY)
BASE=854bc40
HEAD=docs only (logs/assist/A5-M5-agent-skill-20260906-0800.md + A5-M5-agent-skill-W1-delta-20260906-0900.md + 本 checkpoint)
FILES=logs/checkpoints/Lane-A5-M5-agent-skill-checkpoint-20260906-0915.md
VERIFY=git status 仅 A2 未跟踪件；git diff --name-only 空；grep 自测 S1~S6 全 PASS（见 §2）；无产品代码
CHECKPOINT=logs/checkpoints/Lane-A5-M5-agent-skill-checkpoint-20260906-0915.md
MERGE_NOTES=docs-only；无 patch；A5 红线=Skill 复用 M2-4（AGSK_1，与 A2 check-core-boundary.py 同源）；实施责任候选 A16，依赖 M5-1(A2)+M5-2(A3)；板无 W2，A5 不写产品代码
NEXT=A2 完成 M5-1.b → A3 落 M5-2 capability.rs → A0 签 M5-4.a/M5-5.a（建议交 A16）；或 A0 补 M5-W2 授权 A5 实现
```
