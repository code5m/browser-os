# M2-tool-library-ui-static-shell（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §4.3 **M2-5**（工具箱面板 / 工具清单枚举）· 需求 **#2 小工具框架**
> 状态：⏸ **静态壳方案 / mock 数据 / 不接真实加载运行时 / 不宣称 PASS**
> 关联：`M2-tools-seed-html-prework-20260902-1146.md`（5 个 HTML 本体）· `M2-7.b-prework-20260902-1055.md`（ToolMeta/打包）· 勘误 **E1/E5**

---

## 1. 目标

产出**工具库（工具箱）UI 静态壳方案**：工具卡片、分类/搜索、离线状态、启动失败态、权限提示、详情面板、mock 数据、验收截图点。
**不接真实加载运行时**——不实现 `list_tools`、不创建子 webview、不加载 `asset://`。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 实测 |
|---|---|
| 工具箱前端面板 | ❌ 无。22 个 `.vue` 中无 Tool 相关 |
| `list_tools` 命令 | ❌ 不在 59 个白名单命令中 |
| 5 个种子 HTML | ❌ 全部不存在（勘误 E1） |
| `include_dir!` 打包 | ❌ `src-tauri/build.rs` 仅 40 B，极简 |
| 前端依赖 | `vue`/`pinia`，无路由、无 UI 库 |
| 安全配置 | ⚠️ `withGlobalTauri = true`、`csp: null`、`assetProtocol.scope` 仅覆盖 `/usr/share`、`/usr/local/share`、`$HOME/.local/share`、`$HOME/.icons`（**不含** `workspace/tools/`）→ 勘误 E5 |

---

## 3. 必改文件候选（本批次**只写方案**）

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/types/tool.ts` | 新增 | `ToolMeta` / `ToolState` 前端类型 |
| `src/mocks/tools.ts` | 新增 | mock 工具清单（含各类异常态） |
| `src/stores/useToolStore.ts` | 新增 | 状态机（静态壳阶段读 mock） |
| `src/components/system/ToolLibraryPanel.vue` | 新增 | 容器 |
| `src/components/system/ToolCard.vue` | 新增 | 工具卡片 |
| `src/components/system/ToolDetailPanel.vue` | 新增 | 详情面板 |
| `src/components/system/ToolEmptyState.vue` | 新增 | 空态 |
| `src/components/system/ToolPermissionHint.vue` | 新增 | 权限提示条 |
| `src/components/layout/ActivityBar.vue` | 改动 | 增加「工具箱」入口 |
| `src-tauri/src/tools/*.html` | **另一张卡** | 见 `M2-tools-seed-html-prework` |

---

## 4. 数据结构契约（与 `M2-7.b` 的 `ToolMeta` 对齐）

```ts
// src/types/tool.ts
export type ToolCategory = 'converter' | 'text' | 'dev' | 'media' | 'utility' | 'custom';
export type ToolSource = 'builtin' | 'user';
export type ToolStatus = 'available' | 'offline-ready' | 'missing' | 'load-failed' | 'permission-denied' | 'disabled';

export interface ToolMeta {
  id: string;                    // 稳定 id（builtin 用文件名主体；user 用路径 sha256 前 16 位）
  name: string;                  // 来自 <meta name="tool-name">
  description: string;           // 来自 <meta name="tool-description">
  category: ToolCategory;        // 来自 <meta name="tool-category">
  version: string;               // 来自 <meta name="tool-version">
  source: ToolSource;            // builtin = 打包内；user = workspace/tools/
  /** 逻辑路径：builtin 为 'tools/xxx.html'；user 为绝对路径 */
  path: string;
  /** 解析出的实际加载 URL（静态壳为 mock） */
  loadUrl?: string;
  /** 图标：单字符 emoji 或内联 SVG data-URI；不允许外链 */
  icon?: string;
  /** 声明需要的宿主能力（未来 ACL 细化） */
  capabilities: string[];
  /** 文件大小（字节） */
  size?: number;
  /** 最后修改时间（ISO 8601），user 工具用 */
  updatedAt?: string;
}

