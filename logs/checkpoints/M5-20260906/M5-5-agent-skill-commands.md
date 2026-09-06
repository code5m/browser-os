# M5-5 Agent/Skill 命令与权限（install/remove/run/chat）

> 子卡 ID：**M5-5** · 需求 #12 · `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A16**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L567（`M5-5 Agent/Skill 命令与权限`）
> 主预研：`logs/assist/M5-12.a-prework-20260902-1055.md` §4.1-4.2
> 配套：`M5-4-agent-skill-runtime.md`（runtime 后端）· `M5-2-rmcp-mcp-policy.md`（capability.rs 共用）
>
> **W3** BLOCKED（待 A16 = A5 W4 实施期承接）· **W4** ACTIVE（**A5 command policy shell · 不注册实际命令**；详见本卡顶部 `[W4 next-card acceptance criteria]` 段）

---

## [W4 next-card acceptance criteria · 2026-09-06 17:55 CST] A5 M5-5 W4 实施期 acceptance criteria（command policy shell · 不注册实际命令）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L154（**A5 M5-W4** *"...no second execution path, installer, network listener, model provider or download path."*）+ L153-160 + L134-168 硬约束 + A5 W3 next-card（`logs/assist/A5-M5-agent-skill-W3-next-card-20260906-1715.md`）。
> **本卡 W4 与 M5-4 W4 的关系**：A5 W4 dispatch L154 派发 **双卡**——M5-4 domain + 校验 + preview + policy script；M5-5 command policy shell + 权限闸门定义 + preview 二次确认弹窗契约。**M5-5 W4 不注册 15 条新 Tauri 命令**（W4 优先不加 command 硬约束）；仅冻结"未来注册命令"所需的 policy shell + permission preview API 契约 + ACL 闸门定义。
> **A1 W4 角色**：A1 W4 **不**改 §1~§11 决策史；仅在头部加本 `[W4 next-card acceptance criteria]` 段，**明确 A5 W4 实施期 3 项 AC + 5 项 hard stops**，供 A5 / A10 / A11 / A0 验收。

### W4 A5 M5-5 实施期 acceptance criteria（3 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **command policy shell 冻结** | `scripts/check-agent-skill-command-policy.py` 脚本三模式（self-test / default / `--expect-pending`）PASS；policy 守门项至少 8 条：① 未来 `skill_install` / `skill_remove` / `skill_run` / `skill_cancel` 必须先经 capability 校验（复用 `MCP_CAPABILITY_V1`）② ACL 末条恒为 `list_artifact_images`（新命令必须插其之前）③ `skill_run` 必须先经 permission preview 静态裁定（无 second execution path）④ `agent_chat` 走二次确认 ⑤ 流式 emit 必须经 `A6 锁定的 Tauri event 名 + 镜像类型` ⑥ install/remove 必须经 `RootsProvider::allowed_roots()` 校验 ⑦ concurrency 互斥键 = `agent_id`/`skill_id` 而非 `task_id` ⑧ history FIFO 500 上限 | `scripts/check-agent-skill-command-policy.py` self-test + default + `--expect-pending` 三模式 PASS |
| AC-2 **permission preview API 契约冻结** | `preview_skill_run(skill_def, args) -> RunPreview` / `preview_agent_chat(agent_def, messages) -> ChatPreview` 两个纯函数冻结在 `domain.rs` 或新 `command_preview.rs`；纯函数（不调 runtime、不写文件、不发请求）；返回 `capability` 集合 + `touches_fs` + `returns_url` + `requires_acl_level` + `requires_secondary_confirmation: bool`；供 M5-4 W4 permission preview 段 + UI 二次确认弹窗消费 | `cargo test command_preview::preview_skill_run` + `cargo test command_preview::preview_agent_chat` + A11 比对 `src/types.ts` 镜像 |
| AC-3 **不注册 15 条新 Tauri 命令** | `grep -E "skill_install\|skill_remove\|skill_run\|agent_chat" src-tauri/src/bridge.rs` 仅出现 `// not-implemented-yet` 占位 / 注释；`src-tauri/permissions/default-commands.toml` 末条仍恒为 `list_artifact_images`，**未**新增 15 条 skill/agent 命令 | `git diff src-tauri/permissions/default-commands.toml` 无新增命令行 + A10 抽查 `bridge.rs` |

### W4 A5 M5-5 实施期 hard stops（5 项）

| HS | 约束 | 来源 |
|----|------|------|
| W4-HS1 | **不注册 15 条新 Tauri 命令**（W4 优先不加 command）| PARALLEL_COMMAND_BOARD L166 + 本卡 §0 §1 |
| W4-HS2 | **不执行 skill**（"Do not execute skills yet"）—— `skill_run` / `agent_chat` 仅类型/注释/纯函数预览，**不**调 runtime | PARALLEL_COMMAND_BOARD L154 |
| W4-HS3 | **无 second execution path / installer / network listener / model provider / download path / npm 依赖 / GUI panel** | PARALLEL_COMMAND_BOARD L154 + L165 |
| W4-HS4 | **capability 真源单点**（必须复用 `MCP_CAPABILITY_V1` + `is_mcp_capability_allowed`）| A2 W2 + A3 W3 + R-B3 |
| W4-HS5 | **所有 lane 必须从 `origin/master` pull，不 push** | PARALLEL_COMMAND_BOARD L168 |

