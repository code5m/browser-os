# Phase 0 — Agent E：Checker Test Infrastructure Audit

日期：2026-09-17
范围：审计所有 Checker 的 self-test / fixtures / 退出码 / JSON / strict 约定；不擅自重写 Checker。

## 统一约定（已确认，来源 scripts/GATE-CONTRACT.md）

| 退出码 | 含义 |
|---|---|
| 0 | 本检查点覆盖指标全部 PASS |
| 1 | 任一指标 FAIL |
| 2 | 非法参数 / 用法错误（USAGE_ERROR） |

CLI 面（建议，已被本项目既有 checker 采用）：
- `--help` / `-h` → 退出 0
- `--self-test` → fixture 模式，退出 0=ALL_PASS / 1=FAIL
- `--json` → 机器可读输出
- `--strict` → WARN 升级为 FAIL

## 现状（Phase 0 三个核心 Checker 已对齐）

| Checker | --help | --self-test | --json | --strict | 退出码 | 自测结果 |
|---|---|---|---|---|---|---|
| check-command-set-consistency.py | ✓（新增） | ✓ | ✓（新增） | ✓（兼容占位） | 0/1/2 | ALL_PASS |
| check-grid-close-logic.mjs | ✓（新增） | ✓ | ✓（新增） | ✓（兼容占位） | 0/1/2 | ALL_PASS |
| check-ui.mjs | ✓ | ✓ | ✓ | ✓ | 0/1/2 | PASS |

备注：本仓库 42 个脚本含 self-test 字面量（含基线/聚合等非 checker 脚本），其中多数策略 checker
（check-*-policy.py / check-*.mjs）已具备 `--self-test`。本次 Phase 0 将 IPC / Grid 两个原先
**缺少自测与 CLI 一致性**的 checker 拉齐到统一约定。

## 重点发现（审计对象）

1. **self-test 测不到真实 regex（A 旧版）**：原 check-command-set-consistency.py 无 self-test，
   且 registered_commands / invoked 两个提取逻辑存在盲区（漏第二个/第三个 generate_handler! 块、
   漏泛型 invoke）。现已内置 5 类 fixture（多块注册、泛型/多行/注释 invoke、未注册命令、KNOWN 抑制、
   真实 new drift 不被抑制）覆盖提取与 diff 逻辑。
2. **fixture 与生产逻辑不同步（B 旧版）**：原 G1#4 断言 `schedulePosition()`，而源码早已改用
   `relocate()`；G2 冻结错误公式。现已用合成源码做正向+负向 fixture，验证检查器对「复位缺失 /
   gridOpen 旧耦合回归 / host 守卫缺失 / missing relocate」均能正确 FAIL。
3. **JSON 模式结构不稳定**：A/B 旧版无 --json；现已提供稳定结构
   `{check, status, ...}`（A）/ `{check, status, passed, failed, failures}`（B），便于 CI 解析。
4. **strict 行为漂移**：C(check-ui) 的 --strict 将 WARN 升级 FAIL；A/B 以 --strict 作为兼容占位
   （当前无 WARN 级别，行为等价于默认）。建议后续统一：无 WARN 的 checker 收到 --strict 不得改变结果
   语义，仅作向前兼容。

## 审计结论（未越界）

- 未重写任何不属于 Phase 0 范围的产品/策略 checker。
- 仅对 IPC / Grid 两个本阶段归属 checker 补齐 self-test + 统一 CLI；其逻辑由各归属 Agent（A/B）负责。
- 建议（非强制）：其余策略 checker 维持既有 0/1/2 契约；新 checker 默认提供 --help/--self-test/--json。
