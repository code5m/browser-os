# Lane A5 · M5-W3 next-card delta（M5-4 / M5-5 Agent/Skill runtime 下一实现卡）

> LANE=A5　WAVE=M5-W3 Parallel Dispatch　STATUS=PASS_WITH_DOCS（整包交付）
> BASE=`98a3b01`（W3 加 17:10 CST，A0 经 `712a14c` 后推进；拉取 `--ff-only` 已同步）
> ROLE（按 `PARALLEL_COMMAND_BOARD.md` §M5-W3 行 153）：**SUPPORT DOCS ONLY**；`A5 准备 M5-4/5 下一实现卡；执行复用 M2-4 显式；no runtime code`
> 配套：`logs/assist/A5-M5-agent-skill-20260906-0800.md`（W0）+ `A5-M5-agent-skill-W1-delta-20260906-0900.md`（W1）+ `logs/checkpoints/Lane-A5-M5-agent-skill-checkpoint-20260906-0915.md`
> 本卡更新对象：`logs/checkpoints/M5-20260906/M5-4-agent-skill-runtime.md` / `M5-5-agent-skill-commands.md`（A1 原卡）

---

## 0. 本 delta 目的

A1 的 M5-4/5 原卡写于 mainline 更早阶段，其 §0 假设「M5-1 ✅ / M5-2 ✅(capability.rs 落地) / M2-4 ✅」。本次对当前 `98a3b01` 做实码锚点核验，发现**两处依赖事实与原卡不符**，必须把 M5-4/5 的「下一可执行切片」重新钉死，避免 A16 误判已解锁。本 delta **不改任何产品代码**，仅产出修正后的实现卡切片。

---

## 1. 当前 mainline 锚点核验（grep 实证）

