# M2-script-library-ui-static-shell（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §4.1 **M2-3**（脚本库 UI）· 需求 **#1 脚本库中心** / **#4 常用 Linux 命令库**
> 状态：⏸ **静态壳方案 / mock 数据 / 不执行真实脚本 / 不宣称 PASS**
> 契约依赖：`M2-3.a-prework-20260902-1055.md`（ScriptMeta 契约）· 安全依赖：`script-execution-safety-taskcard-20260902-1146.md`

---

## 1. 目标

产出**脚本库 UI 静态壳方案**：脚本列表、参数弹窗、危险参数二次确认、运行/成功/失败/取消四态、输出区、历史记录 mock、审计展示位。
**不执行真实脚本**——零 `invoke`，零 `shell` 插件调用，全部走 mock 执行器（模拟流式输出/延时/失败）。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 实测 |
|---|---|
| 脚本库前端组件 | ❌ 无。22 个 `.vue` 中无 Script 相关 |
| `run_script` 命令 | ❌ 不在 59 个白名单命令中 |
| `ScriptMeta` 领域模型 | ❌ `src-tauri/src/domain.rs` 中无 |
| 可复用组件 | ✅ `src/components/shared/ConfirmModal.vue`（二次确认） |
| 可复用审计展示 | ✅ `src/components/workspace/AuditPanel.vue`（审计面板，读 `audit.json`） |
| shell 能力已开 | ⚠️ `capabilities/default.json` 已含 `shell:allow-spawn`（`args: true`）+ `allow-stdin-write` + `allow-kill` → **ACL 不兜底，参数校验必须在 `run_script` 内部（K4）** |

---

## 3. 必改文件候选（本批次**只写方案**）

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/types/script.ts` | 新增 | `ScriptMeta` / `ScriptRunState` / `ScriptHistoryEntry` 前端类型 |
| `src/mocks/scripts.ts` | 新增 | mock 脚本库 + mock 执行器 |
| `src/stores/useScriptStore.ts` | 新增 | 状态机（静态壳阶段调 mock 执行器） |
| `src/components/system/ScriptPanel.vue` | 新增 | 容器（列表 + 详情 + 输出） |
| `src/components/system/ScriptCard.vue` | 新增 | 脚本卡片 |
| `src/components/system/ScriptParamModal.vue` | 新增 | 参数弹窗（含危险参数二次确认） |
| `src/components/system/ScriptOutput.vue` | 新增 | 输出区（流式/ANSI/清空/导出） |
| `src/components/system/ScriptHistory.vue` | 新增 | 历史记录（mock） |
| `src/components/system/ScriptAuditBadge.vue` | 新增 | 审计展示位（占位，指向真实 audit） |
| `src/components/shared/ConfirmModal.vue` | **复用** | 危险参数二次确认 |
| `src/components/layout/ActivityBar.vue` | 改动 | 增加「脚本」入口 |

---

## 4. 数据结构契约（与 `M2-3.a` 对齐）

```ts
// src/types/script.ts
export type ParamType = 'string' | 'number' | 'bool' | 'path' | 'enum' | 'password';

export interface ScriptParam {
  name: string;                 // 传给脚本的参数名
  label: string;                // UI 显示名
  type: ParamType;
  required: boolean;
  default?: string | number | boolean;
  /** enum 专用 */
  options?: { value: string; label: string }[];
  /** 危险参数：需要二次确认 + 审计 */
  dangerous?: boolean;
  /** 校验正则（前端 + 后端双校验，后端为准） */
  pattern?: string;
  /** 危险原因说明，会显示在二次确认弹窗 */
  dangerReason?: string;
  placeholder?: string;
  help?: string;
}

export interface ScriptMeta {
  id: string;
  name: string;
  description: string;
  category: 'deploy' | 'ops' | 'git' | 'data' | 'custom';
  /** 解释器；禁止由用户传入（见 script-execution-safety-taskcard 的「禁 sh -c」） */
  interpreter: 'bash' | 'sh' | 'python3' | 'node';
  /** 脚本文件的绝对路径，必须 canonicalize + 位于允许根目录内 */
  scriptPath: string;
  params: ScriptParam[];
  /** 需要的能力标签，用于审计与未来 ACL 细化 */
  capabilities: ('net' | 'fs-write' | 'fs-delete' | 'sudo' | 'db')[];
  timeoutSec: number;           // 默认 60
  builtin: boolean;
  version: string;
}

