import { createLifecycleContributionCapability } from "../../capability/platform/contributed"
import { activateWechatLifecycle, suspendWechatLifecycle } from "./lifecycle"
import { wechatManifest } from "./manifest"
import { WECHAT_CAPABILITY_ID } from "./public"

const wechat = createLifecycleContributionCapability(
  wechatManifest,
  [{
    id: "wechat.main.panel",
    type: "surface",
    slot: "workbench-main",
    view: "wechat",
    label: "公众号工作台",
    icon: "📣",
    panelState: true,
    load: () => import("../../components/system/WechatWorkbenchPanel.vue"),
  }],
  activateWechatLifecycle,
  suspendWechatLifecycle,
)

export { WECHAT_CAPABILITY_ID, WECHAT_WORKBENCH_VIEW } from "./public"
export const wechatCapability = wechat.capability
export const registerWechatContributions = wechat.register
