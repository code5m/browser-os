// src/capabilities/tools/state/useToolsStore.ts
// Tools（工具箱）能力语义 owner（Capability Library Expansion v1，STAGE H）。
//
// 新建：原 ToolBox.vue 直接把状态放在组件内（无 owner），违反「State Owner 可寻址」要求；
// 本 store 把工具列表/加载/打开意图收敛为可寻址 owner，组件只经 public 消费。
//
// 安全：工具在**独立子 webview**（label=`tool-<id>`，`tool://` 协议）中打开，零能力隔离；
// 工具 HTML 即便调用 invoke() 亦被 ACL 拒绝（门禁 TOOL_CAPABILITY_LEAK / SEED_CAPABILITY_LEAK）。
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../../../bridge";
import type { ToolMeta } from "../../../types";
import { registerToolsCleanup, toolsLifecycleSnapshot } from "../lifecycle";

export const useToolsStore = defineStore("tools", () => {
  const tools = ref<ToolMeta[]>([]);
  const error = ref<string | null>(null);
  registerToolsCleanup(() => bridge.closeTools());
  const lifecycleReady = () => toolsLifecycleSnapshot().active;

  const builtin = computed(() => tools.value.filter((t) => t.source === "builtin"));
  const user = computed(() => tools.value.filter((t) => t.source === "user"));

  async function load() {
    if (!lifecycleReady()) return;
    error.value = null;
    try {
      tools.value = await bridge.listTools();
    } catch (e) {
      error.value = String(e);
    }
  }

  function open(t: ToolMeta) {
    if (!lifecycleReady()) return;
    bridge.openTool(t.id).catch((e) => (error.value = `打开失败：${String(e)}`));
  }

  return { tools, error, builtin, user, load, open };
});
