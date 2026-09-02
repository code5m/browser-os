# plugin-permission-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §7.3 **M5-10~M5-12**（需求 #15 插件系统）**+** §4.3 **M2-5/M2-6**（需求 #2 小工具框架）· 跨里程碑
> 状态：⏸ **任务卡 / 未改配置 / 未执行验收 / 不宣称 PASS**
> 关联：勘误 **E5** · `M5-15.a-prework-20260902-1055.md` · `M2-7.b-prework-20260902-1055.md` §5.R3

---

## 1. 目标

针对 `withGlobalTauri = true` 输出**插件隔离整改卡**：把「HTML 工具 / 插件可以调用任意命令」的现状，整改成「最小能力 + 显式声明 + 可审计」。

🚨 **本卡第一优先级是可行性判定**：若 Tauri v2 无法按 webview 粒度关闭全局 Tauri，则**形态②插件（webview 内运行）必须放弃**，改为形态①（独立进程 / stdio）。

---

## 2. 现状证据（2026-09-02 实测）

### 2.1 全局 Tauri 已开启

```json
// src-tauri/tauri.conf.json
{
  "app": {
    "withGlobalTauri": true,
    "windows": [],
    "security": {
      "csp": null,
      "assetProtocol": {
        "enable": true,
        "scope": ["/usr/share/**", "/usr/local/share/**", "$HOME/.local/share/**", "$HOME/.icons/**"]
      }
    }
  }
}
```

### 2.2 能力授予面过宽

```json
// src-tauri/capabilities/default.json
{
  "identifier": "default",
  "description": "默认能力：主窗口 + 浏览器子窗口均可调桥命令与终端",
  "windows": ["main", "browser"],
  "permissions": [
    "core:default",
    "core:window:allow-create",
    "browser-tabs:default",
    "default-commands",
    { "identifier": "shell:allow-spawn",
      "allow": [
        { "name": "bash", "cmd": "bash", "args": true },
        { "name": "sh",   "cmd": "sh",   "args": true },
        { "name": "powershell", "cmd": "powershell", "args": true }
      ] },
    "shell:allow-stdin-write",
    "shell:allow-kill"
  ]
}
```

```json
// src-tauri/capabilities/browser-remote.json
{
  "identifier": "browser-remote",
  "windows": ["main", "grid-child-*"],
  "webviews": ["browser", "tab-*", "grid-*"],
  "remote": { "urls": ["https://*", "http://*"] },
  "permissions": ["core:default", "remote-collect"]
}
```

### 2.3 高危命令暴露面

`src-tauri/permissions/default-commands.toml` 共 **59 个命令**，`default.json` 的 `windows: ["main","browser"]` 意味着主窗口与浏览器窗口都能调：

| 高危命令 | 能力 |
|---|---|
| `write_file` / `create_file` / `delete_path` / `rename_path` | 任意文件写/删/改名 |
| `launch_app` | 启动任意应用 |
| `eval_in_tab` | 在页签内执行 JS |
| `configure_repo` / `request_sync` / `confirm_sync` | 触发 git 同步（可能出网） |
| `term_spawn` / `term_write` | 任意命令执行（等价于 shell） |
| `shell:allow-spawn`（`args: true`） | **ACL 不校验参数**，等于放行任意 bash |

### 2.4 前端是否依赖全局 Tauri（待查证，实现时必须先查）

```bash
grep -rn "__TAURI__" src/ src-tauri/src/ | wc -l
```

> 本批次**已把该命令列入验收清单但尚未执行**，实现前必须先跑（见 §9）。

### 2.5 供应链面

- 用户工具来自 `workspace/tools/`（用户可自行替换）→ 若其 webview 继承 `default` capability，等于任意本地 HTML 都能调 59 个命令。

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| `src-tauri/tauri.conf.json` | ① `withGlobalTauri` → `false` ② `csp` 显式设置 ③ `assetProtocol.scope` 收窄并按需扩 | 必须（若判定可关） |
| `src-tauri/capabilities/default.json` | ① `windows` 精确到 `main` ② 移除 `shell:allow-spawn` 的宽泛 `args: true` 或拆出 ③ 拆高危命令到独立 capability | 必须 |
| **新增** `src-tauri/capabilities/tools.json` | 工具 webview 专用最小 capability（仅 `tools:*` 命令） | 必须 |
| **新增** `src-tauri/capabilities/plugin-*.json` | 每类插件一份最小 capability | 必须（形态②可行时） |
| `src-tauri/permissions/default-commands.toml` | 拆分为 `safe-commands` + `privileged-commands` | 必须 |
| **新增** `src-tauri/permissions/tools-commands.toml` | 工具可用命令白名单 | 必须 |
| `src-tauri/capabilities/browser-remote.json` | `remote.urls` 由 `*` 收窄 | 必须 |
| `src/**` | 全部改用 `import { invoke } from "@tauri-apps/api/core"` | 必须（若关全局） |
| **新增** `src-tauri/src/audit.rs` 或扩展 `workspace.rs` | 插件/工具权限拒绝与调用的审计 | 建议 |

