# Lane A9 — M5-W3 整包交付（插件 seam 对齐 A3 MCP + A5 Agent/Skill）Checkpoint

> 生成：2026-09-06 17:30 CST · Lane A9（M5-W3 · SUPPORT DOCS ONLY · 整包交付）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W3 Parallel Dispatch → A9：Align plugin seam to A3 MCP registry and A5 Agent/Skill boundary; Delta note; keep Ed25519/form③ explicit.

```
LANE=A9
STATUS=PASS
BASE=98a3b01 (HEAD: docs(M5): dispatch W3 parallel implementation lanes; 已 git pull --ff-only 同步 origin/master)
HEAD=logs/checkpoints/Lane-A9-M5-plugin-W3-delta-20260906-1730.patch
FILES=logs/assist/A9-M5-plugin-W3-delta-20260906-1730.md, logs/checkpoints/Lane-A9-M5-plugin-W3-delta-20260906-1730.md
VERIFY=见下 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-plugin-W3-delta-20260906-1730.md
MERGE_NOTES=见下
NEXT=A18 (M5-10/11 runtime) / A19 (M5-12 UI)，在 D46 capability.rs（A3/M5-2/W3）落地后由 A0 签发；卡片对齐合并建议交 A1
```

## 本切片交付（A9 M5-W3 整包，docs-only）

| 要求（M5-W3 A9） | 落点 |
|---|---|
| Align plugin seam to A3 MCP registry | §1/§2：capability 单一真源=`capability.rs`（D46），由 `check-mcp-policy.py::MCP_CAPABILITY_DRIFT` 守；`check-plugin-policy.py` 同族 A3 脚本风格；无第二执行路径/无网络监听与 A3 MCP 原则同向 |
| Align to A5 Agent/Skill boundary | §2.#3/#4：插件 runtime 落 `mvp_core`+复用 `script_runner`（同 A5 `skill_runtime/agent_runtime`+AGSK_1）；18 命令注册机制同 A5 的 15 命令（bridge/main/ACL前置/bridge.ts-types/check_invocation_source/审计脱敏） |
| no plugin runtime code | 仅对齐文档；未改 `src/`/`src-tauri/`/`scripts`/ACL/三主文档 |
| keep Ed25519/form③ explicit | §3：form③ + Ed25519 fail-closed + 信任根(SB-11) 显式保留，未被 W3 任何产物推翻 |

## Verify（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --porcelain -- src/ src-tauri/ scripts/   # 期望：空（W3 仅 A2/A3 可动产品码）
find src-tauri -name capability.rs   # 期望：无（D46/SB-12 未解锁）
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：末条
```

结果：产品代码工作树无 A9 改动；`capability.rs` 仍不存在（D46/SB-12 未解锁）；ACL 末条（list_artifact_images）未变。

## Merge Notes

- **冲突**：无。仅 `logs/assist/A9-M5-plugin-W3-delta-20260906-1730.md` + 同名 checkpoint；与 A2/A3/A5/A16 无文件交集。
- **与 A1 边界**：W3 中 A1 负责 M5-10/11/12 卡片 reconcile；本 Lane 不编辑这些卡（FORBID §5），对齐内容以 assist 文档表达，待 A0/A1 合并。
- **对 A3 协同**：建 `capability.rs` 时纳入 `PLUGIN_CAPABILITY_V1`（与 MCP/SKILL/AGENT 并列），使 `MCP_CAPABILITY_DRIFT` 一次覆盖四类。
- **统一解锁依赖**：插件(SB-12) = D46 = A5/A16 闸#2 = 同一事件（A3 建 capability.rs）。
- **A9 插件 docs 链完整就绪**：W0(1100)→W1 delta(1530)→W1 seam(1630, 已随 712a14c 集成)→W3 delta(1730)。待 D46 解锁后由 A18/A19 进入实现。

## FORBID 遵守

- 仅写 `logs/assist/` 与 `logs/checkpoints/Lane-A9-*.md`；未触产品代码、ACL、三主文档、`scripts/pre-merge.sh`。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。
