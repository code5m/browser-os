# Recovery & Operational Safety Layer

> 目标：在 AI 长期协作开发下，即使未来 Agent、大规模重构、Native 修改导致异常，也能 **快速定位 → 判断影响 → 回滚 → 恢复运行 → 保留证据**。
> 本层**不是功能开发**，不修改任何业务逻辑。

---

## 1. 原则（全部文档与脚本共同遵守）

| 原则 | 含义 |
|---|---|
| `RECOVERABILITY FIRST` | 任何改动前先保证"能回去"：有 tag、有 snapshot、有数据备份 |
| `DO NOT FIX BY GUESSING` | 先证据后动作；禁止凭猜测改业务代码 |
| `SNAPSHOT BEFORE CHANGE` | 改动前 `./scripts/snapshot.sh <label>` |
| `TAG BEFORE RISKY CHANGE` | 跨模块 / Native / 迁移 / 发布前打 annotated tag |
| `DATA BACKUP BEFORE MIGRATION` | 任何数据结构/格式迁移前备份数据目录 |
| `DIAGNOSTICS BEFORE ROLLBACK` | 先 `collect-diagnostics.sh` 取证，再决定回滚到哪 |
| `ROLLBACK IS A FIRST CLASS FEATURE` | 回滚点与回滚命令是交付物的一部分，不是临时技巧 |

## 2. 禁止范围（本层红线）

本层**不得**修改：

- Browser/Grid 业务逻辑
- Bookmark
- Credential
- Terminal 行为
- Workspace 行为
- Native lifecycle

并且**禁止**：

- 为了方便恢复而修改业务代码
- 降低 checker（改宽断言、注释断言、降阈值）
- 增加 ignore（用忽略绕过门禁）
- 删除失败测试 / 失败门禁

## 3. 什么时候用什么

| 你现在的情况 | 用哪个 | 命令 |
|---|---|---|
| **正要改代码 / 重构 / 迁移**（事前） | Snapshot +（风险高时）Tag | `./scripts/snapshot.sh before-xxx` |
| **软件异常：崩溃 / 卡死 / 空白 / Native 报错** | Diagnostics（先取证） | `./scripts/collect-diagnostics.sh crash-xxx` |
| **判断"是不是这次改动引入的"** | Snapshot 对比 + git 定位 | 见 `git-workflow.md` |
| **代码改坏了，要回到已知好状态** | Git 回滚（按稳定点） | 见 `git-workflow.md` ROLLBACK_MATRIX |
| **数据/配置坏了、误删、迁移失败** | 数据备份还原 | 见 `data-backup.md` |
| **构建/安装/启动/桌面图标失败，版本装错** | 发布恢复 | 见 `release-recovery.md` |
| **原生崩溃（panic / segfault / WebView 崩）** | 崩溃处置 | 见 `crash-handling.md` |

**顺序铁律**：异常发生 → **先 Diagnostics** → 判断影响 → **再决定**回滚/还原/重装。不要一上来就回滚（会丢失现场证据）。

## 4. 交付物索引

| 文档 | 负责 | 内容 |
|---|---|---|
| `README.md` | F | 本文件：总入口与选择指南 |
| `git-workflow.md` | A | 稳定点（tag）语义、回滚矩阵、tag 纪律 |
| `snapshot.md` | B | `scripts/snapshot.sh` 用法与内容解读 |
| `diagnostics.md` | C | `scripts/collect-diagnostics.sh` 用法、脱敏规则、定位流程 |
| `data-backup.md` | D | 数据分类、备份/还原、凭据红线 |
| `release-recovery.md` | E | 构建/安装/启动/版本错配恢复与发布产物结构 |
| `crash-handling.md` | F | 崩溃分类、取证顺序、处置清单 |

| 脚本 | 只读 | 产物 |
|---|---|---|
| `scripts/snapshot.sh` | ✅ | `.snapshots/<date>-<label>.txt` |
| `scripts/collect-diagnostics.sh` | ✅ | `diagnostics/<timestamp>/{system,process,app,git}.txt` |

## 5. 安全保证（工具侧）

两个脚本均承诺：

1. **只读**：不写任何被 git 跟踪的文件，不改 git 状态（无 add/commit/checkout/stash/reset/clean）。
2. **不碰运行时**：不 kill / 不重启 / 不 signal 任何进程。
3. **不采敏感**：不读 keyring、不读 cookie、不读用户文件内容；应用数据只采**文件名 + 大小 + 时间**，日志只采**脱敏后的尾部**。
4. 唯一写入目标是自身产物目录（`.snapshots/`、`diagnostics/`），均为本地未跟踪产物。

> 产物目录故意**不加入 .gitignore**：本层禁止"增加 ignore"。它们就是未跟踪的本地取证文件，不参与提交。

## 6. 验证本层自身

```bash
bash -n scripts/snapshot.sh && bash -n scripts/collect-diagnostics.sh   # 语法
./scripts/snapshot.sh selftest && ./scripts/collect-diagnostics.sh selftest
npm run check                    # 门禁不得因本层下降
node scripts/runtime-phase1-browser-grid.mjs   # Phase 1 运行时契约仍须 35/35
```
