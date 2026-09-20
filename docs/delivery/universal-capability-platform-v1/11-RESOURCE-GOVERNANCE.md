# 11 — Resource Governance

## 资源归属（每个能力在 manifest 中声明 ownership）

| 能力 | owned 资源 | evidence |
|---|---|---|
| browser | WEBVIEW、CHILD_PROCESS | `scripts/measure-resources.mjs` |
| terminal | PTY、CHILD_PROCESS | `scripts/measure-resources.mjs` |
| bookmark | CACHE（磁盘持久化） | `src/capabilities/bookmark/state/useBookmarkStore.ts` |
| workspace | （无重资源，诚实声明为空） | — |

资源类型：`WEBVIEW / PTY / CHILD_PROCESS / DB_CONNECTION / WATCHER / BACKGROUND_TASK / SOCKET / CACHE`。

## ABSENT 证据（本次实测）

| 断言 | 结果 | 口径 |
|---|---|---|
| framework-only → 零注册、零贡献 | PASS | PLT2-11/11b（结构级） |
| Browser absent → 零 Browser 贡献（无 WebView 宿主注册） | PASS | PLT2-12a（结构级） |
| Terminal absent → 零 Terminal 贡献（无 PTY 出生点） | PASS | PLT2-12b（结构级） |
| Browser absent → **进程级** WebView 数为 0 | **UNKNOWN** | 今晚无运行实例，不估算 |
| Terminal absent → **进程级** PTY/子进程数为 0 | **UNKNOWN** | 同上 |

> 口径规则（§44）：能可靠测量才写 `MEASURED`；否则写 `DECLARED` / `UNKNOWN`，**禁止伪精确**。

## DISABLED 证据

- Bookmark disable ⇒ 贡献 0（PLT2-14 流程内含）。
- Browser/Terminal 尚未支持 disable（HP0），因此其「停用后资源是否消失」**尚未可验证**，
  这也是它们保持 HP0 的原因。

## RELEASE（C5）

`destroy` 不得只是「存在一个函数」（§29）。当前 **无任何能力声称 C5**：
Browser 的 WebView 释放与 Terminal 的 PTY/子进程释放尚无 before/after 测量证据。