---

## 4. 契约 / 配置设计

### 4.1 目标配置形态

```jsonc
// src-tauri/tauri.conf.json（目标）
{
  "app": {
    "withGlobalTauri": false,        // ← 关闭全局注入
    "windows": [],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset: http://asset.localhost; font-src 'self'; connect-src 'self' ipc: http://ipc.localhost; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
      "assetProtocol": {
        "enable": true,
        "scope": [
          "$APPDATA/**",                     // 内置工具（include_dir! 落地后）
          "$HOME/.local/share/com.jizhijiandan.mvp/mvp-browser-os/workspace/tools/**",
          "$HOME/.icons/**"
        ]
      },
      "freezePrototype": true,
      "dangerousDisableAssetCspModification": false
    }
  }
}
```

> ⚠️ `style-src 'unsafe-inline'` 是因为种子工具内联 `<style>`。**更好的做法**是给每个工具内联样式加 nonce/hash；若 CSP 改造成本过高，可先接受 `style-src 'unsafe-inline'`，但 **`script-src` 绝不加 `'unsafe-inline'` / `'unsafe-eval'`**。

### 4.2 capability 拆分

```
capabilities/
├── default.json          → windows: ["main"]；safe-commands + core:default
├── browser-remote.json   → webviews: ["browser","tab-*","grid-*"]；remote.urls 收窄
├── tools.json            → webviews: ["tool-*"]；仅 tools-commands
└── plugin-<id>.json      → webviews: ["plugin-<id>"]；按 manifest 声明生成
```

```jsonc
// capabilities/tools.json（草案）
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "tools",
  "description": "工具箱 webview 的最小能力：仅允许工具自省与剪贴板",
  "webviews": ["tool-*"],
  "permissions": [
    "core:default",
    {
      "identifier": "tools-commands",
      "allow": ["tool_self_info", "clipboard_write"]
    }
  ]
}
```

> ❗ 工具 webview **不得**授予 `default-commands`。工具若需要宿主能力，走宿主注入的受控 `postMessage` 桥（本卡不设计桥协议，属 `M2-5/M2-7` 范围）。

### 4.3 权限分级（沿用 `M5-15.a` §5）

| 级别 | 含义 | 示例 | 授予方式 |
|---|---|---|---|
| `none` | 纯离线，零宿主能力 | 5 个种子工具 | 自动 |
| `read` | 只读宿主数据 | 读取 artifacts | 安装时提示 |
| `write` | 可写宿主数据 | 保存笔记 | 安装时二次确认 |
| `exec` | 可触发脚本/进程 | 调用脚本库 | **默认拒绝**，需用户显式开启 + 审计 |
| `net` | 可出网 | 调用外部 API | **默认拒绝**，需白名单域名 |

**fail-closed 判定**（与 `M5-15.a` 一致）：manifest 声明的能力若未被显式授予 → **拒绝执行**而非降级放行。

### 4.4 错误页

插件/工具加载或权限被拒时，展示统一错误页（由宿主渲染，**不是**插件自己渲染）：

```
┌──────────────────────────────────────────┐
│  🔒 已阻止                                │
├──────────────────────────────────────────┤
│  插件「xxx」请求了未授予的能力：            │
│    • shell:execute                        │
│                                           │
│  该请求已被拒绝（fail-closed）。            │
│                                           │
│  [ 查看权限设置 ]  [ 卸载插件 ]  [ 关闭 ]  │
└──────────────────────────────────────────┘
```

| 字段 | 内容 |
|---|---|
| 标题 | 🔒 已阻止 |
| 插件名 + 版本 | — |
| 被拒能力列表 | 逐条 |
| 时间 | ISO 8601 |
| 操作 | 查看权限设置 / 卸载 / 关闭 |

