# Agent A — Git Recovery Strategy（稳定点与回滚矩阵）

> 所有 tag/commit 均来自**实际仓库**（`git tag` 实测，2026-09-18）。不存在的 tag 明确标注"未创建/未找到"，不做假设。

---

## 1. 当前仓库事实（实测基线）

| 项 | 值 |
|---|---|
| branch | `master` |
| HEAD | `ad587bc` — `test(phase1): add runtime acceptance harness for browser/grid semantics` |
| `git describe` | `semantic-phase1-browser-grid-code-pass-1-gad587bc`（HEAD 距最近 tag 1 个提交） |
| 领先 origin/master | 7 个提交（**本地未 push**） |
| tag 总数 | 39 |

---

## 2. 稳定点（Stable Points）语义表

| Tag | 指向提交 | 日期 | 意义 | 可否作为回滚点 |
|---|---|---|---|---|
| `semantic-phase1-browser-grid-code-pass` | `a30fd57` | 2026-09-18 | **Phase 1 代码 + 静态门禁通过**；运行时 GUI 验收仍 pending。含 Owner/Intent API、Visibility Controller、C1–C11 门禁 | ✅ **当前最佳回滚点**（代码语义收敛后） |
| `semantic-phase0-policy-pass` | `0a60ec4` | 2026-09-17 | Phase 0 仓库策略/语义治理通过 | ✅ 次选（回退到 Phase 1 之前） |
| `semantic-phase0-infra-pass` | `399ae89` | 2026-09-17 | Phase 0 checker 基础设施通过（带已知产品债） | ✅ 更保守（连策略都未含） |
| `baseline-m1-0-approved` | `9cdf1e5`（提交 `04ad3f5`） | 2026-09-07 | **用户实测认可基线**（M1-0 里程碑，稳定好用）—— 后续 M2~M5 功能多但引入 bug | ✅ 最后手段（功能最少、最稳） |
| `phase-04-m6-closed` | `4b088c2` | 2026-09-13 | Phase 04 M6 Reduced Scope 收口（browser 同步防御收敛） | ✅ 领域性回滚点 |
| `milestone-m5-ui-20260909` | `71e7e1e` | 2026-09-09 | M5 UI 里程碑（窗口控件、终端、书签、webview 可见性修复） | ✅ UI 回归时可用 |
| `m5-w18-r3b-accepted` | `4c69c46` | 2026-09-08 | M5-W18 R3B 验收基线 | ⚠️ 研究/验收标记 |
| `master-pre-grid-mp` | `5c4b39c` | 2026-08-24 | 宫格多进程改造前 | ✅ 宫格架构问题时的对照点 |
| `v0.2.0` … `v0.11.0` | — | — | **版本标签（与 deb 版本是两套体系，见 §5）** | ⚠️ 非语义稳定点，谨慎 |
| `diag/codearts-tui-render-20260915` | `3b5190e` | 2026-09-15 | 诊断标记（CodeArts TUI 排查），**非稳定点** | ❌ 仅取证用途 |

### 计划中 / 不存在的 tag（如实记录）

| Tag | 状态 |
|---|---|
| `semantic-phase1-browser-grid-pass` | **尚未创建** —— 需先完成 native GUI 运行时验收（R7/R8 原生 bounds、R10 关闭释放）。创建前不得声称 Phase 1 完整 PASS |
| `semantic-baseline-v1` | **仓库中不存在**（`git tag` 无此 tag）。现有等价物为 `semantic-phase0-*-pass` 与 `baseline-m1-0-approved` |

---

## 3. Tag 纪律（稳定点规则）

1. **语义 tag 一律 annotated**（`git tag -a`），message 写清"通过什么、还 pending 什么"。
2. **tag 不可移动、不可删除**：需要修正就新建 tag（如 `-v2`），禁止 `git tag -f` 覆盖。
3. **打 tag 前必须通过门禁**：`npm run build` + `npm run check` + 相关 checker self-test + 运行时 harness。
4. **风险改动前打 tag**：跨模块重构 / Native（src-tauri）改动 / 数据格式迁移 / 发布。
5. **诊断标记用前缀 `diag/`**，与稳定点区分，禁止当作回滚点。
6. 本仓库**未 push**是常态（本地领先 7 提交）；回滚前先确认没有依赖未提交的工作（见 §4 前置）。

---

## 4. 回滚前置（每次必做）

```bash
# 1) 取证（先别回滚）
./scripts/snapshot.sh before-rollback-<reason>
./scripts/collect-diagnostics.sh rollback-<reason>

# 2) 确认现场：有没有未提交的重要工作
git status --porcelain
git stash list

# 3) 只读定位引入点（不要改代码）
git log --oneline -15
git log --oneline <good-tag>..HEAD -- src/ src-tauri/src/
git bisect start HEAD <good-tag>     # 必要时才用；bisect 会移动 HEAD，结束务必 git bisect reset
```

> **禁止**：未快照就回滚；未确认 stash 就 `git reset --hard`；用 `git clean -fd` 清掉未跟踪的取证产物。

---

## 5. 两套版本体系（重要，避免误判）

- **deb 包版本** = `src-tauri/tauri.conf.json` 的 `version`，**恒为 `0.1.0`**。
- **git 标签** `v0.2.0 … v0.11.0` 是**另一套**发布序号体系。

