# BrowserOS 技术债清单与验收矩阵（2026-10-11）

> 审计范围：code5m/browser-os，PR #26。代码已提交、实际功能可用、安装版验收通过和合并 master 是四种不同状态。此清单记录事实，不把 UI 入口或静态检查当作端到端交付。

| 等级 | 债务 | 当前真实证据 | 关闭条件 |
|---|---|---|---|
| P0 | Agent 对话/执行 | `src/bridge.ts` 中 `AGENT_SKILL_COMMANDS_AVAILABLE=false`；`AgentChatPanel.vue` 为只读 | 有真实受授权模型执行、会话、取消、状态、历史、密钥隔离、异常及权限 E2E |
| P0 | Skill 安装/运行 | Rust 仅 parse/validate/permission_preview，`SkillExec` 已要求只引用既有脚本或命令 | 经确认安装、原 `script_runner` 执行、取消、参数白名单、审计、运行历史、越权拒绝用例 |
| P0 | PR #26 全量验证 | 2026-10-11 CI Full Validation 失败于 Rust format；本地运行 Rust fmt/check 处理 | 相同 commit 的五项 CI ALL_PASS、Rust 单测、桌面安装版 GUI、体积门禁、pre-merge |
| P0 | 浏览器/文件/宫格 | Core 操作复杂；PR #26 改动尚无安装版新旧标签和宫格 GUI 证据 | 网页/路径当前标签复用、地址栏焦点、宫格开关和文件视图连续多轮 GUI 真实操作 PASS |
| P1 | 地址栏混合建议/收藏 | PR #26 实现网页/目录/文件入口，Home/Bookmark 独立 owner | 百度旧标签/新建标签/最近文件/目录、收藏夹导入、同步增删、重启持久化、去重、键盘测试 |
| P1 | Graph 写路径安全 | PR #26 新增 `graph_import_json`、`graph_rebuild_documents` 和前端入口；本地 `cargo check` 通过 | Rust 单测+原生 IPC/ACL/语义+GUI 导入/文件索引/重启恢复/隐私/异常全部 PASS |
| P1 | Graph ↔ Notes ↔ KB | 现有知识中心是导航层级；文件目录元数据索引不等于文档内容索引 | 知识实体、笔记、KB 统一关联可追溯且增量索引、删除一致性、查询及 UI E2E |
| P1 | 工具库逐项验收 | 工具 HTML 内置 JSON、Base64、时间戳、正则、Cron；页面可打开不等于功能已验 | 各工具输入、输出、异常、关闭释放、用户工具可信边界实测 |
| P2 | Plugin 运行与 Agent/Skill 集成 | Plugin 为声明式资源包生命周期，执行接口并未开放 | 可信来源及权限边界、真实生命周期和执行/卸载残留证据 |
| P2 | 工程体积/资源 | Terser minify 比原方案更小，尚须以最新 commit CI 为准 | 不修改冻结 25.2% 预算，冷启/RSS/CPU/GUI 性能 + 安装包体积可复跑 |
| P2 | 规范与产品说明 | 一些旧文档仍称 Graph 只读；Agent/Skill 执行状态须保持一致 | Manifest、页面文字、语义登记、真实运行结果同一事实来源 |

## 无人值守关单顺序
1. 每次先处理阻断当前 CI 的 P0，保持原门禁不降低。
2. 分能力补齐真实后端，优先 Agent/Skill -> Graph 知识资产 -> 地址栏与核心稳定性。
3. 每一项同时交付 Rust/TS/权限/语义/测试证据，不能仅编辑 UI 或文档。
4. CI + 安装版 GUI + 体积全部通过之后才解除 Draft 并合并 `master`，不以 PR 可合并标志代替测试通过。
5. 对尚无真实执行证据的项继续标记 OPEN；此文档不声称上述功能已经关闭。
