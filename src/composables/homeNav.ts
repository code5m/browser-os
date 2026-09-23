// Home 导航缝（共享 composable）——解耦 Workspace 能力对 Home 能力 public 的直接依赖。
//
// 背景：Workspace 文件树右键「收藏目录」需要把目录加入 Home 收藏。
// 若 Workspace 直接 import Home 的 public entry，则须声明 workspace→home 依赖；
// 而 home→workspace 已是必须依赖（home 打开目录经 workspace.enterDir / filePath），
// 双向必须依赖会构成 CB-04 必须依赖环（阻断装配）。
// 本文件是**窄接口**（非垃圾桶）：只暴露 1 个收藏命令，内部经 Home 公共边界访问。
// 归属：shared（src/composables）——能力边界门禁对 shared 不判跨能力越界。
// 同族既有缝：browserNav.ts / recentsNav.ts / terminalNav.ts。
import { favoriteDirectory } from "../capabilities/home/public"

/** 把指定目录加入 Home 收藏（Home 缺失时为 no-op，不阻断 Workspace）。 */
export async function favoriteHomeDirectory(dir: string): Promise<void> {
  favoriteDirectory(dir)
}