| # | 断言 | 实测 | 结论 |
|---|---|---|---|
| G1 | M5-1.a core 边界已落地 | `src-tauri/src/core/{mod.rs,keyring_store.rs}` 存在；`scripts/check-core-boundary.py` 守 `CORE_TAURI_IMPORT`/`CORE_APP_HANDLE`/`CORE_BRIDGE_REF`（无 tauri/AppHandle/crate::bridge/第二执行路径） | ✅ 已合 |
| G2 | M5-1.b  seam 注入（ProgressSink/PathResolver/RootsProvider） | `core/mod.rs:19` 仅**计划**（`切片 1/2：注入 ... 后搬入 B 类模块`）；`grep "pub trait ProgressSink\|PathResolver\|RootsProvider"` = 0 | ⏳ 未落码，A2 进行中 |
| G3 | M5-2  policy gate 已落地 | `src-tauri/src/security_policy.rs`（76 KB）存在 | ✅ 已合 |
| G4 | capability 常量列表 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`/`MCP_CAPABILITY_V1` | `grep -rn` 全 `src-tauri/src` = 0；`CapabilityRef`/`AclLevel` 亦全仓 0 | ❌ **尚未定义**——M5-2 只落了 policy 壳，常量列表待补 |
| G5 | M2-4 执行锚点 | `script_runner.rs:841` `start_run` / `:880` `start_command` | ✅ 稳定（行号较 W1 偏移 +2，锚点仍有效） |
| G6 | ACL 末条 K1 | `default-commands.toml:118` = `list_artifact_images` | ✅ 末条恒为 |
| G7 | W2 常量集中 | `domain.rs` 已含 `IMAGE_*`/`SESSION_*` 常量（行 84–429） | ✅ 集中完成 |

**关键修正**：原卡 §0 把 `capability.rs` 标为「✅ 落地」是**过时**的。实际 capability 类型与常量列表全仓缺失（G4）。`AclLevel`/`SkillExec`/`CapabilityRef`/`AgentDef`/`SkillDef` 也尚未进 `domain.rs`（G4）。这些现在是 M5-4 **自己的写入项**，不是前置依赖。

---

## 2. 依赖闸门（重钉）

M5-4/5 真正解锁条件（相对原卡修正）：

1. **M5-1.a（✅ 已合）**：提供 `core/` 边界，`check-core-boundary.py` 强制 runtime 不得 `use crate::bridge`、不得建第二执行路径。
2. **M5-1.b seam（⏳ A2）**：提供 `ProgressSink`/`PathResolver`/`RootsProvider` trait。在落码前，M5-4.a 运行时**只能**依赖 `script_runner` + `domain.rs`，且**严禁 `use crate::bridge`**（与 W1-delta 一致）。落码后可逐步把路径/进度解析切到 seam。
3. **capability 常量列表（⏳ 待补）**：`SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1` 必须由 M5-2.a（A3）或 M5-4.a 在 `security_policy.rs` 单源定义（与 `MCP_CAPABILITY_V1` 同文件，防漂移）。**M5-4 不得自建独立 capability.rs**。
4. **M5-2 确认闸门范式（✅ 已合 `security_policy.rs` + `bridge.rs` 两段式）**：M5-5 命令闸门直接复用，不新造。

> 结论：M5-4 当前**可启动其类型定义与解析层**（domain.rs 类型 + skill_parser.rs），但**执行层落库前**须等 G4、G2 至少其一到位。为此把 M5-4 拆成 `M5-4.a`（类型+解析，零执行依赖）与 `M5-4.b`（执行+流式，依赖 G2/G4）。

---

## 3. M5-4.a 下一可执行切片（最小可集成）

> 责任 Lane 候选 **A16**（A0 签发时定）。本 delta 仅描述卡，不写码。

### 3.1 写入清单（修正自原卡 §3）

| 文件 | 性质 | 说明 | 依赖 |
|---|---|---|---|
| `src-tauri/src/domain.rs` | **新增类型** | `AclLevel{Safe,Confirm,Dangerous}` / `SkillExec{ScriptRef,CommandRef,Sequence}`（**禁** InlineScript/RawShell）/`CapabilityRef`/`AgentDef`/`SkillDef`/`AgentRunRecord`/`SkillInstallState` | G7 常量已在 |
| `src-tauri/src/skill_parser.rs` | **新增** | YAML/JSON 解析 + 校验；`SkillExec` 命中 InlineScript/RawShell → 拒（K6 机器门） | 仅 domain.rs |
| `src-tauri/src/skill_runtime.rs` | **新增** | 安装/删除/列表（三态闸门仅 Confirm/Dangerous 弹）；`run_skill` **必须**走 `script_runner::start_run`/`start_command`（G5 锚点） | G1 边界；G5 执行；G4 待补后加 capability 校验 |
| `src-tauri/src/agent_runtime.rs` | **新增（仅类型+骨架）** | `AgentDef` 宿主 + 多方言归一类型；`ExternalCli` 声明**走 script_runner**，编译期禁止 `tokio::spawn` stdio（第二路径 P0） | G1；G2 前不得引 bin 状态 |

### 3.2 红线（最高优先级，机器可查）

- **AGSK_1（执行复用）**：`skill_runtime.rs`/`agent_runtime.rs` 内 `grep -nE 'std::process|Command::new|"-c"|tokio::spawn'` 针对直起进程必须为 0；与 `check-core-boundary.py`「creates a second execution path」同源。
- **AGSK_2（命令+ACL）**：任一经 `bridge.rs` 暴露的命令，必须同包含 source check + ACL 项 + `bridge.ts` 类型同步 + `pre-merge.sh` 覆盖（W3 Hard Stop 第 4 条）。
- **AGSK_3（禁 Inline）**：`SkillExec` 仅 `ScriptRef`/`CommandRef`/`Sequence`；`skill_parser` 单测拒 InlineScript/RawShell（K6）。
- **K1**：ACL 末条恒 `list_artifact_images`（`default-commands.toml:118`）；M5-5 插 15 条新命令须在其**之前**。
- **禁 `use crate::bridge`**：由 `check-core-boundary.py` 守（G1）。

### 3.3 门禁

```bash
bash scripts/check-core-boundary.py          # 必须 PASS（无 tauri/AppHandle/crate::bridge/第二路径）
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bash scripts/pre-merge.sh                      # ALL_PASS
```

---

## 4. M5-5.a 下一可执行切片（命令与权限）

> 依赖 M5-4.a（runtime 稳定）+ G4（capability 列表）。

- 15 条命令（`skill_*` 7 + `agent_*` 8）入 `bridge.rs` + `bridge.ts` 镜像（原卡 §4.1）。
- 每个命令必须：source check + ACL 项（插 `list_artifact_images` 之前）+ `bridge.ts` 类型同步 + `pre-merge.sh` 覆盖（W3 Hard Stop 4）。
- 两段式确认闸门复用 `bridge.rs:688-790`（`request_sync`/`confirm_sync` 范式），`Dangerous` 必弹 + keyring 二次认证。
- 审计写 `skill-runs.json`/`agent-runs.json`（独立 500 上限 FIFO），**禁**记输入/响应正文（K3）。
- `CapabilityRef` 校验走 `security_policy.rs` 单源（G4）。

---

## 5. 自测（A5 docs 范围，grep 实证，非 cargo）

见 §1 G1–G7：全部已实跑。额外：
- A5 全量文档 `grep -nE "^\s*(pub )?fn |impl |use crate|std::process" logs/assist/A5-M5-agent-skill-*.md` = 仅描述性引用，无产品代码 ✅。
- 红线锚点 `script_runner.rs:841/880`、`default-commands.toml:118` 当前有效 ✅。

---

## 6. 阻塞与建议（给 A0）

1. **G4 缺口**：`SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1`/`CapabilityRef`/`AclLevel` 全仓缺失。建议 A0 在 M5-W3 显式指派 A3 M5-2.a 补齐 `security_policy.rs` 的 capability 常量列表（与 MCP 共用单文件），否则 M5-4/5 执行层与命令层 capability 校验无单源。
2. **G2 进度**：M5-1.b seam 未落码；M5-4.a 执行层暂以「`src-tauri/src/` + 仅 `script_runner` + `domain.rs` + 禁 `crate::bridge`」为临时形态，待 seam 到位再重构。
3. **A16 责任**：M5-4/5 实现责任候选仍为 A16（原卡顶部），本 delta 仅把卡推进到「可签发」状态；A5 不抢实现。
4. **W3 Hard Stop 遵守**：A5 本波零产品代码，仅 `logs/assist/` 文档；未移动 `NEXT`；未 push。

---

## 7. 输出模板回填

```text
LANE=A5
STATUS=PASS_WITH_DOCS
WAVE=M5-W3 (SUPPORT DOCS ONLY)
BASE=98a3b01
FILES=logs/assist/A5-M5-agent-skill-W3-next-card-20260906-1715.md
VERIFY=git status 干净（仅本 delta 未跟踪）；G1~G7 grep 全 PASS（见 §1）；零产品代码
MERGE_NOTES=docs-only；M5-4/5 下一实现卡（M5-4.a 类型+解析 / M5-4.b 执行+流式 / M5-5.a 命令+权限）；修正原卡依赖误判（capability 列表 G4 缺失、M5-1.b seam G2 未落码）；执行复用 M2-4 显式（AGSK_1 与 check-core-boundary.py 同源）；实施候选 A16
NEXT=A3 补 M5-2.a capability 常量列表 → A2 落 M5-1.b seam → A0 签 M5-4.a 交 A16（或 A5 若 W4 授权）
```
