# Lane A9 — M5-W4 整包交付（插件 manifest/lifecycle 对齐 A5 Agent/Skill 实际产物）Checkpoint

> 生成：2026-09-06 18:20 CST · 校正于 18:35（对照 A5 实际产物）· Lane A9（M5-W4 · SUPPORT DOCS ONLY · 整包交付）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W4 Parallel Dispatch → A9：Align plugin manifest/lifecycle plans to A5 Agent/Skill policy; Plugin delta note.

```
LANE=A9
STATUS=PASS
BASE=f7ad35a (HEAD: docs(M5): dispatch W4 agent memory and skill lanes; 已 git pull --ff-only 同步 origin/master)
HEAD=logs/checkpoints/Lane-A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.patch
FILES=logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md, logs/checkpoints/Lane-A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md
VERIFY=见下 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md
MERGE_NOTES=见下
NEXT=A18 (M5-10/11 runtime) / A19 (M5-12 UI)，在 A0 裁决 §4 能力真源收口后由 A0 签发
```

## 本切片交付（A9 M5-W4 整包，docs-only）

| 要求（M5-W4 A9） | 落点 |
|---|---|
| Align plugin manifest/lifecycle to A5 Agent/Skill policy | §2 manifest schema 对齐（PluginManifest ↔ AgentDef/SkillDef 同构；复用 `CapabilityRef`）；§3 生命周期对齐（经 `PermissionPreview` 的 Dangerous 闸门 / 卸载删目录 / 500 审计 / Keyring 信任根，同 A5） |
| no plugin product code | 仅对齐文档；未改 `src/`/`src-tauri/`/`scripts`/ACL/三主文档 |
| ⚠️ 新发现交叉 lane 不一致 | §4：能力单一真源已碎成 `domain.rs`(MCP, L1540) 与 `security_policy.rs`(Skill/Agent, L2067/2070) 两文件，违反 D46；A3/A5 漂移门均按名硬编码，插件 `PLUGIN_CAPABILITY_V1` 落点被阻塞 → 交 A0 裁决（收口 domain.rs 或新建 capability.rs + 泛化漂移门正则） |
| 策略脚本对齐 | §5：`check-plugin-policy.py` 须镜像 A5 实际 `check-agent-skill-policy.py`（探针+AGSK_*/K1/能力漂移） |
| Ed25519/form③ explicit | §6：form③ + Ed25519 fail-closed + 信任根(SB-11) 显式保留，未被 W4 推翻 |

## Verify（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --porcelain -- src/ src-tauri/ scripts/ | grep -E "A9|plugin" || echo "[无 A9 产品码改动]"
grep -rn "CAPABILITY_V1" src-tauri/src/domain.rs src-tauri/src/security_policy.rs
#  期望：domain.rs:1540 MCP_；security_policy.rs:2067 SKILL_；:2070 AGENT_；插件 PLUGIN_ 尚无
grep -n "PluginManifest\|PLUGIN_CAPABILITY_V1" src-tauri/src/domain.rs || echo "[插件未实现，符合范围]"
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：118
```

结果：产品代码工作树无 A9 改动；能力真源碎片化（§4）；插件类型未实现；ACL 末条（list_artifact_images:118）未变。

## Merge Notes

- **冲突**：无。仅 `logs/assist/A9-M5-W4-plugin-manifest-lifecycle-20260906-1820.md` + 同名 checkpoint；与 A2/A3/A4/A5/A6/A7/A8/A10/A11 无文件交集。
- **⚠️ 交叉 lane 不一致（需 A0 裁决）**：能力白名单单一真源碎成 `domain.rs`(MCP) 与 `security_policy.rs`(Skill/Agent)，违反 D46；A3 `MCP_CAPABILITY_DRIFT` 与 A5 `c_capability_drift` 均按名硬编码，插件 `PLUGIN_CAPABILITY_V1` 落点被阻塞。推荐收口 domain.rs(a)/新建 capability.rs(b) + 泛化漂移门正则 `\b[A-Z_]*CAPABILITY_V1\s*[=:]`。
- **与 A1 边界**：W4 中 A1 正在 reconcile 卡片（工作树已见 M5-0/1/1.b/2/3/4 修改态）；本 Lane 不编辑 M5-10/11/12 卡，对齐内容以 assist 文档表达。
- **对 A18 实施契约**：插件类型落 `domain.rs`（与 A5 同文件）；`capabilities` 复用 `CapabilityRef`+`reason` 独立字段；生命周期复用 A5 范式（`PermissionPreview` 闸门 + 卸载删目录 + `plugin-runs.json` 500 审计 + Keyring）；`check-plugin-policy.py` 同族 A5；Ed25519 fail-closed 保留；`PLUGIN_CAPABILITY_V1` 落点待 §4 裁决。
- **A9 插件 docs 链完整就绪**：W0(1100)→W1 delta(1530)→W1 seam(1630, 已集成)→W3 delta(1730, 已拣入)→W4 delta(1820)。待 §4 能力真源裁决后由 A18/A19 进入实现。

## FORBID 遵守

- 仅写 `logs/assist/` 与 `logs/checkpoints/Lane-A9-*.md`；未触产品代码、ACL、三主文档、`scripts/pre-merge.sh`。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。
