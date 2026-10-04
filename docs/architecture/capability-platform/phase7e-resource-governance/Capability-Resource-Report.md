# Capability Resource Report

> 由 `scripts/capability-resource-report.mjs` 从 `capability-registry` 真源生成。
> **口径 = DECLARED RESOURCE CLASS（声明式分类），不是实测数值。**

## 1. 测量现状（诚实声明）

```text
measured_memory_per_capability: NOT_AVAILABLE
measured_cpu_per_capability: NOT_AVAILABLE
webview_instance_count: NOT_MEASURED
declared_resource_class: AVAILABLE
```

> 本项目**没有**按能力的资源实测机制。任何"关闭某功能省了 X MB"的说法都不可靠，
> 本报告只给出声明式分类与可预期释放对象（登记为 Debt-7A-1）。

## 2. 能力资源总表

| 能力 | 资源分类(DECLARED) | 可暂停 | 可销毁 | 常驻 | 可组合性 | 关闭后可预期释放 |
|-|-|-|-|-|-|-|
| browser | HEAVY + WEBVIEW + NATIVE | ⬜ | ⬜ | ✅ | TARGET_COMPOSABLE | 释放 native webview —— TARGET，当前不可物理卸载 |
| grid | VERY_HEAVY + MULTI_WEBVIEW + NATIVE | ⬜ | ⬜ | ✅ | TARGET_COMPOSABLE | 释放 N 个 native webview —— TARGET，资源治理最大收益点，当前不可 |
| workspace | LIGHT | ✅ | ⬜ | ⬜ | TARGET_COMPOSABLE | 仅释放轻量状态，收益很小 |
| terminal | PROCESS + PTY | ⬜ | ✅ | ⬜ | TARGET_COMPOSABLE | 终止 PTY 子进程 —— TARGET，当前不可（PTY 不能安全冻结） |
| bookmark | LIGHT | ✅ | ✅ | ⬜ | COMPATIBILITY_WRAPPED | 仅释放少量内存状态 —— 本身极轻，收益有限（今晚唯一真实可编排试点） |
| credential | SECURITY_SENSITIVE | ⬜ | ⬜ | ✅ | NOT_COMPOSABLE_BY_DESIGN | 不可释放（keyring 句柄常驻，安全边界） |
| database | NETWORK + SECRET | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 断开连接、释放连接池 —— TARGET |
| git | MEDIUM + NETWORK | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 释放工作区索引与远程连接 —— TARGET |
| agent | MEDIUM + NETWORK | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 停止会话与网络请求 —— TARGET |
| skill | LIGHT | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 仅释放轻量状态 |
| plugin | HEAVY + NATIVE | ⬜ | ⬜ | ⬜ | NOT_COMPOSABLE_BY_DESIGN | runtime LOCKED，不支持释放 |
| knowledge_graph | MEDIUM | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 释放图索引内存 —— TARGET |
| notes | LIGHT | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 仅释放轻量状态 |
| resource_collection | MEDIUM + BACKGROUND | ✅ | ⬜ | ⬜ | TARGET_COMPOSABLE | 停止后台采集 —— TARGET |
| task | BACKGROUND | ✅ | ✅ | ⬜ | TARGET_COMPOSABLE | 停止后台调度，消除定时唤醒 —— TARGET |
| session | LIGHT | ⬜ | ⬜ | ✅ | NOT_COMPOSABLE_BY_DESIGN | 不可释放（关停链路） |
| script | PROCESS | ⬜ | ✅ | ⬜ | TARGET_COMPOSABLE | 终止脚本进程 —— TARGET |
| workbench | MEDIUM | ✅ | ⬜ | ⬜ | TARGET_COMPOSABLE | 释放汇总视图状态 —— TARGET |

图例：`CURRENTLY_COMPOSABLE`（真正可独立启停）/ `COMPATIBILITY_WRAPPED`（兼容包装）/
`TARGET_COMPOSABLE`（目标态未实现）/ `NOT_COMPOSABLE_BY_DESIGN`（安全或常驻，设计上不参与组合）。

## 3. Composition Profiles（演示口径）

### Minimal (`minimal`)

最小可用集合 —— 只保留文件工作区与书签

```text
workspace
bookmark
```

> 其中仅 bookmark 是 COMPATIBILITY_WRAPPED，workspace 仍为 TARGET_COMPOSABLE

### Developer (`developer`)

开发向集合 —— 工作区 + 浏览器 + 终端 + 版本控制 + 数据库

```text
workspace
browser
terminal
git
database
bookmark
```

> 除 bookmark 外均为 TARGET_COMPOSABLE；今晚不能真正按需装配

### Full (`full`)

全部已登记能力

```text
workspace
browser
grid
terminal
bookmark
credential
database
git
agent
skill
plugin
knowledge_graph
notes
resource_collection
task
session
script
workbench
```

> 演示口径；credential/session/plugin 为 NOT_COMPOSABLE_BY_DESIGN

## 4. 演示免责声明

> 三个 Profile 是"组合配置的设计口径"，不是今晚已实现的按需装配能力。 今晚真正接入 Capability Runtime 的只有 bookmark（COMPATIBILITY_WRAPPED）， 其余能力的启停仍为 TARGET_COMPOSABLE。禁止在演示中声称"可以一键切换 Profile"。
