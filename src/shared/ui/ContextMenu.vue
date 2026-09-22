<script setup lang="ts">
/**
 * ContextMenu — 右键菜单**容器**，从现有产品 `.ctx-menu` 模式提炼。
 *
 * 来源（UI-0 审计）：global.css:78 `.ctx-menu`，真实消费者
 *   - capabilities/workspace/ui/ArtifactPanel.vue（成果右键菜单）
 *   - capabilities/workspace/ui/FilePanel.vue（文件右键菜单）
 * 两处容器结构完全一致：定位浮层 + @click.stop，故满足 Rule of Two。
 *
 * 边界（重要）：
 *  - 本组件**只提供容器**，菜单项内容一律由消费者通过默认 slot 提供。
 *  - 因此它不知道「新建文件 / 删除成果 / 复制路径」等任何业务语义 —— 业务留在 Capability。
 *  - 仅 2 个坐标 prop，无 business switch、无 variant、无 type。
 *
 * 视觉等价：无 <style> 块（不产生 data-v-xxx），样式继续由全局 `.ctx-menu` 提供。
 * v-if / @contextmenu.prevent 等通过 attrs 透传到根节点，行为与迁移前一致。
 */
defineProps<{
  /** 菜单左上角 X（px） */
  x: number;
  /** 菜单左上角 Y（px） */
  y: number;
}>();
</script>

<template>
  <div class="ctx-menu" :style="{ left: x + 'px', top: y + 'px' }" @click.stop><slot /></div>
</template>
