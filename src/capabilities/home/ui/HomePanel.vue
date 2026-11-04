<script setup lang="ts">
import { computed, onMounted, ref, defineAsyncComponent } from "vue";
import { useHomeStore } from "../state/useHomeStore";
import HomeLaunchers from "./HomeLaunchers.vue";
import HomeRecents from "./HomeRecents.vue";
import HomeShortcuts from "./HomeShortcuts.vue";
import HomeShortcutEditor from "./HomeShortcutEditor.vue";
import HomeChangelog from "./HomeChangelog.vue";

const home = useHomeStore();
// Lazy: only load WeChat when the user opens this draft-only tool.
const showWechatDrafts = ref(false);
const WechatDraftPanel = defineAsyncComponent(() => import("../../../components/system/WechatDraftPanel.vue"));

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
        <button type="button" title="公众号素材上传与草稿箱（不会发布或群发）" aria-controls="home-wechat-drafts" :aria-expanded="showWechatDrafts" @click="showWechatDrafts = !showWechatDrafts">
          📣 公众号草稿箱
        </button>
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
    <section v-if="showWechatDrafts" id="home-wechat-drafts" class="home-wechat-drafts" aria-label="公众号草稿管理">
      <WechatDraftPanel />
    </section>
    <HomeShortcuts />
    <HomeRecents />
    <HomeChangelog />
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
  color: var(--chrome-text, #3c4043);
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
  border: 1px solid var(--chrome-border, #dadce0);
  background: var(--chrome-toolbar, #fff);
  color: var(--chrome-text, #3c4043);
  border-radius: 6px;
  padding: 5px 12px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}
.home-actions button:hover {
  border-color: var(--home-accent, #2b6cb0);
  color: var(--home-accent, #2b6cb0);
}
.home-actions button:focus-visible {
  outline: 2px solid var(--home-accent, #2b6cb0);
  outline-offset: 2px;
}

/* 面板状态横幅：载入/错误，淡色块，不喧宾夺主（共享验收 #2 连贯状态） */
.home-state {
  margin: 0 0 16px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 12px;
  color: var(--chrome-text, #3c4043);
  background: var(--chrome-address, #f1f3f4);
  border: 1px solid var(--chrome-border, #dadce0);
}
.home-state-error {
  color: #a8071a;
  background: #fff1f0;
  border-color: #ffccc7;
}

.home-wechat-drafts {
  margin: 0 0 20px;
  padding: 18px;
  border-radius: 12px;
  background: var(--chrome-toolbar, #fff);
  color: var(--chrome-text, #3c4043);
  border: 1px solid var(--chrome-border, #dadce0);
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