### 4.5 审计

| 事件 | 是否记 audit | 备注 |
|---|---|---|
| 插件安装 / 启用 / 禁用 / 卸载 | ✅ | 动作级 |
| 权限被拒（fail-closed 触发） | ✅ | 含插件 id + 能力名 |
| 插件每次调用宿主命令 | ⚠️ 摘要 | 高频 → 独立 `plugin-audit.json`，摘要进 `audit.json`（**K5**：1000 条上限） |
| 工具启动 / 失败 | ✅ | 动作级 |
| 工具输出内容 | ❌ | 不记内容（隐私） |

---

## 5. 实现要点（步骤化）

### 步骤 0：**可行性判定（阻塞项，必须先做）**

| 问题 | 查证方式 | 结论影响 |
|---|---|---|
| Tauri v2 是否支持按 webview 粒度关闭 `withGlobalTauri`？ | 查官方文档 / `tauri.conf.json` schema | 若**不能** → 形态②插件 **BLOCKED**，转形态① |
| `capabilities` 能否按 webview label 精确授予？ | 已有 `browser-remote.json` 用 `webviews: ["browser","tab-*"]` → ✅ 可行 | 工具/插件可各自独立 capability |
| `asset:` 协议 scope 是否支持 `$APPDATA` 变量？ | 查 `assetProtocol.scope` 文档 | 决定内置工具加载方式 |
| CSP 是否会破坏现有主窗口功能？ | 先设宽松 CSP 再逐步收紧 | 逐步硬化 |

**判定结论必须写回本文档 §5.0 并同步到汇总**，不得含糊。

### 步骤 1：清点全局 Tauri 依赖

```bash
grep -rn "__TAURI__" src/ src-tauri/src/ | wc -l
```
把每处改为 `import { invoke, listen } from "@tauri-apps/api/core"`（`package.json` 已有 `@tauri-apps/api ^2.0.0`）。

### 步骤 2：关 `withGlobalTauri` + 设 CSP

先关全局 → 全量回归主窗口功能 → 再上 CSP（CSP 最容易出白屏，单独一步）。

### 步骤 3：拆 capabilities

按 §4.2 拆分；`default.json` 的 `windows` 从 `["main","browser"]` 收窄到 `["main"]`（浏览器窗口改为 `browser-remote.json` 授予其所需的最小集）。

### 步骤 4：收窄 `remote.urls`

`["https://*", "http://*"]` → 若业务上必须放行任意域（浏览器页签），则**更不能**让这些 webview 持有 `default-commands`；改为只授予 `remote-collect`。

### 步骤 5：`shell:allow-spawn` 收窄

把 `args: true` 的宽泛放行，改为**不授予**或改为固定脚本白名单（具体形态见 `script-execution-safety-taskcard`）。

