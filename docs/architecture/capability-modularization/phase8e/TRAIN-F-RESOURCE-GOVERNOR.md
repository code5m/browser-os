# TRAIN F — Resource Governor + Real Profiles（Phase 8E）

> 日期：2026-09-20
> 结论：**PASS（诚实边界内）**。真实 Composition Profile 落地（minimal 端到端零 PTY/零 WebView）；
> Resource Governor v1 为薄协调层（不持业务 state、不绕过 owner）；资源测量诚实分类为 UNKNOWN（headless 无运行实例，不估算）。
> 新增门禁：`scripts/check-composition-profiles.mjs`（11/11 PASS）
> tag：`capability-phase8e-resource-governor-profiles-pass`

---

## 1. 真实 Composition Profile（不是 UI hide）

`src/capability/profiles.ts` 定义 **单源** `CAPABILITY_PROFILES`：
- `minimal`   = bookmark + workspace（**无 browser / 无 terminal**）
- `developer` = bookmark + workspace + browser + terminal
- `full`      = 同上（当前 4 能力；预留吸纳未来新增）

`bootstrapCapabilityRuntime(profile)` 只注册 profile 列出的能力；未列出的**不注册** → 其贡献槽为空
→ 不加载内部 store / 不创建重资源。这是可组合性的端到端证据，而非仅隐藏按钮。

`check-composition-profiles.mjs` 真实调用 bootstrap（esbuild 转译真实代码，按 profile 各建独立模块实例）：
- PROF-01 minimal 只注册 bookmark+workspace
- PROF-02 minimal **不注册 terminal**（→ 无 PTY 出生点）
- PROF-03 minimal **不注册 browser**（→ 无 WebView）
- PROF-04 developer/full 注册全部 4 能力
- PROF-05 各 profile bootstrap activated=true（能力层失败不打断启动）

## 2. Resource Governor v1（薄协调层）

`src/capability/resourceGovernor.ts`：`createResourceGovernor(runtime)` 只持 runtime 引用，**零业务 state**。
暴露 `activate/background/suspend/hibernate/destroy`，全部委托 rt 公共转换 API。

诚实边界（**不谎报 C4/C5**）：
- Governor **不**释放资源（不 kill PTY / 不关 webview / 不碰密钥）；资源释放是各能力 owner 职责。
- Terminal `suspendable:false`（manifest 与 capabilities.yaml 同步修正为 `supported: [ACTIVE]`），
  `rt.suspend("terminal")` 抛 `SUSPEND_NOT_SUPPORTED` → Governor 原样透传（GOV-03）。
- `destroy("terminal")` 经 `rt.disable`，但 terminal 为 ACTIVE 且不可 suspend → 运行时抛
  `INVALID_TRANSITION`（ACTIVE 不可直接 disable，须先 suspend）→ Governor 不绕过 owner 强杀 PTY（GOV-04/05）。
  **这正是 C4/C5 未达的诚实信号**：Terminal 一旦激活，v1 无 lifecycle 拆解路径。

GOV-01/02 静态证明：Governor 不 import 任何能力 owner/store/UI、不持有任何业务 state 符号。

## 3. 真实资源测量（诚实分类，不估算）

`scripts/measure-resources.mjs` 读 `/proc` 真实计数（main RSS / 子进程数 / WebKit 进程数 / PTY master 数），
按维度分类：
- **MEASURED**：有运行中目标进程且 /proc 可读 → 真实读数
- **DECLARED**：能力声明了资源类别（capabilities.yaml resources.class），未实测
- **UNKNOWN**：无运行实例 / /proc 不可读

**当前（headless CI）结果为 UNKNOWN**——没有运行中的 `mvp-browser-os` 进程，故不报任何数字，
**禁止用经验值冒充 MEASURED**。要实测：启动应用后设 `MVP_BROWSER_PID`（或 `--pid`）复跑。

对比矩阵（Train F §14 要求）的填写状态：

| 维度 | Browser absent | Browser present | Grid 非激活/激活/销毁 | Terminal absent/present/销毁 | Minimal/Developer/Full |
|---|---|---|---|---|
| 实测分类 | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | MEASURED(profile 注册集) |
| 说明 | 需运行实例 | 需运行实例 | 需运行实例 | 需运行实例 | profile 注册为代码级事实 |

> 注：profile 注册集是**代码级 MEASURED**（bootstrap 真实只注册列出的能力），这是「absent → 零重资源」
> 的可组合性证明；但进程级资源占用需真机，列为 UNKNOWN 直至实测。

## 4. 诚实性审查

| 禁止项 | 自查 |
|---|---|
| 只是搬目录？ | 否。新增 profiles.ts / resourceGovernor.ts（薄协调层），修正 terminal 生命周期声明矛盾 |
| God Runtime？ | 否（GOV-01/02）。Governor 只持 runtime 引用 |
| 绕过 owner？ | 否（GOV-04/05）。destroy 走 rt.disable，不直调 killTerm |
| 误报 C4/C5？ | 否。suspend 诚实拒绝；destroy 不可达；明确记为未达 |
| 估算冒充 measured？ | 否。measure-resources 无实例即 UNKNOWN |
| Checker 失守？ | 否。新增 1 门禁（11/11），未删未放宽 |
| 放宽规则？ | 否 |

## 5. 门禁实数

```
check-composition-profiles.mjs   PASS 11/11
npm run check                    PASS
npm run build                    PASS
```

NEXT：Train G — 终检（全部门禁 + 红队）+ tag `capability-modularization-v1-code-pass`。
