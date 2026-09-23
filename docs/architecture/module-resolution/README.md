# Runtime Resolution Governance（运行时解析治理）

> 本目录是 `mvp-browser-os-v3` 的**单一职责治理域**，与 `semantic-governance/`（State/Intent/Owner/Writer/SideEffect/Resource）和 `capability-registry/`（能力语义/边界）**并列、互不重复**。

## 起源：H01 BLOCKING FAIL

FINAL HUMAN ACCEPTANCE 的 H01（full profile 启动）被用户真实 GUI 判定 FAIL：
`npm run build` / `npm run check` 全 PASS，但 `npm run tauri dev` 时 WebView 弹出 Vite error overlay，
请求已删除的旧模块路径（`src/components/home/*`、`src/components/workspace/{VaultPanel,ToolBox}.vue`）→ ENOENT。

根因：**`components/*` → `capabilities/*/ui/` 的物理迁移发生在同一 dev 生命周期内，WebKitGTK 磁盘缓存保留了迁移前的旧模块**；
重载时 webview 执行旧模块 → 旧相对 import → 命中已删除路径。源码零旧路径引用、构建 PASS，但 dev 运行时缓存未失效
（`Cache-Control: no-cache` 不足以让 WebKitGTK 失效）。

这是一次「**L1 源码 PASS / L2 构建 PASS ≠ L3 dev 模块图 PASS ≠ L4 运行时 PASS**」的真实事故，暴露机器验收盲区。

## 单一语义升级

在既有治理模型上**新增两个治理对象**，不改变既有语义：

- **Module（模块）**：一个 governed logical module 只有一个 canonical implementation。
- **RuntimeBinding（运行时绑定）**：模块在运行时的加载绑定（capability contribution / 父组件相对 import）唯一指向 canonical，不得指向旧地址。

完整原则：

```
ONE MEANING
ONE OWNER
ONE WRITER
ONE IMPLEMENTATION
ONE RUNTIME BINDING
ONE RESOURCE LIFECYCLE
```

## 机器可消费资产（本目录）

- `module-identity.yaml` — canonical module identity（R10 真源）
- `module-migrations.yaml` — migration tombstone（R11 真源；R12 派生 TOMBSTONED 列表）

## 门禁（checker 消费上述 YAML）

| 规则 | 脚本 | 职责 | 分层 |
|---|---|---|---|
| **R10** Canonical Module Location | `scripts/check-canonical-module-location.mjs` | 每个 governed module 只能有一个 canonical implementation；旧地址不得再作为真实文件存在；src 内同 basename 只能有一份 | `npm run check`（静态、快速） |
| **R11** Stale Module Reference | `scripts/check-stale-module-reference.mjs` | 读取 tombstone，扫描可执行引用面（src/、scripts/、src-tauri/、根配置），任何 executable/runtime reference 不得再指向 tombstoned `from` | `npm run check`（静态、快速） |
| **R12** Fresh Dev Runtime | `scripts/check-dev-runtime-startup.mjs` | fresh vite dev 真实爬取模块图：VITE_ENOENT=0 / TRANSFORM_ERROR=0 / TOMBSTONED_MODULE_REQUEST=0 / no-store 生效 / 缺失模块 negative fixture 有区分力 | `npm run check:runtime`（拉起 vite 启动烟测） |

### 运行方式

```bash
# 静态门禁（含 R10/R11），CI / 无 GUI 可跑
npm run check

# 运行时门禁 R12（默认自起 fresh vite 于 1437 端口）
npm run check:runtime

# R12 也可对已有 dev server 跑（R12B 真实运行时证据）：
R12_BASE_URL=http://localhost:1421 npm run check:runtime
# 并解析真实 tauri dev 日志（L4 运行时层）：
TAURI_DEV_LOG=/tmp/tauri-dev-H01b.log R12_BASE_URL=http://localhost:1421 npm run check:runtime
```

### 区分力（self-test）

- `node scripts/check-canonical-module-location.mjs --self-test` → 注入「canonical 缺失 / 旧地址仍有第二实现 / 同 basename 两份」必须 FAIL，合法单实现必须 PASS。
- `node scripts/check-stale-module-reference.mjs --self-test` → 运行时生成引用 tombstoned 路径的 fixture 必须被抓到；当前真树命中必须为 0。
- R12 内置 `negative fixture`：人为请求不存在模块必须不被识别为合法模块（content-type 判定，规避 SPA fallback 误判）。

## 最小修复（H01 根因）

`vite.config.ts` 的 `server.headers` 由 `no-cache` 升级为 **`no-store`**：dev 模块永不持久化，每次从 dev server 取最新，
彻底消除「dev 模块被 webview 持久化缓存 → 迁移后旧模块仍被执行」的盲区。

**禁止**以下假修复（均已规避）：重新创建旧目录 / 建 compatibility wrapper / 给旧路径加 alias / 复制组件成第二实现 / 关闭 overlay / 清用户数据 / 降低 checker。

## 证据等级（诚实标注）

- **L1 SOURCE** = PASS（`grep` 旧路径字面量 = 0，R11 扫描可执行面 = 0）
- **L2 BUILD_GRAPH** = PASS（`npm run build`）
- **L3 DEV_MODULE_GRAPH** = PASS（R12A：vite dev module smoke，194 模块 0 失败）
- **L4 RUNTIME** = PASS（R12B：真实 tauri dev 日志 ENOENT=0 / TOMBSTONED_REQUEST=0）

机器门禁只能证明 L1–L3 与（可解析日志时的）L4；**最终 GUI 目视验收（H01_HUMAN_RETEST）仍由用户执行**，Agent 不代判。
