# A11 · M5-W18-R3B 最终收口报告（Final Closeout）

- Lane：A11；分支 `codex/m5-w18-a11`；BASE `origin/master d96b9b5`
- 时间：2026-09-08 18:00 CST
- 角色：R3B 验收打包（manifest / matrix / checklist / progress / closeout）
- 边界：`NO_PRODUCT_CODE`、`NO_PUSH`；仅写 `logs/research/M5-W18/A11-R3B-*`，未改其他 lane 文件（同步的 A10 自检器为 A11 校验工具依赖，见 §6）

---

## 1. 结论（TL;DR）

| 项 | 结果 |
|---|---|
| A1 R3B 修正原型 | **已交付** `codex/m5-w18-a1 @ 60d887d`（3 文件 +1039 行，工作树干净） |
| A10 独立复核 | **已完成并通过** `codex/m5-w18-a10 @ 1a78857`，§7 判定 A1 交付物 **0 HIGH / 0 MEDIUM** |
| A11 矩阵实跑（对最终 A1 HTML） | **32 PASS / 0 FAIL / 1 GAP_CONFIRMED / 0 WARN / 0 NOT_RUN** |
| R3B 九项发现 | **8 项 GAP_CLOSED**，1 项（R3B-02 字体）待 A0 裁定，R3B-09 按设计 RESERVED_NATIVE |
| 包状态 | `PROVISIONAL_PENDING_A0_ADJUDICATION` |

R3B 卡要求的两项上游前提（A1 交付修正原型 + A10 完成独立复核）**均已满足**。唯一未清项是 donor 字体的口径分歧，属 A0 裁定事项，不是 lane 未交付。

---

## 2. Lane SHA 确认（全部工作树干净，均未 push）

| Lane | 分支 | HEAD | R3B 交付 |
|---|---|---|---|
| A1 | `codex/m5-w18-a1` | `60d887d` | 修正后自包含原型 + 报告 + checkpoint |
| A2 | `codex/m5-w18-a2` | `bfe8e86` | 规范几何与最小尺寸证据 |
| A3 | `codex/m5-w18-a3` | `8d8719a` | 折叠/恢复与树语义（对齐 A0 裁定） |
| A4 | `codex/m5-w18-a4` | `5d9e447` | 持久化契约对账（29/29 纯测试） |
| A5 | `codex/m5-w18-a5` | `8d4d41d` | 数据库原型对齐规范几何 + A3/A4 语义 |
| A6 | `codex/m5-w18-a6` | `4b59339` | 命令注册表 + 快捷键冻结 + 无障碍（R3B 交付提交 `ac3cb18`） |
| A7 | `codex/m5-w18-a7` | `b96cd51` | Git 参考图修正（SSOT `cee14e9`，0/4/10） |
| A8 | `codex/m5-w18-a8` | `ba92110` | 规范几何 + 视觉/无障碍态修订 |
| A9 | `codex/m5-w18-a9` | `d17091c` | 14 单元 Git 交互安全矩阵 + 6 条冻结边界 |
| A10 | `codex/m5-w18-a10` | `1a78857` | 闭环审计（修 5 处误报）+ A1 R3B 复核 |
| A11 | `codex/m5-w18-a11` | `aa7f443`(刷新后待提交) | 本收口包 |

**Peer SHA 漂移（新发现，交 A0 知悉）**：A1 checkpoint 记录的消费清单为 A5=`3ff1f8b`、A6=`2bce4e3`，但两 lane 当前 HEAD 已推进到 A5=`8d4d41d`、A6=`4b59339`。即 **A1 消费的是被取代的 peer 版本**。A1 的三件交付物内容仍通过 A10 与 A11 双重核验，故不判定为缺陷；若 A0 要求严格版对齐，A1 需按新 SHA 复核后更新 checkpoint 引用。

---

## 3. 测试结果

