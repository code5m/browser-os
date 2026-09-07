# A11 · BUG-HUNT 截断三路复跑 Checkpoint（B8 / B10 / B12）

```text
LANE: A11
STATUS: PASS_WITH_DEBT
SCOPE: logs/bug-hunt/A11-B8-B10-B12-recheck-20260907-2150.md ; logs/checkpoints/A11-BUG-HUNT-recheck-20260907-2150.md ; logs/checkpoints/Lane-A11-BUG-HUNT-recheck-20260907-2150.patch
DELIVERED: B8/B10/B12 逐条复跑 + 四分类证据矩阵 + 2 处原主张校正 + 归属建议
VERIFY: 静态取证（源码/配置/类型契约比对），21:50 时点；未跑真机
METRICS: N/A（不改产品代码、不触发构建）
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A11-BUG-HUNT-recheck-20260907-2150.patch
RISKS: 静态取证为主；B10 文件被其它 lane 改写；4 项 UNVERIFIED 仍 OPEN
NO_PUSH: confirmed
```

完整矩阵见：`logs/bug-hunt/A11-B8-B10-B12-recheck-20260907-2150.md`

## 结论速览

| 判定 | 条目 |
|---|---|
| ✅ CONFIRMED（仍是问题） | **B8-1** `DbValue` 大小写漂移（`types.ts:601-607` PascalCase vs `domain.rs:1093` `rename_all=snake_case`；通路已上线 `main.rs:1468` + `bridge.ts:378`）· **B8-2**（部分：`types.ts:679/701/704` camelCase vs `domain.rs:1971/2009/2012` snake_case）· **B12-02** 三源一致门禁仍缺（无 `check-command-set-consistency.py`）· **B12-01** `open_tool` 缺 ACL（注册+调用但 permissions 无条目）· **B10-b** `MainArea.vue:253` `v-else` 与 `:251` `FileEditor v-if` 错配（注释自陈「W17(A7) 兜底」）→ 疑似 A7 回归 |
| 🔄 STALE / 重分类 | **B12-03** collect 三命令（`src-tauri/injected/collect.js:120/150/167`）的 ACL 移除系**有意设计**（`remote-collect.toml` 注释 + `check_remote_invocation` 意图令牌补偿）；原引用路径 `src/injected/collect.js` 已失效 → 「未授权=bug」不成立，残留为运行时令牌通路待验 |
| ❌ UNSUPPORTED / 降级 | **B10-a** `bridge.rs:4311 pub main: ProcStat` 非 `Option` → 「缺 `main` 即 TypeError 白屏」不可达，降为 P3 防御性加固 · **B10-c** 抽查 `ActivityBar.vue:175`、`useWorkspaceStore.ts:634` 的 `JSON.parse` 均在 try/catch 内 |
| ⏸ UNVERIFIED（仍 OPEN） | B8「invoke 未 catch」· B8「并发响应覆盖」· B10「XSS / 异步兜底覆盖度」· 另 4 处 `JSON.parse`（`useSystemStore.ts:24` 等）未逐条核对 |

## 前置确认（指令要求）

`ActivityBar.vue` / `MainArea.vue` / `StatusBar.vue` 均被其它 lane 改写（` M`）→ **B10 结论基于在制代码**；`types.ts`/`domain.rs`/`bridge.ts`/`main.rs`/`default-commands.toml`/`collect.js` 未见本地改动。

## 归属建议

- B8-1/B8-2 → **A7**（已派，仍 OPEN；建议补 `DbValue`/`SkillDef`/`AgentDef` fixture）
- B12-02/B12-01 → **A9**（一致性门禁应同时覆盖 C−A：bridge.ts 有调用、main.rs 未注册的执行类命令）
- **B10-b → A7**（HOLD 规则下的「具体验收发现」，应重开 A7 修该回归）
- B10-a/B10-c → P3 加固，归 **A6**
- B12-03 残留（意图令牌）→ **A8 真机 + A9**
- UNVERIFIED 四项 → 建议单开「截断项专项复跑」

## 声明

零产品代码改动；未运行原生客户端（凡用户可见结论均标「结构推断，未真机验证」）；未 reset/clean 他人改动；未 commit、未 push。
