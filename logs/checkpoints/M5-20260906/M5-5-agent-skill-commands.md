# M5-5 Agent/Skill 命令与权限（install/remove/run/chat）

> 子卡 ID：**M5-5** · 需求 #12 · `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A16**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L567（`M5-5 Agent/Skill 命令与权限`）
> 主预研：`logs/assist/M5-12.a-prework-20260902-1055.md` §4.1-4.2
> 配套：`M5-4-agent-skill-runtime.md`（runtime 后端）· `M5-2-rmcp-mcp-policy.md`（capability.rs 共用）

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
