import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { bridge } from "../../../bridge";
import type { ImageRef } from "../../../types";
import {
  buildPreviewItems,
  bumpRetry,
  clearFailed,
  clampScale,
  clampTranslate,
  clampTranslate as clampOffset,
  dragEnabled,
  isFailed as isFailedPure,
  markFailed as markFailedPure,
  nextIndex,
  prevIndex,
  retryNonce as retryNoncePure,
  scaleForButton,
  scaleForDoubleClick,
  scaleForWheel,
  type PreviewItem,
} from "../../../utils/imagePreview";

// M2-2.b 图片预览状态（画廊 → 灯箱的唯一数据源）。
//
// 为什么单独一个 store：灯箱全局挂在 `App.vue`（供后续工具箱/会话复用），
// 画廊挂在 `ArtifactPanel`，两者不是父子关系，需要一个共享状态把它们连起来；
// 也更符合既有范式（`useGitStore` / `useSessionStore` 同样是「面板 + 全局弹窗」组合）。
//
// 依赖边界：
//   - 后端调用一律走 `bridge`（本 store 不直接 invoke）
//   - 不持有任何 `source_url` 原文（PreviewItem 结构上只有 host）
//   - 只产出**绝对路径**，转 `asset://` 的 `convertFileSrc` 留给组件层

export const useImagePreviewStore = defineStore("imagePreview", () => {
  /** 目录基准（后端 workspace_images_dir，只取一次） */
  const dir = ref<string>("");
  const dirLoading = ref(false);
  const dirError = ref<string | null>(null);

  const items = ref<PreviewItem[]>([]);
  const index = ref(-1);
  const open = ref(false);

  const scale = ref(1);
  const tx = ref(0);
  const ty = ref(0);

  /** 加载失败的图片 id → 重试次数（用于给 <img> 换 key 强制重新加载） */
  const retries = ref<Record<string, number>>({});
  const failed = ref<Record<string, boolean>>({});

  const dragging = ref(false);
  const dragOrigin = ref<{ x: number; y: number; tx: number; ty: number } | null>(null);

  const current = computed<PreviewItem | null>(() =>
    index.value >= 0 && index.value < items.value.length ? items.value[index.value] : null,
  );
  const count = computed(() => items.value.length);
  const canDrag = computed(() => dragEnabled(scale.value));
  const zoomLabel = computed(() => `${Math.round(scale.value * 100)}%`);

  /** 取目录基准（幂等缓存；失败不静默重试，交由调用方再次触发） */
  async function ensureDir() {
    if (dir.value || dirLoading.value) return dir.value;
    dirLoading.value = true;
    dirError.value = null;
    try {
      dir.value = await bridge.workspaceImagesDir();
    } catch (e) {
      // 失败时保持空串：buildAssetSrc 会 fail-closed 返回空，UI 显示占位图
      dirError.value = String(e ?? "").slice(0, 120);
      dir.value = "";
    } finally {
      dirLoading.value = false;
    }
    return dir.value;
  }

  function resetView() {
    scale.value = 1;
    tx.value = 0;
    ty.value = 0;
    dragging.value = false;
    dragOrigin.value = null;
  }

  /** 打开灯箱：先过滤出可渲染项（inline_data_url 与无 rel_path 的不进预览） */
  async function openAt(images: ImageRef[], startIndex = 0) {
    await ensureDir();
    items.value = buildPreviewItems(images, dir.value);
    failed.value = {};
    retries.value = {};
    index.value = items.value.length > 0 ? clampIndex(startIndex, items.value.length) : -1;
    resetView();
    // 没有可预览的落盘图片时不弹空灯箱（契约：inline 图不在本卡范围）
    open.value = items.value.length > 0;
  }

  function close() {
    open.value = false;
    index.value = -1;
    items.value = [];
    failed.value = {};
    retries.value = {};
    resetView();
  }

  function clampIndex(i: number, len: number): number {
    if (len <= 0) return -1;
    if (!Number.isInteger(i)) return 0;
    if (i < 0) return 0;
    return i > len - 1 ? len - 1 : i;
  }

  function next() {
    const i = nextIndex(index.value, items.value.length);
    if (i === index.value) return;
    index.value = i;
    resetView();
  }

  function prev() {
    const i = prevIndex(index.value, items.value.length);
    if (i === index.value) return;
    index.value = i;
    resetView();
  }

  function goto(i: number) {
    const t = clampIndex(i, items.value.length);
    if (t < 0 || t === index.value) return;
    index.value = t;
    resetView();
  }

  function zoomIn() {
    scale.value = scaleForButton(scale.value, 1);
  }

  function zoomOut() {
    scale.value = scaleForButton(scale.value, -1);
  }

  function resetZoom() {
    scale.value = 1;
    tx.value = 0;
    ty.value = 0;
  }

  /** 双击：1× → 2×，其余复位到 1×（冻结契约） */
  function toggleZoom() {
    scale.value = scaleForDoubleClick(scale.value);
    if (scale.value === 1) {
      tx.value = 0;
      ty.value = 0;
    }
  }

  /** 滚轮缩放：到边界即停，不抛出也不回绕 */
  function wheelZoom(deltaY: number) {
    scale.value = scaleForWheel(scale.value, deltaY);
    if (scale.value === 1) {
      tx.value = 0;
      ty.value = 0;
    }
  }

  function startDrag(x: number, y: number) {
    if (!dragEnabled(scale.value)) return;
    dragging.value = true;
    dragOrigin.value = { x, y, tx: tx.value, ty: ty.value };
  }

  function moveDrag(x: number, y: number) {
    if (!dragging.value || !dragOrigin.value) return;
    tx.value = clampTranslate(dragOrigin.value.tx + (x - dragOrigin.value.x), scale.value);
    ty.value = clampOffset(dragOrigin.value.ty + (y - dragOrigin.value.y), scale.value);
  }

  function endDrag() {
    dragging.value = false;
    dragOrigin.value = null;
  }

  /** 单图加载失败：只标记该格/该张，**不关闭灯箱**（冻结契约，纯函数 T-prev-2） */
  function markFailed(id: string) {
    failed.value = markFailedPure(failed.value, id);
  }

  /** 重试：清失败标记 + 递增 nonce（组件据此换 <img> key 重新加载） */
  function retry(id: string) {
    failed.value = clearFailed(failed.value, id);
    retries.value = bumpRetry(retries.value, id);
  }

  function retryNonce(id: string): number {
    return retryNoncePure(retries.value, id);
  }

  function isFailed(id: string): boolean {
    return isFailedPure(failed.value, id);
  }

  return {
    dir,
    dirLoading,
    dirError,
    items,
    index,
    open,
    scale,
    tx,
    ty,
    dragging,
    retries,
    failed,
    current,
    count,
    canDrag,
    zoomLabel,
    ensureDir,
    openAt,
    close,
    next,
    prev,
    goto,
    zoomIn,
    zoomOut,
    resetZoom,
    toggleZoom,
    wheelZoom,
    startDrag,
    moveDrag,
    endDrag,
    markFailed,
    retry,
    retryNonce,
    isFailed,
    clampScale,
  };
});