export type RunStatus = 'idle' | 'running' | 'success' | 'failed' | 'cancelled' | 'timeout';

export interface ScriptRunState {
  runId: string;
  scriptId: string;
  status: RunStatus;
  startedAt: string;
  endedAt?: string;
  exitCode?: number;
  /** 流式累积；上限见 M3-terminal-history-taskcard 的历史上限口径（建议 5000 行） */
  output: string[];
  truncated: boolean;
  error?: string;
}

export interface ScriptHistoryEntry {
  runId: string;
  scriptId: string;
  scriptName: string;
  /** ⚠️ 参数脱敏后存储：password 类型一律 '***' */
  paramsSummary: Record<string, string>;
  status: RunStatus;
  startedAt: string;
  durationMs: number;
  exitCode?: number;
  /** 关联审计条目 id（指向 audit.json） */
  auditRef?: string;
}
```

**红线**：`ScriptHistoryEntry.paramsSummary` 中 `type === 'password'` 的参数**一律存 `'***'`**（K3 凭据不入日志/审计/领域模型）。

---

## 5. 组件拆分

```
ScriptPanel.vue                    （容器：左列表 + 右详情/输出）
├── ScriptToolbar.vue              （搜索 / 分类过滤 / 新建 / 刷新）
├── ScriptList.vue
│   └── ScriptCard.vue             （名称/描述/分类标签/危险徽标/运行按钮）
├── ScriptEmptyState.vue           （空态）
├── ScriptDetail.vue               （选中脚本的说明 + 参数摘要 + 「运行」主按钮）
├── ScriptParamModal.vue           （参数表单 + 危险参数二次确认）
│   └── ConfirmModal.vue           （复用，危险参数专用）
├── ScriptOutput.vue               （输出区：状态条 + 流式输出 + 清空/复制/取消）
└── ScriptHistory.vue              （历史记录抽屉/侧栏）
    └── ScriptAuditBadge.vue       （审计展示位）
```

| 组件 | 职责 | 不做什么 |
|---|---|---|
| `ScriptPanel` | 布局与编排 | 不执行脚本 |
| `ScriptParamModal` | 参数收集 + 校验 + 危险确认 | 不拼命令字符串 |
| `ScriptOutput` | 流式展示 + 取消入口 | 不解析 ANSI 为 HTML（禁 `v-html`） |
| `ScriptHistory` | 历史列表（脱敏） | 不落盘（静态壳阶段） |

---

## 6. 交互设计

### 6.1 脚本列表

| 项 | 方案 |
|---|---|
| 布局 | 左侧列表（宽 280 px，可拖拽）+ 右侧详情 |
| 卡片内容 | 名称 / 一句话描述 / 分类徽标 / 🚨危险徽标（若 `capabilities` 含 `fs-delete`/`sudo`）/ 最近运行状态点 |
| 搜索 | 名称 + 描述模糊匹配（前端 `filter`） |
| 分类过滤 | 全部 / deploy / ops / git / data / custom |
| 排序 | 名称升序（默认）；最近运行在前（可选） |
| 空态 | 「还没有脚本」+ 「新建脚本」/「导入目录」按钮 |
| 搜索无结果 | 「没有匹配的脚本」+ 清除搜索 |

### 6.2 参数弹窗

| 项 | 方案 |
|---|---|
| 触发 | 点卡片「运行」→ 弹窗（**无论是否有参数**，用于最终确认） |
| 表单渲染 | 按 `ScriptParam[]` 动态渲染；`enum`→下拉，`bool`→开关，`path`→文本框 + 「选择」按钮（静态壳不接文件对话框），`password`→`type=password` |
| 必填校验 | 提交时前端校验，`required && empty` → 内联红字 |
| 正则校验 | 按 `pattern` 校验；**前端校验只是体验，后端必须重校验（fail-closed，K4）** |
| 危险参数 | `dangerous: true` 的参数：① 输入框红色边框 ② 旁边常驻 🚨 ③ 提交时弹 `ConfirmModal` 二次确认，文案含 `dangerReason` |
| 无参数脚本 | 弹窗内仅显示「确认运行 `<name>`？」+ 危险能力提示 |
| Esc / 遮罩点击 | 关闭（**运行中不响应 Esc 关闭**） |

### 6.3 危险参数二次确认（关键）

```
┌─────────────────────────────────────────┐
│  ⚠️ 危险操作确认                          │
├─────────────────────────────────────────┤
│  脚本：清理构建产物                        │
│  危险参数：                               │
│    • target_dir = /home/u/project/build  │
│      原因：将递归删除该目录下所有文件      │
│  声明能力：fs-delete                      │
│                                          │
│  此操作不可撤销。请确认路径无误。           │
│                                          │
│        [ 取消 ]      [ 我已确认，运行 ]    │
└─────────────────────────────────────────┘
```

**红线**：二次确认的「确认」按钮**必须显式点击**，不得有默认焦点（防回车误触）。

### 6.4 输出区状态机

```
idle ──run──> running ──┬── exit 0 ──> success
                        ├── exit≠0 ──> failed
                        ├── 用户取消 ──> cancelled
                        └── 超时 ────> timeout
