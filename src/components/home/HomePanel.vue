<script setup lang="ts">
import { onMounted } from "vue";
import { useHomeStore } from "../../stores/useHomeStore";
import type { HomeShortcut } from "../../stores/useHomeStore";

const home = useHomeStore();

onMounted(() => home.seedDirShortcuts());

function onOpen(s: HomeShortcut) {
  home.open(s);
}

const TYPE_LABELS: Record<HomeShortcut["type"], string> = {
  url: "🌐 网页",
  app: "🚀 应用",
  dir: "📁 目录",
};
const TARGET_LABELS: Record<HomeShortcut["type"], string> = {
  url: "网址",
  app: "启动命令",
  dir: "目录路径",
};
const TARGET_PLACEHOLDERS: Record<HomeShortcut["type"], string> = {
  url: "https://kimi.moonshot.cn",
  app: "如 firefox / 应用 exec",
  dir: "/home/you/Documents",
};
</script>

<template>
  <div class="home-panel">
    <div class="home-head">
      <span class="home-title">🏠 主页</span>
      <div class="home-actions">
        <button @click="home.startAdd" title="新增快捷方式">＋ 新增</button>
        <button @click="home.resetDefault" title="恢复默认">↺ 默认</button>
      </div>
    </div>

    <div v-if="home.shortcuts.length" class="home-grid">
      <div
        v-for="s in home.shortcuts"
        :key="s.id"
        class="home-card"
        :title="s.target"
        @click="onOpen(s)"
      >
        <div class="home-icon">{{ s.icon }}</div>
        <div class="home-name">{{ s.name }}</div>
        <div class="home-type">{{ TYPE_LABELS[s.type] }}</div>
        <div class="home-ops" @click.stop>
          <button class="op" @click="home.startEdit(s)" title="编辑">✏️</button>
          <button class="op" @click="home.remove(s.id)" title="删除">🗑</button>
        </div>
      </div>
    </div>
    <div v-else class="home-empty">暂无快捷方式，点「＋ 新增」添加常用网页或应用</div>

    <!-- 编辑弹窗 -->
    <div v-if="home.editing.open" class="home-modal-mask" @click.self="home.cancelEdit">
      <div class="home-modal">
        <div class="hm-title">{{ home.editing.id ? "编辑快捷方式" : "新增快捷方式" }}</div>
        <div class="hm-row">
          <label>类型</label>
          <select v-model="home.editing.type">
            <option value="url">🌐 网页</option>
            <option value="app">🚀 应用</option>
            <option value="dir">📁 目录</option>
          </select>
        </div>
        <div class="hm-row">
          <label>名称</label>
          <input v-model="home.editing.name" placeholder="如：Kimi" />
        </div>
        <div class="hm-row">
          <label>{{ TARGET_LABELS[home.editing.type] }}</label>
          <input
            v-model="home.editing.target"
            :placeholder="TARGET_PLACEHOLDERS[home.editing.type]"
          />
        </div>
        <div class="hm-row">
          <label>图标</label>
          <input v-model="home.editing.icon" placeholder="emoji，如 🔍 🚀 📺" />
        </div>
        <div class="hm-btns">
          <button class="primary" @click="home.saveEdit">保存</button>
          <button @click="home.cancelEdit">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.home-panel {
  flex: 1;
  overflow-y: auto;
  padding: 18px;
  background: linear-gradient(135deg, #eef2fb 0%, #f7f9ff 45%, #f0f4fa 100%);
}
.home-grid {
  display: grid !important;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)) !important;
  gap: 14px;
}
.home-card {
  position: relative;
  background: #fff;
  border: 1px solid #e8ebf0;
  border-radius: 12px;
  padding: 18px 10px 12px;
  text-align: center;
  cursor: pointer;
  transition: all 0.15s;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}
.home-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 14px rgba(43, 108, 176, 0.12);
  border-color: #c6d8ef;
}
.home-ops {
  position: absolute;
  top: 6px;
  right: 6px;
  display: none !important;
  gap: 2px;
}
.home-card:hover .home-ops {
  display: flex !important;
}
</style>
