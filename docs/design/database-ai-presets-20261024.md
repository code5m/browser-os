# 数据库连接与 AI 宫格组合增量

日期：2026-10-24。实现：Vue / TypeScript；本地导入工具使用 Python + Linux Secret Service。

## 数据库连接

- 列表每条连接提供编辑入口；保留连接 ID、生产标记和读写偏好。编辑本身不切换查询源。
- 密码留空复用原 Keyring 凭据；只有明确填写新密码才替换。存在查询、事务会话或表格草稿时阻止更改连接，避免运行中的会话仍指向旧地址。
- 未验证连接通过“验证并启用”操作完成实际握手，成功才登记启用；失败保持待验证状态。
- `scripts/import-skill-database-profiles.py` 默认 dry-run，仅 `--apply` 执行本机配置写入。显式传入两个技能目录，不执行技能文件中的 SQL、shell 或示例程序。
- 本机本次新增 22 条去重 MySQL / PostgreSQL 配置，原有 5 条保留，共 27 条。密码仅存本机 Keyring；元数据备份在原配置旁，权限 0600。配置不进 Git。
- 每个端点使用文档有依据的代表数据库，不猜测该端点有全部业务库权限。Redis、ClickHouse 与库归属不明的中转端口不伪装成受支持 SQL 连接。
- 伊利 dev/uat/prod 真实客户端 preflight 全部失败：不得宣称连通；停止后续查库。未修改任何共享或生产数据。

## AI 宫格

当前是多网页工作区，不是 API 聚合器。无需把 API Key 填进网址；各站登录、订阅、地区限制和具体模型选择由各站负责。不承诺仅凭网页入口就可使用所有版本模型。

提供 15 个服务入口：豆包、DeepSeek、Kimi、Qwen、元宝、智谱清言、文小言、星火、ChatGPT、Gemini、Grok、Perplexity、Copilot、Mistral、Poe；不含 Claude Code，也未默认加入 Claude。

预置 12 套组合：国内日常、国际通用、代码交叉检查、资料检索对照、中文写作对照、推理答案对照、中外双视角、办公资料、创意发散、国内六站、国际六站、九站广度对照。

组合名称仅表示使用意图，不构成模型能力或订阅推荐。2–12 格支持独立网址配置，最多保存 32 套自定义模板。模板仅持久化名称、布局和无凭据网址；禁止用户信息、凭据查询串、片段及非安全协议。

切换组合走既有宫格重建入口，受现有内存与能力准入限制；不调整 GTK、权限或 WebView 生命周期。切换会重新加载网页，界面提示先保存网页草稿。高内存组合可能被既有资源守卫降格。

## 验证边界

- `node scripts/check-grid-ai-presets.mjs`：预置组合、数量、URL 与存储数据校验。
- `python3 tests/check-skill-db-import.py`：纯假数据验证解析、去重、生产标志与凭据分离，不连接真实服务。
- `node scripts/check-developer-studio.mjs`：连接会话保护与 SQL 行为。
- `scripts/check-browser-ui-details.cjs`：真实 Vue 组件、模拟外部 IPC；连接编辑入口、自定义模板保存/删除、切换结果。
- 实际站点登录后发送、安装版 UI 和全部门禁仍需独立验收，不能由组件 fixture 或构建成功替代。
- 发布构建补充 `build.rs` 对 `../dist` 的递归变更跟踪；`node scripts/check-release-assets.mjs` 保护配置契约。此前构建输出只包含配置/权限目录跟踪，不能可靠证明新前端嵌入二进制。

官方入口核对：<https://www.doubao.com/chat/>、<https://www.kimi.com/>、<https://chat.qwen.ai/>、<https://chatgpt.com/>、<https://gemini.google.com/>、<https://grok.com/>、<https://www.perplexity.ai/>。DeepSeek 调研访问遇 403，不作为登录/发送已通过证据。
