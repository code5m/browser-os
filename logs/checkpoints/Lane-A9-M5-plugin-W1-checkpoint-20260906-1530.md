# Lane A9 — M5-W1 插件系统 Delta 整包交付 Checkpoint

> 生成：2026-09-06 15:30 CST · Lane A9（M5-W1 · SUPPORT DOCS ONLY · delta note）
> 依据：`PARALLEL_COMMAND_BOARD.md` → §M5-W1 Implementation Dispatch → A9 行（SUPPORT DOCS ONLY；capability boundary 之后准备插件系统，保留 form③ 与 Ed25519 提案）。

```
LANE=A9
STATUS=PASS
BASE=404f514 (HEAD: docs(M5): integrate prework and dispatch core boundary wave; 领先 origin/master 2)
HEAD=logs/checkpoints/Lane-A9-M5-plugin-W1-pack-20260906-1530.patch
FILES=logs/assist/A9-M5-plugin-W1-delta-20260906-1530.md, logs/assist/A9-M5-plugin-system-prework-20260906-1100.md (§10 附录), logs/checkpoints/Lane-A9-M5-plugin-W1-checkpoint-20260906-1530.md
VERIFY=见下 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-plugin-W1-checkpoint-20260906-1530.md
MERGE_NOTES=见下 §Merge Notes
NEXT=A18(M5-10/11 runtime, 18 命令 + Ed25519 plugin_signature.rs) / A19(M5-12 UI)，在 M5-1 core boundary 落地后由 A0 签发；D-1(check-plugin-policy.py 待建) 归 A18 实现期
```

## 交付内容（M5-W1 A9 Must Deliver = Delta note）

| 要求（M5-W1 A9 行） | 落点 |
|---|---|
| 在 capability boundary 之后准备插件系统 | delta §1（对齐 A2 §8 A9 行：类型落 core/domain.rs，命令/UI 落 bin，form③ 复用 M2-4 满足 R-B4/R-B5） |
| 保留 form③ | delta §2.1 + §3（A10 复审未提阻断，G-6 形态冲突已解决） |
| 保留 Ed25519 提案 | delta §2.2（对齐 A1 `M5-10` §4.4，1100 §2.3 SUPERSEDED，§10 附录记录） |
| 除非 A10 找到阻断项 | delta §3（A10 1410 对 form③/Ed25519 **未提阻断**；唯一硬门 D-1 归 A18） |

## Verify（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --porcelain -- src/ src-tauri/ scripts/   # 期望：空（A9 未改产品代码）
grep -n "Ed25519" logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md   # 期望命中
grep -n "M5-10/11/12 插件" logs/assist/A10-M5-security-review-20260906-1410.md       # 期望：✅
```

结果：产品代码工作树干净；Ed25519 / 18 命令 / capability.rs 共用 / plugin-invokes.json 均在 A1 权威卡落地；A10 复审无插件/form③ 阻断。

## Merge Notes

- **冲突**：无。A9 仅写 `logs/assist/A9-M5-plugin-*.md` 与 `logs/checkpoints/Lane-A9-M5-plugin-W1-*.md`；与 A2（core boundary）、A1（M5-20260906 卡）、A10（复审）、A18/A19（未来实现）无文件交集。
- **与 W0 预研关系**：W0 的 `A9-M5-plugin-system-prework-20260906-1100.md` 已被 A0 集成（commit `404f514`）；本 W1 通过**追加 §10 附录 + 新 delta note** 做 reconciliation，不重写 W0 主体，保留其生命周期/zip-slip/`can_invoke` 7 步/SB-1~SB-10/N1~N16 分析有效。
- **关键偏差裁定**：① 签名由"延后"改为"默认 Ed25519"（对齐 A1 `M5-10` §4.4）；② 命令面由 6 条扩至 18 条（A1 `M5-10` 10 + `M5-11` 8）；③ 审计文件由 `plugin-audit.json` 改为 `plugin-invokes.json`（A1 `M5-11` §4.4）；④ 权限主模型由自研 `acl_level` 改为 `capability.rs` 共用（A1 `M5-10` §4.2）。
- **新增阻塞**：SB-11（Ed25519 信任根管理）交 A18/A0。
- **下游硬门**：D-1（M5 策略脚本 `check-plugin-policy.py` 待建 + 挂 `pre-merge.sh`）归 A18 实现期，非 A9 范围。

## FORBID 遵守

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh`、ACL、三份主文档。
- 未移动 `NEXT`；未提交、未 push（board Merge Rule：仅 A0 可推送）。