/** UI 运行态，禁止混入持久化结构 */
export interface ToolState {
  status: ToolStatus;
  /** 最近一次失败原因（仅展示） */
  error?: string;
  /** 是否正在启动（防重复点击） */
  launching: boolean;
}
```

**红线**：`ToolMeta.icon` **不得**是外链 URL（离线 + CSP 双重红线）。

---

## 5. 组件拆分

```
ToolLibraryPanel.vue               （容器：工具栏 + 网格 + 详情抽屉）
├── ToolToolbar.vue                （搜索 / 分类 / 来源过滤 / 刷新 / 打开工具目录）
├── ToolGrid.vue
│   └── ToolCard.vue               （图标/名称/描述/来源徽标/状态徽标/启动按钮）
├── ToolEmptyState.vue             （空态：区分「无工具」「过滤无结果」「目录不可读」）
├── ToolDetailPanel.vue            （右侧抽屉：完整描述、路径、版本、能力、打开/禁用/卸载）
├── ToolPermissionHint.vue         （权限提示条：需开启某能力/路径不在 scope 内）
└── ToolErrorState.vue             （启动失败态：错误 + 重试 + 反馈入口）
```

| 组件 | 职责 | 不做什么 |
|---|---|---|
| `ToolLibraryPanel` | 布局与编排 | 不创建 webview |
| `ToolCard` | 单卡片四态 | 不发起真实加载 |
| `ToolDetailPanel` | 详情与操作入口 | 不做删除（删除走 `ConfirmModal`） |
| `ToolPermissionHint` | 能力/scope 缺失提示 | 不自行改配置 |

---

## 6. 交互设计

### 6.1 工具卡片

| 区域 | 内容 |
|---|---|
| 图标 | 40×40 圆角块，居中 emoji 或内联 SVG；无图标时用类别首字母 |
| 名称 | 单行省略 |
| 描述 | 最多 2 行省略（`line-clamp: 2`） |
| 徽标行 | 来源徽标（内置=蓝 / 用户=灰）+ 分类标签 + 版本（小字） |
| 状态徽标 | 见 §6.4 |
| 悬浮 | 卡片抬升 + 显示「打开」「详情」两个小按钮 |
| 点击 | 整卡点击 = 打开（静态壳弹出「静态壳：未接运行时」提示） |

### 6.2 分类与搜索

| 项 | 方案 |
|---|---|
| 分类 | 全部 / converter / text / dev / media / utility / custom；横向 chip，可单选 |
| 来源过滤 | 全部 / 内置 / 用户（下拉或 chip 组） |
| 搜索 | 名称 + 描述模糊匹配，忽略大小写；空结果给「没有匹配的工具」+ 清除 |
| 排序 | 内置优先 → 名称升序；或最近使用优先（静态壳可选） |
| 计数 | 工具栏右侧显示「共 N 个工具（内置 a / 用户 b）」 |
| 布局 | CSS Grid `repeat(auto-fill, minmax(180px, 1fr))` |

### 6.3 详情面板

| 字段 | 展示 |
|---|---|
| 头部 | 图标 + 名称 + 版本 + 来源徽标 |
| 描述 | 完整多行 |
| 路径 | 「内置资源：`tools/json-tool.html`」或「`/home/u/…/workspace/tools/x.html`」+ 复制路径按钮 |
| 分类 | chip |
| 声明能力 | chip 列表；无能力时显示「无（纯离线）」 |
| 元信息 | 大小 / 更新时间 |
| 操作 | 打开（主按钮）/ 禁用 / 卸载（仅 user）/ 在文件管理器中显示（仅 user） |
| 权限提示 | 若声明的能力未被宿主授予 → `ToolPermissionHint` 黄色提示条 |

### 6.4 状态机与状态徽标

| `ToolStatus` | 徽标 | 含义 | 卡片可点 |
|---|---|---|---|
| `available` | 无（正常） | 可加载 | ✅ |
| `offline-ready` | 灰「离线可用」 | 已验证零外链 | ✅ |
| `missing` | 红「文件缺失」 | 元数据在但 HTML 不在 | ❌（提示） |
| `load-failed` | 橙「加载失败」 | 上次打开失败 | ✅（重试） |
| `permission-denied` | 红「权限不足」 | 能力/scope 未授予 | ❌（显示提示） |
| `disabled` | 灰「已禁用」 | 用户手动禁用 | ❌ |

### 6.5 离线状态标识

每个卡片右上角一个小徽标：

| 标识 | 含义 |
|---|---|
| 🟢 绿点「离线可用」 | 静态校验通过：零外链、单文件自包含 |
| ⚪ 灰点「未校验」 | 尚未做离线校验（默认） |
| 🔴 红点「含外链」 | 检测到 `http(s)://` 引用 → **禁止加载**，详情面板给出外链清单 |

