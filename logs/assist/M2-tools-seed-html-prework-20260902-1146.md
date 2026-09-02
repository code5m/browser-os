# M2-tools-seed-html-prework（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §4.3 **M2-6**（5 个内置种子工具文件）· 需求 **#2 小工具框架**
> 状态：⏸ **方案 / 任务卡，未编写任何 HTML，未接运行时，不宣称 PASS**
> 关联：勘误 **E1**（`errata-to-taskcards-20260902-1146.md`）

---

## 0. 前置结论（必须先接受）

`src-tauri/src/tools/` 目录**不存在**，5 个 HTML **全部为零**。本卡的任务是**从零落盘编写**，不是「补加载框架」。

| 项 | 实测（2026-09-02） |
|---|---|
| `ls src-tauri/src/tools` | `No such file or directory` |
| `find . -name "*tool*.html"`（排除 node_modules/target/dist/variants/tauri-browser-tabs） | 零命中 |
| `list_tools` 命令 | 不在 59 个白名单命令中 |
| 前端工具箱面板 | 22 个 `.vue` 中无 |

风格参照：仓库根 `variants/*.html`（15 个单文件自包含 HTML），**读 1~2 个即可**，不要全读。

---

## 1. 目标

产出 5 个**可直接离线运行**的单文件 HTML 种子工具，并使其可被后续的 `list_tools` + 打包嵌入 + 前端工具箱三步加载。本卡**不接真实运行时**（不写 `list_tools`、不改 `build.rs`、不加面板）。

---

## 2. 路径规划（唯一口径）

| 工具 | 文件名 | 路径 | 类别 `tool-category` |
|---|---|---|---|
| Cron 生成器 | `cron-tool.html` | `src-tauri/src/tools/cron-tool.html` | `converter` |
| 正则工坊 | `regex-tool.html` | `src-tauri/src/tools/regex-tool.html` | `text` |
| JSON 工具 | `json-tool.html` | `src-tauri/src/tools/json-tool.html` | `text` |
| Base64 | `base64-tool.html` | `src-tauri/src/tools/base64-tool.html` | `converter` |
| 时间戳 | `timestamp-tool.html` | `src-tauri/src/tools/timestamp-tool.html` | `converter` |

- 内置工具目录：**`src-tauri/src/tools/`**（随 `include_dir!` 打包进二进制）。
- 用户工具目录（后续，非本卡）：`~/…/mvp-browser-os/workspace/tools/`（运行期读盘）。
- ❌ 不得放到 `src/tools/`（前端源码目录，会被 vite 误处理）。
- ❌ 不得放 `public/`（不经 `include_dir!`，打包后路径不可控）。

---

## 3. 每个工具的功能边界（写死，避免实现时发散）

### 3.1 `cron-tool.html`

| 项 | 内容 |
|---|---|
| 输入 | 5 段或 6 段 cron 表达式（分 时 日 月 周 [年]）；可选起始时间 |
| 输出 | ① 各字段人类可读描述；② 未来 N 次（默认 5，可配 1~20）执行时间（本地时区，ISO + 可读两种） |
| 支持语法 | `*`、`*/n`、`a-b`、`a,b`、数字、`?`（周/日位）、`L`（仅日/周位，可选） |
| 不支持 | `@yearly` 等宏（可作为输入提示，不必解析）；秒级 6 段中的「年」仅展示不做推算 |
| 空态 | 首次打开显示示例 `*/5 * * * *` 已填入，下方有说明 |
| 错误态 | 字段越界 / 语法非法 → 该字段下方红字提示，不弹 `alert` |

### 3.2 `regex-tool.html`

| 项 | 内容 |
|---|---|
| 输入 | 正则表达式文本 + flags（`g i m s u y`）+ 待匹配文本 |
| 输出 | 匹配列表（含索引、分组）、高亮预览、替换预览 |
| 多语言转换 | 把正则在 **Java / Python / Rust / JS / IDEA / VSCode** 六种「字符串转义风格」间转换（Java 需 `\\`，Rust 用 raw string `r"…"`，Python 用 `r'…'`） |
| 不支持 | 复杂回溯保护（超时即中断并提示「可能存在灾难性回溯」） |
| 错误态 | 正则语法错误 → 显示引擎原始 message，**不抛未捕获异常** |
| 超时 | 匹配超过 1000 ms → 中断并提示（防卡死） |

### 3.3 `json-tool.html`

