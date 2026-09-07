# Lane A9 — M5-W17 主页 UI 逻辑测试 Checkpoint（START TEST）

> 生成：2026-09-07 18:00 CST · Lane A9（M5-W17 · START TEST）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W17 Desktop Client Completeness and Home Recovery Dispatch → A9（行 1193）：**START TEST** — `scripts/check-home-ui-logic.mjs` (new), `scripts/pre-merge.sh` only if integration is required, `logs/checkpoints/`；DOM/source-level checks for A5 home actions, accessible names, bounded lists, and no sensitive copy. **Do not modify product components.**
> BASE：`052b18a`（HEAD：`docs(M5-W15): release-readiness bundle`）
> 范围：W17 允许有界前端/启动体验产品代码；**禁止新增运行时权限、远程执行或特权集成**。A9 仅写测试脚本，不碰产品组件。

```
LANE=A9
STATUS=PASS (START TEST；新脚本 check-home-ui-logic.mjs 通过 29 项断言，0 失败)
BASE=052b18a
HEAD=logs/checkpoints/Lane-A9-M5-W17-home-ui-logic-20260907-1800.patch
FILES=scripts/check-home-ui-logic.mjs,
      logs/checkpoints/Lane-A9-M5-W17-home-ui-logic-20260907-1800.md
VERIFY=node scripts/check-home-ui-logic.mjs （期望 HOME_UI_RESULT=PASS，exit 0，29 ok / 0 failed）
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W17-home-ui-logic-20260907-1800.md
MERGE_NOTES=见下
NEXT=A11 终验可直接 node scripts/check-home-ui-logic.mjs；A0 集成时建议在 pre-merge.sh 接线（避免跨 lane 冲突，本包不含 pre-merge.sh 改动）
```

## 交付物

- **`scripts/check-home-ui-logic.mjs`**（新建，无依赖，headless 静态源码/DOM 级检查）：扫描 `src/components/home/*.vue` 合集 + `src/stores/useHomeStore.ts`，覆盖 W17 A9 四项要求。

## 检查结果（W17 A9 要求项）

| 组 | 检查 | 结果 |
|---|---|---|
| G1 home actions | 收藏网页/目录、新增、恢复默认、打开动作齐全；主要工作区入口(HomeLaunchers 宫格)、v-for 网格、空态文案 | ✅ 8/8 |
| G2 accessible names | 全部 21 个交互控件（button/select/input）均有可访问名称：按钮含 `title`/`:aria-label`/文本；`<select>`/`<input>` 经 `<label for>` 程序化关联 | ✅ 21/21 |
| G3 bounded lists | `VISIBLE_LIMIT=12` 常量 + `.slice(0, VISIBLE_LIMIT)` 渲染裁剪；畸形数据容错（store.load try/Array.isArray 与 HomeShortcuts.norm/safeType 防御归一化） | ✅ 3/3 |
| G4 no sensitive copy | 首页组件与 store 不含硬编码 sk-/AKIA/token=/password=/secret=/Authorization/Bearer/api_key/credential（示例 URL `kimi.moonshot.cn`、占位路径 `/home/you/Documents` 非敏感，不计入） | ✅ 1/1 |

合计 **29 ok / 0 failed**，`HOME_UI_RESULT=PASS`，`exit 0`。

## 关键说明

- A5 已将主页拆分为 4 组件：`HomePanel.vue`（外壳 + 4 动作按钮）、`HomeLaunchers.vue`（17 个主要工作区入口，含 `:aria-label`/`:aria-current`）、`HomeShortcuts.vue`（v-for 网格、`home.open`、`VISIBLE_LIMIT=12` 有界渲染、`norm`/`safeType` 畸形归一化、空态）、`HomeShortcutEditor.vue`（dialog + `aria-modal`/`aria-labelledby`、`<label for>` 关联、Tab/Shift+Tab 焦点闭环、Esc 关闭）。脚本据此扫描全部组件合集（非单一文件）。
- **bounded lists 落地在渲染层**（`VISIBLE_LIMIT=12`），store 层 `addShortcut`/`saveEdit` 仍直接 `push` 无硬上限（防御纵深项，非阻塞；UI 已保证有界与畸形容错）。脚本对任一层有界即判 PASS。
- 观察项（非阻塞）：`store.open` 失败提示回显 `e?.message`（运行时可能含路径），建议稳定文案，属 A5 后续项。

## Verify（可复现）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
node scripts/check-home-ui-logic.mjs
# 期望：
#   [G1] 8 ok  [G2] 21 ok  [G3] 3 ok  [G4] 1 ok
#   主页 UI 逻辑测试：通过 29，失败 0
#   HOME_UI_RESULT=PASS
#   exit 0
```

## Merge Notes

- **冲突风险规避**：未修改 `scripts/pre-merge.sh`（该文件为 685 行共享门禁脚本，A3 的 `check-home-store-logic.mjs` 与 A6 的 `check-client-navigation-logic.mjs` 同 wave 亦可能接线；按 board「跨 lane 冲突须出二进制补丁而非覆盖」指引，本包不含 pre-merge.sh 改动，由 A0 集成时统一接线）。
- **零产品代码改动**：新增脚本为测试件，未改任何 `.vue`/store/桥/ACL/capability（符合 W17 A9「Do not modify product components」与 W17 硬停「no new backend contract」）。
- 未 push（board Merge Rule：仅 A0 推送）。

## FORBID 遵守（W17）

- 仅新增 `scripts/check-home-ui-logic.mjs` + 本 checkpoint；**未改任何产品组件**，未改 `pre-merge.sh`/`capability.rs`/ACL/三份主文档。
- 未移动 `NEXT`；未 push。
