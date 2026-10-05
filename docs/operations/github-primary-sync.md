# GitHub 主仓库与多端安全同步

GitHub `origin` 是本仓库唯一主事实源。Gitee `gitee` 是由 GitHub Actions 更新的备份镜像；不要从 Gitee 反向覆盖 GitHub，也不要在日常开发时直接 push 到 Gitee。

## 新电脑第一次使用

```bash
git clone https://github.com/code5m/browser-os.git
cd browser-os
./scripts/install-git-hooks.sh
```

安装脚本只写入当前仓库的 Git 配置：版本化 Hook 路径、仅快进 pull、fetch 自动 prune。可安全重复执行。

## 开始工作

```bash
./scripts/sync-repo.sh
```

它总会先刷新 GitHub refs。工作树干净且本地仅落后时，才会做 fast-forward 更新；有未提交修改、本地领先、分叉、detached HEAD 或网络错误时，只报告现状，不会 reset、rebase、merge、stash 或删除文件。

## 正常开发与 GitHub Web 修改

```bash
git add .
git commit -m "..."
git push
```

`pre-push` 会在 push 前刷新 `origin` 并只允许新 branch/new tag、branch fast-forward，以及同 SHA 的 tag no-op。它阻止远端删除、分叉、远端领先、非快进和同名不同 SHA tag。GitHub 在真正接收 push 时仍负责处理并发竞争：如果另一台电脑或 GitHub Web 刚写入新提交，GitHub 会拒绝过期的 non-fast-forward push，且本地提交保持不变。

GitHub Web 修改直接进入主事实源，不依赖本机 Hook。其他电脑下次运行 `./scripts/sync-repo.sh` 即可安全发现或快进这些修改。

## GitHub 到 Gitee 镜像

`.github/workflows/mirror-to-gitee.yml` 在 GitHub branch/tag push 后运行：

- Gitee branch 仅在不存在、相同或可 fast-forward 时更新；落后以外的状态（Gitee 更前或分叉）会失败并保留两边 ref。
- 新 tag 会创建；同 SHA tag 无操作；同名不同 SHA 会失败。
- 不使用 force、`--mirror` 或删除 ref；GitHub 的 branch/tag 删除不会传播到 Gitee。
- Gitee 暂时不可用只会使镜像 workflow 失败，不会回滚 GitHub。

需要在 GitHub 仓库添加两个 Actions secrets 后镜像才能认证：

1. `GITEE_USERNAME`：拥有 `lizhx/browser-os` 写权限的 Gitee 用户名。
2. `GITEE_TOKEN`：该 Gitee 账户创建的、只用于仓库写入的最小权限个人访问令牌。

在 GitHub 打开 `code5m/browser-os`，依次进入 **Settings → Secrets and variables → Actions → New repository secret**。不要把令牌发送到聊天、写入仓库或放入 `.env`。

## 服务端边界

本机 Hook 是辅助层，不能覆盖 GitHub Web、未安装 Hook 的电脑或被绕过的客户端。GitHub `master` 必须在服务端禁止 force push 和删除分支，同时保留普通 direct push；这才是最终边界。