| 项 | 内容 |
|---|---|
| 输入 | 任意文本（JSON / JSON5 容忍度：允许尾逗号、注释？**不允许**，严格 JSON） |
| 操作 | 格式化（2 空格）/ 压缩 / 校验 / 键排序（递归、稳定）/ 转义-反转义 |
| 输出 | 结果 + 字节数 + 节点数 |
| 错误态 | 解析失败 → 显示 `JSON.parse` 的 message 并**定位行列号**（自己算：从 error message 的 position 反推） |
| 体积保护 | 输入 > 2 MB 时提示「过大，可能卡顿」，仍允许继续 |

### 3.4 `base64-tool.html`

| 项 | 内容 |
|---|---|
| 输入 | 文本 or Base64 串 |
| 输出 | 编码 / 解码；UTF-8 安全（`TextEncoder` + `btoa(String.fromCharCode(...bytes))`，分片避免栈溢出） |
| 变体 | Standard / URL-Safe（`+`→`-`，`/`→`_`，去 `=`）/ 保留 padding 开关 |
| 空态 | 左右两个 textarea，实时双向（带 150 ms 防抖） |
| 错误态 | 非法 Base64 → 内联红字，不清空已成功内容 |

### 3.5 `timestamp-tool.html`

| 项 | 内容 |
|---|---|
| 输入 | 时间戳（秒 / 毫秒自动识别：10 位=秒，13 位=毫秒）/ 日期时间字符串 |
| 输出 | 双向转换；本地时区 + UTC + ISO 8601 三种展示 |
| 实时时钟 | 顶部一个每秒自增的当前时间戳（可暂停） |
| 空态 | 默认填入当前时间 |
| 错误态 | `Invalid Date` → 内联提示 |

---

## 4. HTML 结构契约（5 个工具统一骨架）

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <!-- 三个 meta 是 list_tools 的元数据来源，缺一即视为非法工具 -->
  <meta name="tool-name" content="Cron 生成器">
  <meta name="tool-description" content="Cron 表达式生成、校验与未来执行时间推算">
  <meta name="tool-category" content="converter">
  <meta name="tool-version" content="1.0.0">
  <title>Cron 生成器</title>
  <style>/* 全部内联，见 §5 */</style>
</head>
<body>
  <header class="tool-head">
    <h1 class="tool-title">Cron 生成器</h1>
    <p class="tool-desc">…</p>
  </header>

  <main class="tool-body">
    <section class="pane pane-input">
      <label class="field">
        <span class="field-label">Cron 表达式</span>
        <input id="cron-input" class="field-control" type="text" aria-label="Cron 表达式" placeholder="*/5 * * * *">
        <span class="field-error" id="cron-error" role="alert" hidden></span>
      </label>
    </section>

    <section class="pane pane-output">
      <div class="output-head">
        <span>结果</span>
        <button type="button" data-action="copy">复制</button>
      </div>
      <pre class="output" id="output" aria-live="polite"></pre>
    </section>
  </main>

  <footer class="tool-foot">
    <span class="offline-badge">离线运行 · 数据不出本机</span>
  </footer>

  <script>/* 全部内联，见 §5 */</script>
</body>
</html>
```

**强制约定**：

| 约定 | 要求 |
|---|---|
| `<html lang>` | 必须 `zh-CN` |
| 4 个 `<meta name="tool-*">` | 必须齐全（`tool-name` / `tool-description` / `tool-category` / `tool-version`） |
| DOM 根类 | `.tool-head` / `.tool-body` / `.tool-foot` 三层 |
| 输入容器 | `.pane.pane-input`；每个字段包在 `.field` 内，含 `.field-label` + `.field-control` + `.field-error` |
| 输出容器 | `.pane.pane-output` > `.output-head` + `pre.output` |
| 错误展示 | 统一用 `.field-error`（`hidden` 属性控制显隐），**禁止 `alert` / `confirm` / `prompt`** |
| 复制按钮 | `data-action="copy"`，用 `navigator.clipboard.writeText`，失败降级 `document.execCommand` 并提示 |
| 无障碍 | 每个输入控件必须有 `<label for>` 或 `aria-label`；主操作可键盘触发 |

---

## 5. CSS / JS 约束

### 5.1 CSS

| 约束 | 要求 |
|---|---|
| 位置 | 全部内联 `<style>`，**零外部样式表、零 `@import`、零 `url(http…)`** |
| 字体 | 仅系统字体栈（`system-ui, -apple-system, "Noto Sans CJK SC", sans-serif`），**零网络字体** |
| 变量 | 统一在 `:root` 定义 `--mvp-bg / --mvp-fg / --mvp-accent / --mvp-border / --mvp-error / --mvp-muted` |
| 主题 | 默认深色（与主应用 `#1e1e1e` 基调一致）；**可选**支持 `prefers-color-scheme: light` |
| 布局 | 桌面双栏（≥900 px）、窄屏单栏；用 CSS Grid，不用 float |
| 单位 | 禁止 px 硬编码字号（用 rem），便于宿主缩放 |
| 图标 | 只用 Unicode 字符或内联 SVG，**零图标字体 / 零外链** |