→ 判断"装的是哪一版"必须看**构建时间 + commit + 二进制 hash**，不能看 deb 版本号（同版本 `0.1.0` 会被重复覆盖安装）。安装态证据从 `diagnostics/*/app.txt` 取（`dpkg`、`size`、`mtime`、`sha256`）。

---

## 6. ROLLBACK_MATRIX

### 场景 1：Checker 回归（某个门禁开始失败）

- **Recovery Point**：失败门禁对应的上一次通过提交；若整批失败则 `semantic-phase1-browser-grid-code-pass`。
- **判定**（关键）：本仓库**存在既有失败门禁**——`check-terminal-policy.py`、`check-terminal-ui-logic.mjs`（Terminal 既有债，非本阶段引入）。**先区分"既有失败"与"新增失败"**：
  ```bash
  git stash push -u            # 暂存当前改动
  npm run check                # 干净树上跑
  git stash pop
  ```
  若干净树也失败 → 是既有债，不得为了让它变绿而改 checker。
- **Command**（定位到具体提交后，优先定点 revert，不要整体回退）：
  ```bash
  git log --oneline -5 -- scripts/check-xxx.mjs
  git revert --no-edit <bad-sha>          # 定点撤销该提交
  npm run check && npm run build
  ```
- **Risk**：revert 可能连带撤销同提交内的业务改动 → 用 `git show <sha> --stat` 先看清范围；禁止改断言让它"变绿"。

### 场景 2：前端回归（UI 空白 / 宫格异常 / 交互错乱）

- **Recovery Point**：`semantic-phase1-browser-grid-code-pass`（`a30fd57`）；更保守用 `semantic-phase0-policy-pass`。
- **Command**（推荐：只读地把旧版文件取出来对比/验证，不移动分支）：
  ```bash
  git worktree add --detach /tmp/rb-check <tag>      # 隔离验证
  cd /tmp/rb-check && npm ci && npm run build && npm run check
  ```
  确认为代码回归后，在工作树定点回退：
  ```bash
  git checkout <tag> -- src/stores/useBrowserStore.ts src/stores/useLayoutStore.ts
  npm run build && npm run check && node scripts/runtime-phase1-browser-grid.mjs   # 须 35/35
  ```
- **Risk**：若异常来自**数据**（localStorage / app data）而非代码，回滚代码不生效 → 转 `data-backup.md`；`git checkout <tag> -- <file>` 会覆盖工作树文件，先快照。

### 场景 3：Rust / Native 崩溃（启动即崩、panic、segfault）

- **Recovery Point**：最后一份 Native 改动前的 tag（`semantic-phase1-browser-grid-code-pass`，或 Native 最后一次改动之前）。
- **Command**：
  ```bash
  ./scripts/collect-diagnostics.sh native-crash      # 先取证：app.txt 有 ldd/webkit，logs 有 panic 栈
  git log --oneline -10 -- src-tauri/src/
  git checkout <last-good> -- src-tauri/src/         # 定点回退 Rust
  cd src-tauri && cargo check && cargo test          # 再构建
  ```
  若只是"当前装的二进制坏了"而非源码问题 → 走 `release-recovery.md` 重装已知好 deb。
- **Risk**：`cargo` 全量构建耗时长；**数据 schema 可能与旧代码不兼容** → 回退前先按 `data-backup.md` 备份数据目录。

### 场景 4：坏迁移（数据/格式被改坏）

- **Recovery Point**：**数据备份**（不是 git tag——代码回退救不了已落盘的数据）。
- **Command**：
  ```bash
  pkill -f mvp-browser-os || true        # 先停应用（落盘用 atomic_write，运行中拷贝会拿到 tmp）
  # 还原备份中的业务文件（见 data-backup.md 清单）
  ./scripts/collect-diagnostics.sh post-restore
  ```
  应用自身有**自动侧车保护**：损坏文件会被 rename 为 `<file>.json.corrupt`（保留原内容），先去看有没有 `.corrupt` 可就地取回。
- **Risk**：把**新数据**还原到**旧代码**上可能再次损坏 → 代码版本必须与备份时代匹配；还原顺序：先代码后数据，或两者一起回退。

### 场景 5：发布失败（构建/安装/启动/版本错配）

- **Recovery Point**：已知好的 deb 产物（或 `baseline-m1-0-approved` 时代构建）；本仓库当前**无 `releases/` 目录**，需按 `release-recovery.md` 建立。
- **Command**：
  ```bash
  ./scripts/collect-diagnostics.sh release-fail      # 取 binary hash / desktop entry / dpkg 状态
  npm run release:install                            # 重新构建 + 停旧实例 + --reinstall + 校验
  bash scripts/verify-installed-client.sh --install  # 等价入口
  ```
- **Risk**：deb 版本恒为 `0.1.0`，`apt install` 不会"降级"——要回到旧代码必须**从旧 tag 重新构建**；`--reinstall` 会覆盖 `/usr/bin/mvp-browser-os`，覆盖前记录其 sha256（取证已含）。

---

## 7. 回滚后必做的验证（回归闸门）

```bash
npm run build                                   # 1) 编译
npm run check                                   # 2) 全量静态门禁（注意 terminal 既有债）
node scripts/check-view-intent.mjs --self-test  # 3) Phase 1 门禁自检 11/11
node scripts/check-grid-close-logic.mjs         # 4) 12/12
node scripts/runtime-phase1-browser-grid.mjs    # 5) 运行时契约 35/35
./scripts/snapshot.sh after-rollback            # 6) 留证
```
