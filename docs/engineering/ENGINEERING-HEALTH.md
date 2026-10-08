# BrowserOS 工程健康中心

工程健康中心是**已嵌入桌面应用的中文只读面板**，不是静态 Markdown 表格。

## 打开方式

打开 BrowserOS → 设置 → 工程健康 · 验证与证据 → **打开工程健康中心**。按“刷新状态”可以再次查询 GitHub。

## 数据真源和错误语义

面板直接从 GitHub 公共 API 查询 \`master\` 的最新 SHA、对应 SHA 的五条工作流运行记录，以及 v4 stable tag SHA。仅在同一 SHA 的最新 push 运行成功且未超过 48 小时，才显示“通过”。

- **通过**：当前 master SHA 对应 workflow 成功、证据不超过 48 小时。
- **失败**：当前 master SHA 对应最近运行不成功。
- **运行中**：最新运行尚未完成。
- **已过期**：上次同 SHA 成功，但超过 48 小时。
- **未知**：GitHub 不可达、API 限速、缺少运行结果、master SHA 无法核实。

未知、过期、运行中绝不算 PASS。静态的 Rust command/AppState 数量和 Git 证据预算只是配置值，不能替代 CI 的当前状态。Gitee 自动镜像已退役，不列入阻断门禁。

完整验收包括 Architecture/Capability、Runtime、生产构建、Rust fmt/check/test、安装版冷启动、Full GUI、GUI Artifact、Git integrity/build metrics 和 pre-merge。具体证据以对应 Actions Run 为准。

## 测试入口

\`\`\`sh
npm run check:engineering-health
npm run check
npm run build
\`\`\`

测试覆盖新 SHA 不匹配、失败/运行中/过期/网络缺失不误报为通过。

## 发布链

GitHub CI 的 SBOM/校验和及制品来源证明不等于已向用户发布了签名安装包。没有真实发行制品与验证记录时，单独发布签名仍为 BLOCKED，不能假绿。
