<script setup lang="ts">
/**
 * EmptyState — 从现有产品真实 `.empty` 模式**提炼**出的 shared primitive。
 *
 * 来源（UI-0 审计）：global.css:76 `.empty { color:#bbb; text-align:center; padding:16px 0; }`
 * 被 12 个文件、13 处以「纯文本空态」形式重复手写。
 *
 * 设计约束（UI PRESERVATION 最高原则）：
 *  - 渲染结果必须与旧实现**逐字节等价**：同样的 class="empty"，同样的标签，同样的 a11y 属性。
 *  - 因此本组件**不带 <style> 块**（不产生 data-v-xxx 属性），样式继续由全局 `.empty` 提供，
 *    computed style 与迁移前完全一致 —— 零视觉漂移。
 *  - `live` 默认 false：只有原实现本来就带 role="status" aria-live="polite" 的调用点才开启，
 *    避免「顺手改善 a11y」造成结构不等价。
 *
 * 反 God Component / 反 prop explosion：
 *  - 仅 3 个 prop，全部为**通用**渲染参数（文案 / 标签 / 播报），无任何业务语义。
 *  - `as` 用于匹配宿主容器（<ul> 内必须是 li），不是业务分支。
 */
withDefaults(
  defineProps<{
    /** 空态文案；内容含插值时用默认 slot 代替 */
    text?: string
    /** 渲染标签，必须与宿主容器匹配；默认 div */
    as?: "div" | "p" | "li"
    /** 是否带 role="status" aria-live="polite" —— 仅当原实现已有时才开启 */
    live?: boolean
  }>(),
  { as: "div", live: false }
);
</script>

<template>
  <component
    :is="as"
    class="empty"
    :role="live ? 'status' : undefined"
    :aria-live="live ? 'polite' : undefined"
  ><slot>{{ text }}</slot></component>
</template>
