import type { CapabilityDefinition } from "../../capability/types"
import { defineIntegratedCapability } from "../../capability/platform/integrated"
import { WECHAT_CAPABILITY_ID, WECHAT_WORKBENCH_VIEW } from "./public"

export const wechatManifest: CapabilityDefinition = defineIntegratedCapability({
  id: WECHAT_CAPABILITY_ID,
  name: "微信公众号工作台",
  semanticOwner: "WechatWorkbenchPanel",
  maturity: "C3",
  version: "1.0.0",
  provides: ["wechat.workbench", "wechat.material.upload", "wechat.draft"],
  dependencies: ["bridge"],
  resident: false,
  supported: ["ACTIVE", "SUSPENDED"],
  resourceClass: ["LIGHT"],
  suspendable: true,
  destroyable: true,
  persistence: { scope: "runtime_only", sensitive: false },
  contributions: [{
    id: "wechat.main.panel",
    slot: "workbench-main",
    type: "surface",
    view: WECHAT_WORKBENCH_VIEW,
  }],
  publicContract: [{
    name: "WECHAT_WORKBENCH_VIEW",
    locator: "src/capabilities/wechat/public.ts",
  }],
  deactivationPolicy: "graceful",
  hotPlugLevel: "HP2",
  limitationReason: "HP2：仅注册懒加载工作台页面；关闭页签不触发发布、群发或定时发送。",
})
