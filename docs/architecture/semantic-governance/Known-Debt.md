# Known Debt（已知债务登记）

> 本文件登记**已确认但不修复**的债务。登记的目的是让债务**显式存在、不静默消失**，同时明确"本阶段不修"。
> 处置纪律：任何一条债务的修复都必须由独立的后续任务单驱动，并同步更新本文件与相关门禁。

| ID | 债务 | 状态 | 是否阻塞 Phase 1 |
|---|---|---|---|
| Debt-001 | Grid UDS socket cleanup | KNOWN DEBT | 否 |
| Debt-002 | `toggleGridToolbar` | KNOWN DEBT | 否 |
| Debt-003 | `closeGridCell` orphan API | KNOWN DEBT | 否 |
| Debt-004 | Terminal checker | KNOWN DEBT | 否 |

---

## Debt-001

### Grid UDS socket cleanup

状态：

```text
KNOWN DEBT
```

事实：

- `~/.local/share/com.jizhijiandan.mvp/sock/` 下的 `grid-<pid>-<n>.sock` 文件在应用退出后**未被清理**，持续累积。
- 实测（2026-09-19）：本次实例残留 4 个；目录**累计 138 个**。
- 最早可追溯到 **2026-08-25**，**非 Phase 1 引入**（Phase 1 提交为 2026-09-18）。

影响：

- 文件污染（目录中堆积历史 socket 文件）。
- **当前无功能影响**（socket 按 pid 命名，不会冲突；启动即创建新的）。

禁止：

```text
本 Phase 修复。
```

补充：清理动作会触碰用户数据目录，执行前必须备份，且不得影响现有用户数据（见 `docs/operations/recovery/data-backup.md`）。

---

## Debt-002

### toggleGridToolbar

状态：

```text
KNOWN DEBT
```

原因：

- **无调用方**（死代码，零引用，故当前不触发）。
- **依赖方向风险**：函数体内使用了 `useBrowserStore()` 但该模块**未 import**；改为静态 import 会形成 `layout ↔ browser` 循环依赖。

禁止：

```text
本 Phase 修复。
```

补充：修复需同时解决依赖方向（例如把意图下沉或改为运行时注入），属架构调整，不做"补个 import"式局部修改。

---

## Debt-003

### closeGridCell orphan API

状态：

```text
KNOWN DEBT
```

原因：

- API 存在（`useBrowserStore.closeGridCell(i)` → `closeGridOne(i)`）。
- **当前无 UI 产品入口**（单格关闭 UI 在当前版本已不存在，与 `closeGridOne` 同状态）。

禁止：

```text
本 Phase 修复。
```

补充：`scripts/check-view-intent.mjs` 对其有"定义但未被引用"的信息性 WARN（不计入门禁失败），属预期。

---

## Debt-004

### Terminal checker

状态：

```text
KNOWN DEBT
```

原因：

- **Phase 外** —— 不属于 Browser/Grid 语义范围。
- **与 Browser/Grid 无关**（本次及前序 Phase 未触碰任何终端文件）。
- 具体表现：`check-terminal-policy.py` 与 `check-terminal-ui-logic.mjs` 在 `pre-merge.sh --self-test` 中失败，为既有债。

禁止：

```text
本 Phase 修复。
```

补充：判定"新增失败 vs 既有失败"时，务必先回到干净树验证（`git stash` 后跑门禁），不得为了让门禁变绿而放宽断言（见 `docs/operations/recovery/git-workflow.md` 场景 1）。

---

## 处置原则

1. **不静默消失**：债务只能显式登记、显式关闭，禁止悄悄删除或修改本文件来"消化"债务。
2. **不在本阶段修**：上述四条均不在 Phase 1 范围内；修复需独立任务单。
3. **禁止以降低门禁的方式"修复"**：不得放宽断言、增加 ignore、删除失败测试。
4. **修复后同步**：关闭某条债务时，同步更新本文件、相关 checker 与 `docs/operations/recovery/` 中引用它的文档。
