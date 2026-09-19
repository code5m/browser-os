# 01 · 项目概述与治理背景

## 1. 项目是什么

`mvp-browser-os-v3`（极智简单·浏览器 OS）是一个 Tauri 2 + Vue 3 的桌面浏览器操作系统：
多页签浏览器、宫格（Grid）多进程 webview、终端、收藏夹、凭据管理等，前端状态以 Pinia store 为核心。

## 2. 为什么需要语义治理（Before）

AI 协作修改这类前端代码时，最易产生**隐性语义分裂**：

- **状态第二真源**：同一个开关状态在两处 store 各自声明，改一处另一处不同步。
- **意图多入口**：同一个用户操作出现多个函数名（`closeGrid` / `destroyGrid` / `shutdownGrid`），
  语义混淆导致"关宫格"变成"销毁资源"。
- **凭据泄露**：密码/令牌被不经意写进 Pinia / localStorage / 日志，前端可见即泄露。
- **副作用失控**：组件直接 `buildGrid` / 直写 `termPanes`，绕过生命周期守护。

这些问题**编译通过、运行不崩**，但在迭代中累积成难以复现的 bug，且 AI 续做时无从感知。

## 3. 现在怎么做（After）

引入 **Semantic Governance v1**：

| 支柱 | 作用 |
|-|-|
| **Registry** | 把"谁拥有什么状态、谁可以写、哪个意图是 canonical、哪些副作用面受管控"写成单一真源 YAML |
| **Checker** | 机器读取 Registry，在代码里自动校验重复状态 / 越权写入 / 重复意图 / 凭据泄露 |
| **Gate** | 提交前 `pre-merge.sh` 自动跑 Checker，违规即阻断合并（exit 1）|
| **Recovery** | 快照 + 诊断 + Git 完整性恢复流程，改坏可回退 |

## 4. 收益

- AI 每次修改都被审计"有没有破坏语义契约"，把**隐性的**语义风险变成**显式的、可阻断的**门禁。
- 治理真源是文档（Registry），不是某个人脑；新成员/新 Agent 接手可读 Registry 即刻理解约束。
- 剩余风险（Known Debt）显式登记，不静默消失，管理层可见真实状态。

> 注意：Registry + Checker 是"守护层"，不替代真实代码迁移。Phase 6A/6B 已发生真实代码迁移
> （删双真源、收敛 writer），Checker 只是证明其不回潮。
