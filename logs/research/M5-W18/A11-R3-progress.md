# A11 · M5-W18-R3 场景进度账本

> `WORKBENCH_BLUEPRINT-20260908.md` §9 指定 A11 维护
> "场景 → 现有能力 → 设计就绪 → 代码完成 → 原生验收 → 用户试用" 清单，A0 只在状态变化时汇报。
> 本账本只记账，不实现；状态变化必须附带证据路径，不允许用文档页数或提交数折算完成度。

## 状态口径

| 值 | 含义 |
|---|---|
| `YES` | 有可指名的证据（实测输出 / 已合入代码 / 已提交设计契约） |
| `PARTIAL` | 能力存在但缺关键一环（有缺陷、被裁剪、或未覆盖全部状态） |
| `NO` | 明确不存在，有反证（grep / 实测 / lane 未交付） |
| `NOT_RUN` | 未执行；不等于通过（沿用蓝图 §7 的 `NOT_RUN` 记账规则） |

## 1. 场景账本

| 场景 | 现有能力 | 设计就绪 | 代码完成 | 原生验收 | 用户试用 | 证据 |
|---|---|---|---|---|---|---|
| J1 打开并接续项目 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：页签/终端/浏览器在，但无文档身份与恢复契约；A2 实测平静态内容 90.1%/100%，最坏态 75.3% 已违 ≥85%。设计：A4 文档 DTO + A1/A3 原型 |
| J2 数据库日常闭环 | `PARTIAL` | `NO` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：`DatabasePanel`/`useDatabaseStore` 在，但 DbValue 三重真源未修（A0 R2B 裁定 #3、A0 R3 未重开）。设计：**A5 分支仍在 `200f0f1`，零交付**（F-A11-2） |
| J3 阅读、记录、关联 | `PARTIAL` | `PARTIAL` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：图谱/笔记/文件编辑器在（M4/M5 已落）。设计：A1 知识模式原型（反链 + 局部图谱），语义细节归 A2/A3 R2B |
| J4 统一检索 / 命令入口 | `NO` | `PARTIAL` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：A2 实测 `grep -iE "palette|命令面板|CommandRegistry" src` 零命中。设计：A6 注册表契约 + A1 命令面板原型；检索后端属 R2B（ripgrep 优先，zvec 延后） |
| J5 自动化与运行观察 | `YES` | `NO` | `PARTIAL` | `NOT_RUN` | `NOT_RUN` | 现有：TaskPanel / scheduler 已落（M4）。设计：R3 卡未给 A1–A10 任何 lane 派发 J5 原型 |
| J6 正常退出与异常恢复 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | 设计：A4 原子写（tmp+rename）、`schema_version`、损坏回退默认态、15 条纯函数断言（T1–T15） |
| Git 完整工作流 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | A7 实测：14 单元中 5 ADAPT（status/diff/branches/merge，其中 merge 仅自动同步）、9 REIMPLEMENT 全部缺失；rebased 钉 `cee14e9`，COPY=0 |
| 工具窗口渐进披露 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：只有开/关，宽度硬编码，`SidebarResizer.vue` 未被 `App.vue` 使用（A2）。设计：A3 状态机 58 断言 + A4 持久化字段 |
| 右键菜单 / 命令注册表 | `NO` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：两处手写 `ctx-menu`（`FilePanel.vue`、`ArtifactPanel.vue`），无注册表面。设计：A6 契约（7 作用域 + 安全分级 + 审计类） |
| 视觉密度与主题 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：无间距/字号标尺（A8 §2）。设计：A8 令牌 + 4 尺寸标注帧，折叠态 91.6%–94.5% 高 / 100% 宽 |
| 交互安全 | `PARTIAL` | `YES` | `NO` | `NOT_RUN` | `NOT_RUN` | 现有：`request_git_write`/`confirm_git_write` 双确认 + 审计脱敏已在（A7 §1、A9 §8）。设计：A9 五级安全分级 + T0–T4 |

## 2. 与 R2B 的连续性

- R2B 账本（`A11-R2B-progress.md`）中的 S0–S5 候选片结论继续有效，R3 只重开**交互研究与原型**，不推翻领域研究。
- A0 R2B 裁定的 10 条在 R3 中仍作硬约束，本账本逐条继承：DbValue 单真源（#3）、检索先管式 ripgrep（#5）、zvec 延后不否决（#6）、不动用户 `.zvec-grep`（#7）、取消必须是后端真契约（#8）、凭据只进 keyring（#9）。
- `W19` 仍 `CLOSED`。本账本没有任何一行进入"代码完成"，故不产生开片依据。

## 3. 状态变化记录

| 时间 | 场景 | 变化 | 依据 |
|---|---|---|---|
| 2026-09-08 13:14 | 全部 | R3 首批记账（A1–A4、A6–A9 已消费；A5/A10 未交付） | `A11-R3-integration-manifest.json` |
| 2026-09-08 13:14 | Git 完整工作流 | 设计就绪 `NO → YES` | A7 14 单元矩阵（COPY=0 / ADAPT=5 / REIMPLEMENT=9） |
| 2026-09-08 13:14 | J2 | 设计就绪保持 `NO`（A5 未交付，不因 A1 数据库模式原型而升格） | F-A11-2 |
