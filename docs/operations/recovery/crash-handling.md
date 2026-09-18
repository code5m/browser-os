# Agent F — Crash Handling（崩溃与挂起处置）

> 第一步永远是取证：**`./scripts/collect-diagnostics.sh <symptom>`**。本文只讲分类与决策，不讲"猜着改"。

---

## 1. 故障分类与首选动作

| 症状 | 最可能层 | 首选证据 | 首选动作 |
|---|---|---|---|
| 启动即退出 / `panic` / `SIGSEGV` | Rust / Native | `app.txt` 的 `crash.log` 尾 + `ldd` 原生链接；`system.txt` 依赖版本 | 定位到最近 Native 改动 → `git-workflow.md` 场景 3 |
| 宫格打开后某格白屏/崩 | WebView 子进程 | `process.txt` 的 `WebKitWebProcess` 计数 | 宫格自 v0.2.0-grid-mp 起有**多进程隔离**，子窗崩溃不应带崩主窗；若主窗也崩 → Native 问题 |
| 界面卡死（有进程、无响应） | 主线程阻塞 / 渲染 | `process.txt` Top CPU/RSS + 僵尸数 | 取证后重启；若可复现 → 定位改动提交 |
| 浏览器区/宫格区**空白**但不崩 | 视图状态机 | `app.txt` + 跑 `node scripts/runtime-phase1-browser-grid.mjs`（须 35/35） | Phase 1 语义回归 → `git-workflow.md` 场景 2（历史上 B9-4 就是此类） |
| 数据读不出来 / 提示损坏 | 数据层 | 数据目录里的 `<file>.json.corrupt` 侧车文件 | `data-backup.md` 场景 4（应用有自动侧车保护，先就地取回） |
| 装了新包后起不来 | 发布层 | `app.txt` 二进制 sha256 / mtime / `dpkg` | `release-recovery.md` §2 恢复流程 |
| 关闭应用后进程残留 | 生命周期 | `process.txt` 应用与子进程列表 | 取证记录 PID；**不要**盲目 `kill -9` 后立刻重启（丢现场）——先记录再清理 |

---

## 2. 取证清单（每次崩溃都要留）

```bash
./scripts/collect-diagnostics.sh crash-$(date +%H%M%S)
./scripts/snapshot.sh crash-$(date +%H%M%S)
```

必须记录到报告里的**最小集合**：

1. `git.txt`：HEAD、`describe`、dirty 文件（**判定"跑的是不是这份源码"**）。
2. `app.txt`：二进制 sha256 + mtime + `dpkg` 版本（**判定"装的是哪一版"**）。
3. `app.txt`：`crash.log` 与最新 session 日志的**脱敏尾部**（panic 栈在这里）。
4. `system.txt`：webkit2gtk / libsoup / gtk 版本（排除环境漂移）。
5. `process.txt`：WebKit 子进程数、僵尸数、Top RSS（泄漏/残留）。

---

## 3. 决策树

```text
崩溃/异常
   │
   ├─ 取证（diagnostics + snapshot）────────────┐
   │                                            │
   ├─ 二进制 hash / mtime 与源码 HEAD 不匹配？──┴─► 是 → release-recovery.md（重装/重建）
   │
   ├─ 原生依赖缺失或版本漂移？ ─────────────────► 是 → 修环境，不是业务 bug
   │
   ├─ 数据目录出现 *.json.corrupt？ ────────────► 是 → data-backup.md（先取回侧车，再考虑还原）
   │
   ├─ 视图状态异常但进程健康？ ─────────────────► 是 → 跑 runtime harness；不过 → git-workflow 场景 2
   │
   └─ Native 改动后新出现的崩溃？ ──────────────► 是 → git-workflow 场景 3（回退 src-tauri/src）
```

---

## 4. 处置红线

- **不得**在取证前重启/重装/回滚（会丢失现场）。
- **不得**凭猜测改业务代码（Browser/Grid、Bookmark、Credential、Terminal、Workspace、Native lifecycle 均在本层禁改范围）。
- **不得**为让崩溃"消失"而降低 checker、加 ignore、删失败测试。
- **不得**在应用运行中拷贝数据目录（落盘用 atomic_write tmp+rename，会拿到 `.tmp` 中间态）——先停应用。
- **不得**复制/导出 keyring 凭据或 cookie（见 `data-backup.md` §1 红线）。

---

## 5. 恢复后必做

```bash
npm run build
npm run check
node scripts/check-view-intent.mjs --self-test
node scripts/check-grid-close-logic.mjs
node scripts/runtime-phase1-browser-grid.mjs      # 须 35/35
./scripts/snapshot.sh post-crash-recovery         # 留证
```

若崩溃可复现且已定位到具体提交：按 `git-workflow.md` §6 定点 `git revert`，并**补一条回归门禁**（新增断言，而不是放宽断言）。
