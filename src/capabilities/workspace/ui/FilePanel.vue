<script setup lang="ts">
import { ContextMenu, ContextMenuItem } from "../../../shared/ui";
import { onMounted, ref, watch } from "vue";
import { useFileStore } from "../state/useFileStore";
import { useWorkbenchStore } from "../../../stores/useWorkbenchStore";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import FileTreeNode from "./FileTreeNode.vue";
import { Crosshair, ChevronsDownUp, ChevronsUpDown, FilePlus, FolderPlus, RefreshCw } from "@lucide/vue";

// ide=true：左树右编辑（文件主视图）；ide=false：纯树（浏览视图右侧 Dock 窄栏）
const props = withDefaults(defineProps<{ ide?: boolean }>(), { ide: false });
const ws = useFileStore();
const workbench = useWorkbenchStore();
const layout = useLayoutStore();

// 文件夹树宽度拖拽调整（鼠标按住右边缘手柄左右拖）
function startResize(e: MouseEvent) {
  const startX = e.clientX;
  const startW = layout.fileTreeWidth;
  const move = (ev: MouseEvent) => layout.setFileTreeWidth(startW + ev.clientX - startX);
  const up = () => {
    window.removeEventListener("mousemove", move);
    window.removeEventListener("mouseup", up);
  };
  window.addEventListener("mousemove", move);
  window.addEventListener("mouseup", up);
}

const ftreeBody = ref<HTMLElement | null>(null);
// Local presentation-only menu state; file operations remain owned by useFileStore.
const fileActionsOpen = ref(false);

const vLazyThumb = {
  mounted(el: HTMLElement, binding: { value: () => void }) {
    if (!("IntersectionObserver" in window)) {
      binding.value();
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        binding.value();
        observer.disconnect();
      },
      { rootMargin: "480px 0px" }
    );
    observer.observe(el);
    (el as HTMLElement & { __lazyThumbCleanup?: () => void }).__lazyThumbCleanup = () => observer.disconnect();
  },
  unmounted(el: HTMLElement) {
    (el as HTMLElement & { __lazyThumbCleanup?: () => void }).__lazyThumbCleanup?.();
  },
};

function scrollToLocate() {
  const el = ftreeBody.value?.querySelector(".tnode.locate") as HTMLElement | null;
  el?.scrollIntoView({ block: "center" });
}

// 定位目标变化后滚动到可视区域
watch(
  () => ws.locateTarget,
  () => setTimeout(scrollToLocate, 80)
);

onMounted(async () => {
  if (!ws.treeRoots.length) await ws.loadTree();
  // 打开软件 / 进入目录后自动定位到当前打开位置
  ws.locateCurrent();
  setTimeout(scrollToLocate, 140);
});
</script>

