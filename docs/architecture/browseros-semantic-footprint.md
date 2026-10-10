# BrowserOS 语义驱动瘦身：共享纯函数与体积治理

**状态：** 本文记录实施边界与可复现检查方法；实际体积收益必须以 CI 构建与测量结果为准，不能用源文件减少量代替安装包减少量。

## 本次最小实施（与 UI PR #20 分离）

1. 将 `formatRelative` 及其唯一私有补零函数原样迁入 `src/shared/pure/time/relative.ts`，无行为改写。冻结语义包括：空/非法 ISO →「—」；过去「前」、未来「后」；秒按 `Math.trunc` 截断；分/小时/天使用原格式。
2. Scheduler 领域 `src/utils/taskUi.ts` 通过直接 import 并 **re-export** 相同函数维持公开 API；Task 内部所有调用继续使用同一导入符号。
3. Agent/Skill `src/utils/agentSkillUi.ts` 改为直接依赖纯函数，不再为了时间展示跨域引用 Scheduler 大型逻辑文件。避免可选模块跨业务依赖，具体构建体积仍以打包结果为准。
4. 新增 `scripts/check-shared-pure-time.mjs`，以 esbuild 加载真实 TypeScript，校验时间边界、旧 API 出口、直接共享依赖与非浏览器纯函数约束。该脚本接入现有 `npm run check`。
5. **不**合并同名但语义不同的方法，例如 `fmtSize` (KB/MB) 与 `formatBytes` (KiB/MiB)、文本错误脱敏与结构化配置字段脱敏；不创建通用业务 Store，不新增第二意图 Owner。

## 模块及打包边界

- `src/shared/pure/` 只能包含无副作用的通用计算，无 Vue/Pinia/Tauri、无 Store/DOM、无隐式持久化，也不依赖 `capabilities/`。
- `src/capabilities/*` 持有数据与动作的语义 Owner；跨模块交互仍走已有 Public API。不能因为压缩而改动浏览器 WebView / 文件资源管理器 / 宫格生命周期。
- 已有 xterm 分包、按需加载和 Vite ES2022 target 保留。单纯增加 chunk 不是安装包减重：分包影响**首屏加载**，不代表 `dist` 或 `.deb` 总字节数降低。
- `bookmark.entry-button` 是遗留 Contributor，当前 UI 不渲染但仍在 Manifest 中声明为热插拔兼容接口。本次 **不直接删除**；要先取得外部消费与热插拔覆盖证据，再实施受控废弃。避免误删造成“瘦身但功能退化”。
- 不抬高 `TOTAL_BYTES_GROWTH_LIMIT_PCT = 25.2`，不改历史基线，不禁用 / mock 语义门禁。

## 正式测量与验收

在完整依赖的 CI/Linux 环境执行：

```sh
node scripts/check-shared-pure-time.mjs
npm run check
npm run build
python3 scripts/measure-build-metrics.py --self-test
python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-052b18a.json --skip-build
python3 scripts/measure-capability-footprint.py --out /tmp/browseros-footprint.json
bash scripts/pre-merge.sh
```

构建指标必须分别记录 `dist` 总字节数、实际首次加载、产物压缩大小、安装后体积（在 Linux 构建安装包时），以及模拟 GUI/真实安装回归。未测量的不报告数值。比较同一平台、相同依赖锁文件和同样构建步骤。只有 Full Validation（非 ui/docs 跳过模式）与必要 GUI 及语义测试均通过才能合并。

**单主源：** GitHub `code5m/browser-os`；本专项分支与 PR #20 独立审核、独立回滚。
