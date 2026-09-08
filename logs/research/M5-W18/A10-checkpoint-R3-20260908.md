# A10 lane checkpoint — M5-W18-R3（独立 UX 与参考复核）

- Lane：A10
- 工作树：`/home/ainfinit/.codex/worktrees/m5-w18-a10/mvp-browser-os-v3`
- 分支：`codex/m5-w18-a10`
- 起点：`git rebase origin/master` → 与 `origin/master`（`200f0f1 docs(M5-W18): redirect UX research around Rebased`）同点；原有 6 个 R2B 提交已被上游合入（rebase 输出 `skipped previously applied`，非丢失）
- 时间：2026-09-08

## 1. 本波交付

| 文件 | 说明 |
|---|---|
| `logs/research/M5-W18/A10-R3-reference-and-feasibility-review-20260908.md` | 主报告：referenced 身份/许可边界核验 + 被否决原型量化复核 + 当前产品基线 + Git 全流程可实现性矩阵 + 视口算术 + 所有权冲突 + 复核闸门 G1–G8 + 发现登记 F01–F15 |
| `logs/research/M5-W18/A10-R3-viewport-budget.py` | 可复算的视口预算脚本（纯算术，自包含，零产品影响），用于机械校验 A2/A3/A8 的 px/% 数字 |
| `logs/research/M5-W18/A10-checkpoint-R3-20260908.md` | 本文件 |

## 2. 关键结论（详见主报告 §0）

1. `DetachHead/rebased` = `JetBrains/intellij-community` fork（已确认），**固定修订 `2896562e69ff2cac3c90eb3aae4bcce0d4aa9a99`（2026-09-06）**。
2. 其 `LICENSE.txt` = **JetBrains Open-Source Build Terms v1.3（2026-06-15 生效）**，SPDX `NOASSERTION`，含遥测/个人数据、Feedback 授权、出口管制、捷克法+仲裁等附加义务 ⇒ **JetBrains 作者文件默认只能 `REIMPLEMENT_FROM_BEHAVIOR`，`COPY` 需 A0 书面接受 v1.3**。
3. `NOTICE.txt` 3 行：IntelliJ IDEA / JetBrains s.r.o. 归属义务；本产品为 **MulanPSL-2.0**（`package.json`/`Cargo.toml` 未声明 license 字段，F02b）。
4. rebased 自研增量 = **上游平台源码内的小改**（`platform/vcs-log/impl/**`、`platform/tasks-platform-impl/**`、`platform/platform-resources/**`、`platform/build-scripts/**`、`platform/core-api/**`）⇒ R3 想要的"Git log 双落位"源码复用判 REJECT。
5. 被否决原型（1440 帧）中央内容仅 **892px 宽 / 592px 高（61.9% / 65.8%）**，量化了用户"更小更挤"的裁决。
6. R3 Git 全流程：**git2 0.19 已就位**（rebase/cherry-pick/merge/stash/blame/worktree API 齐备，无 `git` CLI 子进程），但现有只有 3 读 + 7 个路径/分支粒度写操作；hunk 暂存与冲突解决/交互式 rebase 是真正工作量。
7. 体积：上限 767,405B（25.2%），干净 052b18a ≈767,059B ⇒ **余量 ≈346B**；最新采集为脏树 795,517B（29.8%）。⇒ 原型不得被当作单片可实现。
8. 视口：1440 折叠态侧栏上限 **115.2px**、1024 上限 **81.9px**（双 48px 栏在 1024 FAIL）；底部 240px 在折叠态必违规；主行≈34 + omni 74 = 108px > 80px 上限。

## 3. 验证记录（本 lane 自测）

```bash
# 启动门禁
cat .workspace-identity && pwd && git status --short --branch && git log --oneline -12   # 分支/路径/身份一致，工作树干净
git fetch origin && git rebase origin/master                                             # Successfully rebased（6 个 R2B 提交已在上游，skipped）

# 参照核验（网络）
api.github.com/repos/DetachHead/rebased                  # fork=true, parent=JetBrains/intellij-community, license=other/NOASSERTION
api.github.com/repos/DetachHead/rebased/commits/master   # sha 2896562..., 2026-09-06
raw .../LICENSE.txt                                      # JETBRAINS OPEN-SOURCE BUILD TERMS v1.3 (91 行)
raw .../NOTICE.txt                                       # 3 行归属
api .../commits/<sha> ×11                                # 自有提交改动路径（platform/** 为主）
api .../contents/plugins（fork vs 上游）                  # 95 vs 97
raw .../module-set-plugins/generated/**                   # 构建配置差异（裁剪点）

# 产品事实（本工作树源码）
src-tauri/src/main.rs:1225-1226                          # inner_size 1200x800 / min 900x600
src-tauri/permissions/default-commands.toml:75-79        # git_* 3 + 写闸门 2；allow 135 条
src-tauri/src/bridge.rs:2109/2123/2140/2367/2480         # git 命令实现位置
src-tauri/src/domain.rs:234-264                          # GitWriteOp 7 种（路径/分支粒度）
src-tauri/Cargo.toml                                     # git2 = "0.19"，无 git CLI 子进程
scripts/measure-build-metrics.py:39                      # TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2
logs/m0-build-metrics/build-metrics-{4f0e8ab,052b18a}.json # 612,943B / 795,517B
src/** grep palette|quickopen|cmdK                       # 0 命中（无命令注册表/面板）

# 自产脚本
python3 logs/research/M5-W18/A10-R3-viewport-budget.py   # 输出四尺寸侧栏/竖向 chrome 上限（见主报告 §6）
```