```

| 状态 | 状态条 | 输出区 | 可用操作 |
|---|---|---|---|
| `idle` | 灰色「未运行」 | 空态引导 | 运行 |
| `running` | 蓝色脉冲 + 已耗时秒表 + **「取消」按钮** | 流式追加（自动滚到底，除非用户上滚） | **取消** |
| `success` | 绿色「成功 · 退出码 0 · 耗时 3.2s」 | 完整输出 | 复制 / 清空 / 重新运行 |
| `failed` | 红色「失败 · 退出码 1 · 耗时 0.8s」+ 错误摘要 | 完整输出 + 尾部红字错误 | 复制 / 清空 / 重新运行 |
| `cancelled` | 黄色「已取消 · 耗时 12.4s」 | 取消前的输出（保留） | 复制 / 清空 / 重新运行 |
| `timeout` | 橙色「超时（60s）· 已终止进程组」 | 超时前输出 | 复制 / 清空 |

| 输出区细节 | 方案 |
|---|---|
| 上限 | 5000 行；超出丢弃最旧并**在顶部显示「已省略前 N 行」** |
| 自动滚动 | 默认贴底；用户上滚 > 50px 后暂停自动滚，回到底部恢复 |
| ANSI | 静态壳**不做** ANSI 着色（避免 `v-html`）；若要做，必须用 xterm.js 或白名单过滤库，**禁 `v-html`** |
| 复制 | 复制全文；大输出（>1 MB）提示 |
| 清空 | 仅清视图，不影响历史 |

### 6.5 历史记录（mock）

| 项 | 方案 |
|---|---|
| 位置 | 右侧抽屉 / 底部折叠面板 |
| 字段 | 时间 / 脚本名 / 状态点 / 耗时 / 参数摘要（脱敏）/ 退出码 |
| 排序 | 时间倒序，最近 50 条 |
| 交互 | 点击展开看输出摘要（前 200 行）；「重新运行」按钮（带原参数，但需再次确认危险参数） |
| 空态 | 「暂无运行记录」 |
| 审计展示位 | 每条历史右侧一个 `ScriptAuditBadge`，显示 `auditRef` 短 id + 「查看审计」跳转 `AuditPanel`；静态壳阶段显示灰色「审计将在接入后显示」 |

---

## 7. mock 数据设计（`src/mocks/scripts.ts`）

```ts
import type { ScriptMeta, ScriptHistoryEntry, ScriptRunState } from '../types/script';

