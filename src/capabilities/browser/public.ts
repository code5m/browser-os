// Browser 能力公共边界（Phase 8D Train C）
//
// Shell（App / MainArea / TopBar / UnifiedTabBar / ActivityBar / StatusBar / SidebarResizer）与其它能力
// 经此消费 Browser，而**不直接 import 能力内部**（src/capabilities/browser/state/*）。
//
// 注意：本文件提供再导出与无状态只读适配器，不持有也不创建任何状态（第二真源禁止）。
// 语义 owner（useBrowserStore）与物理路径解耦，由 Semantic Registry 的 owner_implementations locator 解析。
export { useBrowserStore } from "./state/useBrowserStore"
export { useResourceStore, displayUrl, formatDuration, formatSize } from "./state/useResourceStore"
export { useImagePreviewStore } from "./state/useImagePreviewStore"

import { useBrowserStore as browserOwner } from "./state/useBrowserStore";
import { isCapabilityActive } from "../../capability/runtimeSingleton";
export function readCodingBrowserContext(){
  if(!isCapabilityActive('browser'))throw new Error('BROWSER_CAPABILITY_INACTIVE');
  const tabs=browserOwner().tabs;
  return {tabs:tabs.slice(0,50).map(tab=>{let address='[internal or unavailable]';try{const url=new URL(tab.url);if(['https:','http:'].includes(url.protocol)){url.username='';url.password='';url.search='';url.hash='';address=url.toString();}}catch{}return {id:tab.id,title:tab.title.slice(0,200),url:address};}),truncated:tabs.length>50};
}