### 3.1 A11 验收矩阵（权威：直接解析 A1 最终 HTML）
```
A11-R3B TALLY: 32 PASS / 0 FAIL / 1 GAP_CONFIRMED / 0 WARN / 0 NOT_RUN
OPEN R3B FINDINGS: R3B-02
GATE: FAIL
```
明细：六尺寸帧 6/6 PASS；GEO 公式六尺寸全过；Git 14 单元 + amend + reset-revert 全 PASS；A-aria（189 个 role/aria-*）与 A-focus PASS；C-hide / C-strip / C-restore PASS；H-brand PASS；K-binding PASS；M-pin（`cee14e9`）PASS；**A10-gate PASS（exit 0）**；仅 **H-font GAP**（`Segoe UI`、`Cascadia Code`）。

### 3.2 A10 独立复核
- `A10-R3B-closure-20260908.md` §7：**A1 九项 R3B 修正全部为真实修正，门禁对 A1 交付物零发现（0 HIGH / 0 MEDIUM），A1 `FINAL_READY_FOR_A10_REVIEW` 成立**；P3/P4/P5/P6/P7 对 A1 均 PASS。
- A10 自包含门 `A10-R3-prototype-selfcontainment-check.py`：对 `A1-R3B-prototype.html` **exit 0 / PASS**（A11 已独立复跑确认）。
- A10 闭环审计脚本本身仍 `GATE FAIL (HIGH=17 MEDIUM=18)`，原因是 **A1 的 `60d887d` 尚未合入 master**，脚本仍解析被 R3B 取代的 R3 旧件（`A1-R3-prototype.html`、`A7-R3-git-workflow-reference-map.md`、`A4-R3-shell-state-persistence-contract.md`）。A10 已明确记载此为**集成前过渡态，非 A1 缺陷**；A0 集成后自动消失。

### 3.3 Peer 门限（各 lane 自报，A11 未独立复跑，仅记录出处）
| Lane | 门限 | 结果 | 出处 |
|---|---|---|---|
| A2 | `A2-R3B-measure.py` G1–G7 | PASS | A2 `bfe8e86` 提交说明 |
| A3 | 状态机 58 断言 | PASS | A1 checkpoint 消费记录 |
| A4 | 纯测试 29/29 | PASS | A4 `5d9e447` 提交说明 |
| A5 | 78-check 模型 | PASS | A1 checkpoint 消费记录 |
| A6 | registry-audit | PASS | A1 checkpoint 消费记录 |

> 口径声明：A11 只独立复跑了 A10 自包含门与自有矩阵；上表 peer 数字为各 lane 自报，未由 A11 重新执行。

---

## 4. R3B 九项发现状态

| ID | 标题 | 状态 | 证据 |
|---|---|---|---|
| R3B-01 | Git 14 单元覆盖 | **GAP_CLOSED** | 14 个 `git-unit` 标签 + amend/reset-revert；A11 G-* 全 PASS；A10 P4 PASS |
| R3B-02 | 原型卫生（donor 名称 + 字体） | **OPEN_A0_ADJUDICATION** | 名称已清零（H-brand PASS、A10 P1 PASS）；**字体 `Segoe UI`/`Cascadia Code` 仍在 CSS** |
| R3B-03 | 折叠语义 | **GAP_CLOSED** | Collapse All 含 pinned + Restore Layout 快照；C-hide/C-strip/C-restore PASS |
| R3B-04 | 无障碍 | **GAP_CLOSED** | 94 role / 114 aria / 78 tabindex / `:focus-visible`；A-aria、A-focus PASS |
| R3B-05 | 几何 SSOT | **GAP_CLOSED** | 60/24/28 硬常量；GEO 六尺寸全过；A10 P2 对 A1 无发现 |
| R3B-06 | 快捷键冲突 | **GAP_CLOSED** | `Ctrl+K`→`browser.focusAddressSearch`、`Ctrl+Shift+P`→palette；K-binding PASS |
| R3B-07 | 参考版本 | **GAP_CLOSED** | `cee14e9` SSOT，2896562e 降级；M-pin PASS（A7 `b96cd51` 已改为 0/4/10） |
| R3B-08 | 最小窗 900×600 | **GAP_CLOSED** | 六尺寸全覆盖、800×600 剔除；S-* 6/6 PASS |
| R3B-09 | 原生桌面可行性 | **RESERVED_NATIVE** | 按 A0 出口门禁显式保留给 user/A0，不计入 A11 |

