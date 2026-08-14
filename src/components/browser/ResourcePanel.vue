<script setup lang="ts">
import { computed, ref } from "vue";
import { useBrowserStore } from "../../stores/useBrowserStore";
import { useLayoutStore } from "../../stores/useLayoutStore";

const browser = useBrowserStore();
const layout = useLayoutStore();
const resFilter = ref("");

const resFiltered = computed(() => {
  const r = browser.resources;
  if (!r) return [];
  const f = resFilter.value.trim().toLowerCase();
  if (!f) return r.items;
  return r.items.filter(
    (it: ResourceItem) => it.res_type.includes(f) || it.absolute.toLowerCase().includes(f)
  );
});
</script>

<template>
  <div class="side-inner">
    <div class="tabs">
      <span>🕸 网页资源</span>
      <span v-if="browser.resources" class="res-count">{{ browser.resources.items.length }} 项</span>
      <button class="close" @click="layout.sidebarOpen = false" title="收起">✕</button>
    </div>
    <div v-if="!browser.resources" class="empty">
      <p>打开网页后，这里会列出它引用的</p>
      <p>JS / CSS / SVG / 图片 / 字体 等资源位置</p>
      <p style="margin-top:6px;color:#2b6">👉 先在顶部输入网址点「打开浏览」</p>
    </div>
    <div v-else>
      <div class="res-meta">页面：{{ browser.resources.page_url }}</div>
      <input class="res-filter" v-model="resFilter" placeholder="过滤（类型或 URL）" />
      <ul class="file-list">
        <li
          v-for="(r, i) in resFiltered"
          :key="i"
          class="res-item"
          :class="'rt-' + r.res_type"
        >
          <span class="rtag">{{ r.res_type }}</span>
          <a class="rabs" :href="r.absolute" target="_blank" :title="r.absolute">{{ r.absolute }}</a>
        </li>
        <li v-if="!resFiltered.length" class="empty">无匹配资源</li>
      </ul>
    </div>
  </div>
</template>
