# AI UI CHANGE SURFACE REPORT — AI 可维护性验收（§33）

> 抽样验证：普通 Agent 改 UI 时，需要读多少、影响面怎么查、怎么验证。

---

## A. 修改一个 shared primitive（例：EmptyState）

**需要读**
1. `src/shared/ui/EmptyState.vue`（组件本体）
2. `src/shared/ui/index.ts`（公开入口）
3. `docs/architecture/ui-system/SHARED-UI-CONTRACT.md`（规则：依赖法 / props 纪律 / a11y / 样式）

**不需要读**
- 任何 Capability 内部、任何业务 store、App.vue 历史。

**影响面怎么查**
```bash
# 1) 机器可读 catalog 里的 consumers 字段（权威）
grep -n "id: shared.empty-state" docs/architecture/ui-system/ui-components.yaml

# 2) 全仓真实引用（交叉验证）
grep -rn "EmptyState" src --include=*.vue --include=*.ts

# 3) 门禁保证 catalog 与真实文件不漂移（68/68）
node scripts/check-ui-boundaries.mjs   # UI-10
```

**验证命令**
```bash
node scripts/verify-ui-pilot.mjs        # SSR 渲染，与 before DOM 逐字节比对
node scripts/check-ui-boundaries.mjs    # UI-01/02/08 shared 纯净性
npm run check                            # 全量门禁
npm run build
```

**结论**：影响面可从 catalog 一次查全，且有机器校验兜底。✅

---

## B. 修改 Browser UI（例：Omnibox / 宫格）

**需要读**
- `src/capabilities/browser/`（manifest / index / state / ui）
- `src/capability/contribution/types.ts`（若要改贡献）

**不需要读**
- Terminal / Workspace / Git / Database / Plugin 的任何内部。
  （`check-ui-boundaries.mjs` UI-04 保证能力之间不互相 import 内部；
   跨能力只可走 `public` 出口且须在 manifest 声明。）

**注意**：Browser 的重资源（Grid 子进程 / WebView）只能由 Browser 自己的生命周期创建
（`src/capabilities/browser/resource/guard.ts`），Shell 无权触发 —— 由
`check-capability-resource-boundary.mjs` 与 `runtime-resource-absence.mjs` 守护。

**结论**：局部可读、边界机器可验。✅

---

## C. 新增一个 Capability 并使用 UI 库

**已有能力**（无需改 Shell）
| 扩展点 | 是否需改 Shell |
|---|---|
| `workbench-main` surface | ❌ 不需要 |
| `browser-sidebar` surface | ❌ 不需要 |
| `browser-host` surface | ❌ 不需要 |
| `address-bar-actions` / `activity-bar-trailing` navigation | ❌ 不需要 |
| **`browser-dock` surface** | ❌ **UI-4 起不需要**（页签由贡献声明 label/icon/order 自动生成） |
| 新增菜单项 | ⚠️ 仍需改（`NAV_MENU_SECTIONS` 硬编码） |
| 新增状态栏项 | ⚠️ 仍需改（`StatusBar` 直读 store） |

**达成：5 / 7 扩展点无需改 Shell。**

**使用 shared UI**
```ts
import { EmptyState, ContextMenu, ContextMenuItem } from "<相对路径>/shared/ui";
```
规则见 `SHARED-UI-CONTRACT.md`：禁止依赖内部路径、禁止让 shared 沾业务。

---

## D. 抽样结论

| 场景 | 判定 |
|---|---|
| A 改 shared primitive：影响面可查 + 有验证 | ✅ |
| B 改 Browser UI：无需理解其它能力 | ✅ |
| C 新 Capability：可复用 UI 库 + 多数扩展点无需改 Shell | ✅（菜单/状态栏仍是 GAP） |

**已知限制（诚实）**
- 菜单项 / 状态栏项尚无贡献类型（Rule of Two 未满足，未造万能 API）。
- 11 处 Shell→业务面板直渲仍在（各自需人工视觉评估，已全部显式 baseline）。
- 视觉等价目前由 **SSR DOM 逐字节比对 + token 值比对**证明；
  像素级观感仍需 Final Human Visual Acceptance（**HUMAN_VISUAL = PENDING**）。
