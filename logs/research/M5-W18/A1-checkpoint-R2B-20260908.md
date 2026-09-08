# A1 · M5-W18-R2B Checkpoint

```text
LANE=A1
DISPATCH=M5-W18-R2B
STATUS=READY_FOR_REVIEW
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a1/mvp-browser-os-v3
BRANCH=codex/m5-w18-a1
BASE=434e63f
HEAD=<pending commit>
CONSUMED_PEERS=none（A1 为壳层契约起草方，不消费其他 lane R2B 成果）
FILES=logs/research/M5-W18/A1-workbench-shell-R2B-20260908.md, logs/research/M5-W18/A1-checkpoint-R2B-20260908.md, logs/research/M5-W18/A1-wireframe-prototype-R2B.html
CORRECTIONS=C1 前端测试"0个"更正为"0框架单测+20 MJS逻辑测试+29 Python策略测试"; C2 W17-D6归属修正(归A7/A0非A1); C3 useConnectionStore不存在已确认
VERIFY=read-only: grep/wc/python3 静态扫描 + 线框 HTML 浏览器可打开; 未运行构建或产品测试(research-only)
PROPOSED_SLICES=S0(DbValue唯一源), S1(工作台外壳)
OPEN_DECISIONS=原生WebView重定位需A9/A11实机验证; 编辑器升级依赖需A5评估; 布局持久化schema需A11; 体积delta需A11复测
NEXT=交A3/A5/A8签收壳层契约; 交A10独立审查; 交A11登记
NO_PRODUCT_CODE=true
NO_PUSH=true
```

## What A1 R2B delivered

### 1. 旧功能 → 新工具窗口全覆盖清单（§3）
- 19 MainView 全覆盖 ✅
- 18 MODULE_META 全覆盖 ✅
- 16 ☰ 菜单项全覆盖 ✅
- J1-J6 工作流全有入口映射 ✅

### 2. 状态契约（§4）
- 工作区（WorkspaceContext）：id/rootPath/name/recents + 生命周期
- 文档（DocIdentity/DocTab）：key/type/workspaceId/location/dirty + 关闭/恢复
- 工具窗口（ToolWindow）：side/size/activeItem/visible/focusReturn + 隐藏≠销毁
- 请求（RequestHandle）：requestId/status + 取消幂等/旧响应失效
- 导航（NavResult）：resourceType/location/returnPosition
- 持久化（PersistedState）：schemaVersion + 凭据/SQL不落盘
- 焦点与资源生命周期区分：关闭视图/关闭文档/关闭工作区/取消任务/销毁会话 5 种语义

### 3. 线框设计（§5）
- 1440×900：左260+底240+右240(按需)
- 1024×720：右隐藏+底180+左可折叠
- 800×600：左/右/底全折叠+☰菜单弹出
- 焦点/键盘/IME 规格：Tab循环/Ctrl+Shift+E/D/A/Ctrl+P/IME跳过/焦点返回
- 与 A3/A5/A8 衔接点

### 4. 共享壳层唯一修改者清单（§6）
- A1 拥有：App.vue / useLayoutStore.ts / ActivityBar / MainArea / UnifiedTabBar / StatusBar / Sidebar / home/*
- A3 拥有：graph/*
- A5 拥有：DatabasePanel / useDatabaseStore / dbUi
- A8 拥有：搜索相关（新增）
- 冲突避免：A1 先冻结壳层契约，A3/A5/A8 在契约内嵌入

### 5. 前端测试分类更正（§7）
- 框架单测：0（缺 vitest/jest）
- MJS 逻辑测试：20（`check-*-logic.mjs`）
- Python 策略测试：29（`check-*.py`）
- 更正 R1/R2 的"0 前端测试"表述

### 6. S0/S1 候选实现卡（§7.2, §7.3）
- S0: DbValue 唯一源（A4 owner, PROPOSED_NOT_AUTHORIZED）
- S1: 工作台外壳（A1 owner, PROPOSED_NOT_AUTHORIZED）

### 7. J1/J6 完整性验证（§9）
- J1 打开并接续项目：8 步全支持 ✅
- J6 正常退出与异常恢复：6 步全支持 ✅

## Corrections from R1/R2

| # | Correction | Evidence |
|---|---|---|
| C1 | 前端测试"0个" → "0框架单测+20 MJS+29 Python" | `EXECUTED_SYNTHETIC_TEST`: `ls scripts/check-*-logic.mjs \| wc -l` → 20 |
| C2 | W17-D6 归 A7/A0 非 A1 | `CURRENT_PRODUCT`: debt ledger L615 |
| C3 | useConnectionStore 不存在 | `CURRENT_PRODUCT`: `find src -name "*onnection*tore*"` → 无 |

## Boundary compliance

- Zero product-code changes; no dependency install; no build/test run; no push.
- All output under `logs/research/M5-W18/A1-*` only.
- R1/R2 reports preserved as drafts (not silently rewritten).
- Every factual claim tagged with evidence type per R2B contract §1.
- Wireframe HTML is self-contained static prototype with synthetic data only.
- Proposed slices marked `PROPOSED_NOT_AUTHORIZED`.

## Unresolved questions dispatched

- U1 (原生 WebView 重定位) → A9/A11
- U2 (编辑器升级依赖评估) → A5
- U3 (布局持久化 schema) → A11
- U4 (modTabs → docTabs 迁移兼容) → A1 (S1 实施时)
- U5 (体积 delta) → A11
