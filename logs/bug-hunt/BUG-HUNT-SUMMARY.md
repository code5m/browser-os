# Bug Hunt 综合汇总（12 路只读猎错）

- 项目：`mvp-browser-os-v3`（Tauri2 + Vue3 桌面浏览器 OS）
- HEAD：`052b18a`，工作树含其它 lane 未提交改动（以工作树为准）
- 时间：2026-09-07
- 方法：12 个只读智能体（B1–B12），按"失效模式"切片，零产品代码改动、不编译、零冲突。
- 工具限制：运行环境无写文件权限，各 lane 报告以对话文本返回，未自动落盘。本报告为去重汇总。
- 截断说明：**B8 / B10 / B12** 三路输出超长被平台截断，已捕获前部关键发现（见各 lane 行），完整版建议重跑补全。

---

## 一、严重度总览（去重后）

| 严重度 | 数量 | 说明 |
|---|---|---|
| P0 崩溃/数据丢失/安全绕过 | 5 | 死锁复燃、进程挂起、数据静默全丢、调度重复执行、剪贴板明文落盘 |
| P1 功能失效/安全 | 14 | 命令执行绕过、XSS、未授权命令、类型漂移、孤儿进程、OOM 等 |
| P2/P3 健壮性/加固 | 十余项 | 脱敏不全、无清理、无限重启、越界坐标等 |

> 多处"同一根因"被多 lane 独立命中（已合并）：`collect.js` 三命令未授权（B1-2/B9-3/B12-03）。`open_tool` 的主窗口 ACL 漏配（B1-1/B12-01）已于 2026-09-07 修复，并纳入一致性与工具策略门禁。

---

## 二、P0 必修清单（Top 5，按风险）

1. **B9-1 · 子 webview 死锁根因复燃（P0 卡死）**
   `tauri-browser-tabs/.../platform/linux.rs:83-84` 离屏分支重新调用 `gtk_webview.hide()`，违反 `PROJECT-RULES.md` 规则 3.5（对渲染中 WebKitGTK 子 webview 调 hide = 历史死锁根因）。切页签/布局守护 400ms 重放即可触发主线程卡死。
   → 修复：删除 `set_child_visible(false)+hide()`，仅保留「`gtk_fixed_move` 到屏幕外 + 原尺寸 `size_allocate` + `queue_draw`」。

2. **B3-1 · 脚本正常退出后 supervisor 永久挂起 + 并发槽泄漏（P0 挂起/功能失效）**
   `src-tauri/src/script_runner.rs:657-667` 正常退出分支只 `join_output_readers` 不杀进程组；脚本内 `&` 后台化子进程继承 stdout fd → reader 永不 EOF → `finish` 永不执行 → 该 run 永远 `Running`。连续 8 次后 `MAX_CONCURRENT_RUNS` 触顶，脚本功能实质失效。
   → 修复：正常退出分支先 `terminate_group(pgid)` 再 join；或给 reader join 加超时。

3. **B4-1/B4-2 · workspace 系非原子写 + 反序列化静默默认空（P0 数据丢失）**
   `workspace.rs:57/374/426/396` 的 artifacts/repos/bookmarks/audit 用 `fs::write` 覆盖写（未用 `atomic_write`）；加载侧 `unwrap_or_default()` 静默返回空。崩溃中损坏 → 下次保存把空列表覆盖写回 → **数据无痕蒸发**（audit 因热路径最致命）。
   → 修复：统一改 `atomic_write`；加载失败改「`.corrupt` 备份 + 告警」而非静默清空（对齐 `tasks.rs` 范式）。

4. **B5-1 · 调度崩溃窗口导致任务重复执行（P0 重复执行）**
   `scheduler.rs:413-416` 内存写 `last_fired_at` 晚于实际 `start_run` spawn（651 行），整轮末尾才 `save_tasks_at`（453）。窗口内崩溃/掉电 → `tasks.json` 上 `last_fired_at` 仍旧 → 重启后该 slot 重复触发。
   → 修复：先判重落盘 `last_fired_at` 再 spawn（牺牲偶尔漏跑换绝不重复）。

5. **B11-1 · 系统剪贴板明文落 localStorage + 明文展示（P0 敏感泄露）**
   `useSystemStore.ts:18/27-31/86` 把剪贴板历史（最多 50 条）`JSON.stringify` 明文落盘、跨重启留存；`ClipboardPanel.vue:31/34` 明文渲染。复制的密码/token 任何人可读文件获取。
   → 修复：默认不落盘，或落盘/展示前对 `item.text` 跑 `redactSecrets`。