### W4 验证清单（供 A11 收口）

- `python3 scripts/check-agent-skill-command-policy.py --self-test` PASS
- `python3 scripts/check-agent-skill-command-policy.py` PASS
- `python3 scripts/check-agent-skill-command-policy.py --expect-pending` PASS（如有 PENDING）
- `cargo test --manifest-path src-tauri/Cargo.toml command_preview` PASS
- `git diff src-tauri/permissions/default-commands.toml` 0 新增命令行
- `grep -c "skill_install\|skill_remove\|skill_run\|agent_chat" src-tauri/src/bridge.rs` 仅匹配占位/注释
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- **A10 复审 PASS**（no second execution path / no installer / no network / no model provider / no download / capability 真源单点 / ACL 末条恒为 `list_artifact_images`）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W4-*.md`

### W4 A1 不修订范围

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 实施步骤 / §12 反向边清单 / §13 FORBID 遵守记录**：A1 W4 **不动**（决策史保持 W1 原文；W4 AC 在本顶部段单列）。
- **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W4 不动（policy 脚本由 A5 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。
- **本卡与 M5-4 关系**（A5 实施期双卡并行）：M5-4 domain/校验/preview；M5-5 command policy shell + permission preview API + ACL 闸门定义。两条 AC 互补、不重叠。

---

## 0. 编号与锚定

- 批次任务号 `M5-5`；需求号 #12；WBS L567 一致。
- 依赖：M5-4 ✅（runtime 后端稳定）+ M5-2 ✅（capability.rs + 确认闸门）

---

## 1. GOAL

把 `AgentDef` / `SkillDef` 操作封装为 **Tauri 命令**（前端唯一入口），含：安装、删除、列表、详情、运行（Skill）/聊天（Agent）、取消运行、查看历史；统一走 `capability.rs` 公共白名单 + 两段式确认闸门（与 `request_sync`/`confirm_sync` 范式同款）；所有命令必进 ACL，**末条仍为 `list_artifact_images`**。

---

## 2. READ

1. `logs/assist/M5-12.a-prework-20260902-1055.md` §4.1-4.2（**全读**，命令/权限/审计）
2. `M5-4-agent-skill-runtime.md` §4.5（ACL 三态）
3. `src-tauri/src/bridge.rs:688-790`（两段式确认闸门）
4. `src-tauri/src/workspace.rs`（500 上限 FIFO 范式）
5. `src-tauri/permissions/default-commands.toml`（基线 58 条；插入位置 = `list_artifact_images` 之前）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/bridge.rs` | 修改 | 新增 `skill_install` / `skill_remove` / `skill_list` / `skill_get` / `skill_run` / `skill_cancel` / `skill_runs` 共 7 条 |
| `src-tauri/src/bridge.rs` | 修改 | 新增 `agent_install` / `agent_remove` / `agent_list` / `agent_get` / `agent_chat` / `agent_chat_cancel` / `agent_chat_history` / `agent_runs` 共 8 条 |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 15 条新命令于 `list_artifact_images` 之前；**保持末条恒为 `list_artifact_images`** |
| `src-tauri/src/skill_runtime.rs` | 修改 | `install_skill` / `remove_skill` 走确认闸门 |
| `src-tauri/src/agent_runtime.rs` | 修改 | `chat` 走"二次确认"（A2A 委派场景） |
| `src-tauri/src/skill-runs.json` `src-tauri/src/agent-runs.json` | 持久化 | 独立文件，500 上限 FIFO |
| `src/bridge.ts` `src/types.ts` | 新增 | TS 镜像 15 条命令 + 类型 |

---

## 4. 关键契约

### 4.1 15 条命令清单（与 §3 一致）

| 命令 | 入参 | 出参 | 闸门 | ACL 风险 |
|---|---|---|---|---|
| `skill_install` | `SkillDef` | `SkillInstallResult` | **Confirm** 或 **Dangerous** 必弹 | Medium |
| `skill_remove` | `{ id, version? }` | `bool` | 删危险 = 必弹 | Low |
| `skill_list` | `{ filter? }` | `SkillSummary[]` | — | Low |
| `skill_get` | `{ id, version }` | `SkillDef` | — | Low |
| `skill_run` | `{ id, inputs }` | `RunId` | **按 SkillDef.acl 决定** | 按 `acl` |
| `skill_cancel` | `{ run_id }` | `bool` | — | Low |
| `skill_runs` | `{ filter? }` | `RunRecord[]` | — | Low |
| `agent_install` | `AgentDef` | `AgentInstallResult` | 必弹（含 `delegate_to/delegated_from` 提示） | Medium |
| `agent_remove` | `{ id }` | `bool` | 必弹 | Low |
| `agent_list` | `{ filter? }` | `AgentSummary[]` | — | Low |
| `agent_get` | `{ id }` | `AgentDef` | — | Low |
| `agent_chat` | `{ id, message, session_id? }` | `SessionId` | A2A 委派场景必弹 | Medium |
| `agent_chat_cancel` | `{ session_id }` | `bool` | — | Low |
| `agent_chat_history` | `{ session_id }` | `ChatHistory` | — | Low |
| `agent_runs` | `{ filter? }` | `RunRecord[]` | — | Low |

