# Lane A9 — M5-W1 第二切片（插件 seam + 策略脚本设计）Checkpoint

> 生成：2026-09-06 16:30 CST · Lane A9（M5-W1 · SUPPORT DOCS ONLY · 第二切片）
> 依据：`PARALLEL_COMMAND_BOARD.md` → §M5-W1 Implementation Dispatch → A9 行（SUPPORT DOCS ONLY；本切片为 W1 delta 的可执行化续片）。

```
LANE=A9
STATUS=PASS
BASE=854bc40 (HEAD: feat(M5): add core boundary gate; 领先 origin/master 4)
HEAD=logs/checkpoints/Lane-A9-M5-plugin-seam-20260906-1630.patch
FILES=logs/assist/A9-M5-plugin-impl-seam-20260906-1630.md, logs/checkpoints/Lane-A9-M5-plugin-seam-20260906-1630.md
VERIFY=见下 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-plugin-seam-20260906-1630.md
MERGE_NOTES=见下
NEXT=A18 (M5-10/11 runtime: 18 命令 + Ed25519 plugin_signature.rs + 落 core 纯逻辑 + check-plugin-policy.py 实现) / A19 (M5-12 UI)，在 M5-2 capability.rs (SB-12) 落地后由 A0 签发
```

## 本切片交付（A9 M5-W1 第二切片，docs-only）

| 要求（M5-W1 A9 行） | 落点 |
|---|---|
| 在 capability boundary 之后准备插件系统 | seam §2 给出 core/bin 落点（对齐 A2 §8 A9 行 + `core/mod.rs` 切片 0b/0a/1/2）；core 纯逻辑须过 `check-core-boundary.py` 7 ACTIVE 码位 |
| 保留 form③ 与 Ed25519 | seam §3 `PLUGIN_FORM_ILLEGAL`/`PLUGIN_SECOND_EXEC`/`PLUGIN_SIG_BYPASS` 把 form③ + Ed25519 fail-closed 落成机检（不重复裁定，引用 W1-delta） |
| no plugin product code | 仅设计；未改 `src/`/`src-tauri/`/`scripts/pre-merge.sh`/ACL/三主文档 |

## Verify（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --porcelain -- src/ src-tauri/ scripts/   # 期望：仅 A2 既有 core 边界改动，无 A9 新增产品码
grep -n "pub mod keyring_store" src-tauri/src/core/mod.rs          # 期望：24
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1   # 期望：118
find src-tauri -name capability.rs   # 期望：无（SB-12 依赖未解锁）
```

结果：产品代码工作树无 A9 改动；core 边界（mod.rs:24）+ ACL 末条（default-commands.toml:118）已落地；`capability.rs` 仍不存在（SB-12 未解锁）。

## Merge Notes

- **冲突**：无。A9 仅写 `logs/assist/A9-M5-plugin-*.md` 与 `logs/checkpoints/Lane-A9-M5-plugin-seam-*.md`；与 A2（core 边界产品码）、A3（M5-2 capability.rs）、A18/A19（未来实现）无文件交集。工作树中 A2 的 `logs/checkpoints/M5-20260906/M5-1.b-seam-trait-injection-and-b-extract.md`（未跟踪）非本 lane 产物，未触碰。
- **与 W0/W1 关系**：W0 预研（1100，已集成 404f514/5ca8f9f）+ W1 delta（1530，在 `Lane-A9-M5-plugin-W1-pack-20260906-1530.patch`）+ 本切片（1630）构成 A9 完整插件 docs 链；本切片不重复前两者分析，只补可执行 seam + 机检。
- **新增项**：SB-12（capability.rs 依赖，归 A3/M5-2）；`check-plugin-policy.py` 码位设计（解 A10 D-1，归 A18 实现期）。
- **下游硬门**：SB-12（capability.rs 未落地）+ D-1（策略脚本待建）均归 A18；core 切片 0b/0a 归 A2。

## FORBID 遵守

- 仅写 `logs/assist/` 与 `logs/checkpoints/Lane-A9-*.md`；未触产品代码、ACL、三主文档、`scripts/pre-merge.sh`。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。
