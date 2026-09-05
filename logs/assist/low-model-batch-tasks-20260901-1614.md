# Low Model Batch Tasks

STATUS=READY
ROUTE=AI:FAST
DATE=2026-09-01 16:14 CST
GOAL=让低模型批量完成 M1/M2/M3 的低风险前置任务；不移动主线 NEXT，不签主任务 PASS。
CURRENT_MAIN_NEXT=M1-4

## 总提示词

```text
你正在 /home/ainfinit/.codex/worktrees/7350/mvp-browser-os-v3 的 feature-M0-baseline 分支接手低模型批量前置任务。

当前主线 NEXT=M1-4，主任务是 AI:DEEP/R:high，你不能实现 M1-4、不能移动 NEXT、不能宣称任何主线 WBS PASS。

你的目标：按下面任务顺序尽可能完成所有 LOW_MODEL_BATCH 任务。每个任务必须独立产物、独立提交、失败不阻塞后续任务。你只能做低风险前置工作：现状盘点、契约草案、人工验收模板、mock/UI 草图、测试夹具草案。禁止改安全策略、生命周期、Git 写操作、默认浏览器接入、冻结证据。

全局执行规则：
1. 开工先运行：git status --short --branch
2. 每个任务只写指定 WRITE 文件，除非任务明确允许。
3. 每个任务完成后运行：git diff --check
4. 文档/模板任务不要求跑 pre-merge；如果写了可编译代码或 UI mock，必须跑 npm run build。
5. 每个任务独立提交，提交信息用任务卡里的 COMMIT。
6. 某任务失败时，写入对应文件，标记 STATUS=BLOCKED，说明原因、已试命令、下一步，然后继续下一个任务。
7. 全部任务结束后，写 logs/assist/low-model-batch-summary-<YYYYMMDD-HHMM>.md，列出 DONE/BLOCKED/COMMITS/WORKTREE。
8. 最终工作树必须干净。

严禁：
- 不移动 AI-模型切换与接手清单.md 顶部 NEXT。
- 不把 M1-4/M1-5/M1-7/M2/M3 主任务标 PASS。
- 不实现默认浏览器、Git 写能力、脚本执行通道、本地 WebView 隔离、终端并发核心。
- 不删除或改动 logs/m0-*、logs/checkpoints/M0-* 冻结证据。
- 不合并多个任务到一个提交。

LOW_MODEL_BATCH 任务如下，必须按顺序逐个执行。
```

## 任务 1：M1-3-gui-check

```text
TASK_ID=M1-3-gui-check
ROUTE=AI:FAST
GOAL=整理收藏 UI 的可执行 GUI 目视验收表，不签 M1-3 二次 PASS。
READ=logs/checkpoints/M1-3-20260901-1123.md, src/components/browser/BookmarkStar.vue, src/components/browser/BookmarkPanel.vue, src/stores/useBookmarkStore.ts, src/components/layout/ActivityBar.vue, src/components/layout/MainArea.vue
WRITE=logs/assist/M1-3-gui-check-<YYYYMMDD-HHMM>.md
FORBID=不改产品代码；不移动 NEXT；不宣称 GUI 已人工通过；不把静态审查当目视验收。
COMMANDS=git status --short --branch; rg 检索 BookmarkStar/BookmarkPanel 挂点; git diff --check
PASS_CRITERIA=输出 3 项目视验收表：星标收藏/取消、侧栏跳转删除、重启后不丢；每项含操作步骤、预期结果、证据要求、PASS/FAIL 填写位。
COMMIT=docs(M1-3-gui-check): add bookmark gui verification checklist
NEXT=保持 M1-4
```

## 任务 2：M1-5.a-assist

```text
TASK_ID=M1-5.a-assist
ROUTE=AI:FAST
GOAL=为 Git 只读能力做现状盘点、命令契约草案和测试用例草案。
READ=src-tauri/src/sync.rs, src-tauri/src/bridge.rs, src-tauri/src/domain.rs, src/bridge.ts, src/stores/useWorkspaceStore.ts, 详细设计与实施计划.md §3.2
WRITE=logs/assist/M1-5.a-assist-<YYYYMMDD-HHMM>.md
FORBID=不实现 Git 命令；不新增 IPC；不运行破坏性 git 命令；不移动 NEXT。
COMMANDS=git status --short --branch; rg -n "git|Repo|diff|status|branch|commit|push|checkout" src-tauri/src src; git diff --check
PASS_CRITERIA=输出可复用函数盘点、建议 DTO、status/diff/branch_list 命令契约、只读安全边界、大 diff 截断策略、测试用例草案。
COMMIT=docs(M1-5.a-assist): prepare git readonly contract notes
NEXT=保持 M1-4
```