> 离线校验规则见 `M2-tools-seed-html-prework-20260902-1146.md` §6。静态壳阶段用 mock 字段 `offlineVerified: boolean` 模拟。

### 6.6 启动失败态

```
┌───────────────────────────────────────┐
│  ⚠️ 工具启动失败                        │
├───────────────────────────────────────┤
│  JSON 工具                             │
│  错误：资源未找到 (asset://tools/…)    │
│                                        │
│  可能原因：                             │
│   • 工具未随应用打包（include_dir!）    │
│   • 用户工具文件已被移动或删除          │
│                                        │
│  [ 重试 ]  [ 查看详情 ]  [ 关闭 ]       │
└───────────────────────────────────────┘
```

**禁止**：失败时静默关闭、只 `console.error` 不展示、或弹出原始 Rust 错误堆栈给用户。

### 6.7 权限提示

```
┌────────────────────────────────────────────────────┐
│  🔒 此工具需要额外权限                               │
│  需要：clipboard:write                              │
│  当前未授予。启用后工具才能使用复制功能。             │
│                              [ 暂不 ]  [ 去设置 ]   │
└────────────────────────────────────────────────────┘
```

「去设置」跳转到 `SettingsPanel.vue`（已存在）。**权限不得在工具运行时静默授予**。

---

## 7. mock 数据设计（`src/mocks/tools.ts`）

```ts
import type { ToolMeta, ToolState, ToolStatus } from '../types/tool';

export const MOCK_TOOLS: ToolMeta[] = [
  // 5 个内置种子（真实实现后由 list_tools 返回）
  { id: 'cron-tool',      name: 'Cron 生成器', description: 'Cron 表达式生成、校验与未来执行时间推算',
    category: 'converter', version: '1.0.0', source: 'builtin', path: 'tools/cron-tool.html',
    icon: '⏰', capabilities: [], size: 24_512 },
  { id: 'regex-tool',     name: '正则工坊', description: '正则测试、高亮匹配与多语言转义转换',
    category: 'text', version: '1.0.0', source: 'builtin', path: 'tools/regex-tool.html',
    icon: '🔍', capabilities: [], size: 31_744 },
  { id: 'json-tool',      name: 'JSON 工具', description: '格式化 / 压缩 / 校验 / 键排序',
    category: 'text', version: '1.0.0', source: 'builtin', path: 'tools/json-tool.html',
    icon: '{}', capabilities: [], size: 18_432 },
  { id: 'base64-tool',    name: 'Base64 转换', description: '文本与 Base64 互转，UTF-8 安全',
    category: 'converter', version: '1.0.0', source: 'builtin', path: 'tools/base64-tool.html',
    icon: '🔤', capabilities: [], size: 12_288 },
  { id: 'timestamp-tool', name: '时间戳转换', description: '时间戳与日期双向转换，支持秒/毫秒与 UTC',
    category: 'converter', version: '1.0.0', source: 'builtin', path: 'tools/timestamp-tool.html',
    icon: '🕓', capabilities: [], size: 15_360 },

  // 用户工具（覆盖各种异常态）
  { id: 'u1', name: '我的二维码', description: '把文本转成二维码（作者：我）',
    category: 'utility', version: '0.2.1', source: 'user',
    path: '/home/u/.local/share/mvp-browser-os/workspace/tools/qr.html',
    icon: '▣', capabilities: ['clipboard:write'], size: 41_984, updatedAt: '2026-08-30T14:00:00+08:00' },

  { id: 'u2', name: '颜色拾取器', description: '从剪贴板读取颜色并展示色板',
    category: 'media', version: '0.1.0', source: 'user',
    path: '/home/u/.local/share/mvp-browser-os/workspace/tools/color.html',
    icon: '🎨', capabilities: ['clipboard:read'], size: 9_216, updatedAt: '2026-08-25T10:00:00+08:00' },

  // ⚠️ 以下 4 个专门用来验收异常态
  { id: 'u3', name: '含外链的工具', description: '演示：检测到外部 CDN 引用，禁止加载',
    category: 'utility', version: '1.0.0', source: 'user',
    path: '/home/u/.../tools/cdn.html', icon: '🌐', capabilities: [], size: 3_072 },   // → 红点「含外链」

  { id: 'u4', name: '已丢失的工具', description: '演示：元数据在但文件已被删除',
    category: 'utility', version: '1.0.0', source: 'user',
    path: '/home/u/.../tools/gone.html', icon: '❓', capabilities: [], size: 0 },        // → missing

  { id: 'u5', name: '需要额外权限的工具', description: '演示：声明了未授予的能力',
    category: 'dev', version: '1.0.0', source: 'user',
    path: '/home/u/.../tools/needs-perm.html', icon: '🔒', capabilities: ['shell:execute'] }, // → permission-denied

  { id: 'u6', name: '上次加载失败的工具', description: '演示：可重试的失败态',
    category: 'dev', version: '1.0.0', source: 'user',
    path: '/home/u/.../tools/broken.html', icon: '⚠️', capabilities: [], size: 6_144 }, // → load-failed
];

export const MOCK_TOOL_STATES: Record<string, ToolState> = {
  'u3': { status: 'permission-denied', error: '检测到外部引用：https://cdn.jsdelivr.net/...' },
  'u4': { status: 'missing',           error: '文件不存在：/home/u/.../tools/gone.html' },
  'u5': { status: 'permission-denied', error: '能力未授予：shell:execute' },
  'u6': { status: 'load-failed',       error: '资源未找到 (asset://tools/broken.html)' },
};

export const MOCK_TOOL_EMPTY: ToolMeta[] = [];
```

