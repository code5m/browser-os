<script setup lang="ts">
import { onMounted } from "vue";
import { useToolsStore } from "../state/useToolsStore";

const store = useToolsStore();
onMounted(store.load);
</script>

<template>
  <div class="toolbox">
    <header class="tb-head">
      <h2>工具箱</h2>
      <button class="tb-reload" @click="store.load">刷新</button>
    </header>
    <p v-if="store.error" class="tb-err">{{ store.error }}</p>

    <section v-if="store.builtin.length">
      <h3>内置工具</h3>
      <div class="tb-grid">
        <button
          v-for="t in store.builtin"
          :key="t.id"
          class="tb-card"
          :title="t.description || t.id"
          @click="store.open(t)"
        >
          <span class="tb-name">{{ t.name }}</span>
          <span class="tb-cat">{{ t.category }}</span>
        </button>
      </div>
    </section>

    <section v-if="store.user.length">
      <h3>我的工具（workspace/tools）</h3>
      <div class="tb-grid">
        <button
          v-for="t in store.user"
          :key="t.id"
          class="tb-card"
          :title="t.description || t.id"
          @click="store.open(t)"
        >
          <span class="tb-name">{{ t.name }}</span>
          <span class="tb-cat">{{ t.category }}</span>
        </button>
      </div>
    </section>

    <p
      v-if="!store.builtin.length && !store.user.length && !store.error"
      class="tb-empty"
    >
      暂无工具
    </p>
  </div>
</template>

<style scoped>
.toolbox {
  padding: 16px;
  color: var(--text, #ddd);
  height: 100%;
  overflow: auto;
}
.tb-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.tb-head h2 {
  font-size: 16px;
  margin: 0;
}
.tb-reload {
  background: #2d2d2d;
  color: #ddd;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 4px 10px;
  cursor: pointer;
}
.tb-reload:hover {
  border-color: #4a9eff;
}
.tb-err {
  color: #e06c75;
  font-size: 13px;
}
.tb-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 12px;
  margin: 10px 0 18px;
}
.tb-card {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-start;
  background: #252526;
  border: 1px solid #3a3a3a;
  border-radius: 10px;
  padding: 14px;
  cursor: pointer;
  color: inherit;
  text-align: left;
}
.tb-card:hover {
  border-color: #4a9eff;
}
.tb-name {
  font-size: 14px;
  font-weight: 600;
}
.tb-cat {
  font-size: 12px;
  color: #888;
}
.tb-empty {
  color: #888;
  font-size: 13px;
}
h3 {
  font-size: 13px;
  color: #aaa;
  margin: 14px 0 4px;
}
</style>
