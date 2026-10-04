// Browser 导航缝（共享 composable）——解耦 Workspace/其它能力对 Browser 能力内部的直接依赖。
//
// 背景：Workspace 的「采集当前网页选中内容」「打开最近 URL」需要读/写 Browser 的当前 url/title。
// 直接 import Browser 内部会让 Workspace 依赖 Browser（capability→capability 反向边）。
// 本文件是**窄接口**（非垃圾桶）：只暴露 3 个导航谓词/命令，内部经 Browser 公共边界访问。
// 归属：shared（src/composables）——能力边界门禁对 shared 不判跨能力越界。

import { useBrowserStore } from "../capabilities/browser/public"

export function currentBrowserUrl(): string {
  return useBrowserStore().url || "about:blank"
}

export function currentBrowserTitle(): string {
  return useBrowserStore().activeTab?.title || currentBrowserUrl()
}

export function navigateBrowser(url: string): void {
  const browser = useBrowserStore()
  browser.url = url
  browser.openBrowser()
}

/**
 * 请求 Browser Host 在 Shell 布局切换后重新同步原生 WebView 矩形。
 * Workspace 只表达布局意图，不直接持有 Browser store。
 */
export function relocateBrowser(): void {
  useBrowserStore().relocate()
}