### 5.2 JS

| 约束 | 要求 |
|---|---|
| 位置 | 单个内联 `<script>`，放 `</body>` 前 |
| 语言 | 纯 ES2020，**零框架、零构建、零 polyfill**（WebKitGTK 2.4x 起支持） |
| 严格模式 | 首行 `'use strict';`，整体包在 IIFE 内，**零全局变量** |
| 外部通信 | **零 `fetch` / `XMLHttpRequest` / `WebSocket` / `importScripts`** |
| Tauri 调用 | ❌ **禁止** `window.__TAURI__`（见勘误 E5）；后续如需宿主能力走宿主注入的 `postMessage` 桥，本卡不涉及 |
| 全局兜底 | 顶部注册 `window.addEventListener('error', …)` 与 `'unhandledrejection'`，把错误渲染进 `.field-error`，**绝不白屏** |
| 持久化 | 仅 `localStorage`，键名前缀 `mvp-tool:<tool-id>:`；`try/catch` 包裹（隐私模式可能抛错） |
| 防抖 | 输入类工具 150 ms 防抖；cron/时间戳的实时时钟用 `setInterval` 1 s，页面 `visibilitychange` 时暂停 |
| 复制 | `navigator.clipboard` + 降级，失败给可读提示 |

---

## 6. 离线资源红线

| 红线 | 校验方式 |
|---|---|
| 零 `http(s)://` 外链 | `grep -rn "https\?://" src-tauri/src/tools/*.html` 排除注释后应 **0 命中** |
| 零外部 `<script src>` / `<link href>` | 同上 |
| 零 `url(http…)` / `@import url(...)` | 同上 |
| 零网络字体 | `grep -rn "@font-face\|fonts.googleapis" …` 应 0 命中 |
| 单文件 ≤ 100 KB | `wc -c` |
| 5 个合计 ≤ 300 KB | `wc -c` 求和 |

> 唯一的例外：注释里可以出现「参考 https://…」的说明性 URL，但必须在同一行带 `<!--` 注释标记，校验时用 `grep -v '<!--'` 过滤。

---

## 7. 输入输出契约（通用）

| 项 | 约定 |
|---|---|
| 输入来源 | 仅两种：用户键入、粘贴。**不读剪贴板自动填充**（隐私红线） |
| 输出去向 | 两种：页面内 `pre.output` 展示、用户主动「复制」。**不自动写剪贴板** |
| 数据出本机 | ❌ 绝对不允许（无网络栈） |
| 状态持久化 | 仅 `localStorage`，键前缀 `mvp-tool:<tool-id>:` |
| 副作用 | 零（不改文件系统、不启动进程、不开新窗口） |

### 7.1 统一错误对象（内部约定）

```js
/** @typedef {{ok:true, value:string} | {ok:false, error:string}} ToolResult */
```

每个工具的核心函数统一返回 `ToolResult`，由统一渲染函数 `render(result)` 决定展示成功/失败。这样：

- 错误态样式一致；
- 新增工具无需重写渲染逻辑；
- 便于后续加单测（纯函数）。

---

## 8. 错误态 / 空态矩阵

| 场景 | 表现 | 反面（禁止） |
|---|---|---|
| 首次打开（空态） | 输入框有示例值或占位符；输出区显示灰色引导文案 | ❌ 空白 `pre` + 无提示 |
| 输入为空 | 「请输入内容」内联提示，输出区保持上次成功结果 | ❌ 清空输出 / 报错 |
| 输入非法 | 对应字段 `.field-error` 红字 + `aria-invalid="true"` | ❌ `alert` / ❌ 白屏 / ❌ 未捕获异常 |
| 输入超大（>2 MB） | 顶部黄条提示「内容较大，处理可能较慢」 | ❌ 直接卡死 |
| 正则灾难性回溯 | 1000 ms 超时中断 + 提示 | ❌ 页面冻结 |
| `localStorage` 不可用 | 静默降级为「不持久化」，功能照常 | ❌ 抛异常中断 |
| 剪贴板权限拒绝 | 「复制失败，请手动选择」 | ❌ 静默无反馈 |

---

