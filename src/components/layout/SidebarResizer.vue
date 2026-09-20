<script setup lang="ts">
import { useLayoutStore } from "../../stores/useLayoutStore";
import { useBrowserStore } from "../../capabilities/browser/public";

const layout = useLayoutStore();
const browser = useBrowserStore();

function startResize(e: MouseEvent) {
  layout.leftResizing = true;
  const startX = e.clientX;
  const startW = layout.sidebarWidth;
  const move = (ev: MouseEvent) => {
    if (!layout.leftResizing) return;
    layout.setSidebarWidth(startW + ev.clientX - startX);
    browser.relocate();
  };
  const up = () => {
    layout.leftResizing = false;
    window.removeEventListener("mousemove", move);
    window.removeEventListener("mouseup", up);
  };
  window.addEventListener("mousemove", move);
  window.addEventListener("mouseup", up);
}
</script>

<template>
  <div
    v-if="layout.sidebarOpen && !layout.leftResizing"
    class="resizer"
    @mousedown="startResize"
  ></div>
</template>
