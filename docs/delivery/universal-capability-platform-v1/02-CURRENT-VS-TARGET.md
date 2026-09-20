# 02 — CURRENT vs TARGET（严格区分，禁止把目标写成现状）

## 已达成（CURRENT，有真实验证）

| 能力/属性 | 现状 | 验证方式 |
|---|---|---|
| Bookmark / Workspace / Browser / Terminal | **C3** | 既有 C3 门禁今晚全部复跑通过 + 新平台门禁 `check-capability-platform.mjs` |
| Building Block Contract v1 | 已实现，机器可读 | `src/capability/platform/contract.ts` + PLT2-02 |
| Capability Catalog | 4 项，机器可读 | `capability-demo list` |
| Dependency Resolver | 真实工作，deterministic | PLT2-03/04/05/06/07/08 |
| Assembly Engine | 真实驱动 bootstrap | PLT2-19a/b/c（`bootstrapAssembly`） |
| Framework-only 自动引导 | PASS | PLT2-10/11/11b |
| 16 组合插拔矩阵 | 16/16 deterministic | PLT2-09 |
| 运行时生命周期模型 | 8 态 + 合法迁移表 | PLT2-13/13b |
| Hot-Plug Bookmark（HP1+HP2） | PASS | PLT2-14a..f |
| 强依赖方存在时拒绝移除 | PASS | PLT2-15 |
| 零第二真源（catalog 与定义同一对象） | PASS | PLT2-17 |
| 契约与真实注册零漂移 | PASS | PLT2-14b |

## 未达成（TARGET，明确标注为未做）

| 目标 | 现状 | 原因/计划 |
|---|---|---|
| Git / Database 成为积木 | **C1**（仅 owner 物理分离） | 需 manifest + contribution + Shell 解直连；未做，不谎报 |
| Task / Graph / Plugin / Skill / Agent 成为积木 | **C0**（未被 catalog 收录） | 面板仍由 Shell 直连懒加载 |
| Browser / Terminal 达到 HP1 | **HP0** | WebView / PTY 停用释放策略未验证 |
| HP3 运行时安装卸载 | **未做** | 需动态加载器/沙箱，风险高，明确延后 |
| 物理多包（npm workspace） | **未迁移** | 见 07 诚实裁决 |
| Shell 完全零能力知识 | **部分** | 4 个积木已零直连；MainArea 仍直连约 10 个未积木化面板 |
| 进程级资源测量（真实实例） | **UNKNOWN** | 今晚无 GUI 实例；结构性证据已 PASS |

## 关键判据

**分类完成 ≠ 全部 C3。** 清单里每一项都被分类（`UNCLASSIFIED=0`），但只有 4 项真正成为积木。
