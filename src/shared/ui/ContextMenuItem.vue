<script setup lang="ts">
/**
 * ContextMenuItem — 右键菜单项，从现有产品 `.ctx-item` / `.ctx-item.danger` 提炼。
 *
 * 只有 1 个通用 prop（`danger` = 危险操作样式），点击行为由消费者通过 attrs 透传。
 * 菜单项**文案与动作**全部由 slot 提供，因此本组件不含任何业务语义。
 *
 * 视觉等价：无 <style> 块；danger 时渲染 class="ctx-item danger"，与迁移前一致。
 */
withDefaults(defineProps<{ danger?: boolean }>(), { danger: false });
</script>

<template>
  <div class="ctx-item" :class="{ danger }"><slot /></div>
</template>
