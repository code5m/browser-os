# 12 — Plug / Unplug Matrix（2⁴ = 16 组合）

命令：`node scripts/capability-demo.mjs matrix`（结果 deterministic）。

| 组合 | 结果 | 资源（聚合） | 拒绝原因 |
|---|---|---|---|
| ∅ framework-only | VALID | none | — |
| bookmark | VALID | CACHE | — |
| workspace | VALID | none | — |
| bookmark+workspace | VALID | CACHE | — |
| browser | VALID | WEBVIEW, CHILD_PROCESS | — |
| bookmark+browser | VALID | CACHE, WEBVIEW, CHILD_PROCESS | — |
| workspace+browser | VALID | WEBVIEW, CHILD_PROCESS | — |
| bookmark+workspace+browser | VALID | CACHE, WEBVIEW, CHILD_PROCESS | — |
| terminal | VALID | PTY, CHILD_PROCESS | — |
| bookmark+terminal | VALID | CACHE, PTY, CHILD_PROCESS | — |
| workspace+terminal | VALID | PTY, CHILD_PROCESS | — |
| bookmark+workspace+terminal | VALID | CACHE, PTY, CHILD_PROCESS | — |
| browser+terminal | VALID | WEBVIEW, CHILD_PROCESS, PTY | — |
| bookmark+browser+terminal | VALID | CACHE, WEBVIEW, CHILD_PROCESS, PTY | — |
| workspace+browser+terminal | VALID | WEBVIEW, CHILD_PROCESS, PTY | — |
| bookmark+workspace+browser+terminal | VALID | CACHE, WEBVIEW, CHILD_PROCESS, PTY | — |

**16/16 VALID，0 意外失败，0 意外通过。**

## 说明（诚实）

- 现有 4 个能力之间没有强制互斥/冲突，因此 16 个组合**全部合法**是真实结论，不是测试偷懒。
- 「缺席」的判据不是 UI 隐藏：缺席的能力不进 `RESOLVED`、不产出任何 `CONTRIBUTION`、
  其资源也不出现在聚合结果里（见表：没有 browser 的行就没有 WEBVIEW）。
- 若要看到 `EXPECTED_REJECT`，使用：
  `node scripts/capability-demo.mjs use invalid-missing-dependency`
  → `RESULT REJECTED / UNKNOWN_CAPABILITY`（启动前拒绝，不是崩溃）。
