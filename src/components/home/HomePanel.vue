<script setup lang="ts">
import { useHomeStore } from "../../stores/useHomeStore";
import type { HomeShortcut } from "../../stores/useHomeStore";

const home = useHomeStore();

function onOpen(s: HomeShortcut) {
  home.open(s);
}
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
        <div class="home-type">{{ s.type === "url" ? "🌐 网页" : "🚀 应用" }}</div>
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
          </select>
        </div>
        <div class="hm-row">
          <label>名称</label>
          <input v-model="home.editing.name" placeholder="如：Kimi" />
        </div>
        <div class="hm-row">
          <label>{{ home.editing.type === "url" ? "网址" : "启动命令" }}</label>
          <input
            v-model="home.editing.target"
            :placeholder="home.editing.type === 'url' ? 'https://kimi.moonshot.cn' : '如 firefox / 应用 exec'"
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