---

## 三、P1 高价值清单（跨 lane 去重）

- **B2-1（P1 命令执行绕过）** `security_policy.rs:154/200` `check_launch_target` 仅按 6 个 shell 的 basename 黑名单 + 10 个元字符拦截；`/usr/bin/env bash -c "id"`、`python3 -c "import os;os.system('id')"`、或指向 shell 的符号链接均可绕过。门禁只查"是否调用了函数"看不到拦截完整性。
- **B2-2（P1 存储型 XSS 风险）** `security_policy.rs:344` `check_html` 全仓库仅单测调用、生产零接入 → 设计的 HTML 净化闸门从未生效。
- **collect.js 三命令未授权（宫格采集/终端失效，同源）** `injected/collect.js:120/150/167` 调 `collect_selection/save_note/request_open_terminal`，但 `remote-collect.toml:8` 只放行 `report_*` → 远端 webview 右键菜单报 not allowed。命中 `PROJECT-RULES` 规则 3.6。需补 ACL（或让 collect.js 在远端跳过）。（B1-2 / B9-3 / B12-03 同源）
- **已修复：open_tool 缺 ACL（工具箱打开失效，同源）** 主窗口 `default-commands.toml` 已允许 `open_tool`；三源一致性检查不再将其列为允许漂移，工具策略自检加入 ACL 缺失变异。（B1-1 / B12-01）
- **B3-2（P1 孤儿进程）** `script_runner.rs:657-667/911-912` 正常退出未杀进程组，条目已终态被 `kill_all_running` 跳过 → 后台子进程连应用退出都不回收。
- **D25（P1）** `terminal.rs:330-336/473-475` `on_channel_dead` 只 `kill_group` 不 `wait` → 僵尸 shell + `terminals` 表项残留（挂账债，仍未修）。
- **B5-2（P1 状态不一致）** `scheduler.rs:309/453` 与 `bridge.rs:1011/1052/1081` 对 `tasks.json` 各自 load-modify-save 无共享锁 → 互相覆盖，用户编辑可静默回滚。
- **B6-1（P1 OOM）** `tools.rs:162` `read_user_tool` 用 `read_to_string` 整文件读入用户可控的 `workspace/tools/*.html`，无大小上限 → 单文件即可 OOM。
- **B9-4（P1 空白）** `useBrowserStore.ts:458-468` `closeGridAll` 后未复位活动页签坐标、`mainView` 不变、watch 不触发 → 浏览器区空白，须手动切视图恢复。
- **B8-1（P1 解码全错，截断前已确认）** `types.ts:601-607` `DbValue` 用 PascalCase（`{Text}`），Rust `domain.rs:1093` 用 snake_case（`{"text":...}`）→ 数据库面板所有单元格渲染错误。
- **B8-2（P1 字段漂移，截断前已确认）** `SkillDef`/`AgentDef` 的 `display_name`/`system_prompt` 等 TS 用 camelCase、Rust 用 snake_case，逐字段漂移。
- **B11-2（P1 错误原文回显）** 10+ 处 `e?.message ?? e` / `String(e)` 未接 `redactSecrets`（`useWorkspaceStore`/`useSystemStore`/`useDatabaseStore.describeError`/`useAgentStore.fail`/`useBrowserStore` 注入脚本/`App.vue` 等），后端已脱敏但前端未二次接力。
- **B11-3（P1 XSS）** `markdown.ts:43` 链接替换 `$2` 取自 `https?://[^)]+` 未转义引号 → `[x](https://a.com/"onmouseover="alert(1))` 经 `v-html`（FileEditor.vue:29 / FilePanel.vue:52）注入属性执行。
- **B11-7（P1 落盘）** `useHomeStore.ts:98/180/71` + `ActivityBar.vue:186` 把快捷方式（含 app 命令体如 `mysql -uroot -pPASS`）与最近目录绝对路径明文落 localStorage。

---

## 四、P2/P3 关键项（择要）