---

## 8. 错误态 / 空态矩阵

| 场景 | 表现 | 禁止 |
|---|---|---|
| 完全无工具 | 空态 + 「浏览文档了解如何添加工具」+ 「打开工具目录」 | ❌ 空白 |
| 过滤无结果 | 「没有匹配的工具」+ 清除过滤 | ❌ 与真空态混淆 |
| 用户工具目录不可读 | 黄色提示条「无法读取 `workspace/tools/`（权限或不存在）」+ 其余工具照常显示 | ❌ 整个面板报错 |
| 工具文件缺失 | 卡片红徽标 + 详情给出路径 | ❌ 仍可点「打开」 |
| 含外链 | 红点「含外链」+ **禁止加载** + 详情列出外链清单 | ❌ 放行加载 |
| 权限不足 | 红徽标 + `ToolPermissionHint` + 「去设置」 | ❌ 静默授予 |
| 启动失败 | 失败弹窗（§6.6）+ 重试 | ❌ 静默关闭 / ❌ 原始堆栈 |
| 重复点击打开 | `launching` 期间按钮禁用 | ❌ 打开多个实例 |
| 工具名超长 | 单行省略 + `title` 悬浮 | ❌ 撑破布局 |

---

## 9. 验收截图点（人工 GUI 必拍）

| # | 场景 | 判据 |
|---|---|---|
| S1 | 工具箱首屏（11 个 mock 工具） | 网格整齐、徽标正确 |
| S2 | 空态 | 有引导 |
| S3 | 过滤无结果 | 文案区分 |
| S4 | 分类切换（converter/text/dev/media/utility） | 过滤正确 + 计数更新 |
| S5 | 搜索「json」 | 命中 json-tool |
| S6 | 内置/来源过滤 | 内置 5 / 用户 6 |
| S7 | 详情面板（内置工具） | 路径显示 `tools/json-tool.html` |
| S8 | 详情面板（用户工具） | 绝对路径 + 大小 + 更新时间 |
| S9 | 含外链工具（u3） | 红点 + 禁止打开 |
| S10 | 文件缺失工具（u4） | 红徽标 + 路径提示 |
| S11 | 权限不足工具（u5） | 提示条 + 「去设置」 |
| S12 | 加载失败工具（u6） | 失败弹窗 + 重试按钮 |
| S13 | 离线徽标三态（绿/灰/红） | 同屏可见三态 |
| S14 | 超长工具名 | 省略不撑破 |
| S15 | 窄屏（<900 px） | 单/双栏自适应 |

