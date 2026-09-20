# 14 — Leader Demo（3~5 分钟）

一键：`node scripts/capability-demo.mjs demo`
分步（便于边讲边看）：

```bash
node scripts/capability-demo.mjs list
node scripts/capability-demo.mjs resolve framework-only
node scripts/capability-demo.mjs resolve workspace,browser
node scripts/capability-demo.mjs resolve workspace,browser,terminal
node scripts/capability-demo.mjs resolve workspace,terminal
node scripts/capability-demo.mjs runtime-demo bookmark
node scripts/capability-demo.mjs matrix
```

## 脚本与话术

| 步 | 命令 | 话术 |
|---|---|---|
| 1 | `resolve framework-only` | 「这是最小底座：零能力，零资源。」 |
| 2 | `list` | 「这些是积木：身份、依赖、贡献、资源、成熟度都写在契约里。」 |
| 3 | `resolve workspace,browser` | 「产品就是积木组合。」 |
| 4 | `resolve workspace,browser,terminal` | 「加入终端不需要改 Shell，只改装配。」 |
| 5 | `resolve workspace,terminal` | 「拔掉浏览器：WEBVIEW 从资源表里消失，其它能力照旧。」 |
| 6 | `runtime-demo bookmark` | 「应用不重启：插上 3 个贡献，拔掉 0 个，再插回来。」 |
| 7 | `matrix` | 「16 种组合全部确定性通过。」 |

## 结语

> 以前我们开发的是一个越来越大的应用；
> 现在我们开发的是一个底座和一套可独立组合的能力积木。

## 诚实提示（演示时主动说）

- 运行时插拔目前只有 **bookmark** 达到 HP2；浏览器/终端仍是启动前插拔（HP0），
  因为 WebView / PTY 的停用与释放还没做完验证。
- HP3（运行时安装能力包）**没做**，也不演示假的。
- 进程级资源计数今晚是 UNKNOWN（没有跑实例），演示时给的是结构性证据 + 资源归属声明。
