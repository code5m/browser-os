# Checkpoint · Lane A3 · M5-W17 Acceptance Closeout — 关闭 `HOME_NO_SECRET_PERSIST`

```text
LANE: A3
STATUS: PASS
SCOPE: src/stores/useHomeStore.ts (改), src/utils/homeUi.ts (改), scripts/check-home-store-logic.mjs (改)
DELIVERED: 应用（type=app）启动命令体**不再写入浏览器存储**——仅驻留内存（会话内有效）；
           localStorage 只保留非敏感主页元数据（url/dir 的名称·图标·目标）；
           旧数据中的 app 条目在载入时被剔除并随即从存储抹除；新增/编辑 app 时明确提示「仅本次会话有效」。
VERIFY: node scripts/check-home-store-logic.mjs → 通过 105，失败 0（exit 0，较上一版 +21 条回归断言）
        npm run build → ✓ built in 3.97s（exit 0）
        python3 scripts/measure-build-metrics.py → total=795391B largest_js=334208B over_500kb=False
        cargo_warnings=2（既有 grid_process.rs，未增加）fmt_clean=True
        git diff --check → clean
        IDE lint（两文件）→ 0 diagnostics
METRICS: dist.total_bytes=795391（largest_js=334208B，chunk_over_500kb=False，无阈值越界）；
         cargo_warnings=2（基线一致，未增加）；fmt_clean=True
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A3-M5-W17-closeout-persist-20260907-2136.patch
       （**唯一权威补丁**，已取代并删除上一版 Lane-A3-M5-W17-home-store-20260907-1653.patch，
         避免 A0 重复应用；前序状态层内容见 A3-M5-W17-home-store-20260907-1653.md）
RISKS: 见文末「诚实风险」
NO_PUSH: confirmed（未 commit、未 push；按 W17 共享验收 #6 只交补丁 + checkpoint）
```

---

## 1. 债务与根因

指挥板证据：`HOME_NO_SECRET_PERSIST` —— **app 执行体（命令体）仍由 `useHomeStore.ts` 存进浏览器存储**。

`type: "app"` 的 `target` 是**应用启动命令**（如 `myapp --token=… --user=…`），可能携带参数、令牌、本地路径。
改造前 `save()` 是 `JSON.stringify(shortcuts)` 全量落库，等于把可执行命令体与其中的潜在凭据长期留在 localStorage；
最近访问 `recents` 同样会持久化 app 的 `target`，形成第二处泄漏面。

## 2. 处置方案（为什么这样改）

| 方案 | 取舍 | 结论 |
|---|---|---|
| A. 全量继续落库 | 保留债务 | ✗ |
| B. 落库前对命令体做敏感串脱敏 | 脱敏规则无法保证完备（漏一个参数即泄漏），且会静默改写用户命令 | ✗ |
| C. 只持久化可执行名、丢弃参数 | 静默破坏带参快捷方式（用户无感知） | ✗ |
| **D. app 命令体不落库，仅会话内有效** | 彻底消除泄漏面；功能在会话内完整；以提示告知用户 | ✅ 采用 |

实现（纯逻辑层 `homeUi.ts` + store 接线）：
- `HOME_PERSISTED_TYPES = ["url","dir"]`（不含 app）。
- `isStorageSafe(s)`：`type !== "app"`；`toPersisted(list)` 泛型过滤（同时适用于 `HomeShortcut[]` 与 `HomeRecent[]`，保留 `at`）。
- `persistedJson(list)`：供回归断言直接检查落库载荷。
- `save()` / `saveRecents()`：`JSON.stringify(toPersisted(...))`。
- **迁移**：`load()` 先 `normalizeShortcuts` 再 `filter(isStorageSafe)`；若发现旧数据含 app 条目，**立即重写 localStorage** 把它们抹除（不只是不显示）；若剔除后为空则回落 `defaultShortcuts()`。`loadRecents()` 同理。
- **用户沟通**：新增/编辑 app 条目时 toast `HOME_APP_SESSION_ONLY_NOTICE`「已添加（仅本次会话有效）：应用启动命令不会写入浏览器存储。」——不含任何命令体/路径。

