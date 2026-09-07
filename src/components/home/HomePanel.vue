<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useHomeStore } from "../../stores/useHomeStore";
import HomeLaunchers from "./HomeLaunchers.vue";
import HomeRecents from "./HomeRecents.vue";
import HomeShortcuts from "./HomeShortcuts.vue";
import HomeShortcutEditor from "./HomeShortcutEditor.vue";

const home = useHomeStore();

// 常用目录播种（仅首次）：把后端承诺的起始目录补进主页。
onMounted(() => home.seedDirShortcuts());

// 面板三态（A3 契约）：载入 / 错误，给连贯的状态反馈（共享验收 #2）。
// 空态由各 section 自行呈现，这里只补载入/错误的全局横幅。
const state = computed(() => home.panelState);
</script>

<template>
  <div class="home-panel">
    <header class="home-head">
      <h1 class="hh-title">
        <span class="hh-icon" aria-hidden="true">🏠</span>
        <span>主页</span>
      </h1>
      <div class="home-actions">
        <button type="button" title="把当前网页收藏到主页" @click="home.favoriteCurrentPage">
          ☆ 收藏网页
        </button>
        <button type="button" title="把当前目录收藏到主页" @click="home.favoriteCurrentDir">
          📁 收藏目录
        </button>
        <button type="button" title="新增快捷方式" @click="home.startAdd">＋ 新增</button>
        <button type="button" title="恢复默认快捷方式" @click="home.resetDefault">↺ 默认</button>
      </div>
    </header>

    <!-- 载入/错误横幅（共享验收 #2 连贯状态；错误态不回显原始报错，只给稳定文案）。 -->
    <p v-if="state.state === 'loading'" class="home-state" role="status">
      {{ state.message }}
    </p>
    <p v-else-if="state.state === 'error'" class="home-state home-state-error" role="alert">
      {{ state.message }}
    </p>

    <HomeLaunchers />
    <HomeShortcuts />
    <HomeRecents />
    <HomeShortcutEditor />
  </div>
</template>

<style scoped>
/* .home-panel 的 flex/overflow/背景由 global.css 提供，这里只补内部排版 */
.home-panel {
  box-sizing: border-box;
}
.home-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 18px;
}
.hh-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: #1d2129;
}
.hh-icon {
  font-size: 20px;
  line-height: 1;
}
.home-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.home-actions button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 6px;
  padding: 5px 12px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}
.home-actions button:hover {
  border-color: #2b6cb0;
  color: #2b6cb0;
}
.home-actions button:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 2px;
}

/* 面板状态横幅：载入/错误，淡色块，不喧宾夺主（共享验收 #2 连贯状态） */
.home-state {
  margin: 0 0 16px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 12px;
  color: #4e5969;
  background: #f4f6fb;
  border: 1px solid #e8ebf0;
}
.home-state-error {
  color: #a8071a;
  background: #fff1f0;
  border-color: #ffccc7;
}

/* 窄窗口：标题与操作各占一行，操作按钮横向铺开便于点击 */
@media (max-width: 720px) {
  .home-panel {
    padding: 12px;
  }
  .home-head {
    align-items: stretch;
    margin-bottom: 14px;
  }
  .home-actions button {
    flex: 1 1 auto;
  }
}
</style>
