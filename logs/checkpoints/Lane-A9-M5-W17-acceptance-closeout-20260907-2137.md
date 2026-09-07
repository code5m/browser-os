# Lane A9 — M5-W17 Acceptance Closeout Checkpoint（START TEST REVIEW）

> 生成：2026-09-07 21:37 CST · Lane A9（M5-W17 · Acceptance Closeout Dispatch → 行 1217：**START TEST REVIEW**）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W17 Acceptance Closeout Dispatch → A9：
> `scripts/check-home-ui-logic.mjs`, `scripts/check-client-navigation-logic.mjs`, `logs/checkpoints/`；
> **Re-run focused UI logic checks after A3; add only narrow assertions needed for the persistence fix. No component edits.**
> 集成序：`A3 -> A4/A9 -> A2/A8/A10 -> A1 -> A11 -> A0`。A9 仅测试/复查，不写产品代码。
> BASE：`052b18a`（HEAD 未变；A3 的 home-store 持久化修复已在本工作树就位）。
> HOLD 规则：A5/A6/A7 仅检查是否有具体阻塞，不重复开发。

```
LANE=A9
STATUS=PASS（Acceptance Closeout；A9 check-home-ui-logic.mjs 41 项断言全绿，0 失败）
BASE=052b18a
HEAD=logs/checkpoints/Lane-A9-M5-W17-acceptance-closeout-20260907-2137.patch
FILES=scripts/check-home-ui-logic.mjs,
      logs/checkpoints/Lane-A9-M5-W17-acceptance-closeout-20260907-2137.md
VERIFY=node scripts/check-home-ui-logic.mjs （期望 HOME_UI_RESULT=PASS，exit 0，41 ok / 0 failed）
VERIFY2=node scripts/check-client-navigation-logic.mjs （HOLD 复查，A6 脚本，期望 CLIENT_NAV_RESULT=PASS 60/60）
CHECKPOINT=logs/checkpoints/Lane-A9-M5-W17-acceptance-closeout-20260907-2137.md
METRICS=A9 41/41 PASS；A6(navigation) 60/60 PASS（HOLD 复查）；build metrics 未触发（A9 仅改测试脚本，零产品代码）
PATCH=logs/checkpoints/Lane-A9-M5-W17-acceptance-closeout-20260907-2137.patch
RISKS=见下
NO_PUSH=confirmed（board Merge Rule：仅 A0 推送）
```

## 交付物

- **`scripts/check-home-ui-logic.mjs`**（A9 既有测试脚本，本波**仅追加 G5 断言**，未改 G1–G4、未改任何产品组件）：
  - 新增 `[G5] persistence safety (HOME_NO_SECRET_PERSIST)` —— Closeout 唯一真债务的功能回归。
  - G5 同时含**源码接线断言**（防回退）与**功能断言**（真实加载 `src/utils/homeUi.ts` 纯逻辑层）。

## G5 断言明细（新增 9 项，全绿）

| 子类 | 检查 | 结果 |
|---|---|---|
| 5a 源码接线 | `store.save` 经由 `toPersisted(shortcuts)` 过滤后落库 | ✅ |
| 5a 源码接线 | `store.saveRecents` 经由 `toPersisted(recents)` 过滤后落库 | ✅ |
| 5a 源码接线 | `store.load/loadRecents` 经 `.filter(isStorageSafe)` 过滤（迁移期剔除已落库 app 命令体） | ✅ |
| 5b 契约 | `homeUi.isStorageSafe` 已导出 | ✅ |
| 5b 契约 | `homeUi.toPersisted` 已导出 | ✅ |
| 5b 契约 | `HOME_PERSISTED_TYPES` 排除 `app`（仅 `url`/`dir` 可落库） | ✅ |
| 5b 功能 | `isStorageSafe`：`app=false`，`url/dir=true` | ✅ |
| 5b 功能 | `toPersisted([app,url,dir])` 剔除 app 条目（保留 url/dir，长度=2） | ✅ |
| 5b 功能 | `persistedJson` 序列化结果**不含** app 命令体/凭据（不含 `sk-…`/`--token`/`app` 类型） | ✅ |

## 功能回归说明（为何是强断言而非仅源码存在性）

