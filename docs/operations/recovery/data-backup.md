# Agent D — Data Backup & Restore

> 桌面应用最危险的不是代码，而是**数据**。代码能回滚，数据坏了通常不能。
> 本文件基于对仓库的只读源码调研（含 `file:line` 证据）整理，**不含任何真实秘密值**。

---

## 0. 关键路径（实测 + 源码确认）

| 用途 | 路径 |
|---|---|
| 应用数据根（含 WebKit 持久数据） | `${XDG_DATA_HOME:-$HOME/.local/share}/com.jizhijiandan.mvp` |
| **业务数据根 `data_dir()`** | `${XDG_DATA_HOME:-$HOME/.local/share}/com.jizhijiandan.mvp/mvp-browser-os` |
| 运行日志 | `…/com.jizhijiandan.mvp/logs/`（`session-<ts>-<role>-pid<N>.log`、`crash.log`，保留最近 10 个） |
| 宫格 UDS socket | `…/com.jizhijiandan.mvp/sock` |
| 用户笔记（**在应用数据目录之外**） | `~/Documents/极智笔记/*.md`、自选归档 vault |

来源：`identifier = com.jizhijiandan.mvp`（`src-tauri/tauri.conf.json:5`）；业务根由 `app_data_dir().join("mvp-browser-os")` 得到（`src-tauri/src/workspace.rs:16-21`、`sync.rs:24-30`）。
当前实测：数据根约 **421M**（含 WebKit 缓存与仓库工作副本）。

---

## 1. 分类

### ✅ 可备份（低敏）

| 数据 | 位置 | 说明 |
|---|---|---|
| `bookmarks.json` | `data_dir/` | 收藏夹，纯结构数据 |
| `scripts.json` / `scripts/<id>.<ext>` | `data_dir/` | 脚本元数据 + 脚本正文 |
| `snippets.json` | `data_dir/` | 命令片段（⚠️ `argv` 可能内嵌口令，见下） |
| `repos.json` | `data_dir/` | 仓库配置，**结构上不含 token** |
| `audit.json` | `data_dir/` | 审计日志（FIFO 上限 1000） |
| `tasks.json` / `task-runs.json` | `data_dir/` | 定时任务与执行明细 |
| `plugins.json` / `trusted-pubkeys.json` | `data_dir/` | 插件登记簿与信任公钥（不含签名原文） |
| `graph.json` | `data_dir/` | 图谱快照（只读载入） |
| `browser-os-settings`、`workbench-layout-v1`、`browser-os-command-snippet-favorites`、`browser-os-home-shortcuts`、`browser-os-home-recents-v1`、`browser-os-home-dirs-seeded-v2` | 浏览器 localStorage | 偏好与布局 |

### ⚠️ 需要迁移 / 需人工审核（备份前先看一眼）

| 数据 | 风险 | 处理 |
|---|---|---|
| `snippets.json` 的 `argv`、脚本正文 | 命令行可能内嵌 token/口令（`src/utils/homeUi.ts:220` 明确警示） | 可备份，但**还原到新机器前人工检索密钥** |
| `browser-os-recents`、`browser-os-home-recents-v1`、`browser-os-recent-dirs` | 存**原始 URL**（可能含 `?token=`）；显示时才脱敏，存储是原值 | 备份前可选择性剔除；跨机还原视为隐私数据 |
| `terminal-auto-confirm-cli` | **安全开关**（终端危险命令自动确认） | 跨机还原会恢复该危险行为，建议还原后复位为 `0` |
| `trusted-pubkeys.json` | 信任根 | 跨机还原需人工校验来源 |

### 🚫 敏感·禁止复制（备份必须排除）

| 数据 | 位置 | 原因 |
|---|---|---|
| **系统密钥库条目**（service `com.jizhijiandan.mvp`） | keyring | 明文 git token / 数据库口令 / 网站密码。**不可文件化导出**，跨机还原必须由用户重新录入 |
| **`cookies/`**（及 `localstorage/`、`databases/`、`hsts-storage.sqlite`、`WebKitCache/`、`CacheStorage/`、`storage/`、`deviceidhashsalts/`、`mediakeys/`） | 数据根顶层 | **登录态与 cookie**，属凭据类；复制等于导出账号会话 |
| `workspace/<id>.json` + `workspace/images/` | `data_dir/workspace/` | 抓取的网页正文/HTML 与图片，可能是登录后的页面内容 |
| `sessions/<id>.json` | `data_dir/sessions/` | 浏览历史（URL/标题/预览/资源瀑布）。虽经 `redact_sensitive_url` 脱敏且结构上无 cookie/Authorization（domain.rs:334-336,415-417），仍属隐私 |
| `repos/<repo_id>/` | `data_dir/repos/` | 完整 git 工作副本 = **用户文件内容/私有代码**，可能含密钥文件 |
| `~/Documents/极智笔记/*.md`、归档 vault | 应用数据目录之外 | 用户笔记正文 |
| `logs/session-*.log`、`logs/crash.log` | `…/logs/` | stderr 全量镜像，第三方输出未过滤 |

### 🔒 凭据专项规则（红线）

- 凭据**只存在系统 keyring**（服务名 `com.jizhijiandan.mvp`），键格式：
  - git 仓库 token：`<repo_id>`
  - 数据库连接口令：`db:<conn_id>`
  - 导入的网站密码：`browser:<归一化url>:<username>`
