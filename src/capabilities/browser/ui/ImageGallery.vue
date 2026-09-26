<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { ImageRef } from "../../../types";
import { useImagePreviewStore } from "../public";
import {
  buildPreviewItems,
  galleryState,
  gridColumns,
  type PreviewItem,
} from "../../../utils/imagePreview";

// M2-2.b 图片画廊（共用组件，成果库挂点 `ArtifactPanel`）。
//
// 约束（契约见 logs/checkpoints/M2-2.a-20260903-1352.md §4）：
//   - 组件**不直接 invoke**：目录基准来自 store（store 走 bridge）
//   - 字节通道固定为 convertFileSrc + asset://，不扩张 assetProtocol.scope
//   - 溯源只出 host，不渲染 source_url 原文
//   - 只渲染已落盘的 File 型图片（inline_data_url 在本卡范围外）

const props = withDefaults(
  defineProps<{
    images: ImageRef[];
    loading?: boolean;
  }>(),
  { loading: false },
);

const preview = useImagePreviewStore();

const root = ref<HTMLElement | null>(null);
const width = ref(0);
let observer: ResizeObserver | null = null;

onMounted(async () => {
  await preview.ensureDir();
  if (root.value) {
    width.value = root.value.clientWidth;
    if (typeof ResizeObserver !== "undefined") {
      observer = new ResizeObserver((entries) => {
        const w = entries[0]?.contentRect?.width ?? 0;
        if (w > 0) width.value = w;
      });
      observer.observe(root.value);
    }
  }
});

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
});

const items = computed<PreviewItem[]>(() =>
  buildPreviewItems(props.images, preview.dir),
);
const state = computed(() => galleryState(props.loading, items.value.length));
const cols = computed(() => gridColumns(width.value || 640));

/** 绝对路径 → asset://（唯一的字节读取通道） */
function assetSrc(path: string): string {
  return path ? convertFileSrc(path) : "";
}

function openAt(i: number) {
  void preview.openAt(props.images, i);
}
</script>

<template>
  <div ref="root" class="img-gallery">
    <!-- 骨架：与空态互斥（loading 优先） -->
    <div v-if="state === 'loading'" class="skeleton" :style="{ gridTemplateColumns: `repeat(${cols}, 1fr)` }">
      <div v-for="n in cols" :key="`sk-${n}`" class="sk-cell" />
    </div>

    <div v-else-if="state === 'empty'" class="empty">
      <div class="empty-icon">🖼</div>
      <div class="empty-text">该成果暂无可预览的图片</div>
      <div class="empty-hint">仅展示已落盘保存的图片附件</div>
    </div>

    <div v-else class="grid" :style="{ gridTemplateColumns: `repeat(${cols}, 1fr)` }">
      <div v-for="(it, i) in items" :key="it.id" class="cell">
        <button
          class="thumb"
          type="button"
          :title="it.alt"
          @click="openAt(i)"
        >
          <img
            v-if="!preview.isFailed(it.id)"
            :key="`${it.id}#${preview.retryNonce(it.id)}`"
            :src="assetSrc(it.src)"
            :alt="it.alt"
            loading="lazy"
            @error="preview.markFailed(it.id)"
          />
          <div v-else class="fallback">
            <div class="fallback-icon">⚠</div>
            <div class="fallback-text">图片加载失败</div>
            <button class="retry" type="button" @click.stop="preview.retry(it.id)">重试</button>
          </div>
        </button>
        <div class="meta">
          <span class="caption" :title="it.caption || it.alt">{{ it.caption || it.alt }}</span>
          <span class="src">来源 {{ it.host }}</span>
          <span class="dim">{{ it.sizeText }} · {{ it.bytesText }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.img-gallery {
  width: 100%;
  min-width: 0;
}

.grid,
.skeleton {
  display: grid;
  gap: 8px;
  width: 100%;
}

.cell {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.thumb {
  position: relative;
  width: 100%;
  aspect-ratio: 4 / 3;
  padding: 0;
  border: 1px solid var(--border, #2a2f3a);
  border-radius: 6px;
  background: #151922;
  cursor: zoom-in;
  overflow: hidden;
}

.thumb img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}

.thumb:hover {
  border-color: #4a90d9;
}

.fallback {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  height: 100%;
  color: #8b93a7;
  font-size: 12px;
}

.fallback-icon {
  font-size: 20px;
}

.retry {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  border: 1px solid #3a4152;
  background: #212736;
  color: #c8cfdb;
  cursor: pointer;
}

.retry:hover {
  border-color: #4a90d9;
}

.meta {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  font-size: 11px;
  color: #8b93a7;
}

.caption,
.src,
.dim {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sk-cell {
  aspect-ratio: 4 / 3;
  border-radius: 6px;
  background: linear-gradient(90deg, #1b2130 25%, #232a3b 37%, #1b2130 63%);
  background-size: 400% 100%;
  animation: img-sk 1.2s ease-in-out infinite;
}

@keyframes img-sk {
  0% { background-position: 100% 50%; }
  100% { background-position: 0 50%; }
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 20px 8px;
  color: #8b93a7;
  border: 1px dashed #2a2f3a;
  border-radius: 6px;
}

.empty-icon {
  font-size: 22px;
}

.empty-text {
  font-size: 13px;
}

.empty-hint {
  font-size: 11px;
  opacity: 0.75;
}
</style>