## 任务 3：M1-7.a-assist

```text
TASK_ID=M1-7.a-assist
ROUTE=AI:FAST
GOAL=为 Git UI 做组件挂点盘点和 mock UI 设计，不接真实 Git 写操作。
READ=src/components, src/stores, src/types.ts, 详细设计与实施计划.md §3.2, 后续需求TODO.md #5
WRITE=logs/assist/M1-7.a-assist-<YYYYMMDD-HHMM>.md
FORBID=不改产品代码；不接真实 Git 命令；不做 commit/push/checkout；不移动 NEXT。
COMMANDS=git status --short --branch; rg -n "RepoPanel|repo|diff|commit|branch|sync" src/components src/stores src/types.ts; git diff --check
PASS_CRITERIA=输出 Git UI 信息架构、组件挂点、mock 数据结构、交互流程、风险点和 M1-7 后续检查点建议。
COMMIT=docs(M1-7.a-assist): prepare git ui mock design notes
NEXT=保持 M1-4
```

## 任务 4：M2-1-assist

```text
TASK_ID=M2-1-assist
ROUTE=AI:FAST
GOAL=为图片领域与持久化做 ImageRef 草案、路径/MIME/大小限制盘点。
READ=src-tauri/src/domain.rs, src-tauri/src/workspace.rs, src-tauri/src/bridge.rs, src/types.ts, src/bridge.ts, 详细设计与实施计划.md §4
WRITE=logs/assist/M2-1-assist-<YYYYMMDD-HHMM>.md
FORBID=不改 Artifact 结构；不新增 save_image；不移动 NEXT；不宣称 M2-1 PASS。
COMMANDS=git status --short --branch; rg -n "Artifact|Image|image|mime|write_file|data_dir|workspace" src-tauri/src src; git diff --check
PASS_CRITERIA=输出 ImageRef 字段草案、存储目录建议、MIME 白名单、大小上限、路径策略、反向测试用例草案。
COMMIT=docs(M2-1-assist): prepare image persistence contract notes
NEXT=保持 M1-4
```

## 任务 5：M2-2.a-assist

```text
TASK_ID=M2-2.a-assist
ROUTE=AI:FAST
GOAL=图片预览 UI 现状盘点和组件结构设计。
READ=src/components/workspace, src/stores/useWorkspaceStore.ts, src/types.ts, 详细设计与实施计划.md §4
WRITE=logs/assist/M2-2.a-assist-<YYYYMMDD-HHMM>.md
FORBID=不改产品代码；不新增真实图片读写；不移动 NEXT。
COMMANDS=git status --short --branch; rg -n "preview|image|Artifact|FileEditor|gallery|lightbox|zoom" src/components src/stores src/types.ts; git diff --check
PASS_CRITERIA=输出图片预览 UI 组件拆分、状态模型、缩放/灯箱/画廊验收标准、依赖 M2-1 的接口清单。
COMMIT=docs(M2-2.a-assist): prepare image preview ui notes
NEXT=保持 M1-4
```

## 任务 6：M2-3-assist

```text
TASK_ID=M2-3-assist
ROUTE=AI:FAST
GOAL=为脚本领域与持久化做 ScriptMeta 字段、存储文件格式和参数契约草案。
READ=src-tauri/src/domain.rs, src-tauri/src/workspace.rs, src-tauri/src/bridge.rs, src/types.ts, src/bridge.ts, 详细设计与实施计划.md §4, 后续需求TODO.md #1
WRITE=logs/assist/M2-3-assist-<YYYYMMDD-HHMM>.md
FORBID=不实现脚本执行；不新增 run_script；不接 shell；不移动 NEXT。
COMMANDS=git status --short --branch; rg -n "Script|script|command|shell|run_command|request_sync|Channel|audit" src-tauri/src src; git diff --check
PASS_CRITERIA=输出 ScriptMeta 字段、scripts.json 存储格式、参数占位符契约、审计字段、后续 M2-4 执行通道依赖。
COMMIT=docs(M2-3-assist): prepare script metadata contract notes
NEXT=保持 M1-4
```

## 任务 7：M2-5.a-assist