- **禁止**直接复制 keyring 内容；**禁止**任何形式导出明文。
- 备份中**只允许记录元数据**：条目类型、键格式、`has_password` 之类的存在性标记，以及**还原流程**（用户重新录入）。
- `browser-credentials.json` 只含 `key/url/username/imported_at`（**无 password 字段**），属"账号枚举信息"，按敏感处理；测试已断言索引文件不得出现密码字样（`bridge.rs:7301-7305`）。

---

## 2. 数据 / 位置 / 备份 / 还原 总表

| Data | Location | Backup allowed | Restore method | Sensitive | Notes |
|---|---|---|---|---|---|
| 收藏夹 | `data_dir/bookmarks.json` | ✅ | 停应用 → 覆盖文件 → 启动 | 否 | 应用有导入能力，无导出；只能文件级备份 |
| 脚本与片段 | `data_dir/scripts.json`+`scripts/`+`snippets.json` | ✅（审 argv） | 同上 | ⚠️ 可能内嵌密钥 | 还原前人工检索 |
| 任务 | `data_dir/tasks.json`/`task-runs.json` | ✅ | 同上 | 否 | 判重真源是 `tasks.json` 的 `last_fired_at` |
| 审计 | `data_dir/audit.json` | ✅ | 只读参考，一般**不还原** | 否 | FIFO 1000 条 |
| 插件/信任公钥 | `data_dir/plugins.json`/`trusted-pubkeys.json` | ✅ | 覆盖文件 | ⚠️ 信任根 | 还原后校验来源 |
| 仓库配置 | `data_dir/repos.json` | ✅（无 token） | 覆盖文件；**token 需重新录入 keyring** | 否 | token 不在文件里 |
| 工作副本 | `data_dir/repos/<id>/` | 🚫 默认不备份 | 用 git remote 重新 clone | 高 | 用户文件内容 |
| 成果库 | `data_dir/workspace/` | 🚫 | 重新抓取 | 高 | 网页正文+图片 |
| 浏览会话 | `data_dir/sessions/` | 🚫 | 应用内 `session_export` → 剪贴板（不落盘） | 高 | 隐私 |
| WebKit cookie/会话 | 数据根 `cookies/` 等 | 🚫 **禁止** | 重新登录 | 极高 | 凭据类 |
| keyring 凭据 | 系统密钥库 | 🚫 **禁止** | 用户重新录入 | 极高 | 仅记录元数据 |
| 偏好/布局 | localStorage | ✅ | 浏览器存储，需应用内重设或 DevTools 导入 | 低 | 注意 `terminal-auto-confirm-cli` |
| 笔记 | `~/Documents/极智笔记/` | 用户自决 | 用户自行备份 | 高 | 应用数据目录之外 |

---

## 3. 备份流程（DATA BACKUP BEFORE MIGRATION）

```bash
# 1) 停应用（落盘用 atomic_write tmp+rename，运行中拷贝可能拿到 .tmp 中间态）
pkill -f mvp-browser-os || true
sleep 1

# 2) 只备份"可备份"清单，显式排除敏感项
DATA="$HOME/.local/share/com.jizhijiandan.mvp/mvp-browser-os"
DEST="$HOME/mvp-backup-$(date +%F)"
mkdir -p "$DEST"

for f in bookmarks.json scripts.json snippets.json repos.json \
         tasks.json task-runs.json plugins.json trusted-pubkeys.json graph.json; do
  [[ -f "$DATA/$f" ]] && cp -a "$DATA/$f" "$DEST/"
done
cp -a "$DATA/scripts" "$DEST/" 2>/dev/null || true

# 3) 显式不备份（再次确认排除）
#    cookies/ localstorage/ databases/ hsts-storage.sqlite WebKitCache/ CacheStorage/
#    storage/ deviceidhashsalts/ mediakeys/  sessions/  workspace/  repos/

# 4) 记录凭据元数据（不含任何秘密值）
cat > "$DEST/CREDENTIALS-METADATA.md" <<'EOF'
keyring service: com.jizhijiandan.mvp
entry kinds:  <repo_id> (git token) | db:<conn_id> (db password) | browser:<url>:<username> (site password)
restore: 必须由用户在目标机器重新录入；本备份不含任何秘密值。
EOF

# 5) 留证
./scripts/snapshot.sh after-data-backup
```

> 应用自带**自动侧车保护**：损坏文件会被 rename 为 `<file>.json.corrupt`，还原前先看有没有 `.corrupt` 可就地取回。

## 4. 还原流程

1. **先匹配代码版本**：数据格式与代码版本必须同一时代（新数据 + 旧代码 = 再次损坏）。
2. 停应用 → 覆盖文件 → 启动 → 跑 `./scripts/collect-diagnostics.sh post-restore`。
3. **重新录入凭据**（keyring 不可复制）。
4. 若还原后仍异常 → 检查 `.corrupt` 侧车文件，或按 `git-workflow.md` 场景 4 处理。

## 5. 现状差距（如实记录）

- 仓库**没有通用备份/还原功能**（源码调研：NONE FOUND）。
- 现有能力只有：会话导出（**到剪贴板**，不落盘）、收藏夹导入（Chrome/Firefox）、浏览器账号密码 CSV 导入。
- **收藏夹导出功能不存在**（`useBookmarkStore.ts` 只有 importFile）。
- 因此本文件给出的是**手工文件级**备份方案；未来若实现"导出/导入"功能，属新功能开发，不在恢复层范围内。
