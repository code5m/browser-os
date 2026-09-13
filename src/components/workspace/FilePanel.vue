<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import { useWorkbenchStore } from "../../stores/useWorkbenchStore";
import { useLayoutStore } from "../../stores/useLayoutStore";
import FileTreeNode from "./FileTreeNode.vue";

// ide=true：左树右编辑（文件主视图）；ide=false：纯树（浏览视图右侧 Dock 窄栏）
const props = withDefaults(defineProps<{ ide?: boolean }>(), { ide: false });
const ws = useWorkspaceStore();
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
        <span class="ftree-title">📂 文件</span>
        <button title="刷新" @click="ws.refreshTree">⟳</button>
        <button title="定位到当前打开位置" @click="ws.locateCurrent">🎯</button>
        <button title="新建文件" @click="ws.quickNew('file')">📄</button>
        <button title="新建目录" @click="ws.quickNew('dir')">📁</button>
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
          v-model="ws.inlineText"
          class="file-editor fedit-body"
          spellcheck="false"
          placeholder="文件内容..."
        ></textarea>
      </template>
      <div v-else class="fedit-empty">
        <p>👈 点击左侧文件查看 / 编辑</p>
        <p class="dim">文件夹点击展开，右键可新建 / 重命名 / 删除</p>
      </div>
    </div>

    <!-- 文件右键菜单 -->
    <div
      v-if="ws.fileCtx.show"
      class="ctx-menu"
      :style="{ left: ws.fileCtx.x + 'px', top: ws.fileCtx.y + 'px' }"
      @click.stop
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
        <div
          class="ctx-item"
          @click="ws.fileCtx.making === 'file' ? ws.ctxNewFile() : ws.ctxNewDir()"
        >✅ 确认    </div>
      </template>
      <template v-else>
        <div class="ctx-item" @click="ws.fileCtx.making = 'file'">📄 新建文件</div>
        <div class="ctx-item" @click="ws.fileCtx.making = 'dir'">📁 新建目录</div>
        <template v-if="ws.fileCtx.entry">
          <div class="ctx-sep"></div>
          <div class="ctx-item" @click="ws.ctxOpenInNewTab(ws.fileCtx.entry)">📑 新标签打开</div>
          <div class="ctx-item" @click="ws.ctxOpenInExplorer(ws.fileCtx.entry)">🗂 资源管理器打开</div>
          <div class="ctx-item" @click="ws.ctxOpenInTerminal(ws.fileCtx.entry)">💻 命令行终端打开</div>
          <div class="ctx-sep"></div>
          <div class="ctx-item" @click="ws.ctxRename(ws.fileCtx.entry)">✏ 重命名</div>
          <div class="ctx-item danger" @click="ws.ctxDelete(ws.fileCtx.entry)">🗑 删除</div>
        </template>
      </template>
    </div>

    <!-- 移动确认 -->
    <div
      v-if="ws.moveConfirm.show"
      class="move-confirm"
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
.ftree {
  position: relative;
}
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
.move-confirm {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.25);
  display: grid;
  place-items: center;
  z-index: 50;
}
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
</style>