export const MOCK_SCRIPTS: ScriptMeta[] = [
  {
    id: 'builtin-git-status', name: 'Git 批量状态', description: '遍历目录下所有 git 仓库并输出状态',
    category: 'git', interpreter: 'bash', scriptPath: '<builtin>/git-batch-status.sh',
    params: [{ name: 'root', label: '根目录', type: 'path', required: true, default: '~/projects' }],
    capabilities: [], timeoutSec: 60, builtin: true, version: '1.0.0',
  },
  {
    id: 'builtin-clean-build', name: '清理构建产物', description: '递归删除 target/dist/node_modules',
    category: 'ops', interpreter: 'bash', scriptPath: '<builtin>/clean-build.sh',
    params: [{
      name: 'target_dir', label: '目标目录', type: 'path', required: true,
      dangerous: true, dangerReason: '将递归删除该目录下的 target/、dist/、node_modules/',
      pattern: '^/home/[a-z]+/.*$',
    }],
    capabilities: ['fs-delete'], timeoutSec: 120, builtin: true, version: '1.0.0',
  },
  {
    id: 'builtin-db-dump', name: '数据库备份', description: '导出指定库到 SQL 文件',
    category: 'data', interpreter: 'bash', scriptPath: '<builtin>/db-dump.sh',
    params: [
      { name: 'db_name', label: '库名', type: 'string', required: true, pattern: '^[a-zA-Z0-9_]+$' },
      { name: 'password', label: '数据库密码', type: 'password', required: true },
      { name: 'compress', label: '压缩', type: 'bool', default: true },
      { name: 'format', label: '格式', type: 'enum', required: true,
        options: [{ value: 'sql', label: 'SQL' }, { value: 'custom', label: '自定义' }] },
    ],
    capabilities: ['db', 'net'], timeoutSec: 300, builtin: true, version: '1.1.0',
  },
  // … 另加：长输出脚本（测截断）、必失败脚本、超时脚本（测 timeout）、无参脚本
];

