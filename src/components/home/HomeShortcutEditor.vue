<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useHomeStore } from "../../stores/useHomeStore";
import {
  FOCUSABLE_SELECTOR,
  ariaRoleForVariant,
  shouldCloseOnEscape,
} from "../../utils/modalA11y";

const home = useHomeStore();

// 复用 W15 落地的 modalA11y 语义（无新依赖）：编辑对话框是 dialog 变体，
// 允许 Esc 关闭；role 由 ariaRoleForVariant 统一给出，避免各处手写不一致。
const VARIANT = "dialog" as const;

const mask = ref<HTMLElement | null>(null);
const nameInput = ref<HTMLInputElement | null>(null);
let lastFocused: HTMLElement | null = null;

// 打开时把焦点送进对话框，关闭时归还给触发元素（键盘用户不会丢失位置）
watch(
  () => home.editing.open,
  async (open) => {
    if (open) {
      lastFocused = document.activeElement as HTMLElement | null;
      await nextTick();
      nameInput.value?.focus();
    } else if (lastFocused) {
      lastFocused.focus();
      lastFocused = null;
    }
  }
);

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && shouldCloseOnEscape(VARIANT)) {
    e.stopPropagation();
    home.cancelEdit();
    return;
  }
  // Tab / Shift+Tab 焦点闭环：不让焦点跑到背后的主页上
  if (e.key !== "Tab" || !mask.value) return;
  const nodes = Array.from(
    mask.value.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)
  ).filter((n) => !n.hasAttribute("disabled") && n.offsetParent !== null);
  if (!nodes.length) return;
  const first = nodes[0];
  const last = nodes[nodes.length - 1];
  const active = document.activeElement as HTMLElement | null;
  if (e.shiftKey && (active === first || !mask.value.contains(active))) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && active === last) {
    e.preventDefault();
    first.focus();
  }
}
</script>

<template>
  <div
    v-if="home.editing.open"
    ref="mask"
    class="hs-mask"
    @click.self="home.cancelEdit"
    @keydown="onKeydown"
  >
    <div
      class="hs-dialog"
      :role="ariaRoleForVariant(VARIANT)"
      aria-modal="true"
      aria-labelledby="hs-dialog-title"
    >
      <h2 id="hs-dialog-title" class="hs-title">
        {{ home.editing.id ? "编辑快捷方式" : "新增快捷方式" }}
      </h2>

      <div class="hs-row">
        <label for="hs-type">类型</label>
        <select id="hs-type" v-model="home.editing.type">
          <option value="url">🌐 网页</option>
          <option value="app">🚀 应用</option>
          <option value="dir">📁 目录</option>
        </select>
      </div>

      <div class="hs-row">
        <label for="hs-name">名称</label>
        <input id="hs-name" ref="nameInput" v-model="home.editing.name" placeholder="如：Kimi" />
      </div>

      <div class="hs-row">
        <label for="hs-target">
          {{ home.editing.type === "app" ? "启动命令" : home.editing.type === "dir" ? "目录路径" : "网址" }}
        </label>
        <input
          id="hs-target"
          v-model="home.editing.target"
          :placeholder="
            home.editing.type === 'app'
              ? '如 firefox'
              : home.editing.type === 'dir'
                ? '/home/you/Documents'
                : 'https://kimi.moonshot.cn'
          "
        />
      </div>

      <div class="hs-row">
        <label for="hs-icon">图标</label>
        <input id="hs-icon" v-model="home.editing.icon" placeholder="emoji，如 🔍 🚀 📺" />
      </div>

      <div class="hs-btns">
        <button type="button" class="hs-primary" @click="home.saveEdit">保存</button>
        <button type="button" @click="home.cancelEdit">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.hs-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.hs-dialog {
  width: 360px;
  max-width: calc(100vw - 32px);
  background: #fff;
  border-radius: 12px;
  padding: 18px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.18);
}
.hs-title {
  margin: 0 0 14px;
  font-size: 14px;
  font-weight: 600;
  color: #1d2129;
}
.hs-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.hs-row label {
  width: 64px;
  flex-shrink: 0;
  font-size: 12px;
  color: #4e5969;
}
.hs-row input,
.hs-row select {
  flex: 1;
  min-width: 0;
  height: 28px;
  border: 1px solid #d5dbe7;
  border-radius: 6px;
  padding: 0 8px;
  font-size: 12px;
  outline: none;
}
.hs-row input:focus,
.hs-row select:focus {
  border-color: #2b6cb0;
}
.hs-btns {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}
.hs-btns button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 12px;
  cursor: pointer;
}
.hs-btns button:hover {
  border-color: #2b6cb0;
  color: #2b6cb0;
}
.hs-btns button:focus-visible {
  outline: 2px solid #2b6cb0;
  outline-offset: 2px;
}
.hs-primary {
  background: #2b6cb0 !important;
  border-color: #2b6cb0 !important;
  color: #fff !important;
}

@media (max-width: 720px) {
  .hs-dialog {
    width: calc(100vw - 32px);
    padding: 14px;
  }
  .hs-row {
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
  }
  .hs-row label {
    width: auto;
  }
}
</style>
