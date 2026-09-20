// Recents 缝（共享 composable）—— 解耦 Browser 对 Workspace 能力内部的直接依赖。
//
// 背景：Browser 导航（url 变化）时需要写「最近访问」（recents 属 Workspace Core 的跨域状态）。
// 若 Browser 直接 import Workspace 内部，会形成 Browser→Workspace 能力边；
// 而 Workspace / Bookmark 又经公共边界读 Browser（Workspace→Browser），两者相加即出现能力环。
// 本文件是**窄接口**（非垃圾桶）：只暴露 recordRecentUrl 一个谓词。
// 归属：shared（src/composables）——能力边界门禁对 shared 不判跨能力越界。

import { useWorkspaceStore } from "../capabilities/workspace/public"

export function recordRecentUrl(url: string): void {
  if (!url) return
  useWorkspaceStore().addRecentUrl(url)
}
