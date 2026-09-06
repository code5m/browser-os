# Lane A9 — M5-W5 整包交付（插件 manifest/lifecycle 对齐 A6 UI 与 A7 graph 能力）Checkpoint

> 生成：2026-09-06 19:05 CST · Lane A9（M5-W5 · SUPPORT DOCS ONLY · 整包交付）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W5 Parallel Dispatch → A9：Align plugin manifest/lifecycle to A6 UI and A7 graph capabilities; no plugin product code.

```
LANE=A9
STATUS=PASS
BASE=0e76a89 (HEAD: docs(M5): dispatch W5 UI and graph lanes; 已 git pull --ff-only 同步 origin/master)
HEAD=logs/checkpoints/Lane-A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.patch
FILES=logs/assist/A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md, logs/checkpoints/Lane-A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md
VERIFY=见下 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md
MERGE_NOTES=见下
NEXT=A18 (M5-10/11 runtime) / A19 (M5-12 UI)，在 §4 能力真源收口 + A6/A7 W5 代码落地后由 A0 签发
```

## 本切片交付（A9 M5-W5 整包，docs-only，设计基准）

| 要求（M5-W5 A9） | 落点 |
|---|---|
| Align plugin manifest/lifecycle to A6 UI | §2：插件权限预览复用 A5 `PermissionPreview`(domain.rs:2001) + A6 `PermissionPreviewModal.vue`（三态一个组件）；`capabilities` 复用 `CapabilityRef`+`reason` 副文本；生命周期写转移复用 `pendingConfirms` 两段式范式（与 M5-6 同构） |
| Align plugin manifest/lifecycle to A7 graph capabilities | §3：插件图贡献 = capability 声明(`graph:*`) + 经 `graph_*` 命令受控写入（不另开路径）；遵守 K7 不存正文 / `source` 溯源 / 容量边界 / `security_policy` 脱敏单一真源；建议 A7 注册表加 `Plugin` 节点类型 |
| no plugin product code | 仅对齐文档；未改 `src/`/`src-tauri/`/`scripts`/ACL/三主文档 |
| 能力单一真源延续 | §4：W4 §4 碎片化仍 open；W5 新增图能力 `GRAPH_CAPABILITY_V1` 须一并收口（domain.rs(a)/capability.rs(b) + 泛化漂移门正则） |
| 策略脚本对齐 | §5：`check-plugin-policy.py` 须增 A6 UI 红线(`PLUGIN_UI_GATE_REQUIRED`/`bridge.ts` 不绕) + A7 graph 红线(`PLUGIN_GRAPH_VIA_COMMAND`/`PLUGIN_GRAPH_NO_BODY`) |
| Ed25519/form③ explicit | §6：form③ + Ed25519 fail-closed + 信任根(SB-11) 保留，满足 W5 Hard Stop 无 secret 持久化 |

## Verify（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --porcelain -- src/ src-tauri/ scripts/ | grep -iE "A9|plugin|graph" || echo "[无 A9 产品码改动]"
grep -n "pub struct PermissionPreview" src-tauri/src/domain.rs   # 期望：2001
grep -n "fn permission_preview" src-tauri/src/agent.rs           # 期望：38
ls src/components/agent/PermissionPreviewModal.vue 2>&1 || echo "[A6 UI 未在本树：以 M5-6 卡规格为基准]"
grep -rn "pub struct GraphNode\|GRAPH_CAPABILITY" src-tauri/src 2>/dev/null || echo "[A7 graph 未在本树：以 M5-7/8 卡规格为基准]"
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：118
```

结果：产品代码工作树无 A9 改动；A5 `PermissionPreview`(domain.rs:2024)/`permission_preview()`(agent.rs:40) 在；A7 graph 类型已落 `domain.rs:2042-2116`（§3.1 已对照），A6 UI 组件仍不在本树；ACL 末条（list_artifact_images:118）未变。

## Merge Notes

- **冲突**：无。仅 `logs/assist/A9-M5-W5-plugin-manifest-lifecycle-20260906-1905.md` + 同名 checkpoint；与 A2/A3/A4/A5/A6/A7/A8/A10/A11 无文件交集。
- **⚠️ 对齐复核项**：A7 graph 类型已落本树（`domain.rs:2042-2116`），delta §3.1 已对照确认容量/K7/`Plugin` 缺席/溯源偏离；A6 UI 组件（`PermissionPreviewModal.vue` 等）仍不在本树。待 A6 代码经 A0 集成后复核：(1) 插件 `permission_preview()->PermissionPreview` 是否被模态渲染；(2) `Plugin` 节点类型是否入 `GraphNodeKind`；(3) `GRAPH_CAPABILITY_V1` 是否随 §4 收口。
- **与 A1 边界**：W5 中 A1 正在 reconcile 卡片（工作树已见 `M5-0-overview.md` 修改态）；本 Lane 不编辑 M5-10/11/12 卡。
- **对 A6/A7 建议**：插件权限预览复用 A5 `PermissionPreview`+A6 `PermissionPreviewModal`（勿另建）；图节点注册表加 `Plugin`；图写入只走 `graph_*` 命令；`GRAPH_CAPABILITY_V1` 随 W4 §4 收口。
- **对 A18/A19 契约**（W4+W5 合订）：插件类型落 domain.rs；capabilities 复用 CapabilityRef+reason；生命周期复用 A5 范式；插件图贡献经 graph_* 命令 + K7/溯源/容量/脱敏；check-plugin-policy.py 同族 A5 并覆盖 W5 UI/graph 红线；Ed25519 fail-closed 保留；PLUGIN/GRAPH_CAPABILITY_V1 待 §4 裁决。
- **A9 插件 docs 链完整就绪**：W0(1100)→W1(1530+1630 已集成)→W3(1730 已拣入+graph 契约)→W4(1820 已拣入)→W5(1905)。待 §4 收口 + A6/A7 落地后由 A18/A19 实现。

## FORBID 遵守

- 仅写 `logs/assist/` 与 `logs/checkpoints/Lane-A9-*.md`；未触产品代码、ACL、三主文档、`scripts/pre-merge.sh`。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。
