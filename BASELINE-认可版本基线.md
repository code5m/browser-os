# 认可版本基线（Approved Baseline）

> **AI 接手本项目时请优先阅读本文档。**
> 这里记录的是**用户实测认可、稳定可用**的里程碑版本，是后续优化的**基点**。
>
> 确立日期：2026-09-07 ｜ 确立人：lizhaoxin（用户实测确认）

---

## 一、基线锚点

| 项 | 值 |
|---|---|
| **git tag** | `baseline-m1-0-approved` |
| **基线 commit** | `04ad3f5`（完整 `04ad3f57c2410a2338d70e238b8568fff63387e8`） |
| 提交时间 | 2026-09-01 09:15:55 |
| 提交信息 | `feat(M1-0): expose homepage shortcut favorites` |
| 构建时 HEAD | `afaf659`（09:10:37） |
| 已安装二进制 | `/usr/bin/mvp-browser-os`（9,045,568 B，构建于 09:15:04） |
| 已推送远端 | ✅ 在 `origin/master` 上 |

```bash
git checkout baseline-m1-0-approved   # 回到认可版本
git log --oneline baseline-m1-0-approved..master | wc -l   # 距当前 HEAD 的提交数
```

### ⚠️ 关于 51 秒错位（考据说明）

已安装二进制的构建完成时间是 **09:15:04**（`stat` 硬证据），而 `04ad3f5` 提交于 **09:15:55**，晚了 51 秒。

Tauri 构建读的是**工作树快照**而非 HEAD，因此二进制内容 = 含 M1-0 改动的工作树状态，等价于 `04ad3f5` 提交后的代码。

佐证：`04ad3f5` 随附的 `logs/checkpoints/M1-0-20260901-0915.md` 记录 `DATE=2026-09-01 09:15`、`npm run build` **EXIT=0**、`COMMIT=待回填` —— 说明代码先写完并通过构建验证，51 秒后才补提交。

> 确定性：时间锚定为**硬证据**；「二进制内容 = `04ad3f5`」为**强推断**（二进制未内嵌 commit hash）。

---

## 二、为什么认可这个版本

- 用户实测结论：**稳定、好用**（2026-09-07 确认）。
- 后续 M2 ~ M5 增加了大量功能，但**引入了 bug**，稳定性不如本版本。

**这是「功能多但带 bug」与「功能少但可靠」之间的选择 —— 用户选择后者作为基点。**

---

## 三、基线包含什么

| 模块 | 内容 |
|---|---|
| M0-7.c | 验收通过 |
| M1-0 | 主页快捷方式补齐 |

M1-0 具体改动（`git show 04ad3f5`，6 files / +150 / -47）：

- `src/components/home/HomePanel.vue`：新增「☆ 网页」「📁 目录」按钮
- `src/stores/useHomeStore.ts`：新增 `favoriteCurrentPage()` / `favoriteCurrentDir()`，一键把当前网页或目录存为主页快捷方式
- 目录播种 key 升级为 `browser-os-home-dirs-seeded-v2`：从旧版只取前 4 个系统目录，改为补齐 `bridge.getStartDirs()` 返回的全部起始目录

**边界**：这是主页快捷方式补齐，**不是**正式浏览器收藏夹。M1-2 才建 `Bookmark` 领域，M1-3 才做地址栏星标与收藏侧栏。

---

## 四、基线之后新增了什么（即「功能多但有 bug」的部分）

从 `baseline-m1-0-approved` 到 `master` HEAD（`052b18a`）共 **208 个提交**：

| 阶段 | 新增内容 |
|---|---|
| M1-1 | 全套平台图标（含蓝底「简」字品牌图标） |
| M1-2 | `Bookmark` 领域 + `add/list/remove_bookmark` 命令 + ACL |
| M1-3 | 地址栏收藏星标 + 收藏夹侧栏 |
| M2 | 图片预览、脚本库、工具库 |
| M3 | 终端 PTY / resize / 历史 |
| M4 | 定时任务调度、数据库 schema 与迁移 |
| M5 | 命令桥（W8）、图谱（W12）、MCP（W10/W11）、插件清单（W13）、插件管理 UI（W14） |

> 这些功能**未经用户实测认可**，且已知存在 bug。回退/移植时应逐项评估，不要整体采纳。

---

## 五、后续优化策略（待定）

以本基线为基点重新演进，避免在带 bug 的 HEAD 上继续堆叠。候选路径：

| 方案 | 做法 | 适用 |
|---|---|---|
| **A. 基线分支演进**（推荐） | 从 tag 切 `optimize/M1-0` 分支，在其上优化；M2~M5 功能按稳定性逐个 cherry-pick | 想保住「好用」的同时渐进加功能 |
| B. 原地修 bug | 留在 `master` HEAD 修 bug | bug 数量少、定位清晰时 |
| C. 基线只读参考 | 不回退，仅把基线当作「正确行为」的对照基准 | 需要 HEAD 全量功能时 |

**落地前建议先补一份 bug 清单**：把「后续版本哪里不好用」逐条记录下来，才能决定哪些功能该移植、哪些该砍。

---

## 六、相关文档

- 完整排查过程：[Obsidian › mvp-browser-os-安装状态与版本排查](../../../Knowledge-Base/secondBrain/00-知识索引/13-外部开源与技术调研/部署运维/mvp-browser-os-安装状态与版本排查.md)
- M1-0 checkpoint：`logs/checkpoints/M1-0-20260901-0915.md`