## 9. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| 高 | 工具 HTML 落在授予 `default-commands` 的 webview 内 → 可直接 `invoke` 59 个高危命令 | 本卡**硬性禁止** `__TAURI__`；加载侧用独立 capability，见 `plugin-permission-taskcard` |
| 中 | `include_dir!` 未落地前，工具无法被打包加载 | 本卡只写文件；加载另开任务（M2-5/M2-7） |
| 中 | 5 个工具 CSS 风格漂移 | 统一 `:root` 变量 + 骨架类名（§4/§5.1） |
| 中 | cron「未来执行时间」时区/夏令时边界 | 明确只用本地时区；DST 不做特殊处理，但要在说明中标注 |
| 低 | 体积超 100 KB | 5 个工具逻辑都不复杂；regex 工坊最可能超，需控制语言转换表体积 |

---

## 10. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 断网（拔网线 / `nmcli radio wifi off`）后打开 5 个工具 | 全部功能可用 |
| R2 | `regex-tool` 输入 `(` | 显示语法错误，不崩溃 |
| R3 | `regex-tool` 输入 `(a+)+$` + 长文本 `aaaa…a!` | 1000 ms 内中断并提示，页面不冻结 |
| R4 | `json-tool` 输入 `{a:1}` | 报错并给出行列位置 |
| R5 | `base64-tool` 输入含中文文本 → 编码 → 解码 | 往返一致（UTF-8 安全） |
| R6 | `base64-tool` 输入 `!!!not-base64!!!` | 内联错误提示 |
| R7 | `timestamp-tool` 输入 `0` | 1970-01-01（本地时区） |
| R8 | `cron-tool` 输入 `*/5 * * *` | 提示「应为 5 或 6 段」 |
| R9 | `cron-tool` 输入 `60 * * * *` | 提示分钟越界 |
| R10 | 清空 `localStorage` 后重开 | 恢复默认态 |

---

## 11. 验收命令（**均未执行**，实现后必跑）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 11.1 文件齐备
ls -1 src-tauri/src/tools/*.html | wc -l                 # 期望 5

# 11.2 离线红线（排除注释行）
grep -rn "https\?://" src-tauri/src/tools/*.html | grep -v "<!--" | wc -l   # 期望 0
grep -rn "@font-face\|fonts.googleapis" src-tauri/src/tools/*.html | wc -l   # 期望 0
grep -rn "__TAURI__\|fetch(\|XMLHttpRequest\|WebSocket" src-tauri/src/tools/*.html | wc -l  # 期望 0

# 11.3 体积
for f in src-tauri/src/tools/*.html; do
  wc -c "$f" | awk -v F="$f" '$1>102400{print "TOO-BIG: "F" "$1}'
done                                                     # 期望无输出
cat src-tauri/src/tools/*.html | wc -c                    # 期望 <= 307200

# 11.4 元数据齐全
for f in src-tauri/src/tools/*.html; do
  for m in tool-name tool-description tool-category tool-version; do
    grep -q "name=\"$m\"" "$f" || echo "MISSING-META: $f $m"
  done
done                                                     # 期望无输出

# 11.5 禁用 alert
grep -rn "alert(\|confirm(\|prompt(" src-tauri/src/tools/*.html | wc -l   # 期望 0

# 11.6 无障碍最小项
for f in src-tauri/src/tools/*.html; do
  grep -q "aria-label\|<label" "$f" || echo "NO-LABEL: $f"
done                                                     # 期望无输出

# 11.7 离线实跑（人工）
# 在浏览器直接 file:// 打开每个 HTML，断网执行 §10 的 R1~R10
```

---

## 12. 失败动作

| 失败 | 动作 |
|---|---|
| 工具数量不足 5 | 停止，报告缺哪个；**禁止用占位/空壳 HTML 凑数** |
| 离线红线不通过 | 定位外链来源改为内联；不得放宽规则 |
| 体积超限 | 精简内联资源；regex 的语言转换表改为按需生成 |
| `alert` 未清零 | 全部改为 `.field-error` 内联提示 |
| 某工具逻辑错误（如 cron 推算偏差） | 该工具单独回炉，**不影响其它 4 个的验收** |

---

## 13. 推荐模型

| 工具 | 推荐模型 | 理由 |
|---|---|---|
| `json-tool.html` | `AI:FAST` | 逻辑最简单 |
| `base64-tool.html` | `AI:FAST` | 逻辑简单，注意 UTF-8 分片 |
| `timestamp-tool.html` | `AI:FAST` | 逻辑简单 |
| `cron-tool.html` | `AI:DEEP` | 边界多（6 段/越界/DST/未来推算） |
| `regex-tool.html` | `AI:DEEP` | 六种语言转义 + 灾难性回溯保护 |

**人工 GUI 验收**：5 个工具全部必须（离线实跑 + 反向用例 R1~R10）。
