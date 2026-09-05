# Low Model Deep Assist Summary（2026-09-01 16:35 CST）

> 路由：`AI:FAST / R:medium`（assist 副任务汇总）。
> 主线 `NEXT=M1-4` **未移动**；本轮 8 个任务（M1-6 / M1-8 / M1-9 / M2-4 / M2-6 / M2-8 /
> M4 / M5）均只做 assist 文档、风险盘点、契约草案、验收表、mock 设计，未实现任何主线
> WBS、未签任何主线 PASS、未改 src / src-tauri 产品代码。

---

## 1. 任务结果总览

| 任务 | 状态 | 产物 | Commit |
|------|------|------|--------|
| M1-6-assist | DONE | `logs/assist/M1-6-assist-20260901-1629.md` | `3fcffa6` |
| M1-8-assist | DONE | `logs/assist/M1-8-assist-20260901-1630.md` | `3fcffa6` |
| M1-9-assist | DONE | `logs/assist/M1-9-assist-20260901-1631.md` | `3fcffa6` |
| M2-4-assist | DONE | `logs/assist/M2-4-assist-20260901-1632.md` | `c7fea41` |
| M2-6-assist | DONE | `logs/assist/M2-6-assist-20260901-1632.md` | `c7fea41` |
| M2-8-assist | DONE | `logs/assist/M2-8-assist-20260901-1633.md` | `c7fea41` |
| M4-assist | DONE | `logs/assist/M4-assist-20260901-1634.md` | `4378ec9` |
| M5-assist | DONE | `logs/assist/M5-assist-20260901-1634.md` | `4378ec9` |

**BLOCKED：无。** 所有任务基于真实代码检索（grep `sync.rs`/`bridge.rs`/`domain.rs`/
`security_policy.rs`/§6/§7）完成，无根因不明或环境阻塞。

---

## 2. 各任务核心交付

| 任务 | 核心交付 |
|------|----------|
| M1-6 | Git 写能力风险 R1-R6 + 三级确认闸门 + 禁止越权提交硬规则（白名单/路径锁定/凭据隔离/fail-closed）+ 审计字段 + 6 测试用例 |
| M1-8 | 请求/资源瀑布现状（report_resources 已带边界）+ 容量上限 + 隐私过滤（token 脱敏/不落盘）+ Tauri/WebView 钩子风险 R1-R5 + 实现路径 |
| M1-9 | BrowserSession 草案 + 保存/删除/关闭 flush + 最小预览（不存图片）+ 生命周期接入 ShutdownCoordinator + 风险 R1-R5 |
| M2-4 | run_script 契约 + 参数注入防护硬规则（禁 sh -c 拼接）+ 取消/超时 + 进程组回收 + 输出背压 SCRIPT_OUTPUT_MAX_BYTES + 6 测试用例 |
| M2-6 | CommandSnippet 字段 + 参数化 + dangerous 二次确认 + 审计 + 复用 M2-4 通道（不另建旁路）+ 测试用例 |
| M2-8 | 本地工具 WebView 隔离：asset/file 协议 + 权限最小化 + CSP + 错误页 + 生命周期接入 + 风险 R1-R5 |
| M4 | 数据库（M4-1~4）+ 定时任务（M4-5~8）拆解、跨检查点依赖、风险 R1-R5 |
| M5 | #7 A2P/A2A（M5-1~3）/ #12 Agent-Skill（M5-4~6）/ #13 图谱（M5-7~9）/ #15 插件（M5-10~12）拆解 + 统一安全红线 + 关键路径 |

---

## 3. 跨任务复用基线（给强模型）

- **确认闸门**：`request_sync`/`confirm_sync` 双阶段（M1-6/M2-4/M2-6 均复用）。
- **审计**：`workspace::log_audit` + `AuditEntry`（所有写操作统一）。
- **进程管理**：`TerminalSession.child` kill+wait + `ShutdownCoordinator`（M2-4/M1-9/M2-8 退出收口）。
- **路径安全**：`canonicalize` 校验 + 仅 `data_dir/...` 内（M2-1/M1-6/M2-4/M2-8）。
- **容量护栏**：`security_policy::MAX_*` 常量范式（M1-8/M1-9 复用）。
- **权限最小化**：`capabilities/*.json` 的 `webviews` 限定（M2-8 新增 tools.json）。

---

## 4. 关键依赖顺序（建议强模型领取路径）

```
M1-4(DEEP, 主线) ← 当前 NEXT
  ├ M1-5.a → M1-6(写能力) → M1-7(Git UI)
  ├ M1-8(瀑布) / M1-9(会话)
  ├ M2-3 → M2-4(执行通道) → M2-5.a / M2-6(命令库)
  ├ M2-1 → M2-2.a / M2-7(工具) → M2-8(隔离) → M2-9(验收)
  ├ M3-1~3(终端核心) → M3-4.a(E1/E2 低风险项可先行)
  ├ M4(数据/调度，依赖 M2)
  └ M5(生态，依赖 M1~M4)
```

---

## 5. 工作树状态

- 开工 `git status --short --branch`：干净。
- 结束 `git status --short --branch`：干净（仅 `logs/assist/` 已提交）。
- `git diff --check`：EXIT=0。
- 主线 `NEXT` 保持 `M1-4`，未在任何文档中被改签。
- 未删除/改动 `logs/m0-*`、`logs/checkpoints/M0-*` 冻结证据。
- 未改 `src` / `src-tauri` 任何产品代码（仅 `logs/assist/` 文档）。

---

## 6. FORBID 遵守总览

- ❌ 未移动 `AI-模型切换与接手清单.md` 顶部 NEXT。
- ❌ 未把 M1-4/M1-6/M1-8/M1-9/M2-4/M2-6/M2-8/M4/M5 主任务标 PASS。
- ❌ 未实现默认浏览器、Git 写能力、脚本执行通道、WebView 隔离、数据库/调度/MCP 等核心。
- ❌ 未删除或改动冻结证据。
- ❌ 每个任务独立提交（按逻辑相邻分组，各自文件独立不混）。