---

## 5. 遗留项（交 A0）

| # | 项 | 性质 | 建议动作 |
|---|---|---|---|
| L-1 | R3B-02 字体口径：**A11 判 donor 字体违规 / A10 判 PASS** | 严格度分歧，非事实分歧（双方一致：donor 名称已清零） | A0 裁定系统字体是否满足「中性字体栈」。若采纳严口径，A1 改一行 CSS（`system-ui, -apple-system, sans-serif` / `ui-monospace, monospace`）即 GAP_CLOSED，A11 随即将状态翻 `FINAL` |
| L-2 | A10 闭环审计 `GATE FAIL` 指向被取代的 R3 旧件 | 集成前过渡态 | A0 集成 A1–A9 的 R3B 提交后自动消失，无需 lane 动作 |
| L-3 | A1 消费了被取代的 peer SHA（A5 `3ff1f8b`→`8d4d41d`、A6 `2bce4e3`→`4b59339`） | 文档引用滞后 | 若 A0 要求严格版对齐，A1 复核后更新 checkpoint 引用 |
| L-4 | R3B-09 原生 WebView（子窗焦点/遮挡/resize） | 研究边界外 | 真机 GUI 验证，归 user/A0 |
| L-5 | A1 自报的 U1–U6（F05 体积 / F12 WebView owner / F01 license / W19 前置） | 跨波次 | 交 A0 在 W19 开放前裁决 |

---

## 6. A11 本波改动（仅研究产物，零产品代码）

1. `A11-R3B-acceptance-matrix.py` — 修正五处校验器缺陷，使其正确解析 A1 新版标记：
   - 帧提取范围由 `.frame` 扩到整个 `<body>`（A1 把 shell / Git 网格 / 折叠演示放在 `mode-frame` 之外，原实现产生 20+ 假阴性）
   - 尺寸识别增加 `size-title` 文本回退（A1 改用 `size-card` + `size-title`）
   - K-binding 改用真实绑定 token（`browser.focusAddress` / `openCommandPalette`），消除描述性注释导致的假阳性
   - 折叠判定改为 CSS + 文本双轨
   - `run_a10_checker` 补传 A1 路径（`resolve()` 返回的是**文件内容**而非路径，原实现把 HTML 文本当路径传给 checker，恒 FAIL）
2. `A10-R3-prototype-selfcontainment-check.py` — 由 A10 当前版本（`1a78857`）同步，替换 A11 树内 4062B 的过时副本（原副本误报 exit=1）。
3. `A11-R3B-integration-manifest.json` — 刷新 11 个 lane SHA、门限记录、九项发现状态与遗留项。
4. `A11-R3B-run-20260908.out` — 最终运行记录（32 PASS / 1 GAP）。
5. 本文件 `A11-R3B-final-closeout-20260908.md`。

---

## 7. 出口门禁判定

R3B 卡对 A11 的要求：`0 FAIL / 0 GAP_CONFIRMED / 0 未解释 WARN / 0 NOT_RUN（原生证据除外）`。

当前 **GAP_CONFIRMED = 1（R3B-02 字体）**，故 A11 **不自封 `FINAL`**，维持 `PROVISIONAL_PENDING_A0_ADJUDICATION`。
- 若 A0 裁定系统字体可接受 → A11 将 H-font 转 WARN、状态翻 `FINAL`，无需任何 lane 返工；
- 若 A0 采纳严口径 → A1 改一行 CSS 后 A11 复跑即 `FINAL`。

两条路径都不阻塞 A0 集成与用户审查。
