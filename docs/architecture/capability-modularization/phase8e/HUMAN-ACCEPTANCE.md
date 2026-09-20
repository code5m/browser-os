# Capability Modularization v1 — Human Acceptance（人工验收）

> 日期：2026-09-20
> 前置基线：`capability-modularization-v1-code-pass`（commit `04d794b`）。本阶段**不写代码**，
> 只做真实 GUI / runtime 验收。验收通过后才进 Release Closeout。
>
> 原则：自动 checker 已证明 contract / absence / lifecycle 逻辑；本阶段只验**真实用户路径**，
> 不再跑一遍 checker。重点在 WebView / GTK / PTY 的真实行为——这些是自动化覆盖不到的。

---

## 0. 冻结基线（CODE_PASS，本阶段不可改）

```text
Capability Modularization v1
STATUS: CODE_PASS

FINAL COMMIT: 04d794b
FINAL TAG:    capability-modularization-v1-code-pass

CURRENTLY_COMPOSABLE = 4
  Bookmark   C3 OPTIONAL
  Workspace  C3 OPTIONAL
  Browser    C3 OPTIONAL
  Terminal   C3 OPTIONAL

  Database   C1
  Git        C1

Train A → G: COMPLETE
HARD STOP: NONE
HUMAN GUI: PENDING
PUSH: NO
MASTER MERGE: NO
```

> ⚠️ 本阶段**禁止**为「让验收好看」而改动 src/ src-tauri/。任何代码改动都意味着基线漂移，
> 必须回退到 `capability-modularization-v1-code-pass` 重打 tag。架构下一步（C4/C5、Database/Git 抽 C3）
> 明确**不在本阶段**，交后续 train。

---

## 1. 验收方式

- 用已安装版本或 `npm run tauri dev` 起真实应用（GUI / WebView / PTY 全活）。
- 每节逐条勾选 **PASS / FAIL / NA**，FAIL 必须记录现象 + 截图/日志路径。
- 不依赖 checker 输出；以肉眼 + 真实交互为准。
- Grid / Terminal（E / F）为最高优先级人工验项，务必逐项走真实路径。

---

## 2. 验收清单（A–H）

### A. Application Shell
- [ ] 启动无异常白屏 / 无 console 灾难级错误
- [ ] 重启后 Shell 正常恢复
- [ ] 基本导航（ActivityBar / 地址栏 / 视图切换）流畅无卡死

### B. Bookmark
- [ ] 面板打开 / 关闭正常
- [ ] 收藏 / 取消收藏生效
- [ ] 入口可达、面板状态与 `panelOpen` 一致

### C. Workspace
- [ ] Files / Artifact / Repo / Script / Snippet 各自可达
- [ ] 文件打开 / 编辑 / 保存 / 切换正常
- [ ] Recent / Workspace 切换状态正确恢复

### D. Browser
- [ ] 创建页签 / 切换页签正常
- [ ] Browser ↔ Workspace 互通（写 recents / 读 url）无断裂
- [ ] WebView 正常渲染、可交互
- [ ] Browser Dock 正常
- [ ] Session / Network 行为正常

### E. Grid（★ 高优先，重点验）
- [ ] Browser → Grid 进入正常
- [ ] Grid → Browser 返回正常
- [ ] Grid → Workspace / Home 跳转正常
- [ ] **HIDE 不 DESTROY**：隐藏 Grid 后原生资源未销毁，再显示不重建
- [ ] **explicit closeGrid 才 DESTROY**：仅显式关闭才释放
- [ ] **不出现历史问题**：首次跳百度、重复加载、隐藏后状态丢失等回归为零

### F. Terminal（★ 高优先，重点验）
- [ ] xterm 输入 / 输出正常
- [ ] 新建 Terminal 正常
- [ ] resize 真实生效（非仅事件）
- [ ] Dock Terminal 正常
- [ ] Terminal ↔ Workspace / Browser 联动正常
- [ ] close / reopen 后 PTY / 进程生命周期正确（无孤儿进程 / 无泄漏）
- [ ] 连续快速新建 / 关闭 Terminal 不崩、不残留 PTY

### G. Cross-capability
- [ ] Bookmark + Browser 联动
- [ ] Workspace + Browser 联动
- [ ] Workspace + Terminal 联动
- [ ] Browser + Terminal 联动
- [ ] **连续快速切换**多能力不白屏 / 不卡死 / 不残留资源

### H. Restart
- [ ] 正常关闭应用
- [ ] 重启后**应恢复**的状态恢复
- [ ] **不应恢复**的 native resource（PTY / WebView / Grid UDS）不残留

---

## 3. 验收结论记录

| 节 | 结果 | 备注 / 证据 |
|----|------|------------|
| A Shell | ☐ | |
| B Bookmark | ☐ | |
| C Workspace | ☐ | |
| D Browser | ☐ | |
| E Grid | ☐ | |
| F Terminal | ☐ | |
| G Cross | ☐ | |
| H Restart | ☐ | |

- 全部 PASS → 进 §4 Release Closeout。
- 任一 FAIL → 记录现象，回 Agent 修（修完回退基线重打 `code-pass` tag，再重新验收）。**本阶段不修 RED。**

---

## 4. Release Closeout（仅 Human GUI PASS 后执行）

```text
Human GUI PASS
    ↓
更新本 acceptance 文档（结论 + 日期 + 证据）
    ↓
创建最终 tag
   capability-modularization-v1-pass
    ↓
确认 master 无意外变化（git diff master..origin/master / 本地未污染）
    ↓
ff-only merge feature/capability-platform-v1 → master
    ↓
重新跑最终 gates（npm run check / build / 关键 checker）
    ↓
push master + tags（用户授权后）
```

> 最终 tag 命名 `capability-modularization-v1-pass`（非 `code-pass`）——表示 GUI 已验收。
> `code-pass` 仅代表自动化闭环，本阶段过后才升级为 `pass`。

---

## 5. 明确的边界决策（防污染验收基线）

- **不夹带修 pre-merge RED**：M4/M5 历史债（Capability-Resource-Report.md 尾随空白 +
  check-grid-close.mjs FAIL）已归类为历史债，**单列 `Pre-Merge Historical Debt Closure` 阶段处理**。
  不在本次 GUI 验收里偷偷改 Grid checker / 文档空白——否则会污染验收基线、让「为发布顺手改」混入。
- **不推进 C4/C5**：Terminal 物理 suspend / 真机资源释放实测未做，不上报。
- **不拆 Database / Git**：当前 C1（always-loaded 面板），抽可选能力包达 C3 = Debt-8E-5/6，交后续 train。
- **不 merge / 不 push**：由用户/A0 在 GUI PASS 后统一执行（本文件 §4）。

---

## 6. 后续阶段（按顺序，均独立）

1. **Human Acceptance（本阶段，human）** — 真实 GUI/runtime 验收。
2. **Release Closeout（human PASS 后）** — 文档 + tag `capability-modularization-v1-pass` + ff-only merge + push。
3. **Pre-Merge Historical Debt Closure（独立专项）** — 修 M4/M5 RED，不混入 Capability v1 验收基线。
4. **后续 train（可选）** — C4/C5、Database/Git 抽 C3（Debt-8E-5/6）等，均不在本阶段范围。
