# 03 — Framework Core

## 底座当前由什么组成

| 组成 | 代码位置 | 职责 | 不得做什么 |
|---|---|---|---|
| Shell | `src/App.vue`、`src/components/layout/**` | 渲染壳与槽 | 不得 import 能力内部实现 |
| Capability Runtime | `src/capability/runtime.ts` | register/resolve/activate/suspend/enable/disable/**unregister** | 不拥有业务 state |
| Contribution Registry | `src/capability/contribution/registry.ts` | 存 contribution 元数据（id/slot/component） | 不做 Service Locator、不存业务真源 |
| Building Block Contract | `src/capability/platform/contract.ts` | manifest schema + 校验 | 无副作用、零 import 能力/UI |
| Capability Catalog | `src/capability/platform/catalog.ts` | 机器可读清单（只 import manifest 纯数据） | 不得 import 能力 index/state/ui |
| Dependency Resolver + Assembly | `src/capability/platform/assembly.ts` | 依赖解析 + 装配 + 激活顺序 | 不做业务决策 |
| Lifecycle Model | `src/capability/platform/lifecycle.ts` | 8 态状态机与合法迁移 | — |
| Hot-Plug 流程 | `src/capability/platform/hotplug.ts` + `orchestrator.ts` | add/remove 含回滚 | 不得成为 God Object |
| Semantic Governance | Semantic Registry / 各 check-*.mjs | state/owner/writer 唯一真源 | — |

## FRAMEWORK_ONLY 裁决

- **自动化**：**PASS** —— `bootstrapAssembly([])` 成功返回，runtime 零记录、contribution registry 零贡献（PLT2-10/11/11b）。
- **GUI**：**PENDING** —— 未做人工 GUI 验收，不得写成 PASS。

## 底座不知道谁

底座不知道 `browser` / `terminal` / `bookmark` / `workspace` / `git` / `database` / `agent`
的具体实现：它通过 slot 遍历渲染，能力自己决定贡献什么。

**例外（诚实记录）**：
- `MainArea.vue` 仍硬编码 Browser Dock 的页签名（文件/终端/资源/会话）——这是 Shell 在列举具体能力，属于平台缺口（D-10）。
- `MainArea.vue` 仍直接 import 约 10 个尚未积木化的面板（D-1）。