- 加载方式：优先 native TS strip（Node 22.18+/23.6+ 默认），回退 `esbuild.transformSync`（本仓 vite 依赖已带，`node_modules/.bin/esbuild` 存在）。实测 Node v26.7.0 直载成功，9 项功能断言全部运行通过。
- 样例 app 条目：`target = "launch-my-app --token sk-TOPSECRETCMDBODY /home/me/.ssh/id_rsa"`。
- 证明：`persistedJson([app,url,dir])` 的 JSON 字符串既不含 `sk-TOPSECRETCMDBODY`、也不含 `--token`，且 `JSON.parse` 后数组无 `type==="app"` 元素 —— 即 **app 启动命令体/潜在凭据绝不进入浏览器存储**，债务 `HOME_NO_SECRET_PERSIST` 已闭环。

## 前置依赖（A3）确认

- A3 已于 `src/stores/useHomeStore.ts` 落地 Closeout 修复：`save()/saveRecents()` 走 `toPersisted(...)`，`load()/loadRecents()` 经 `isStorageSafe` 过滤并就地抹除旧 app 条目；`src/utils/homeUi.ts` 提供 `isStorageSafe`/`toPersisted`/`persistedJson`/`HOME_PERSISTED_TYPES` 单一真源。
- A9 未改产品代码，仅对该契约做回归断言，符合 W17 A9「Do not modify product components」。

## HOLD 复查（A5/A6/A7）

- **A6（HOLD）**：重新运行其 `scripts/check-client-navigation-logic.mjs` → **60/60 PASS**（`CLIENT_NAV_RESULT=PASS`）。导航真源/窄窗口裁剪/键盘漫游/ActivityBar 源码约束全绿，无回归，无具体阻塞。
- **A5（HOLD）**：`src/components/home/**` 不在 A9 编辑范围；其首页组件经 G1–G4 + G5 静态扫描未见合规回退，且 Closeout 唯一债务归 A3 已闭环，无迫使 A5 重开的验收发现。
- **A7（HOLD）**：`MainArea.vue`/`StatusBar.vue`/`App.vue` shell fallback 不在 A9 编辑范围；Closeout 注记其 shell fallback 已覆盖，无具体阻塞。
- 结论：A5/A6/A7 均无具体阻塞，按 HOLD 规则不重复开发。

## Verify（可复现）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
node scripts/check-home-ui-logic.mjs
# 期望：
#   [G1] 8 ok  [G2] 20 ok  [G3] 3 ok  [G4] 1 ok  [G5] 9 ok
#   主页 UI 逻辑测试：通过 41，失败 0
#   HOME_UI_RESULT=PASS
#   exit 0

node scripts/check-client-navigation-logic.mjs   # A6 HOLD 复查
# 期望：CLIENT_NAV_RESULT=PASS (60/60)
```

## Merge Notes

- **零产品代码改动**：A9 仅扩展测试脚本（`scripts/check-home-ui-logic.mjs`）+ 本 checkpoint；未改任何 `.vue`/store/桥/ACL/capability/`pre-merge.sh`/三份主文档（符合 W17 A9 硬约束与 Closeout「no component edits」）。
- **patch 范围**：`git diff HEAD -- scripts/check-home-ui-logic.mjs`（仅 A9 本波改动，不含其它 lane 工作树污染）。
- **pre-merge 接线建议**：本包不含 `pre-merge.sh` 改动（共享门禁脚本，跨 lane 冲突须由 A0 统一接线）；建议 A0 集成时在 `run_self_test` 增加 `node scripts/check-home-ui-logic.mjs` 步骤（A3 的 `check-home-store-logic.mjs` 同波）。

## RISKS（诚实剩余风险）

- app 快捷方式现仅**会话内有效**（设计使然，`HOME_APP_SESSION_ONLY_NOTICE` 已告知用户）；重启客户端后 app 条目不恢复——这是闭环 `HOME_NO_SECRET_PERSIST` 的代价，非缺陷。
- 原生桌面**可视化验收**仍属用户侧证据（Closeout 注记）：A9 为自动化逻辑/源码级测试，不替代 A8 的实机桌面验收。
- G5 功能回归依赖 `homeUi.ts` 可在 node 直载（native TS 或 esbuild）；本仓实测可用。若极端环境二者皆无，`loadHomeUi()` 返回 null，G5 退化为仅源码接线断言并记 `note`（非阻塞），不会误报 PASS。

## FORBID 遵守（W17 / Closeout）

- 仅扩展 `scripts/check-home-ui-logic.mjs` + 本 checkpoint；**未改任何产品组件**，未改 `pre-merge.sh`/ACL/三份主文档。
- 未移动 `NEXT`；未 push。