**保留**：有界（24/12 上限）、逐字段校验、确定性 id、稳定默认、面板三态、零敏感展示等上一版行为全部不变。

## 3. 新增回归断言（21 条，总计 105）

`HOME_PERSISTED_TYPES` 不含 app；`isStorageSafe` app=false / url·dir=true；`toPersisted` 剔除 app 且不改动原数组；
`persistedJson` **不含命令体 / 不含 `token=` / 不含 `ghp_` / 不含 `--user=root`**，同时保留 url·dir 非敏感元数据且整体零敏感；
最近访问同样剔除 app 且保留 `at`；迁移期（旧存储含 app → 剔除；仅含 app → 空并回落默认）；提示文案非空、零敏感、明示「会话」。

## 4. HOLD lane 检查（A5 / A6 / A7）

按 HOLD 规则**只检查是否有具体阻塞，不重复开发**：

- **A5（`src/components/home/`）**：`HomeShortcuts.vue` / `HomeShortcutEditor.vue` 确实渲染 `app` 类型入口，
  但 `npm run build` 通过、无编译阻塞，功能未破（app 在会话内照常可用）。
  → **判定：无阻塞，A5 保持 HOLD**。可选改进（非必要）：给 app 卡片加「仅本次会话有效」徽标；
  我已用 toast 覆盖告知，故不构成「已证明必要的 UI 调整」。是否做由 A0/A5 决定。
- **A6（`ActivityBar.vue` / `useLayoutStore.ts` / `check-client-navigation-logic.mjs`）**：本次未触碰，导航检查仍绿 → **无阻塞，保持 HOLD**。
- **A7（`MainArea.vue` / `StatusBar.vue` / `App.vue`）**：本次未触碰，shell fallback 已覆盖 → **无阻塞，保持 HOLD**。

（上述文件在工作树中的改动均来自 A5/A6/A7 自身此前的 W17 交付，我未修改。）

## 5. 边界与硬停止合规

- 无新增 Tauri 命令 / bridge / ACL / 权限 / 依赖；无后端改动；无组件样式改动。
- 未启动 M6 权限切片：无命令执行、插件调用、动态加载、网络监听、守护进程、模型调用、Agent/Skill 执行、MCP 运行时、图谱写/导出、后台 worker。
- 未触碰其他 lane 文件；补丁仅含 A3 三个文件。
- **未 commit、未 push**。

## 6. 诚实风险

1. **行为变化需用户感知**：app 快捷方式重启后不再存在（这是消除泄漏的直接代价）。已用 toast 告知，
   但长期看若产品希望保留 app 入口，需要另一套「非命令体」的安全标识方案（属 M6/产品决策，不在本波）。
2. **指标增量归因不纯**：dist total 由上次单 lane 采值 790,073B 变为 795,391B（+5,318B）。
   该增量**不能全部归因于 A3**——期间 A5/A6/A7 亦新增了 home/shell 组件。
   客观事实：`chunk_over_500kb=False`、cargo_warnings=2（基线一致）、fmt_clean=True，**未越阈值**。
   若 A0 集成后需要精确归因，建议以干净 worktree 分别采值。
3. **旧数据的 app 条目会被静默清除**（首次载入即重写存储）。这是刻意的迁移清理，用户失去这些入口；
   若需保留应改走安全标识方案（同风险 1）。
4. **A4 复核**：本波把「不落库」前移到 `toPersisted`，A4 的 `check-home-client-policy.py` 可直接对
   `save()`/`saveRecents()` 路径或 `persistedJson` 输出做静态断言；若其默认门禁需新增码位，属 A4 范围。
5. **仅逻辑层验证**：`check-home-store-logic.mjs` 走纯函数断言，未做真实浏览器 localStorage 端到端验证；
   建议 A8 手工验收时确认「新增 app → 重启 → app 不復现且存储中无命令体」。