未跑：GUI 实点、全量构建、cargo/npm 测试（本波为 docs/research，无产品代码改动，不触发门禁）。

## 4. 范围声明（合规）

- 本波 **零产品代码改动**：未改 `src/**`、`src-tauri/**`、`scripts/**`、`package.json`、`Cargo.toml`、ACL、capability。
- 未触碰他 lane 文件；仅新增 A10 自有研究文件（`logs/research/M5-W18/A10-*`）。
- **未 push**；仅在本 lane 分支提交，交 A0 集成。

## 5. 债务与未决

- D-A10-R3-1：所有像素为 `source-derived`/`estimate`，**未做 GUI 实点**（无 native 通道；按蓝图 §7 不写 PASS）。
- D-A10-R3-2：fork↔上游提交级领先/落后**无法计算**（跨仓 compare 返回 404）；引用一律按 SHA。
- D-A10-R3-3：`plugins/{agent-workbench,evaluation-plugin,junit6_rt_tests}` 归属未核实（可能只是上游时间差）。
- 待 A0 裁决：F05（干净体积重采与分片预算）、F12（原生子 WebView 在工具窗口重排下的 owner 空缺）。
- 待 A7 补齐：F03/F04（继承边界逐文件化、完整插件裁剪清单）。

## 6. 下一步（A10）

1. 待 A2–A9 的 R3 产物落到 `master` 后，按主报告 §8 的 **G1–G8 闸门**逐条机械复核，出 A10 终审（卡片序列第 2 步），并同步给 A1（第 3 步）与 A11（第 4 步）。
2. 终审重点：**G4 孤儿动作**、**G5 无来源百分比**、**G6 视口算术**、**G7 状态双重归属**。
3. 仍不 push；终审同样以 lane 文件 + checkpoint 形式交付。

---

## 7. 第 2 波（2026-09-08 续，提交前状态）

触发：`origin/master` 仍为 `200f0f1`，**A2–A9 的 R3 产物尚未落地**（`git ls-tree origin/master -- logs/research/M5-W18` 无 R3 文件），终审继续等待；本波做不依赖他 lane 的独立核验。

新增产物：

| 文件 | 说明 |
|---|---|
| `logs/research/M5-W18/A10-R3-reference-addendum-20260908.md` | 次级参照许可核验 + 能力交叉验证 + G8 门禁说明 |
| `logs/research/M5-W18/A10-R3-prototype-selfcontainment-check.py` | G8 机械门禁脚本（6 条规则，exit 0/1） |

结论补充：

- **SourceGit = MIT 已核实**（raw LICENSE 抓取成功）；**slio-git 不可核验**（`api.github.com/repos/sk-wang/slio-git` 404，raw LICENSE/README 全 404；许可仅由官网 `slio-git.skwang.uk` 声明）⇒ 记为 `UNVERIFIED`，不得 COPY（**F16**）。
- **F17**：slio-git（Rust + git2-rs + Iced）在**实现栈**上比 Rebased 更贴近本产品，应作为"可行性参照"；Rebased 仍是"产品语义/IA 参照"。两类角色须在 A7 映射表中分开，避免把"行为参照"误读成"可 COPY"。
- **F18**：slio-git 的 `iced/wgpu/similar/syntect/notify/tokio` 在本产品当前约束下全部 BLOCKED（R3 不改依赖；M4 A2 F-1 不引 tokio；预算余量 ≈346B）⇒ 只能行为重实现。
- G8 门禁基线：既有 2 个研究原型 `A1-wireframe-prototype-R2B.html`、`A5-R2B-wireframe.html` **均 PASS**，说明门槛可达；R3 新原型应以 exit 0 交付。

验证命令：

```bash
git fetch origin && git log --oneline origin/master -3            # 仍 200f0f1，无 R3 产物
python3 logs/research/M5-W18/A10-R3-prototype-selfcontainment-check.py   # RESULT: PASS (2/2 files clean)
curl -s https://raw.githubusercontent.com/sourcegit-scm/sourcegit/master/LICENSE | head -1   # The MIT License (MIT)
curl -s https://api.github.com/repos/sk-wang/slio-git | head -c 120                          # 404 Not Found
```

范围：仅新增 A10 lane 自有文件；零产品代码/依赖/ACL 改动；**未 push**。