- **B2-3** `bridge.rs:3608-3632` create_file 用原始 `p` 写入、写后才复检 → TOCTOU 越界写。
- **B2-4** `security_policy.rs:380/446` URL/审计脱敏仅 query-key 白名单，缺 value-pattern/path 扫描；审计 `sanitize_audit_text` 只截断不脱敏。
- **B6-2** `crashlog.rs:161-198` `crash.log` 只 append 永不清理 → 可撑爆磁盘（配合 B6-3）。
- **B6-3** `grid_process.rs:554-598` 宫格子进程崩溃后无限重启、无退避/熔断。
- **B6-4** `grid_process.rs:740-748` 退出不清理 UDS socket 文件，累积残留。
- **B9-2** `linux.rs:37` 离屏 -30000 被乘 scale，HiDPI 下超 X11 int16。
- **B9-5** `commands.rs:228/257` `set_visible/create_tab` 仍调 `webview.hide()`（潜伏违规）。
- **B10（截断）** 已知前部：ActivityBar/StatusBar 直接访问 `stats.main.rss_mb` 未守卫（后端缺 `main` 即 TypeError → onErrorCaptured 白屏）；MainArea 兜底 `v-else` 配对到 `editor` 的 `v-if` 而非主链 —— **疑似 A7 新引入回归**，需补全确认。
- **B11-4/5/6** `target="_blank"` 缺 `rel=noopener`（markdown/Artifact/Resource）；ResourcePanel `r.absolute` 直接绑 href 无 scheme 校验；UnifiedTabBar 原始 URL/路径渲染含 token。
- **B12-02** 三源（main.rs 注册 ∩ ACL ∩ bridge.ts 调用）已增加自动化一致性门禁；`open_tool` 漂移已关闭。其余 Agent/Skill 契约占位与远程采集权限边界仍为已知项。
- **B12（截断）** 命令集合三方差集表已捕获：A−B=`open_tool`；C−A=skill/agent 11 条契约占位（bridge.ts:392-429）。

---

## 五、各 lane 一句话结论

| Lane | 范围 | 结论 |
|---|---|---|
| B1 | 命令总线 | `open_tool` 主窗口 ACL 漏配已修；grid-child 三命令未授权；`read_file/list_dir` 未校验根目录 |
| B2 | 安全策略 | launch 黑名单可绕过；`check_html` 死代码；脱敏单扫不全；无 fail-open |
| B3 | 进程并发 | 脚本退出挂起(P0)+孤儿；D25 仍存；kill 无超时 |
| B4 | 持久化 | workspace 非原子写+静默清空(P0)；save_image 两阶段不一致；ensure 吞错 |
| B5 | 调度 | 崩溃窗口重复执行(P0)；tasks.json 无锁覆盖；R-4 已落实、契约基本遵守 |
| B6 | 资源退出 | tools HTML 无上限 OOM；crash.log 无清理；grid 无限重启；像素炸弹转前端 |
| B7 | 扩展面 | **LOCK 是真锁**（无执行体/真 feature gate）；仅反序列化无大小上限(P2 DoS) |
| B8 | 状态与桥 | DbValue/Skill/Agent 类型漂移(P1)；invoke 未 catch；并发响应覆盖（报告截断） |
| B9 | 浏览器 | **规则3.5 违反复燃(P0)**；collect.js 未授权；closeGridAll 空白 |
| B10 | 面板 UI | 异步兜底覆盖度；解析崩溃/XSS；发现 MainArea v-else 疑似回归（报告截断） |
| B11 | 前端隐私 | 剪贴板明文落盘(P0)；错误回显未脱敏；markdown XSS；F-3 已修、F-2 点已修但扩散 |
| B12 | 门禁 | 三源命令集合已有一致门禁；open_tool 漂移已关闭；collect.js 三命令漏 ACL（报告截断） |

---

## 六、后续建议

1. **立即修 P0**：B9-1（删 hide）、B3-1（杀进程组/超时）、B4 原子写+损坏备份、B5-1（先判重后执行）、B11-1（剪贴板不落盘）。
2. **补 ACL 漂移**：`open_tool` 已进入主窗口权限且由三方比对与工具策略门禁覆盖；仍须裁决 collect.js 三命令是否应进入远程权限集。
3. **安全收口**：launch 黑名单改绝对路径白名单；接入 `check_html` 净化；前端全量接 `redactSecrets`；markdown 链接转义+`rel`。
4. **重跑补全**：B8/B10/B12 报告因超长被截断，建议重跑获取完整版（尤其 B10 的 MainArea 回归需确认范围）。
