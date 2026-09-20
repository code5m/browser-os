// Terminal 导航缝（共享 composable）——解耦 Workspace 能力对 Terminal 能力内部的直接依赖。
//
// 背景：Workspace 的文件树右键「命令行终端打开」需要「切到终端视图 + 在该目录新建 PTY」。
// 若 Workspace 直接 import Terminal 内部，会形成能力内部依赖（CB-01/CB-05）；
// 若声明 `workspace dependsOn terminal`，则 Terminal 缺失时 Workspace 无法装配 ——
// 与「Terminal absent 仍可启动」的组合性目标直接矛盾。
// 本文件是**窄接口**（非垃圾桶）：只暴露 1 个导航命令，内部经 Terminal 公共边界访问。
// 归属：shared（src/composables）——能力边界门禁对 shared 不判跨能力越界。
// 同族既有缝：browserNav.ts / recentsNav.ts。

import { useTerminalStore } from "../capabilities/terminal/public"

/** 打开终端视图并在指定目录新建一个 PTY 会话。 */
export async function openTerminalAt(dir: string): Promise<void> {
  await useTerminalStore().openTerminalAt(dir)
}
