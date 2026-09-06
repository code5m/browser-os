# M5-A11 · W1 边界门验证清单 + pre-merge 挂载建议（assist）

> Lane：**A11** ｜ 2026-09-06 ~08:30 CST ｜ 姊妹件：`logs/checkpoints/M5-A11-W1-verification-delta-20260906-0830.md`
> 用途：给 **A2 / A0** 的「关闭 W1 唯一开放项」操作清单 + 复跑命令块。**本文件为零产品代码的 assist 注记，A11 不代改任何脚本/产品代码。**

---

## A. W1 边界门验证结论速查

| 检查 | 命令 | 结果 |
|---|---|---|
| 脚本 self-test | `python3 scripts/check-core-boundary.py --self-test` | PASS（ACTIVE=7，坏样本=9，含注释/字符串阴性） |
| 默认扫描 | `python3 scripts/check-core-boundary.py` | `all invariants hold`（EXIT 0） |
| pending 码位 | `python3 scripts/check-core-boundary.py --expect-pending` | NONE（阶段一预期） |
| 集成可编译 | `cargo test --manifest-path src-tauri/Cargo.toml` | 329 passed / 0 failed（零回归） |
| 五类泄漏（A10 L1~L5） | 见 delta §2 | 全 PASS |

---

## B. 唯一开放项：pre-merge.sh 挂载边界门（✅ 已关闭，post-commit 快照 ~08:35）

`scripts/pre-merge.sh` 已被 A2 挂载边界门（`git diff` 显示新增 `# 24. M5-1.a core 边界不变量夹具` 与三段 `check-core-boundary.py` 调用，含 `--self-test` / 默认 / `--expect-pending`）。`bash scripts/pre-merge.sh` 现含 `M5-1.a core 边界不变量夹具（CORE_* 码位）…` 步骤且返回 `PRE_MERGE_RESULT=ALL_PASS`。

<details><summary>原建议挂载片段（现已由 A2 落地，保留作参考）</summary>

建议 A2 在 `git diff --check` 之前插入以下片段（位置与现有「M4-8 定时任务 UI 不变量夹具」等平级即可）：

```bash
# M5-W1 core boundary gate（A0 boundary-first；R8：编译器守不住 core⇸tauri，脚本是唯一防线）
echo "[pre-merge] M5-W1 core boundary gate…"
python3 scripts/check-core-boundary.py --self-test || { echo "CORE_BOUNDARY_SELFTEST_FAIL"; exit 1; }
python3 scripts/check-core-boundary.py || { echo "CORE_BOUNDARY_FAIL"; exit 1; }
```

挂载后须复跑确认：

```bash
bash scripts/pre-merge.sh            # 期望含 "[pre-merge] M5-W1 core boundary gate…" 且仍 ALL_PASS
```

</details>

---

## C. A0 签名下一实现 dispatch 前的复跑命令块（一键）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) 边界门三模式
python3 scripts/check-core-boundary.py --self-test && echo L5_SELFTEST_OK
python3 scripts/check-core-boundary.py            && echo L5_DEFAULT_OK
python3 scripts/check-core-boundary.py --expect-pending && echo L5_PENDING_OK
# 2) 五类泄漏（对照 A10 §4）
grep -rE 'use tauri|AppHandle|AppState' src-tauri/src/core 2>/dev/null | grep -vE '//|///' && echo L1_FAIL || echo L1_OK
grep -rEn 'std::process::Command|Command::new|sh -c|bash -c' src-tauri/src/core 2>/dev/null && echo L2_FAIL || echo L2_OK
git diff src-tauri/Cargo.toml | grep -E '^\+\[dependencies\]|^\+[a-z0-9_-]+ = ' && echo L3_FAIL || echo L3_OK
git diff src-tauri/src/bridge.rs src-tauri/src/main.rs src-tauri/permissions/default-commands.toml | grep -E '^\+.*generate_handler!|^\+invoke_handler' && echo L4_FAIL || echo L4_OK
# 3) 行为不变 + 全门
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E 'test result: ok\. [0-9]+ passed' | tail -1
bash scripts/pre-merge.sh            # 挂载边界门后应含边界检查且 ALL_PASS
git diff --check && echo DIFF_CLEAN
```

---

## D. 债务台账延续

- 本 W1 未引入新债务（零产品代码、零新依赖）。
- 既有 M4 债务（D-5/D-6/D-7 等，见 A10 §6）的结构前提 = 本 W1 边界门；**边界门挂载进 pre-merge 前，D-5/D-6 的机器守门仍缺位**，建议 A0 在 M5-2 派发门槛中把「`check-core-boundary.py` 已落且 `--self-test` PASS + 已挂载 pre-merge」列为前置。
