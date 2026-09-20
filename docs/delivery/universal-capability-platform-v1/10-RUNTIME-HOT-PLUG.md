# 10 — Runtime Lifecycle & Hot-Plug

## 生命周期（§18）

`AVAILABLE → REGISTERED → ENABLED → ACTIVE ⇄ BACKGROUND / SUSPENDED → DISABLED → UNREGISTERED`

必须区分的三条语义：

- `AVAILABLE ≠ ENABLED`：在库里 ≠ 已启用
- `ENABLED ≠ ACTIVE`：已启用 ≠ 正在跑
- `DISABLED ≠ UNINSTALLED`：停用 ≠ 卸载

非法迁移由 `assertTransition` 抛 `INVALID_TRANSITION`（PLT2-13）。

## Hot-Plug 等级定义

| 等级 | 含义 | 本项目现状 |
|---|---|---|
| HP0 STATIC | 只能构建/启动前决定 | workspace / browser / terminal |
| HP1 RUNTIME ENABLE/DISABLE | 运行时启停 | bookmark（已验证） |
| HP2 RUNTIME REGISTER/UNREGISTER | 运行时加入/移出 runtime | **bookmark（试点，已验证）** |
| HP3 RUNTIME INSTALL/UNINSTALL | 运行时安装/卸载能力包 | **未做（明确延后）** |

## ADD 流程（含回滚）

```
validate manifest → resolve dependencies → permission check
→ register record → register contributions → lifecycle → verify
任何一步失败 → 撤销已注册的 contributions + 已注册的 record（禁止半注册）
```

## REMOVE 流程

```
dependent check（存在强依赖方 → 默认 REJECT）
→ active work policy（graceful / reject）
→ deactivate → 摘除 contributions → 移除 runtime record → verify 无残留
```

## 本次实测（bookmark，应用不重启）

```
before           contributions=0  registered=[∅]
REGISTER         contributions=3  state=ACTIVE
DISABLE          contributions=0  state=DISABLED
ENABLE           contributions=3  state=ACTIVE
UNREGISTER       contributions=0  state=UNREGISTERED
RESULT           PASS：真正的运行时插拔，不是布尔开关
```

**诚实边界**：`DISABLE` 移除的是 UI 贡献；能力内部的业务动作级拦截（D-8）尚未逐个接入。
Bookmark 无后台行为与重资源，因此不影响本次结论。
