# Checkpoint · Lane A3 · BUG-HUNT Follow-up — B4-1/B4-2 workspace 原子写 + 损坏留证

```text
LANE: A3
STATUS: PASS
SCOPE: src-tauri/src/workspace.rs（单一文件；未触碰 A0/A2/A7 等在改文件）
DELIVERED: workspace 系落盘统一走唯一原子写原语 `session::atomic_write`（tmp+rename），
           不再 `fs::write` 直接覆盖（B4-1）；解析失败不再 `unwrap_or_default()` 静默清空，
           改为「备份 `<file>.corrupt` + 告警」（B4-2），损坏字节可人工取回；
           `log_audit` 序列化失败不再把整份审计抹成空文件。
VERIFY: cargo test workspace → 8 passed（6 条新增 B4 回归 + 2 条既有）
        cargo test（全量）→ 444 passed；0 failed（连续两次）
        rustfmt --edition 2021 --check src-tauri/src/workspace.rs → CLEAN
        python3 scripts/measure-build-metrics.py → total=795517B largest_js=334208B
        cargo_warnings=2（既有，未增加）over_500kb=False
        git diff --check → clean
METRICS: dist.total_bytes=795517；largest_js=334208B；chunk_over_500kb=False；
         cargo_warnings=2（与基线一致，未增加）
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A3-BUG-HUNT-B4-atomic-write-20260907-2206.patch
RISKS: 见文末「诚实风险」
NO_PUSH: confirmed（未 commit、未 push）
```

---

## 1. 动手前确认（dispatch 要求：先确认当前代码未被其它 lane 改写）

- `git status` 显示 `src-tauri/src/workspace.rs` **未被任何 lane 改动**（工作树里 `M` 的 Rust 文件只有
  `src-tauri/src/script_runner.rs` 与 `tauri-browser-tabs/.../linux.rs`，均为 A0 的 B3-1/B9-1 已修项）。
- 因此直接改 `workspace.rs`，无需改为「只交补丁不落盘」的冲突路径。

## 2. 根因（BUG-HUNT B4-1/B4-2，P0 数据丢失）

`workspace.rs` 的 artifacts / repos / bookmarks / audit 四处用 `fs::write` **直接覆盖**目标文件；
加载侧统一 `unwrap_or_default()` 把「解析失败」当成空列表返回。
于是形成无痕蒸发链：**崩溃写坏文件 → 载入静默得到空列表 → 下次保存把空列表覆盖写回 → 用户数据彻底消失**
（audit 是热路径，最致命；`log_audit` 还用 `serde_json::to_string_pretty(&list).unwrap_or_default()`，
序列化一失败就往文件里写**空字符串**，`let _ =` 又把错误吞掉）。

## 3. 改动（全部集中在 `workspace.rs`）

1. **新增共享原语**（复用唯一原子写实现，不新造第二条写路径）：
   - `save_json_list_at<T: Serialize>(path, list, tag)` → `crate::session::atomic_write`（tmp + rename）。
   - `load_json_list_at<T: DeserializeOwned>(path, tag)`：文件不存在/读不到 → 空（正常首次启动）；
     **解析失败 → `eprintln!` 告警 + rename 为 `<file>.corrupt` → 空**（对齐 `tasks.rs` O-A6-7 范式）。
2. `save_artifact`：`fs::write` → `atomic_write`（B4-1）。
3. `load_artifacts`：损坏的成果文件从「静默跳过」改为「备份 .corrupt + 告警」（B4-2）。
4. `save/load_scripts_at`、`save/load_snippets_at`：改走共享原语（原先保存已是原子写，但**读取仍静默清空**）。
5. repos / bookmarks / audit：新增可单测的 `*_at` 变体，落盘原子化、读取损坏留证；
   `load_repos/save_repos`、`load_bookmarks/save_bookmarks`、`load_audit` 改为转发 `_at` 版本（对外签名不变）。