---

## 10. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 高 | 工具 HTML 经 `withGlobalTauri=true` 的 webview 加载 → 可直接 `invoke` 59 个高危命令 | 加载侧必须独立 capability；工具内禁 `__TAURI__` → 勘误 E5 / `plugin-permission-taskcard` |
| 高 | 用户工具来自 `workspace/tools/`，可被任意替换 → 供应链式风险 | 默认「未校验」灰点；含外链直接禁载；详情给出来源路径 |
| 中 | `assetProtocol.scope` 未覆盖应用数据目录 → 打包后加载不到 | 需扩 scope 或走 `include_dir!` 自定义协议；见 `M2-7.b` §5.R3 |
| 中 | 工具内 `localStorage` 与宿主同源共享 | 键前缀 `mvp-tool:<tool-id>:`；后续考虑独立 partition |
| 中 | 静态壳与真实 `list_tools` 字段漂移 | `ToolMeta` 字段以 `M2-7.b` §4.3 为准，本卡与之同构 |
| 低 | 用户工具数量大（>100）导致枚举慢 | 静态壳不优化；真实实现加缓存 + 目录 mtime 判断 |

---

## 11. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 断网打开工具箱 | mock 全部可见（零外链） |
| R2 | 点击含外链工具（u3） | 拒绝加载并给出外链清单 |
| R3 | 点击文件缺失工具（u4） | 提示缺失，不进入加载 |
| R4 | 双击「打开」按钮 5 次 | 只启动 1 个实例 |
| R5 | 用户工具目录被删除 | 黄条提示，内置工具仍正常 |
| R6 | 工具名含 `<script>` | 文本转义，不执行 |
| R7 | 图标字段填 `https://…` | 视为非法，回落类别首字母（不外链） |
| R8 | 权限提示点「去设置」 | 跳转 `SettingsPanel` 并高亮对应项 |
| R9 | 主 JS 体积 | 构建后对照 baseline 505 KB |
| R10 | 静态壳 `invoke` 调用数 | 恒为 0 |

---

## 12. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 12.1 静态壳不得触碰真实加载运行时（K9）
grep -rn "invoke\|__TAURI__\|WebviewBuilder\|asset://" src/components/system/Tool*.vue src/stores/useToolStore.ts | wc -l
# 期望 0

# 12.2 禁 v-html（K8）
grep -rn "v-html\|innerHTML" src/components/system/Tool*.vue | wc -l
# 期望 0

# 12.3 图标不得外链
grep -rn "https\?://" src/mocks/tools.ts | grep -v "^\s*//" | wc -l
# 期望 0（u3 的说明文本需写在注释或字符串说明里，并显式标注为反例）

# 12.4 五种状态全覆盖
grep -n "available\|missing\|load-failed\|permission-denied\|disabled" src/types/tool.ts
# 期望全部命中

# 12.5 构建
npm run build && ls -lh dist/assets/*.js
# 对照 baseline
```

---

## 13. 失败动作

| 失败 | 动作 |
|---|---|
| 出现真实 `invoke`/webview 创建 | 回退到纯 mock；加载运行时是 `M2-5/M2-7` + `plugin-permission-taskcard` 的范围 |
| 含外链工具被放行 | 视为安全缺陷，立即修复 |
| 状态覆盖不全 | 补 mock；不得删除异常态 mock 让验收「好看」 |
| 体积超标 | 组件懒加载；不得放宽阈值 |

---

## 14. 推荐模型

`AI:BALANCED`（网格 + 搜索 + 详情 + 多状态，模式化工作）。
**人工 GUI 验收必做**（15 截图点 + 10 反向用例）。
真实加载运行时（子 webview / `asset:` 协议 / `include_dir!`）→ `AI:DEEP` + 安全评审。