/** mock 执行器：模拟流式输出 + 可取消 + 可失败 + 可超时 */
export function mockRun(
  script: ScriptMeta,
  params: Record<string, unknown>,
  handlers: { onLine(line: string): void; onDone(s: ScriptRunState): void },
): { cancel(): void } { /* 用 setTimeout 分片推 line，cancel() 清 timer */ }
```

**mock 覆盖的 6 类脚本**（每种各 1 个，确保状态机全覆盖）：

| # | 脚本 | 用途 |
|---|---|---|
| 1 | `git-batch-status` | 正常成功，短输出 |
| 2 | `clean-build` | 危险参数 + 二次确认 |
| 3 | `db-dump` | password 参数脱敏 + enum + bool |
| 4 | `long-log-tail` | 输出 8000 行 → 触发 **截断**（测 5000 行上限） |
| 5 | `always-fail` | 退出码 1 → **failed** |
| 6 | `sleep-forever` | 永不结束 → 测 **取消** 与 **超时** |

---

## 8. 错误态 / 空态矩阵

| 场景 | 表现 | 禁止 |
|---|---|---|
| 无脚本 | 空态 + 新建/导入入口 | ❌ 空白 |
| 搜索无结果 | 无结果态 + 清除搜索 | ❌ 与真空态混淆 |
| 参数校验失败 | 字段内联红字 | ❌ `alert` |
| 危险参数未确认 | 不执行，保持弹窗 | ❌ 默认确认 |
| 运行中再次点运行 | 按钮禁用 | ❌ 并发双跑 |
| 取消 | 状态 `cancelled`，保留已产生输出 | ❌ 清空输出 |
| 超时 | 状态 `timeout`，明确提示「已终止进程组」 | ❌ 静默继续跑 |
| 输出截断 | 顶部「已省略前 N 行」 | ❌ 静默丢失 |
| 脚本文件不存在（真实阶段） | 红色错误 + 不进入 running | ❌ 空白运行 |
| 历史为空 | 空态 | ❌ 空白 |

---

## 9. 验收截图点（人工 GUI 必拍）

| # | 场景 | 判据 |
|---|---|---|
| S1 | 脚本列表（含危险徽标） | 卡片信息完整、分类徽标正确 |
| S2 | 空态 | 有引导 |
| S3 | 搜索无结果 | 文案区分 |
| S4 | 参数弹窗（4 种参数类型齐全） | path/enum/bool/password 都渲染正确 |
| S5 | 危险参数二次确认 | 红色边框 + 原因文案 + 无默认焦点 |
| S6 | running 状态条 | 秒表走动 + 取消按钮可用 |
| S7 | success | 绿色 + 退出码 0 + 耗时 |
| S8 | failed | 红色 + 退出码 1 + 错误摘要 |
| S9 | cancelled | 黄色 + 输出保留 |
| S10 | timeout | 橙色 + 明确提示 |
| S11 | 输出截断（>5000 行） | 顶部「已省略前 N 行」 |
| S12 | 历史抽屉 | 脱敏参数显示 `***` |
| S13 | 审计展示位 | 有占位，不假装有数据 |
| S14 | 运行中再次点击运行 | 按钮禁用 |

---

## 10. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 高 | **命令注入**：`shell:allow-spawn` 已 `args: true`，ACL 不兜底 | 参数**只能数组传递**；禁止 `sh -c` 字符串拼接；后端 fail-closed 重校验 → 见 `script-execution-safety-taskcard` |
| 高 | **凭据泄露**：password 参数进历史/审计 | 一律 `'***'`；`ScriptMeta` 内**不得**有密码字段（K3） |
| 中 | 输出无背压导致 OOM | 5000 行上限 + 截断提示；真实实现需环形缓冲 |
| 中 | 取消不彻底（残留子进程） | 静态壳只停 mock；真实实现必须**进程组 kill** + ShutdownCoordinator（依赖 E4/TASK-10） |
| 中 | ANSI 输出用 `v-html` 渲染 → XSS | 静态壳不着色；若着色必须用 xterm 或白名单库，**禁 `v-html`** |
| 低 | `audit.json` 1000 条上限被刷爆 | 自动行为进独立 JSON，摘要进 audit（K5，见 `M4-58.a`） |

---

## 11. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 断网打开脚本库 | mock 正常（零外链） |
| R2 | 危险参数直接回车 | 焦点在「取消」或无默认焦点，**不执行** |
| R3 | 参数填 `; rm -rf /` | 前端 `pattern` 拦截；即使绕过，后端必须拒（静态壳仅验证前端提示） |
| R4 | password 参数运行后看历史 | 显示 `***` |
| R5 | 运行 `sleep-forever` 后点取消 | 状态 `cancelled`，输出保留，秒表停止 |
| R6 | 运行 `long-log-tail`（8000 行） | 视图 ≤5000 行 + 顶部省略提示 |
| R7 | running 时点关闭面板 | 提示「脚本正在运行」，不得静默丢弃 |
| R8 | 输出区上滚后新输出到达 | **不**强制回到底部 |
| R9 | 复制 >1 MB 输出 | 有提示，不卡死 |
| R10 | 主 JS 体积 | 构建后对照 baseline 505 KB |

---

## 12. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 12.1 静态壳不得触碰真实执行通道（K9）
grep -rn "invoke\|__TAURI__\|plugin-shell\|Command(" src/components/system/Script*.vue src/stores/useScriptStore.ts | wc -l
# 期望 0

# 12.2 禁 v-html（K8）
grep -rn "v-html\|innerHTML" src/components/system/Script*.vue | wc -l
# 期望 0

# 12.3 密码脱敏（K3）
grep -n "'\*\*\*'" src/stores/useScriptStore.ts src/components/system/ScriptHistory.vue
# 期望命中

# 12.4 禁 sh -c 拼接（K4，静态壳阶段提前校验 mock 不演示反例）
grep -rn "sh -c\|bash -c" src/components/system/Script*.vue src/mocks/scripts.ts | wc -l
# 期望 0

# 12.5 输出上限
grep -n "5000\|MAX_LINES" src/components/system/ScriptOutput.vue
# 期望命中

# 12.6 构建
npm run build && ls -lh dist/assets/*.js
# 对照 baseline
```

---

## 13. 失败动作

| 失败 | 动作 |
|---|---|
| 出现真实执行调用 | 立即回退；真实执行是 `script-execution-safety-taskcard` 的范围 |
| 密码出现在历史/审计 | 视为 P0 安全缺陷，立即修复并清理已有记录 |
| 取消后进程仍在（真实阶段） | 补进程组 kill；不得只 kill 直接子进程 |
| 输出未截断 | 补环形缓冲；不得靠「用户不会跑那么久」搪塞 |
| 体积超标 | 组件懒加载；不得放宽 505 KB 阈值 |

---

## 14. 推荐模型

`AI:BALANCED`（组件 + 状态机 + 表单，量大但模式固定）。
**人工 GUI 验收必做**（14 截图点 + 10 反向用例）。
真实脚本执行通道 → `AI:DEEP` + 人工安全评审（见 `script-execution-safety-taskcard`）。