```text
TASK_ID=M2-5.a-assist
ROUTE=AI:FAST
GOAL=脚本库 UI 静态页面/交互草图设计，不接执行通道。
READ=src/components, src/stores, src/types.ts, 详细设计与实施计划.md §4, 后续需求TODO.md #1
WRITE=logs/assist/M2-5.a-assist-<YYYYMMDD-HHMM>.md
FORBID=不改产品代码；不接 run_script；不执行 shell；不移动 NEXT。
COMMANDS=git status --short --branch; rg -n "AppPanel|RepoPanel|Terminal|script|command|form|audit" src/components src/stores src/types.ts; git diff --check
PASS_CRITERIA=输出脚本库 UI 信息架构、组件挂点、表单字段、参数弹窗流程、执行状态 mock、验收标准。
COMMIT=docs(M2-5.a-assist): prepare script library ui notes
NEXT=保持 M1-4
```

## 任务 8：M2-7-assist

```text
TASK_ID=M2-7-assist
ROUTE=AI:FAST
GOAL=扫描内置工具文件，输出 ToolMeta 草案、打包方案和用户目录扫描策略。
READ=src-tauri/src/tools, src-tauri/tauri.conf.json, src-tauri/build.rs, src-tauri/src/bridge.rs, src/components/apps, 详细设计与实施计划.md §4
WRITE=logs/assist/M2-7-assist-<YYYYMMDD-HHMM>.md
FORBID=不改 build.rs；不新增 list_tools；不改打包配置；不移动 NEXT。
COMMANDS=git status --short --branch; rg --files src-tauri/src/tools; rg -n "include_dir|bundle.resources|list_tools|tools|AppPanel" src-tauri src; git diff --check
PASS_CRITERIA=输出 5 个种子工具清单、ToolMeta 字段草案、内置资源嵌入方案、用户工具目录扫描策略、ACL 和验收建议。
COMMIT=docs(M2-7-assist): prepare tool catalog packaging notes
NEXT=保持 M1-4
```

## 任务 9：M2-9-assist

```text
TASK_ID=M2-9-assist
ROUTE=AI:FAST
GOAL=为 5 个种子工具整理逐项离线验收表，不签 M2-9 PASS。
READ=src-tauri/src/tools, 详细设计与实施计划.md §4, 后续需求TODO.md #2
WRITE=logs/assist/M2-9-assist-<YYYYMMDD-HHMM>.md
FORBID=不改工具 HTML；不宣称打包后可打开；不移动 NEXT。
COMMANDS=git status --short --branch; rg --files src-tauri/src/tools; rg -n "<script|localStorage|fetch|http|https|import|src=" src-tauri/src/tools; git diff --check
PASS_CRITERIA=输出 json/base64/timestamp/regex/cron 五项验收表：离线性、输入边界、输出正确性、打包可打开证据要求、PASS/FAIL 填写位。
COMMIT=docs(M2-9-assist): add seed tool verification checklist
NEXT=保持 M1-4
```

## 任务 10：M3-4.a-assist

```text
TASK_ID=M3-4.a-assist
ROUTE=AI:FAST
GOAL=终端体验小项盘点：临时历史、resize 静默窗口等前端入口，不改终端核心。
READ=src/components/system/TerminalPane.vue, src/stores/useSystemStore.ts, src-tauri/src/bridge.rs, src-tauri/src/main.rs, 详细设计与实施计划.md §5
WRITE=logs/assist/M3-4.a-assist-<YYYYMMDD-HHMM>.md
FORBID=不改 PTY；不改 term_kill；不改线程/生命周期；不移动 NEXT。
COMMANDS=git status --short --branch; rg -n "term_|Terminal|resize|history|pty|shell|kill" src src-tauri/src; git diff --check
PASS_CRITERIA=输出终端体验项清单、现有入口、可低风险实现项、必须等 M3-1/2/3 后再做的项、验收标准。
COMMIT=docs(M3-4.a-assist): prepare terminal ux notes
NEXT=保持 M1-4
```

## 最终汇总任务

```text
TASK_ID=low-model-batch-summary
ROUTE=AI:FAST
GOAL=汇总本轮低模型批量任务结果。
READ=logs/assist/
WRITE=logs/assist/low-model-batch-summary-<YYYYMMDD-HHMM>.md
FORBID=不移动 NEXT；不宣称主线 PASS。
COMMANDS=git status --short --branch; git log --oneline -20; git diff --check
PASS_CRITERIA=列出每个任务 DONE/BLOCKED、产物路径、commit sha、未完成原因、给强模型的最小输入列表；工作树干净。
COMMIT=docs(low-model-batch): summarize assist task results
NEXT=保持 M1-4
```
