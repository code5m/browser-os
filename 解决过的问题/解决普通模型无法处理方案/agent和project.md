是，**作为“第 0 阶段”，先整理 `AGENTS.md` 和项目规则，是目前最正确的一步**。

但文件名和职责要稍微调整：

```text
AGENTS.md              ← Codex/编码模型如何工作
PROJECT-RULES.md       ← 项目产品与架构不变量
PLANS.md               ← 后续分阶段改造计划
```

不是：

```text
Agent.md
rule.md
```

Codex 正式识别的是大写、复数形式的 **`AGENTS.md`**。其他规则文件可以存在，但属于项目自己的约定，不是 Codex 必需的特殊文件。官方也建议先用 `AGENTS.md` 记录项目目标、编码规范、测试标准、上下文、行为约束和作用范围。

你当前已经有：

```text
PROJECT-RULES.md
```

因此**不要再新建一个重复的 `rule.md`**。否则两个规则文件以后很容易互相冲突。

---

# 最正确的先后顺序

## 第一步：整理 `PROJECT-RULES.md`

先定义：

> 这个项目究竟允许什么、禁止什么、哪些产品决定已经确认、哪些尚未决定。

它负责记录“项目事实和不变量”，不负责指导 Agent 每一步怎么操作。

当前可以写入的已确认规则：

```text
[APPROVED] 继续使用自定义无边框窗口，不恢复系统标题栏。

[APPROVED] 成功操作不显示 Toast 或浮动提示。

[APPROVED] 错误和状态只显示在状态栏或独立布局面板。

[APPROVED] HTML 浮层禁止覆盖原生浏览器 WebView 区域。

[APPROVED] 独立面板必须参与正常布局，打开面板时缩小 WebView，
           不得通过 position: fixed + z-index 覆盖网页。

[APPROVED] Vue 前端保留，后续改造成 WebView-aware Shell，
           不因为 WebView 问题直接换掉 Vue。

[APPROVED] 原生 WebView 的层级问题不能使用无限增大 z-index 解决。

[APPROVED] 原生 GUI 行为未经真实桌面验收，不得写为 PASS。
```

目前不能写成已经确认的：

```text
[PENDING] 关闭页签是否改为自动保存后直接关闭。

[PENDING] 是否增加“最近关闭的页签”和 Ctrl+Shift+T。

[PENDING] 原生 WebView 是继续一个页签一个，
          还是只为当前可见页面创建 1～4 个。

[PENDING] Vue Shell 是增量迁移还是整体重新编写。
```

这一点非常重要：**不要让模型把讨论中的建议误认为已经批准的需求。**

建议每条规则明确标记：

```text
[CURRENT]          当前真实实现
[APPROVED_TARGET]  已批准但尚未实现
[PENDING]          等待你决定
[DEPRECATED]       已废弃，不得恢复
```

例如：

```text
[CURRENT]
关闭页签仍使用 SessionCloseDialog 三选一确认框。

[APPROVED_TARGET]
尚未批准，不得写。

[PENDING]
关闭页签是否改为自动保存并直接关闭。

[DEPRECATED]
新建页签后显示“已新建页签”Toast。
```

这样普通模型就不会把“目标状态”和“当前状态”混为一谈。

---

# 第二步：建立根目录 `AGENTS.md`

`AGENTS.md` 不是写产品需求的地方，而是告诉所有编码模型：

- 开始前必须阅读什么。
- 哪些目录可以修改。
- 哪些目录属于危险区。
- 遇到什么情况必须停止。
- 必须运行哪些验证。
- 什么条件才算完成。
- 什么内容不得擅自决定。

官方建议让 `AGENTS.md` 保持紧凑，把最高价值规则放在前面，而不是把它写成几千行的项目百科。

你项目的根目录版本建议类似：

