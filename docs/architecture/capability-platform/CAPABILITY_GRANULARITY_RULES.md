# Capability Granularity Rules（什么才配叫一块积木）

## 层级

```
Framework  →  Capability  →  Sub-Capability / Domain  →  Component / Service  →  Function
```

## 成为 Capability 至少要具备若干（不是全部，但要有真凭实据）

1. 独立用户价值
2. 独立生命周期（可启停）
3. 独立状态/语义边界（有 owner，且不复制别人的 state truth）
4. 独立资源（或诚实声明「无重资源」）
5. 独立权限
6. 独立 persistence
7. 独立 enable/disable 价值
8. 独立部署/组合价值

## 明确禁止

- ❌ 每个 Vue 组件一个 Capability。
- ❌ 每个 function 一个 Capability。
- ❌ 为了「积木数量」无限细拆。

## 本项目判定实例

| 候选 | 判定 | 理由 |
|---|---|---|
| `bookmark` | Capability | 独立状态/持久化/UI 贡献，可独立启用 |
| `browser` | Capability | 独立 WebView 资源、导航语义、会话 |
| `terminal` | Capability | 独立 PTY 资源、进程生命周期 |
| `workspace` | Capability | 6 个子域 owner + 多个主视图，整体可插拔 |
| `grid` | Sub-Capability（browser 内） | 与 Browser 共享 store 与原生面，单独拆会制造第二真源 |
| `files/artifact/repo/script/snippet` | Sub-Capability（workspace 内） | 独立 owner，但用户价值依赖 workspace 主框架 |
| `editor` | Sub-Capability（workspace 内） | 文件编辑＝workspace 的展示形态 |
| `session` | Domain（browser 内） | 会话恢复随浏览器存在 |
| `TaskPanel`（当前） | **还不算 Capability** | 既没有 manifest，也不在 catalog，Shell 还直接 import 它 |
| `用 המשתמש面 single panel` | Component | 只是 UI 单元 |

## 判定口诀

> 能不能把它从产品里拿掉、而系统仍然成立？
> 拿掉之后它占的资源会不会消失？
> 加进来要不要改 Shell？
> 三个都是「是/N否」清晰可验证，才是一块积木。
