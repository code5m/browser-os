# GitHub 主仓库与多端安全同步

GitHub `origin` 是本仓库唯一主事实源。旧 Gitee 镜像不再自动同步；日常开发、验收、发布和回滚都以 GitHub `code5m/browser-os` / `master` 为准。

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

## Gitee 镜像状态

`.github/workflows/mirror-to-gitee.yml` 已退役为 manual-only 历史占位，不再响应 branch/tag push。Gitee 不参与主线验收，不作为发布事实源，也不应反向覆盖 GitHub。

恢复自动镜像必须作为单独基础设施任务处理，并至少同时恢复：

1. 明确的产品裁决；
2. 可用的 Gitee 凭据与网络稳定性；
3. fast-forward-only 保护；
4. 更新后的 `docs/engineering/governance.json` 与工程文档。

## 服务端边界

本机 Hook 是辅助层，不能覆盖 GitHub Web、未安装 Hook 的电脑或被绕过的客户端。GitHub `master` 必须在服务端禁止 force push 和删除分支，同时保留普通 direct push；这才是最终边界。