```md
# AGENTS.md

## 1. 开始前必读

任何任务开始前必须依次阅读：

1. `AGENTS.md`
2. `PROJECT-RULES.md`
3. 当前任务或阶段文件
4. 与任务直接相关的源码和测试

不得把 `[PENDING]` 决策作为已批准需求实施。

## 2. 项目架构摘要

本项目是：

- Vue + TypeScript 前端外壳
- Tauri + Rust 原生后端
- Linux GTK/WebKitGTK 原生子 WebView
- 自研 `tauri-plugin-browser-tabs`
- Debian `.deb` 发布

原生浏览器 WebView 不是普通 DOM 元素。
HTML `z-index` 不能保证覆盖原生子 WebView。

## 3. 普通模型安全区

通常允许修改：

- `src/components/**`
- `src/domain/**`
- `src/application/**`
- `src/stores/**`
- `src/styles/**`
- `tests/**`
- `scripts/check-*.mjs`
- 文档

## 4. 原生危险区

未经任务明确授权不得修改：

- `src-tauri/src/bridge.rs`
- `src-tauri/src/**/linux.rs`
- `src-tauri/capabilities/**`
- `tauri-browser-tabs/**`

这些目录涉及：

- GTK/WebKitGTK
- 原生窗口层级
- 逻辑/物理像素
- WebView 显隐和销毁
- Wayland/X11
- Tauri 权限

## 5. UI 硬性规则

- 禁止在浏览器区域上新增 HTML Modal、Toast、Popover。
- 禁止使用增大 `z-index` 解决原生 WebView 遮挡。
- 成功操作不得弹浮窗。
- 错误只进入状态栏或独立布局面板。
- 独立面板必须改变布局并触发 WebView 矩形同步。
- 保留自定义无边框窗口。
- 普通 UI 修改不得改变窗口拖动事件路径。

## 6. 原生调用边界

目标架构中：

- Vue 组件不得直接调用 Tauri `invoke`。
- Store 不得直接操作 GTK/WebView。
- 原生调用应通过统一 BrowserRuntime adapter。
- 在该迁移完成前，不得假装该边界已经完全存在。

## 7. 修改原则

- 先进行只读调查，再修改。
- 一次只处理一个明确任务。
- 不顺便大规模重构。
- 不修改任务范围外的文件。
- 不自行决定待定产品需求。
- 不删除无法证明无调用方的状态或接口。
- 优先增加回归测试，再修改实现。

## 8. 停止条件

出现以下情况必须停止并报告：

- 需要修改 GTK/WebKitGTK 原生实现。
- 需要改变 Tauri Command 参数契约。
- 需要改变 WebView 创建、隐藏、销毁时序。
- 需要决定一个 `[PENDING]` 产品需求。
- 测试失败且根因超出当前任务。
- 工作树出现不属于本任务的修改。
- 需要 commit、push、安装或部署但未被明确授权。

## 9. 最低验证

普通前端修改至少运行：

```bash
npm run build
bash scripts/pre-merge.sh
git diff --check
```

Rust 修改至少运行：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
git diff --check
```

安装包修改按任务要求运行：

```bash
npm run tauri build -- --bundles deb
```

## 10. 完成状态

只能使用以下结论：

- `CODE_PASS`：代码和自动检查通过。
- `PACKAGE_PASS`：安装包成功生成并检查。
- `GUI_PENDING`：真实桌面尚未验收。
- `GUI_PASS`：用户明确完成真实桌面验收。
- `BLOCKED`：存在阻断问题。

不得把编译成功写成 GUI PASS。
不得把源码修复写成安装版已经生效。
```

这份文件不需要详细解释每个历史 Bug，只记录模型每次都必须遵守的操作规则。

---

# 第三步：给原生目录增加局部 `AGENTS.md`

建议再增加：

```text
src-tauri/AGENTS.md
tauri-browser-tabs/AGENTS.md
```

根目录负责全项目规则，局部文件负责原生危险区。官方工作流也支持从仓库根目录开始读取适用于当前目录的 `AGENTS.md` 指令。

例如：

```md
# src-tauri/AGENTS.md

## 原生代码规则

该目录涉及 Tauri、Rust、GTK/WebKitGTK 和 Linux 窗口系统。

修改前必须：

1. 阅读根目录 `AGENTS.md`
2. 阅读 `PROJECT-RULES.md`
3. 确认当前任务明确授权修改原生代码
4. 说明涉及的 Tauri Command 和调用方
5. 说明失败及回滚行为

禁止：

- 在多个位置重复进行 scale factor 转换
- 根据前端 CSS 猜测物理像素
- 仅凭编译成功宣布窗口行为正常
- 在没有 revision 或时序说明的情况下增加异步场景同步
- 修改窗口拖动路径而不运行拖动回归检查
```

`tauri-browser-tabs/AGENTS.md` 再严格一些：

```md
# tauri-browser-tabs/AGENTS.md

该目录属于高级风险原生区域。

普通 UI、文案、业务状态任务不得修改本目录。

涉及以下行为必须进行真实桌面验收：

- WebView create/destroy
- show/hide
- move/resize
- size_allocate
- 主窗口与子窗口同步
- 弹窗期间 WebView 隐藏与恢复
- Wayland/X11
```

---

# 第四步：把关键规则变成脚本

只写 Markdown 还不够。

正确关系应该是：

```text
AGENTS.md          告诉模型必须怎么做
PROJECT-RULES.md   定义什么是正确
scripts/           自动阻止明显违规
测试               验证行为
人工验收           验证原生 GUI
```

OpenAI 的工程实践也强调，`AGENTS.md` 用于要求工作流，而确定性部分应交给脚本执行。

你最应该新增的是：

```text
scripts/check-architecture-boundaries.mjs
```

自动检查：

```text
src/components/** 不得导入 @tauri-apps/api
src/stores/** 不得直接 invoke(
App.vue 不得直接调用原生 WebView 命令
禁止重新出现 toast-pop
禁止浏览器区域出现 position: fixed 浮层
只有指定 adapter 能调用 sync_browser_scene
```

否则普通模型可能“读了规则”，但修改时仍不小心越界。

---

# 第五步：再建立 `PLANS.md`

规则确定后，再写迁移计划：

```text
Phase 0：规则、边界和诊断基线
Phase 1：成功提示和错误状态规范化
Phase 2：BrowserRuntime 与 MockRuntime
Phase 3：WebViewSafeShell 与 BrowserViewportAnchor
Phase 4：自动保存关闭页签
Phase 5：统一 syncScene 原生接口
Phase 6：清理旧弹窗和旧调用
Phase 7：安装包及真实桌面验收
```

每个阶段拆成小任务：

```text
目的
允许修改文件
禁止修改文件
不变量
验收命令
人工验收项
停止条件
回滚方式
```

把大改造拆成小阶段，可以减少 Agent 漂移，也方便人工逐步审查。官方工作流同样建议用较小的阶段文件明确目的、边界和验收条件。

---

# 哪些内容不要写进 `AGENTS.md`

不要放：

- 完整历史聊天记录。
- 每次命令输出。
- 所有历史提交。
- 长篇 Bug 调查过程。
- 临时失败信息。
- 尚未批准的方案。
- 重复复制整份 `PROJECT-RULES.md`。
- 几百条无优先级规则。

否则普通模型需要读太多文本，反而更容易漏掉真正关键的限制。

推荐分工：

| 文件 | 内容 |
|---|---|
| `AGENTS.md` | 每次工作都必须遵守的操作规则 |
| `PROJECT-RULES.md` | 产品、架构和安全不变量 |
| `PLANS.md` | 改造阶段和顺序 |
| `logs/checkpoints/` | 已发生的验收和历史证据 |
| `scripts/` | 可自动执行的约束 |
| 当前任务卡 | 本轮允许修改的范围 |

---

# 对你当前项目的最终建议

最正确的实施顺序是：

```text
1. 只读审计现有 PROJECT-RULES.md
2. 清理冲突、过时和重复规则
3. 标记 CURRENT / APPROVED_TARGET / PENDING / DEPRECATED
4. 创建根目录 AGENTS.md
5. 创建原生目录局部 AGENTS.md
6. 增加架构边界检查脚本
7. 创建 PLANS.md
8. 再让普通模型逐任务实施
```

所以答案不是简单的“先写两个 Markdown 就够了”，而是：

> **先建立 `AGENTS.md + PROJECT-RULES.md + 自动门禁`，这是把项目交给普通模型之前最正确的第一步。**

其中一定要使用：

```text
AGENTS.md
```

不要使用：

```text
Agent.md
```

并且继续使用已有的：

```text
PROJECT-RULES.md
```

不要再建立含义重复的：

```text
rule.md
```

第一轮最好只允许高级模型修改这些治理文件和检查脚本，**不修改业务源码、不重构 Vue、不修改 Rust**。这样先把边界定稳，再开始真正改造。