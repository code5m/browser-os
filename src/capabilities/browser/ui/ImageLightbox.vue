<script setup lang="ts">
import { computed, onBeforeUnmount, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useImagePreviewStore } from "../public";
import { createKeyBinder, keyAction } from "../../../utils/imagePreview";

// M2-2.b 图片灯箱（共用组件，全局挂在 `App.vue`，供成果库/后续工具箱与会话复用）。
//
// 交互契约（logs/checkpoints/M2-2.a-20260903-1352.md §4.3）：
//   ESC / 点遮罩关闭；←/→ 切换且**不循环**；缩放 [0.25, 4] 步长 1.2；
//   双击 1×↔2×；拖拽平移仅 scale>1；单图失败显示占位+重试且**不关闭灯箱**；
//   键盘监听仅 open 时绑定、关闭与卸载即解绑（防泄漏）。

const preview = useImagePreviewStore();

const current = computed(() => preview.current);
const atFirst = computed(() => preview.index <= 0);
const atLast = computed(() => preview.index >= preview.count - 1);

function handleKey(key: string) {
  const action = keyAction(key);
  if (!action) return;
  switch (action) {
    case "close":
      preview.close();
      break;
    case "prev":
      preview.prev();
      break;
    case "next":
      preview.next();
      break;
    case "zoom_in":
      preview.zoomIn();
      break;
    case "zoom_out":
      preview.zoomOut();
      break;
    case "zoom_reset":
      preview.resetZoom();
      break;
  }
}

// 键盘监听：绑定器保证幂等，组件只在 open 时 add、关闭与卸载时 remove。
// hooks 内部持有稳定的 listener 引用，保证 add/remove 成对。
let winListener: ((e: KeyboardEvent) => void) | null = null;

const binder = createKeyBinder(
  {
    add: (h) => {
      winListener = (e: KeyboardEvent) => h(e.key);
      window.addEventListener("keydown", winListener);
    },
    remove: () => {
      if (winListener) {
        window.removeEventListener("keydown", winListener);
        winListener = null;
      }
    },
  },
  handleKey,
);

watch(
  () => preview.open,
  (isOpen) => {
    if (isOpen) binder.bind();
    else binder.unbind();
  },
);

onBeforeUnmount(() => {
  binder.unbind();
});

function assetSrc(path: string): string {
  return path ? convertFileSrc(path) : "";
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  preview.wheelZoom(e.deltaY);
}

function onMouseDown(e: MouseEvent) {
  preview.startDrag(e.clientX, e.clientY);
}

function onMouseMove(e: MouseEvent) {
  preview.moveDrag(e.clientX, e.clientY);
}

function onMouseUp() {
  preview.endDrag();
}
</script>

<template>
  <div v-if="preview.open && current" class="modal-mask img-lightbox" @click.self="preview.close()">
    <div class="lb">
      <div class="lb-bar">
        <span class="lb-count">{{ preview.index + 1 }} / {{ preview.count }}</span>
        <span class="lb-dim">{{ current.sizeText }} · {{ current.bytesText }}</span>
        <span class="lb-host">来源 {{ current.host }}</span>
        <div class="lb-zoom">
          <button type="button" @click.stop="preview.zoomOut()" title="缩小">−</button>
          <span class="lb-scale">{{ preview.zoomLabel }}</span>
          <button type="button" @click.stop="preview.zoomIn()" title="放大">+</button>
          <button type="button" @click.stop="preview.resetZoom()" title="复位">复位</button>
        </div>
        <button class="lb-close" type="button" @click.stop="preview.close()" title="关闭 (ESC)">✕</button>
      </div>

      <div
        class="lb-stage"
        :class="{ grab: preview.canDrag, grabbing: preview.dragging }"
        @wheel="onWheel"
        @mousedown="onMouseDown"
        @mousemove="onMouseMove"
        @mouseup="onMouseUp"
        @mouseleave="onMouseUp"
        @dblclick.stop="preview.toggleZoom()"
      >
        <img
          v-if="!preview.isFailed(current.id)"
          :key="`${current.id}#${preview.retryNonce(current.id)}`"
          class="lb-img"
          :src="assetSrc(current.src)"
          :alt="current.alt"
          :style="{
            transform: `translate(${preview.tx}px, ${preview.ty}px) scale(${preview.scale})`,
          }"
          draggable="false"
          @error="preview.markFailed(current.id)"
        />
        <div v-else class="lb-error">
          <div class="lb-error-icon">⚠</div>
          <div class="lb-error-text">图片加载失败</div>
          <button class="lb-retry" type="button" @click.stop="preview.retry(current.id)">重试</button>
        </div>

        <button
          v-show="!atFirst"
          class="lb-nav prev"
          type="button"
          title="上一张 (←)"
          @click.stop="preview.prev()"
        >
          ‹
        </button>
        <button
          v-show="!atLast"
          class="lb-nav next"
          type="button"
          title="下一张 (→)"
          @click.stop="preview.next()"
        >
          ›
        </button>
      </div>

      <div class="lb-foot">
        <span class="lb-caption">{{ current.caption || current.alt }}</span>
        <span class="lb-tip">滚轮缩放 · 双击复位 · 放大后可拖拽 · ESC 关闭</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.img-lightbox {
  z-index: 60;
}

.lb {
  display: flex;
  flex-direction: column;
  width: min(92vw, 1100px);
  height: min(88vh, 820px);
  background: #0f131b;
  border: 1px solid #2a2f3a;
  border-radius: 8px;
  overflow: hidden;
}

.lb-bar,
.lb-foot {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  font-size: 12px;
  color: #a8b0c0;
  background: #141924;
  flex: 0 0 auto;
}

.lb-foot {
  justify-content: space-between;
  border-top: 1px solid #232a38;
}

.lb-bar {
  border-bottom: 1px solid #232a38;
}

.lb-zoom {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: auto;
}

.lb-zoom button,
.lb-close {
  min-width: 26px;
  height: 24px;
  padding: 0 6px;
  border: 1px solid #3a4152;
  border-radius: 4px;
  background: #1b2130;
  color: #c8cfdb;
  cursor: pointer;
}

.lb-zoom button:hover,
.lb-close:hover {
  border-color: #4a90d9;
}

.lb-scale {
  min-width: 46px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}

.lb-stage {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: #0b0e14;
}

.lb-stage.grab {
  cursor: grab;
}

.lb-stage.grabbing {
  cursor: grabbing;
}

.lb-img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  transition: transform 0.08s linear;
  user-select: none;
}

.lb-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: #8b93a7;
  font-size: 13px;
}

.lb-error-icon {
  font-size: 26px;
}

.lb-retry {
  padding: 4px 12px;
  border: 1px solid #3a4152;
  border-radius: 4px;
  background: #1b2130;
  color: #c8cfdb;
  cursor: pointer;
}

.lb-retry:hover {
  border-color: #4a90d9;
}

.lb-nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 40px;
  height: 64px;
  font-size: 26px;
  border: 1px solid #2a2f3a;
  border-radius: 6px;
  background: rgba(20, 25, 36, 0.82);
  color: #c8cfdb;
  cursor: pointer;
}

.lb-nav.prev { left: 10px; }
.lb-nav.next { right: 10px; }

.lb-nav:hover {
  border-color: #4a90d9;
}

.lb-caption {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 60%;
}

.lb-tip {
  opacity: 0.7;
}

.lb-host,
.lb-dim {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
