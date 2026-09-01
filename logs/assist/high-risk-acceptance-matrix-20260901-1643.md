# 高风险验收矩阵（M1-4/M1-6/M1-8/M1-9/M2-4/M2-8/M3）

> 目的：把各 assist/minicard 的反向用例与证据要求汇总，供强模型领取后直接写验收脚本。
> 本文件仅文档，不实现、不移动 NEXT、不签主线 PASS。

## 1. M1-4 默认浏览器接入
- 验收脚本草案：`scripts/accept-m1-4.sh` —— 构建 → `xdg-settings get default-web-browser` 期望值 → 反向 `xdg-open file:///etc/hostname` 应不导航 + toast。
- 反向用例：非 http(s) scheme 拒收；冷启动 URL 不丢。
- 证据要求：`cargo build --release` 0 + `npm run build` 0 + `scripts/pre-merge.sh` ALL_PASS + 手动 xdg-settings 截图 + 反向 toast 日志。

## 2. M1-6 Git 写能力
- 验收脚本草案：`cargo test -p mvp-browser-os git_write` 跑 T-gw-1..6 域模型用例。
- 反向用例：未确认 commit / 白名单外 reset / /etc repo_id / force push / token 泄露。
- 证据要求：6 用例全绿 + `AuditEntry` 抽样不含量明文凭 + `cargo clippy` 0 新增 warning。

## 3. M1-8 请求瀑布
- 验收脚本草案：vitest 单测 `resources_eval.js` 采集函数（mock `performance`）→ 反向 token 脱敏 / 截断 oldest / 不落盘断言。
- 反向用例：`?token=` 明文不现 / >500 条截断 / `file://` 不采集 / 关开关重启清空。
- 证据要求：`npm run test`（vitest）相关用例绿 + 隐私过滤代码审查 + 不写 audit/成果库核查。

## 4. M1-9 浏览器会话
- 验收脚本草案：集成测试 `tab_new`→`save_session`→`flush-sessions`→读 `sessions.json` 断言；反向退出竞态/重入。
- 反向用例：flush 在 ExitRequested 前完成 / 预览 >512B 截断 / 恢复重复 tab 跳过 / 删活跃会话 kill。
- 证据要求：`sessions.json` 含 saved=true 项 + 退出 updated_at 更新 + `ps` 无残留 tab + 原子落盘（tempfile+rename）核查。

## 5. M2-4 脚本执行通道
- 验收脚本草案：`cargo test run_script` 跑 T-ex-1..6（含 `DIR="/etc; rm -rf /"` 注入、超时、orphan、10MB 截断）。
- 反向用例：注入不执行 rm / 超时 killed / 进程组全回收 / 输出截断 / 未声明占位不替换。
- 证据要求：6 用例绿 + `ps` 验证无 orphan + `SCRIPT_OUTPUT_MAX_BYTES` 截断标记 + 审计 `script_killed`。

## 6. M2-8 工具 WebView 隔离
- 验收脚本草案：`cargo build --release` + release 包内启 5 工具 → 工具内 `fetch(http)` 应被 CSP 阻 + `destroy-tools` 退出无残留。
- 反向用例：工具调 run_script 拒 / 外联 fetch 阻 / 恶意 script src 拒 / 越目录 canonicalize 失败。
- 证据要求：`tools.json` 权限审查（无 shell/fs）+ CSP 违例审计 + 错误页渲染核查 + §4.3 目录偏差已修（先建 `src/tools/`）。

## 7. M3 终端增强
- 验收脚本草案：`cargo test` M3-1 输出解耦 + M3-2 orphan 回收（`ps`）+ M3-3 退避（mock 断线）；前端 vitest E1/E2。
- 反向用例：10MB 不卡 / 孙进程全回收 / 退避封顶 30s 可取消 / E1≤40 / E2 去抖不重建 PTY。
- 证据要求：性能基线不退 >10%（§9）+ `ps` 无 orphan + 退避日志 + E1/E2 前端用例绿。

## 跨任务统一证据规则
1. 每个新命令同步改 `permissions/*.toml`（§10 风险：漏 ACL 静默拒绝）。
2. 出网/落盘/执行动作写 `audit.json`；凭据走 `KeyringStore`。
3. 阻塞任务经 `std::thread::spawn`/可取消；退出统一 `ShutdownCoordinator` 收口。
4. `cargo clippy --all-targets` 零新增 warning；前端体积涨幅 ≤15%。
5. 全部不移动 `NEXT`、不代签主线 PASS（由 AI:DEEP 裁决）。
