# Capability Modularization v1 — Phase 8C Scoping（Workspace / Files Physical Modularization）

> 用途：Phase 8B.1 已验收 Bookmark = C3（tag `capability-phase8b-bookmark-composable-pass`）。
> 本文件是 Phase 8C 的**判断 + 分阶段计划**——按接管指令 §13「先判断…再迁移」「禁止机械复制 Bookmark 结构」。
> 生成时间：2026-09-20（独立 Agent 在 8B.1 验收后切入）。
> 主入口：`docs/architecture/HANDOFF_CURRENT_STATE.md` §4s（已指向本阶段）。

---

## 1. 现状事实（判定依据）

```text
src/stores/useWorkspaceStore.ts   ← 35KB 单体 store，同时持有：
                                    files（目录树/当前目录/startDirs/recents）
                                    vault（笔记 Vault）
                                    repo（Git 仓库状态）
                                    git（diff/history/write-confirm）
                                    editor（fileEditorOpen 覆盖层，Markdown 预览）
                                    以及 enterDir/openDirTab/fileEditor 等动作

src/stores/useLayoutStore.ts      ← mainView 枚举含 files/vault/editor/repo/arts；
                                    顶部导航 TOP_NAV_ITEMS / NAV_MENU_SECTIONS 直接引用这些 view；
                                    fileEditorOpen(ref) 控制编辑器覆盖层

src/components/workspace/         ← 所有面板直接由 MainArea 的 v-else-if mainView 渲染：
                                    FilePanel / FileTreeNode / FileEditor / VaultPanel / RepoPanel /
                                    GitPanel / GitHistory / GitDiffViewer / GitWriteConfirmDialog /
                                    PermissionPreviewModal（权限预览弹窗，非"Preview"主目标）
                                    （Script/Task/Agent/Skill/DB 等面板虽在同一目录，属其他域，不归 8C）

src/capability/                  ← runtime + bootstrap(index.ts) + capabilities/(仅 bookmark)
                                    Shell 经 Contribution Registry 消费 bookmark（8B.1 已落地）
```

结论：Workspace 域当前是「单体 store + 一堆 UI 面板 + mainView 硬开关」，**还不是 capability**。
这与 Bookmark 完全不同——Bookmark 迁移前已有独立 `useBookmarkStore` 且 UI 边界清晰；
Workspace 必须先做**状态所有者分解**，否则直接套 Contribution Model 只会造出第二层 wrapper（验收 Q1 驳回）。

---

## 2. 性质判断（Workspace / Files / Preview / Editor）

```text
Workspace  → 上层 DOMAIN（umbrella）。候选 = 一个 Capability shell，manifest 汇总 workspace 贡献；
             或 Service 层聚合。禁止 = 直接 re-export 单体 useWorkspaceStore（wrapper，驳回）。
             先决条件 = 其下子域完成状态所有者分解。

Files      → Workspace 内最自洽的子域。候选 = Sub-Capability（或独立 Capability），
             自有 public boundary + 贡献。先决 = 从单体 store 抽出 useFileStore（Semantic Governance 动作）。

Preview/Editor → Files 的 UI 表面（FileEditor = 编辑 + Markdown 预览）。UI，非 capability。
             作为 Files（子）能力的 UI 贡献（slot 如 workspace-main / workspace-dock）。

Vault / Repo / Git → 与 Files 平级的子域，同样共享单体 store。Files 之后分阶段迁移。
```

---

## 3. 为什么 8C ≠ 机械复制 Bookmark

- Bookmark：迁移前已具备「独立 store + 清晰 UI 边界 + 单面板贡献」，8B.1 只是把它从 Shell 直连改为贡献模型。
- Workspace：**状态仍锁在 35KB 单体 store**，多子域耦合。若直接建 Workspace capability 并 re-export 单体 store，
  等于把 store 包一层壳 = 第二真源风险 + wrapper（验收 Q1 驳回）。
- 因此 8C 的真正前置是 **8C-0：单体 store 所有者分解**（Semantic Governance 敏感动作）。

---

## 4. 分阶段计划

### 8C-0（前置 · Governance 敏感 · 风险承载）
- 目标：把 `useWorkspaceStore` 分解为 `useFileStore` + `useVaultStore` + `useRepoStore` + `useGitStore`。
- 动作：
  - 逐消费者（FilePanel/FileEditor/VaultPanel/RepoPanel/Git*/MainArea/ActivityBar/homeUi 等）改引新 store。
  - 更新 `states.yaml` / `owners.yaml`（每个新 owner 登记，旧 useWorkspaceStore 收敛为薄聚合或删除）。
  - 跑 `check-semantic-registry`(real+self-test) + `check-semantic-closure-logic`，确保无第二真源 / 无静默漏检。
  - 行为等价证明：分解前后同名 selector / action 输出一致（必要时加 closure 断言）。
- HARD STOP 触发：若分解导致行为不等价 / 持久化格式不兼容 → 停下，回退，交由人工裁决。

### 8C-1（pilot · Files 子能力）
- 建 `src/capabilities/workspace/`（manifest/contracts/intents/lifecycle/resource/public/index）
  + `src/capabilities/files/`（独立 Files 子能力，引用 useFileStore）。
- Files 注册贡献：FilePanel(surface workspace-main) + FileEditor/Preview(navigation/workspace-dock)。
- MainArea/ActivityBar 改经 contribution slot 渲染 `mainView==='files'` 分支，去掉该硬开关。
- 复用 8B.1 的 Boundary Checker（CB-02/06）确认 Shell 零 import src/capabilities/workspace|files 内部。
- 验收（类比 BKM-C3-*）：Files 贡献注册 + Shell 去专属知识 + absent 启动 + 单真源 + 构建通过。
- 提交 + （若达 C3 等价）打 `capability-phase8c-files-composable-pass`。

### 8C-2 / 8C-3（Vault / Repo+Git 子能力）
- 同 8C-1 模式，依次迁移；每步独立提交 + checker PASS + 文档。

---

## 5. 立即下一步（本阶段第一个可执行切片）

**8C-0 起步**：先对 `useFileStore` 做最小安全抽取——仅取出 Files 相关 state/action，保持其余仍在
`useWorkspaceStore`（薄聚合），跑 registry/closure 门禁确认无回归。此切片小、可逆、不触碰 UI 开关，
是 8C 正确且低风险的开端。完成后再推进 8C-1。

---

## 6. 复用的既有资产（不要重建）

- Contribution Model：`src/capability/contribution/{types,registry}.ts`（8B.1）
- Boundary Checker：`scripts/check-capability-boundaries.mjs`（CB-02/06）、`check-capability-composition.mjs`（C3/C4-ABSENT）
- Implementation Locator：`docs/architecture/semantic-registry/states.yaml` + `check-semantic-registry.mjs`
- Pilot 自检：`scripts/check-capability-pilot.mjs`（PLT-05 owner 守护，Files 适配器不得 import useFileStore）

---

## 7. 禁止事项（继承 + 8C 新增）

- 继承 8B 全部禁止项（不动 panelOpen owner、不建第二 visibility state、不删/移旧 tag、不 push、不 reset WIP）。
- 禁止把 `useWorkspaceStore` 单体直接 re-export 成一个"Workspace capability"（wrapper）。
- 禁止在未做 8C-0 所有者分解前就给 Shell 挂 Workspace 贡献（会变成 wrapper + 第二真源）。
- 禁止机械复制 Bookmark 目录结构到 Vault/Repo/Git（先各自判断 owner 边界）。
- 禁止为迁移降低 Semantic Governance 门禁（registry/closure 必须仍 PASS）。
