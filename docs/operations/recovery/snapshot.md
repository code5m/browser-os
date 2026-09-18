# Agent B — Snapshot System（只读快照）

脚本：`scripts/snapshot.sh`　产物：`.snapshots/<date>-<label>.txt`

---

## 1. 定位

Snapshot 回答一个问题：**"改动之前，项目到底是什么状态？"**

它不是备份（不复制数据），不是测试（不判定好坏）。它是**可比对的时间戳证据**，让"这次改动引入了什么"可以被证明，而不是被争论。

## 2. 用法

```bash
./scripts/snapshot.sh [label]     # label 默认 manual；不要含 '/'
./scripts/snapshot.sh --list      # 列出最近 20 个快照
./scripts/snapshot.sh --help
```

示例：

```bash
./scripts/snapshot.sh before-native-refactor
./scripts/snapshot.sh before-data-migration
./scripts/snapshot.sh pre-release
```

输出：

```
SNAPSHOT_CREATED
path: /…/.snapshots/2026-09-18-pre-release.txt
timestamp: 2026-09-18T23:14:57+08:00
HEAD: ad587bc64a641a8af6c7e4b4393a5b2f2188770c
branch: master
```

## 3. 快照内容（每一项都是"判断影响"的输入）

| 分区 | 内容 | 为什么需要 |
|---|---|---|
| **GIT** | branch、HEAD(长/短)、subject、`describe`、latest tag、dirty 文件清单、stash 数、tag 总数、`ahead_origin_master`、最近 10 个 tag | 精确定位代码位置；`dirty` 说明"跑的二进制可能不等于源码" |
| **TOOLCHAIN** | node / npm / rustc / cargo 版本 | 排除"工具链漂移"导致的构建差异 |
| **PROJECT VERSION** | package.json 版本、`tauri.conf.json` 的 productName/version/identifier，并注明 **deb 版本 ≠ git v\* tag** | 避免版本误判 |
| **CONFIG CHECKSUMS** | sha256：`package.json`、`tauri.conf.json`、`Cargo.toml`、`capabilities/default.json`、`capabilities/browser-remote.json`、`permissions/default-commands.toml`、`permissions/remote-collect.toml`、`pre-merge.sh`、`check-view-intent.mjs`、`check-grid-close-logic.mjs`、`runtime-phase1-browser-grid.mjs` | 门禁/权限/ACL 是否被悄悄改过，一比就知道 |
| **SOURCE SHAPE** | src 文件数、Rust 模块数/行数、checker 脚本数 | 大规模重构的体量证据 |
| **BUILD ARTIFACTS** | `dist` 体积与文件数、build-metrics 文件 | 构建体积是否越线（25.2% 上限相关） |
| **DISK** | 项目分区 df、`.git` 体积、`src-tauri/target` 体积 | 磁盘/构建缓存导致的异常排查 |

## 4. 什么时候必须快照

- **任何改动之前**（`SNAPSHOT BEFORE CHANGE`）——尤其是跨模块重构、Native 改动、数据迁移、发布。
- 回滚之前（先留现场）。
- 交接 / 换 Agent 之前（让接手方有可比对的基线）。

## 5. 怎么用快照判断影响（比对工作流）

```bash
A=.snapshots/2026-09-18-before-x.txt
B=.snapshots/2026-09-18-after-x.txt
diff -u "$A" "$B" | sed -n '1,120p'
```

重点看：

1. **HEAD / dirty 变化** —— 代码到底动没动、动了哪些文件。
2. **CONFIG CHECKSUMS 变化** —— 若你没打算改门禁/权限，checksum 却变了 → 有人/某个 Agent 改了门禁或 ACL（**红线：不得降低 checker**）。
3. **TOOLCHAIN 变化** —— 构建差异可能来自工具链而非代码。
4. **SOURCE SHAPE 突变** —— 大规模删除/新增，核对是否符合预期范围。

## 6. 只读保证

- 不写任何被 git 跟踪的文件；不执行 `git add/commit/checkout/stash/reset/clean`。
- 不读凭据：不碰 keyring、cookie、用户数据。
- 唯一写入：`.snapshots/`（本地未跟踪产物）。
- label 含 `/` 直接拒绝，防止路径逃逸。

## 7. 保留与清理

- 产物为**未跟踪**文件，不参与提交（本层禁止增加 ignore，因此它们会显示在 `git status` 中，属预期）。
- 建议按 label 语义保留；需要清理时手工 `rm .snapshots/<file>`（**只删本目录**，不要扩大范围）。