### 4.2 确认闸门（与 M5-2 §4.3 第 5 步复用）

- **仅 UI 路径**确认（避免 CLI / MCP 旁路）
- 必显内容：Skill 名称/版本/输入/ACL/目标 capability 列表（**逐项**展示，**禁**折叠成"应用"按钮）
- 取消即拒绝（无"延后"）
- 二次确认（keyring 二次认证）用于 `Dangerous`

### 4.3 审计

- `skill_install` / `skill_remove` / `agent_install` / `agent_remove` 写 `audit.json`（`action=skill_*` / `agent_*`）
- `skill_run` / `agent_chat` 写 `skill-runs.json` / `agent-runs.json`（独立 500 上限）
- **禁**记输入正文/响应正文（K3 + 体量防爆）
- `detail` 含 `id` / `version` / `acl` / `capabilities` / `result`

### 4.4 ACL 与 capability 校验

- 每个命令的 `acl_risk` 是 `default-commands.toml` 的元数据（High/Medium/Low）
- **每次命令执行前**过 `capability.rs`：① capability 集合 ② 风险等级 ③ 与 `Policy` 比对
- 与 MCP/A2A 共用同一份 `capability.rs`（防漂移）

---

## 5. FORBID

- **不**让 15 条命令之外的"暗命令"入前端（`bridge.ts` 是唯一前端调用面）
- **不**让任何命令绕过两段式确认闸门（特别是 `Dangerous`）
- **不**让 ACL 末条不再是 `list_artifact_images`（K1）
- **不**让 `capability.rs` 在多文件定义（与 M5-2 共用同一份）
- **不**让审计正文记录原始输入/响应（防刷爆 + 隐私）
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. ACL 末条
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -5

# B. 15 条新增
grep -nE "skill_(install|remove|list|get|run|cancel|runs)|agent_(install|remove|list|get|chat|chat_cancel|chat_history|runs)" src-tauri/bridge.ts

# C. 反向用例
# N1: 装一个 Dangerous Skill 不弹闸门 → 阻断
# N2: skill_run 装 Safe Skill → 立刻执行；Confirm → 弹；Dangerous → 弹+keyring
# N3: 改 skill.json 触发 manifest_hash 变更 → 下一次 run 走二次确认
# N4: 审计 1000 上限验证
# N5: UI 路径之外（如 CLI）调用 skill_install Dangerous → 拒绝
# N6: agent_chat 携带长 1 MB 文本 → 拒绝（> 64 KB 截断/拒绝）

# D. 审计不被刷爆
python3 - <<'PY'
import json, os
for f in ["skill-runs.json", "agent-runs.json"]:
    p = os.path.expanduser(f"~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/{f}")
    if not os.path.exists(p):
        continue
    d = json.load(open(p))
    print(f"{f}: {len(d)} entries (上限 500)")
PY

# E. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# F. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 15 条命令全部入 `bridge.rs` + `bridge.ts` | 命令 B |
| 2 | ACL 末条仍为 `list_artifact_images` | 命令 A |
| 3 | `Dangerous` Skill 安装必弹闸门 | 单测 N1 |
| 4 | `skill_run` 按 `acl` 闸门执行 | 单测 N2 |
| 5 | Skill 配置变更触发二次确认 | 单测 N3 |
| 6 | CLI 路径调用 `skill_install Dangerous` 拒绝 | 单测 N5 |
| 7 | 审计上限 500 | 命令 D |
| 8 | 长输入截断/拒绝 | 单测 N6 |
| 9 | `cargo test` 全绿 | 命令 E |
| 10 | `pre-merge.sh` ALL_PASS | 命令 F |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| ACL 末条不再是 `list_artifact_images` | 阻断（K1） |
| `Dangerous` 不弹闸门 | 阻断（红线） |
| 审计记录原始输入/响应 | 立即修 + 清泄露条目 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L567 `[ ]` → `[x]`
2. `后续需求TODO.md` §12 状态 `PARTIAL`（留 `M5-6` UI）
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-6`
4. `logs/checkpoints/M5-5.a-2026MMDD-HHMM.md`
5. `M5-14-debt-ledger.md` 增项：审计汇总 UI 是否首期做

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A16 实施填
- **NEXT**：M5-6（Agent/Skill UI），A19

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
