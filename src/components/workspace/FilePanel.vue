<script setup lang="ts">
import { onMounted } from "vue";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import FileTreeNode from "./FileTreeNode.vue";

// ide=true：左树右编辑（文件主视图）；ide=false：纯树（浏览视图右侧 Dock 窄栏）
const props = withDefaults(defineProps<{ ide?: boolean }>(), { ide: false });
const ws = useWorkspaceStore();

onMounted(() => {
  if (!ws.treeRoots.length) ws.loadTree();
});
</script>

<template>
  <div class="file-ide" :class="{ 'with-edit': props.ide }">
    <!-- 左：文件夹树（VSCode 资源管理器式） -->
    <div class="ftree">
      <div class="ftree-head">
        <span class="ftree-title">📂 文件</span>
        <button title="刷新" @click="ws.refreshTree">⟳</button>
        <button title="新建文件 / 目录" @click="ws.onFileContext($event)">＋</button>
      </div>
      <div class="ftree-body">
        <FileTreeNode
          v-for="r in ws.treeRoots"
          :key="r.path"
          :entry="r"
          :depth="0"
          :ide="props.ide"
        />
        <div v-if="!ws.treeRoots.length" class="ftree-empty">加载中…</div>
      </div>
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
        >✅ 确认</div>
      </template>
      <template v-else>
        <div class="ctx-item" @click="ws.fileCtx.making = 'file'">📄 新建文件</div>
        <div class="ctx-item" @click="ws.fileCtx.making = 'dir'">📁 新建目录</div>
        <template v-if="ws.fileCtx.entry">
          <div class="ctx-sep"></div>
          <div class="ctx-item" @click="ws.ctxRename(ws.fileCtx.entry)">✏ 重命名</div>
          <div class="ctx-item danger" @click="ws.ctxDelete(ws.fileCtx.entry)">🗑 删除</div>
        </template>
      </template>
    </div>
  </div>
</template>
