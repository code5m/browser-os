# Lane A9 — M5-10/11/12 插件系统预研 整包交付 Checkpoint

> 生成：2026-09-06 11:00 CST · Lane A9（M5 prework only · docs only）
> 依据：`PARALLEL_COMMAND_BOARD.md` → M5 Dispatch Now · A9 行（M5-10/11/12 plugin system prework，docs only）。

```
LANE=A9
STATUS=PASS
BASE=5ca8f9f (HEAD: docs(M5): dispatch architecture prework lanes; M4 集成基线 a1a2061 已推送 origin/master)
HEAD=logs/checkpoints/Lane-A9-M5-plugin-pack-20260906-1100.patch
FILES=logs/assist/A9-M5-plugin-system-prework-20260906-1100.md, logs/checkpoints/Lane-A9-M5-plugin-checkpoint-20260906-1100.md
VERIFY=见下 §Verify
CHECKPOINT=logs/checkpoints/Lane-A9-M5-plugin-checkpoint-20260906-1100.md
MERGE_NOTES=见下 §Merge Notes
NEXT=A18(M5-10/11 runtime) / A19(M5-12 UI)，在 M4 PASS 且 A13 workspace 下沉评估之后由 A0 签发
```

## 交付内容

| 要求（M5 Dispatch Now · A9） | 落点 |
|---|---|
| M5-10 插件 manifest | §2 `PluginManifest` 契约（form③ 声明式，supersede 旧 form① 进程假设） |
| M5-10 生命周期 | §2.2 扫描加载 / §2.4 状态机（默认 Disabled） |
| M5-11 命令与隔离 | §3.1 命令清单（全进 ACL）/ §3.2 `can_invoke` fail-closed 二次校验 / §3.3 Grant / §3.4 审计 |
| M5-12 管理 UI + 清理 | §4.1 UI（复用 M4 面板模式）/ §4.2 卸载清理（不删共享实体） |
| Plugin task split | §5 子卡 M5-10.a/b、M5-11.a/b、M5-12.a/b + 未来 Lane A18/A19 |
| Security blockers | §6 SB-1~SB-10（交 A0 裁定） |

## Verify（只读，零产品代码）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --porcelain -- src/ src-tauri/ scripts/   # 期望：空（A9 未改产品代码）
git status --porcelain -- logs/assist/A9-M5-plugin-system-prework-20260906-1100.md \
                        logs/checkpoints/Lane-A9-M5-plugin-checkpoint-20260906-1100.md
# 期望：以上两文件为新增（??）
grep -n "withGlobalTauri" src-tauri/tauri.conf.json            # 12: true（SB-1 锚点）
grep -rn "PluginManifest" src-tauri/src/domain.rs || echo "none (M5-10.a 待加)"
tail -1 src-tauri/permissions/default-commands.toml            # list_artifact_images（K1 末条锚点）
```

结果：产品代码工作树干净；新增两份 A9 文档；所有 file:line 锚点与 §0 事实基线一致。

## Merge Notes

- **冲突**：无。A9 仅写 `logs/assist/`、`logs/checkpoints/`，与 A2/A3/A4/A6/A7/A10/A11 未提交产物无文件交集（已核对 `git status`）。
- **依赖**：本预研依赖 M4 已推送的 `script_runner`（`start_run`/`start_command`）、`security_policy`、ACL 末条约定、K1/K3/K5 红线；不依赖任何未落地的产品代码。
- **前置裁定**：§1 继承 `A9-M5-plugin-form-feasibility-20260906-0700.md` 形态裁定——**默认 form③（声明式），form② 仅作 webview UI 壳（条件可行），form①（独立进程）否决**。2026-09-02 的 `plugin-runtime-taskcard`/`plugin-permission-taskcard` 的 form① 进程假设已被本预研 SUPERSEDED，A0 与未来 A18 实现期**不得**按 form① 开工。
- **风险**：SB-1（`withGlobalTauri=true`）需 A13 下沉评估时封死全局引用；SB-2（capabilities 残留 `browser` label）须随插件 capability 一并清理；SB-8（禁新依赖）要求 M5-10/11 仅用现有 crate。

## NEXT

- A0 在 M4 PASS 后签发 M5 领取顺序，先 M5-1 workspace 下沉评估（A13）。
- M5-10/11 归 A18（runtime），M5-12 归 A19（UI）；按 §5 子卡顺序实现。
- 本交付作为 A18/A19 的契约输入，零产品代码改动，交由 A0 拣入（patch 见 `HEAD`）。

## FORBID 遵守

- 未写产品代码；未触 `src/`、`src-tauri/`、`scripts/pre-merge.sh`、三份主文档。
- 未移动 `NEXT`；未提交、未 push。