### 步骤 6：审计接入 + 错误页

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ 在 `withGlobalTauri=true` 下实现形态②插件 | 权限模型形同虚设 |
| 2 | ❌ CSP 里加 `script-src 'unsafe-inline'` / `'unsafe-eval'` | 等于没 CSP |
| 3 | ❌ 给工具/插件 webview 授予 `default-commands` | 59 个高危命令全暴露 |
| 4 | ❌ 以「插件是本地文件所以可信」放行 | 用户工具目录可被替换（供应链） |
| 5 | ❌ 权限未授予时降级放行 | 必须 fail-closed |
| 6 | ❌ 把插件每次调用都写进 `audit.json` | 1000 条上限会被刷爆（K5） |
| 7 | ❌ 删除/修改现有 capability 前不做回归 | 静默拒绝极难排查（K1 同类坑） |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | `withGlobalTauri` 无法按 webview 关闭 → 形态②插件不可行 | 步骤 0 先判定；不可行则转形态①（独立进程 + stdio），并在汇总登记 BLOCKED |
| 高 | 关闭全局 Tauri 后主窗口大面积功能失效 | 步骤 1 先清点；逐处替换；**不得**因麻烦而回退配置 |
| 中 | CSP 导致白屏（尤其内联 `<style>` / `data:` 图片） | 逐步硬化：`default-src 'self'` 起步 → 加 `img-src data:` → 最后处理 style nonce |
| 中 | `asset:` scope 未覆盖 `workspace/tools/` → 用户工具加载不到 | 需显式加 scope 条目（§4.1） |
| 中 | 拆分 capability 后某些命令被静默拒绝 | **K1 同类坑**：ACL 拒绝无日志。实现后必须跑一遍全功能回归 + 检查浏览器控制台 |
| 低 | 59 个命令拆分工作量 | 建议按「只读 / 写 / 执行 / 系统」四类分，不必逐个 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 在浏览器页签打开 `https://example.com`，DevTools 执行 `window.__TAURI__` | `undefined` |
| R2 | 在工具 webview 中执行 `window.__TAURI__.invoke('delete_path', {...})` | 被拒 + 有日志 + 显示错误页 |
| R3 | 外部页面调 `write_file` | 被 ACL 拒绝，且**有可查的日志**（不是静默） |
| R4 | 插件 manifest 声明未授予的 `exec` 能力并调用 | fail-closed 拒绝 + 审计记录 + 错误页 |
| R5 | 关闭全局 Tauri 后跑一遍主窗口全部功能 | 全部正常（无遗漏的 `__TAURI__` 依赖） |
| R6 | CSP 生效后，在工具内 `eval("1+1")` | 被 `script-src 'self'` 拦截（**若**工具确实有内联 script，则需 nonce；但外部注入的 `eval` 必须被拦） |
| R7 | 工具 HTML 含外部 CDN `<script src="https://...">` | 被 CSP 拦截 + 工具被标「含外链」禁载 |
| R8 | 插件高频调用（1000 次/分钟） | `audit.json` 不爆（独立 JSON 承载明细） |
| R9 | 卸载插件 | 资源释放 + capability 移除 + 审计记录 |
| R10 | 用户把恶意 HTML 放进 `workspace/tools/` | 出现在工具箱，但**无任何宿主能力**（`tools.json` 最小集） |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.0 步骤 0：清点（实现前必须执行）
grep -rn "__TAURI__" src/ src-tauri/src/ | wc -l          # 记录基线
grep -rn "withGlobalTauri" node_modules/@tauri-apps/ 2>/dev/null | head -3

# 9.1 全局 Tauri 已关
grep -n "withGlobalTauri" src-tauri/tauri.conf.json        # 期望 false

# 9.2 前端零全局依赖
grep -rn "__TAURI__" src/ | wc -l                          # 期望 0

# 9.3 CSP 已设且不含 unsafe
grep -n "csp" src-tauri/tauri.conf.json                    # 期望非 null
grep -n "csp" src-tauri/tauri.conf.json | grep -c "unsafe-inline\|unsafe-eval"
# 期望：仅 style-src 可能含 unsafe-inline；script-src 绝不含

# 9.4 capability 已拆分
ls -1 src-tauri/capabilities/                              # 期望 >= 3 个
grep -n "\"windows\"" src-tauri/capabilities/default.json   # 期望仅 ["main"]

# 9.5 remote 收窄
grep -n "remote" -A 2 src-tauri/capabilities/browser-remote.json

# 9.6 工具 capability 最小集
grep -n "default-commands" src-tauri/capabilities/tools.json | wc -l   # 期望 0

# 9.7 编译
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 baseline 13 warning

# 9.8 人工 GUI（AI 不得代签）
#   R1: DevTools 执行 window.__TAURI__     → undefined
#   R5: 全功能回归
#   R7: 外链被 CSP 拦截
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 步骤 0 判定「无法按 webview 关闭全局 Tauri」 | **形态②插件标记 BLOCKED**，转形态①；在 `plugin-runtime-taskcard` 与汇总中显式登记，不得含糊放行 |
| 关全局后大面积功能失效 | 逐处补 `import`，**不得**回滚 `withGlobalTauri=false` |
| CSP 导致工具白屏 | 加 nonce/hash 白名单；**不得**加 `script-src 'unsafe-inline'` |
| 命令被静默拒绝查不到原因 | 补日志（临时开启 Tauri ACL 日志）+ 检查 capability 覆盖的 window/webview label 是否匹配 |
| `audit.json` 被刷爆 | 按 K5 拆独立 JSON |
| clippy warning 增加 | 对照 baseline 回退 |

---

## 11. 推荐模型

- 步骤 0 可行性判定 + 方案评审：`AI:DEEP-xhigh`
- 步骤 1~6 实施：`AI:DEEP`
- **人工 GUI 验收必做**：R1（DevTools）、R5（全功能回归）、R7（CSP 拦截）
