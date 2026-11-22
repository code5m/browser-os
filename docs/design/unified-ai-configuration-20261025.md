# BrowserOS 统一 AI 配置

## 当前入口与所有权

BrowserOS“设置 → 统一 AI 配置”提供只读列表、导入本机配置、手动新增和编辑模型、移除、连接测试，以及全局默认和各业务模块的模型分配。实际配置发布和系统凭据写入只属于原生 AI Owner。

在项目目录执行 `npm run ai:import` 仅返回只读发现摘要和 UI 入口提示，不写配置或系统密钥。旧 `--apply`、`--destination` 写入入口明确返回 `AI_IMPORT_NATIVE_REQUIRED`。实际导入必须通过应用内入口，使校验、凭据版本化、配置发布和错误恢复处于同一条受控链路。

`src/settings/public.ts` 导出 `aiService`、`AiScope` 和配置类型；前端通过已有 bridge 调用主窗口专用 `ai_service`，不直接调用系统密钥库，也不创建第二套配置保存器。

## 配置、凭据与失败恢复

非敏感配置文件为 Tauri `app_data_dir` 下的 `mvp-browser-os/ai-profiles.json`。应用数据基目录遵循平台规则：Linux 的 `XDG_DATA_HOME`（默认 `~/.local/share`）、Windows RoamingAppData、macOS `~/Library/Application Support`，再附加应用标识 `com.jizhijiandan.mvp`。

API 密钥保存在系统密钥库：Linux Secret Service、Windows Credential Manager 或 macOS Keychain。Service 为 `com.jizhijiandan.mvp`。模型和插件使用各自的引用命名空间：

- `ai:<模型配置ID>:<版本UUID>`
- `ai-plugin:<来源ID>:<版本UUID>`

已有无版本引用继续由原生读取兼容。更改凭据时创建不可变版本，不能覆盖其他运行请求已捕获的凭据。原生 Owner 先记录非敏感清理意图，再暂存新凭据并原子发布配置；发生保存或同步错误后，清理必须依据重新读取的已持久化配置判断，不能根据“调用返回错误”猜测配置一定未发布。

凭据清理未完成时由现有配置界面显示独立的待清理状态，后续读取可重试。系统密钥库不可用时返回错误，不降级为明文文件。重复导入保留已有模型、插件和其凭据引用；用户编辑模型时空密钥保留旧值，更换服务主机必须重新提供凭据。

模型传输按原生契约移除可选的 `Bearer ` 前缀；MCP/插件环境值保持原样，不能把模型传输的格式规则应用到任意插件凭据。

## 只读发现与原生协议

Python 发现器只解析显式配置，不执行来源中的 Shell、Python 或动态表达式。它不拥有系统密钥写入或配置发布能力。发现结果中的公共元数据不得包含完整凭据或 Bearer 前缀后的实际 token。

原生导入使用 `browseros-ai-import-v1` 管道协议和请求标识。未知请求字段、不匹配的标识、超限输入或输出、无效主体引用和含凭据的元数据都必须拒绝；不能返回部分有效结果并继续发布。

发现器的默认命令行摘要不输出凭据；`--native-export` 仅供原生进程通过有界标准输入输出消费，不能转发给 UI、日志、命令参数或临时文件。前端导入响应仅包含公共配置、计数和状态。

## 业务调用边界

数据库、Git、浏览器和工作台复用同一个配置与提供方入口。业务上下文由用户明确选择，模型回复仅作建议；数据库 SQL、Git 写入、网页脚本和工具副作用仍由对应 Owner 授权执行。前端上下文变化后，不得让旧异步响应覆盖新的上下文。

`ai_service` 属于常驻框架服务并限制为主 WebView。不能向远程网页扩大 AI 配置或凭据权限。密钥仅进入原生请求快照和受控传输，IPC 响应、错误和日志不得带回凭据。

## 提交保护与回归

`npm run check:ai-config` 运行以下回归和扫描：

- `tests/check-local-ai-import.py`：只读发现、元数据与凭据分离、活动版本引用及旧引用兼容、非法引用、损坏配置、密钥库失败、原始值与 Bearer token 阻断。
- `tests/check-ai-native-import.py`：原生导入协议、输出边界、模型和插件元数据防泄漏、废弃写入入口不能修改配置。
- `scripts/check-local-ai-secrets.py`：已知本机值与高置信供应商 token 形态扫描。

扫描器按持久化模型和插件引用只读查询系统密钥；元数据不可读或无效、已匹配凭据不可读时失败关闭。它保留已知值优先和既有高置信 token 规则，只报告文件与错误码，不报告值或所在行。测试使用明确的合成凭据，不通过扩大扫描豁免获得通过。

已登记的 pre-commit、pre-push 和 pre-merge 仍调用既有扫描入口；独立 pre-merge 同时运行两组导入回归。Git Hook 可被人工绕过，该机制不构成任意未知凭据的绝对识别保证。

自动测试结果以同一提交的 CI 记录为准。注入的凭据读取替身只能证明引用和失败处理逻辑，不能替代真实 OS 密钥库测试；源码检查、构建和模拟 IPC 也不能替代安装版 GUI 验收。尚无真实桌面验收证据时保持 `GUI_PENDING`。