6. `log_audit`：**去掉 `unwrap_or_default()`**——现在调 `save_audit_at`，失败只 `eprintln!` 告警，
   原文件保持不被覆盖（不再把审计抹成空文件）。

## 4. 测试（6 条新增 B4 回归 + 1 条既有测试升级）

`b4_atomic_persistence_tests` 新增：
- `repos_roundtrip_is_atomic_without_tmp_leftover`（断言 `atomic_write` 后无 `.json.tmp` 残留）
- `bookmarks_roundtrip_is_atomic_without_tmp_leftover`
- `audit_roundtrip_is_atomic_without_tmp_leftover`
- `corrupt_file_is_backed_up_not_silently_dropped`：**核心**——先写有效数据再写坏，断言返回空、
  `.corrupt` 备份存在、原路径已 rename 走（不会被空列表覆盖写回）、备份内容保留原始字节。
- `corrupt_audit_is_backed_up_and_survives_next_save`：审计损坏留证，且后续再落盘不破坏证据。
- `missing_file_stays_empty_without_backup`：文件不存在属正常启动，**不应**产生 `.corrupt`。
- 既有 `corrupt_or_missing_snippets_file_is_empty` 升级：额外断言 `.corrupt` 备份已生成。

## 5. 验证过程中的两次诚实说明

1. **首次全量 `cargo test` 出现 1 条失败**（`script_runner::…::c3_start_command_reuses_runner_and_persists_record`）。
   该测试在 `script_runner.rs`（**A0 正在改的 B3-1 文件**），且该文件对我改动的 workspace 函数**零引用**
   （grep `workspace::|load_scripts_at|script_runs_file` 均无命中）。
   随后**连续两次全量重跑均为 444 passed / 0 failed**；且同一时段总测试数在 437→438→444 之间变动，
   说明其它 lane 正在并发改文件。**判定：该次失败为并发编辑/竞态导致的瞬态失败，与本改动无关**；
   但因为我无法在冻结的工作树上做严格对照，这一点如实记录，请 A0 集成后复跑确认。
2. **`cargo fmt --all --check` 当前 FAIL，但不是我造成的**：diff 只出现在
   `src-tauri/src/domain.rs`（A7 的 B8  casing 改动）与 `src-tauri/src/security_policy.rs`（A2 的 B2 改动）。
   我**只**用 `rustfmt --check` 校验了自己的 `workspace.rs`（CLEAN），
   并**没有**执行 `cargo fmt --all` 去动这两个不属于我的文件（会覆盖他人未提交工作）。

## 6. 边界合规

- 未新增命令 / bridge / ACL / 权限 / 依赖；未启动任何 M6 权限切片。
- 只复用既有 `session::atomic_write`，**未新造第二条原子写路径**（符合 tasks.rs F10 契约）。
- 未触碰其它 lane 文件；补丁仅含 `src-tauri/src/workspace.rs`。
- 未 commit、未 push。

## 7. 诚实风险

1. **c3 失败的归因**：见 §5-1——我判定为并发竞态，但严格对照未能做，需 A0 在冻结树上复跑确认。
2. **`.corrupt` 只保留一份**：同名再次损坏会覆盖上一次的 `.corrupt`（未做时间戳轮转）。
   考虑到崩溃场景低频、且目标是「留证可人工取回」，当前实现足够；若需多版本留证属后续增强。
3. **行为变化**：损坏后应用表现为「该项列表为空」并留下 `.corrupt`；若上层 UI 此时自动保存，
   会用空列表覆盖**原路径**（但证据已在 `.corrupt`，不会无痕蒸发）——这正是本修复的取舍点。
4. **`load_artifacts` 的损坏成果**会从列表消失（仅备份留证），UI 目前不会提示「有 N 个成果损坏」；
   是否需要用户可见提示属产品/UI 决策，不在本 lane。
5. **未做崩溃注入测试**：本轮用「写坏文件」模拟损坏，未做真实 kill -9 断电验证；建议 A11/A8 在验收时补。
