# Phase 7F — Capability Architecture Preview v1 验收（自动部分）

> 日期：2026-09-19　分支：`feature/capability-platform-v1`
> 状态：**CODE COMPLETE / 人工 GUI 验收 PENDING**

---

## 1. 阶段链路与回滚点

| Phase | 内容 | commit | tag |
|---|---|---|---|
| 7A | 能力架构审计与设计（纯文档） | `82be2f1` | `capability-phase7a-architecture-pass` |
| 7B | 能力契约与登记表 + checker | `e9e2063` | `capability-phase7b-contract-pass` |
| 7C | 最小能力运行时 | `3f8c8ed` | `capability-phase7c-runtime-pass` |
| 7D | Bookmark 试点集成 | `db16a1b` | `capability-phase7d-pilot-pass` |
| 7E | 资源治理 v1 | `decb36a` | `capability-phase7e-resource-pass` |
| 7F | Preview 验收 + 领导材料 | 本提交 | `capability-preview-v1-code-pass` |

总回滚点：`semantic-governance-v1` (`316130d`)

---

## 2. 自动验收结果

```text
能力登记表 check-capability-registry.mjs
  --self-test            13/13 PASS（1 positive + 9 negative + 3 false-positive）
  真实扫描               fail=0 warn=0
  --strict               PASS

能力运行时 check-capability-runtime.mjs
  RT-01..RT-15           15/15 PASS（esbuild 加载真实 src/capability/runtime.ts）

试点集成 check-capability-pilot.mjs
  PLT-01..PLT-08          8/8 PASS（esbuild bundle 真实 src/capability/index.ts）

资源报告 capability-resource-report.mjs
  RPT-01..RPT-08          8/8 PASS（防编造护栏：禁实测数字/禁声称可组合）

语义治理回归（上一阶段冻结门禁，重新验证）
  check-semantic-registry.mjs --self-test   ALL_PASS
  check-semantic-registry.mjs                fail=0 warn=6 info=72 → PASS
  check-semantic-closure-logic.mjs           27/27 PASS
  check-sensitive-side-effects.mjs --self-test  ALL_PASS

前端构建  npm run build                       通过（✓ built in 4.96s）
Git 完整性 git fsck --full                    无 missing/corrupt/broken/invalid
```

---

## 3. 发布候选产物

```text
git HEAD:      decb36ad196bad5d204f08e23213fcd4a3adcf8a
artifact:      artifacts/capability-preview-v1-dist.tar.gz
sha256:        8126e05883fb96fa73a53726439e38c96f13a3899771cfb2c76a145dc4c23cf8
size:          288K
build time:    2026-09-19T23:27:06+08:00
内容:          dist/（前端产物，Vite build 输出）
```

**说明（必须显式）**：
- 本次**未构建**原生桌面安装包（deb / 二进制）——未执行 `tauri build`。
- 因此**不存在** deb 版本号问题；也更谈不上覆盖安装。
- 未执行 `sudo apt install` / `dpkg -i`，`/usr/bin/mvp-browser-os` **未被改动**。

---

## 4. 领导验收材料

```text
docs/delivery/capability-preview-v1/
  README.md                    索引 + 一句话结论 + 三个防误导事实
  01-WHAT-CHANGED.md           为什么模块化 / 现在好在哪
  02-CAPABILITY-MAP.md         18 个能力全景 + 三态区分
  03-COMPOSITION-DEMO.md       Minimal/Developer/Full + 可组合性真实状态
  04-RESOURCE-MODEL.md         资源分类 + 「关了能省什么」诚实回答
  05-STABILITY-AND-RECOVERY.md 隔离性保证 / 6 个回滚点 / 未动当前安装
  06-KNOWN-LIMITATIONS.md      13 条限制 + 12 条债务（未隐藏）
  07-HUMAN-ACCEPTANCE.md       人工 GUI 验收表（PENDING）
```

---

## 5. 明确不声称的事项

```text
❌ 不声称"所有能力已模块化"        → 仅 1 个接入（Bookmark），其余已登记未接入
❌ 不声称"可以按需装配/物理卸载"    → 无物理卸载，Profile 不能一键切换
❌ 不声称"关掉能省 X 内存"          → 零实测，全部 DECLARED
❌ 不声称"界面已验收"              → HUMAN_GUI_ACCEPTANCE = PENDING
❌ 不创建 capability-preview-v1-pass → 等明天人工验收通过后才打
```

---

## 6. 结论

```text
CAPABILITY_PREVIEW_V1_RESULT: CODE_PASS（自动验证全通过，等人工 GUI）
HUMAN_GUI_ACCEPTANCE:         PENDING
READY_FOR_LEADER_ACCEPTANCE:  YES
```