<template>
  <div class="file-ide" :class="{ 'with-edit': props.ide }">
    <!-- 左：文件夹树（VSCode 资源管理器式） -->
    <div v-show="!workbench.collapsed" class="ftree" :style="{ width: layout.fileTreeWidth + 'px' }">
      <div class="ftree-head">
        <span class="ftree-title">文件</span>
        <button title="新建文件" aria-label="新建文件" @click="ws.quickNew('file')"><FilePlus :size="15" /></button>
        <button title="刷新目录" aria-label="刷新目录" @click="ws.refreshTree"><RefreshCw :size="15" /></button>
        <button title="更多文件操作" aria-label="更多文件操作" :aria-expanded="fileActionsOpen" aria-controls="file-actions" @click="fileActionsOpen=!fileActionsOpen">···</button>
      </div>
      <div v-if="fileActionsOpen" id="file-actions" class="file-actions" role="group" aria-label="文件操作">
        <button @click="ws.quickNew('dir');fileActionsOpen=false"><FolderPlus :size="14" /> 新建文件夹</button>
        <button @click="ws.locateCurrent();fileActionsOpen=false"><Crosshair :size="14" /> 定位当前目录</button>
        <button @click="ws.expandAllTree();fileActionsOpen=false"><ChevronsUpDown :size="14" /> 全部展开</button>
        <button @click="ws.collapseAllTree();fileActionsOpen=false"><ChevronsDownUp :size="14" /> 全部折叠</button>
      </div>
      <div class="ftree-body" ref="ftreeBody">
        <FileTreeNode
          v-for="r in ws.treeRoots"
          :key="r.path"
          :entry="r"
          :depth="0"
          :ide="props.ide"
        />
        <div v-if="!ws.treeRoots.length" class="ftree-empty">加载中…</div>
      </div>
      <div class="ftree-resizer" @mousedown.prevent="startResize" title="拖拽调整宽度"></div>
    </div>

    <!-- 右：预览 / 编辑（IDE 模式） -->
    <div v-if="props.ide" class="fedit">
      <template v-if="ws.inlineFile">
        <div class="fedit-head">
          <span class="fedit-name" :title="ws.inlineFile">{{ ws.inlineFile.split("/").pop() }}</span>
          <div class="fedit-actions">
            <button v-if="ws.inlineIsMd" @click="ws.inlineToggleEdit">
              {{ ws.inlineEdit ? "👁 预览" : "✎ 编辑" }}
            </button>
            <button class="primary" @click="ws.saveInline">💾 保存</button>
            <button @click="ws.closeInline" title="关闭">✕</button>
          </div>
        </div>
        <div
          v-if="ws.inlineIsMd && !ws.inlineEdit"
          class="md-preview fedit-body"
          v-html="ws.inlineHtml"
        ></div>
        <textarea
          v-else
          :value="ws.inlineText" @input="ws.setInlineText(($event.target as HTMLTextAreaElement).value)"
          class="file-editor fedit-body"
          spellcheck="false"
          placeholder="文件内容..."
        ></textarea>
      </template>
      <template v-else-if="ws.previewDir">
        <div class="fedit-head">
          <span class="fedit-name" :title="ws.previewDir">{{ ws.previewDir.split("/").pop() || ws.previewDir }}</span>
          <div class="fedit-actions">
            <label class="thumb-size">
              缩略图
              <input
                type="range"
                min="72"
                max="260"
                :value="ws.previewTileSize"
                @input="ws.setPreviewTileSize(Number(($event.target as HTMLInputElement).value))"
              />
            </label>
            <button v-if="ws.compareImages.length" @click="ws.clearCompareImages">清空对比</button>
          </div>
        </div>
        <div class="dir-preview fedit-body">
          <div v-if="ws.previewLoading" class="dir-preview-empty">加载中…</div>
          <div v-else-if="ws.previewError" class="dir-preview-empty warn">{{ ws.previewError }}</div>
          <template v-else>
            <div v-if="ws.compareImages.length" class="compare-strip">
              <div
                v-for="img in ws.compareImages"
                :key="img.path"
                class="compare-card"
                :style="{ width: ws.previewTileSize * 1.4 + 'px' }"
              >
                <img v-if="ws.previewImages[img.path]" :src="ws.previewImages[img.path]" :alt="img.name" />
                <div v-else v-lazy-thumb="() => ws.loadPreviewImage(img)" class="compare-loading">加载中</div>
                <div class="compare-actions">
                  <button title="左移" @click="ws.moveCompareImage(img.path, -1)">←</button>
                  <span :title="img.path">{{ img.name }}</span>
                  <button title="右移" @click="ws.moveCompareImage(img.path, 1)">→</button>
                  <button title="移出对比" @click="ws.toggleCompareImage(img)">×</button>
                </div>
              </div>
            </div>
            <div
              v-if="ws.previewEntries.some(ws.isImageEntry)"
              class="thumb-grid"
              :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${ws.previewTileSize}px, 1fr))` }"
            >
              <button
                v-for="img in ws.previewEntries.filter(ws.isImageEntry)"
                :key="img.path"
                class="thumb-card"
                :class="{ selected: ws.compareImages.some((it) => it.path === img.path) }"
                :title="img.path"
                v-lazy-thumb="() => ws.loadPreviewImage(img)"
                @click="ws.toggleCompareImage(img)"
              >
                <img v-if="ws.previewImages[img.path]" :src="ws.previewImages[img.path]" :alt="img.name" />
                <span v-else-if="ws.previewImageErrors[img.path]" class="thumb-missing" :title="ws.previewImageErrors[img.path]">无法预览</span>
                <span v-else class="thumb-missing">加载中</span>
                <small>{{ img.name }}</small>
              </button>
            </div>
            <div class="dir-list">
              <button
                v-for="entry in ws.previewEntries.filter((it) => !ws.isImageEntry(it))"
                :key="entry.path"
                class="dir-row"
                :title="entry.path"
                @click="entry.is_dir ? ws.openDirPreview(entry) : ws.openFileInline(entry)"
              >
                <span>{{ entry.is_dir ? "📁" : "📄" }}</span>
                <span>{{ entry.name }}</span>
              </button>
            </div>
            <div v-if="!ws.previewEntries.length" class="dir-preview-empty">空目录</div>
          </template>
        </div>
      </template>
      <div v-else class="fedit-empty">
        <p>👈 点击左侧文件查看 / 编辑</p>
        <p class="dim">文件夹点击展开，右键可新建 / 重命名 / 删除</p>
      </div>
    </div>

    <!-- 文件右键菜单 -->
    <ContextMenu
      v-if="ws.fileCtx.show"
      :x="ws.fileCtx.x"
      :y="ws.fileCtx.y"
      @contextmenu.prevent
    >
      <template v-if="ws.fileCtx.making">
        <div class="ctx-title">{{ ws.fileCtx.making === "file" ? "新建文件" : "新建目录" }}</div>
        <input
          v-model="ws.fileCtx.newName"
          class="ctx-input"
          :placeholder="ws.fileCtx.making === 'file' ? '文件名' : '目录名'"
          @keyup.enter="ws.fileCtx.making === 'file' ? ws.ctxNewFile() : ws.ctxNewDir()"
        />
        <ContextMenuItem
          @click="ws.fileCtx.making === 'file' ? ws.ctxNewFile() : ws.ctxNewDir()"
        >✅ 确认    </ContextMenuItem>
      </template>
      <template v-else>
        <ContextMenuItem @click="ws.fileCtx.making = 'file'">📄 新建文件</ContextMenuItem>
        <ContextMenuItem @click="ws.fileCtx.making = 'dir'">📁 新建目录</ContextMenuItem>
        <template v-if="ws.fileCtx.entry">
          <div class="ctx-sep"></div>
          <ContextMenuItem @click="ws.ctxOpenInNewTab(ws.fileCtx.entry)">📑 新标签打开</ContextMenuItem>
          <ContextMenuItem @click="ws.ctxOpenInExplorer(ws.fileCtx.entry)">🗂 资源管理器打开</ContextMenuItem>
          <ContextMenuItem @click="ws.ctxOpenInTerminal(ws.fileCtx.entry)">💻 命令行终端打开</ContextMenuItem>
          <ContextMenuItem v-if="ws.fileCtx.entry.is_dir" @click="ws.ctxFavorite(ws.fileCtx.entry)">☆ 收藏到主页</ContextMenuItem>
          <ContextMenuItem @click="ws.copyPath(ws.fileCtx.entry, true)">复制相对路径</ContextMenuItem>
          <ContextMenuItem @click="ws.copyPath(ws.fileCtx.entry, false)">复制绝对路径</ContextMenuItem>
          <div class="ctx-sep"></div>
          <ContextMenuItem @click="ws.ctxRename(ws.fileCtx.entry)">✏ 重命名</ContextMenuItem>
          <ContextMenuItem danger @click="ws.ctxDelete(ws.fileCtx.entry)">🗑 删除</ContextMenuItem>
        </template>
      </template>
    </ContextMenu>

    <!-- 移动确认 -->
    <div
      v-if="ws.moveConfirm.show"
      class="modal-mask"
      @click="ws.cancelMove"
      @contextmenu.prevent
    >
      <div class="move-confirm-box" @click.stop>
        <div class="mc-title">确认移动</div>
        <div class="mc-body">
          将 <b>{{ ws.moveConfirm.name }}</b> 移动到<br />
          <span class="dim">{{ ws.moveConfirm.dst }}</span> ？
        </div>
        <div class="mc-actions">
          <button class="primary" @click="ws.confirmMove">移动</button>
          <button @click="ws.cancelMove">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ftree { position:relative; background:#fafcff; }
.ftree-head { background:#f7f9fc; gap:5px; padding:7px; }
.ftree-head button { border:0; background:transparent; padding:5px; border-radius:7px; }
.file-actions { display:flex; flex-direction:column; padding:5px; border-bottom:1px solid #e4eaf2; }
.file-actions button { display:flex; align-items:center; gap:8px; padding:7px 11px; text-align:left; border:0; background:transparent; color:#344054; border-radius:6px; font-size:12px; }
.file-actions button:hover { background:#edf3fc; }
.ftree-resizer {
  position: absolute;
  top: 0;
  right: -3px;
  width: 6px;
  height: 100%;
  cursor: col-resize;
  z-index: 5;
}
.ftree-resizer:hover {
  background: rgba(91, 157, 255, 0.45);
}
/* 移动确认：复用全局 .modal-mask（已批准固定浮层模式），本组件内不再使用 position: fixed */
.move-confirm-box {
  background: #fff;
  border-radius: 8px;
  padding: 16px 18px;
  min-width: 280px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25);
}
.mc-title {
  font-weight: 600;
  margin-bottom: 8px;
}
.mc-body {
  font-size: 13px;
  color: #4e5969;
  margin-bottom: 14px;
  line-height: 1.6;
}
.mc-body .dim {
  color: #9096a1;
  word-break: break-all;
}
.mc-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.mc-actions button {
  border: 1px solid #d5dbe7;
  background: #fff;
  color: #4e5969;
  padding: 5px 14px;
  border-radius: 5px;
  cursor: pointer;
}
.mc-actions button.primary {
  background: #2b6cb0;
  border-color: #2b6cb0;
  color: #fff;
}
.thumb-size {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #4e5969;
}
.thumb-size input {
  width: 110px;
}
.dir-preview {
  overflow: auto;
  padding: 10px;
  background: #f7f9fb;
}
.dir-preview-empty {
  color: #86909c;
  padding: 24px;
  text-align: center;
}
.dir-preview-empty.warn {
  color: #b45309;
}
.compare-strip {
  display: flex;
  gap: 10px;
  overflow-x: auto;
  padding: 0 0 10px;
  margin-bottom: 10px;
  border-bottom: 1px solid #dfe5ec;
}
.compare-card {
  flex: none;
  background: #fff;
  border: 1px solid #dfe5ec;
  border-radius: 6px;
  overflow: hidden;
}
.compare-card img {
  width: 100%;
  height: 180px;
  display: block;
  object-fit: contain;
  background: #111827;
}
.compare-loading {
  height: 180px;
  display: grid;
  place-items: center;
  background: #111827;
  color: #cbd5e1;
  font-size: 12px;
}
.compare-actions {
  display: grid;
  grid-template-columns: 26px 1fr 26px 26px;
  gap: 3px;
  align-items: center;
  padding: 5px;
}
.compare-actions span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
}
.compare-actions button {
  width: 24px;
  height: 24px;
  padding: 0;
}
.thumb-grid {
  display: grid;
  gap: 10px;
  align-items: start;
}
.thumb-card {
  min-width: 0;
  border: 1px solid #dfe5ec;
  background: #fff;
  border-radius: 6px;
  padding: 6px;
  cursor: pointer;
}
.thumb-card.selected {
  border-color: #2b6cb0;
  box-shadow: inset 0 0 0 2px rgba(43, 108, 176, .18);
}
.thumb-card img,
.thumb-missing {
  width: 100%;
  aspect-ratio: 1 / 1;
  display: grid;
  place-items: center;
  object-fit: cover;
  background: #eef1f6;
  border-radius: 4px;
  color: #86909c;
  font-size: 12px;
}
.thumb-card small {
  display: block;
  margin-top: 5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  color: #4e5969;
}
.dir-list {
  display: grid;
  gap: 4px;
  margin-top: 10px;
}
.dir-row {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: flex-start;
  border: 1px solid #e5e6eb;
  background: #fff;
  border-radius: 5px;
  padding: 6px 8px;
  font-size: 12px;
}
</style>
