<script setup lang="ts">
// 渐变标题栏（对应 prototype.html 的 .titlebar）
</script>

<template>
  <header class="titlebar">
    <span class="dots"><i></i><i></i><i></i></span>
    <span class="title">浏览器OS融合</span>
  </header>
</template>

<style scoped>
.titlebar {
  height: 22px;
  background: linear-gradient(90deg, #2b6cb0, #4299e1);
  display: flex;
  align-items: center;
  padding: 0 10px;
  color: #fff;
  font-size: 12px;
  -webkit-app-region: drag;
  user-select: none;
  flex-shrink: 0;
}
.dots {
  display: flex;
  gap: 5px;
  margin-right: 10px;
}
.dots i {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.7);
  display: inline-block;
}
.title {
  font-weight: 600;
}
</style>
