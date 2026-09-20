<script setup lang="ts">
import { useArtifactStore } from "../../stores/useArtifactStore";
import ImageGallery from "../shared/ImageGallery.vue";

const art = useArtifactStore();
</script>

<template>
  <div class="tree">
    <div v-for="node in art.tree.nodes" :key="node.host" class="domain">
      <div class="domain-name">📁 {{ node.host }} <span class="count">{{ node.items.length }}</span></div>
      <div
        v-for="item in node.items"
        :key="item.id"
        class="item"
        :class="{ active: art.current && art.current.id === item.id, sel: art.selected.has(item.id) }"
      >
        <input type="checkbox" :checked="art.selected.has(item.id)" @click.stop="art.toggle(item.id)" />
        <div class="item-body" @click="art.openArtifact(item)" @contextmenu="art.onArtifactContext($event, item as any)">
          <div class="title">{{ item.title }}</div>
          <div class="meta">
            {{ item.created_at.slice(0, 10) }}
            <span v-for="t in item.tags" :key="t" class="tag">#{{ t }}</span>
          </div>
        </div>
        <button class="del" @click.stop="art.removeArtifact(item)">✕</button>
      </div>
    </div>
    <div v-if="!art.flatArtifacts.length" class="empty">暂无成果。打开浏览器 → 右键 → 保存选区/整页</div>

    <!-- 成果右键菜单 -->
    <div
      v-if="art.ctxMenu.show"
      class="ctx-menu"
      :style="{ left: art.ctxMenu.x + 'px', top: art.ctxMenu.y + 'px' }"
      @click.stop
    >
      <div class="ctx-item" @click="art.ctxReveal">📂 打开所在目录</div>
      <div class="ctx-item" @click="art.ctxOpenSource">🔗 在浏览器打开来源</div>
      <div class="ctx-item danger" @click="art.ctxRemove">🗑 删除</div>
    </div>

    <!-- 编辑区 -->
    <div v-if="art.current" class="editor">
      <h3>编辑成果</h3>
      <input :value="art.editTitle" @input="art.setEditTitle(($event.target as HTMLInputElement).value)" placeholder="标题" />
      <input :value="art.editTags" @input="art.setEditTags(($event.target as HTMLInputElement).value)" placeholder="标签（逗号分隔）" />
      <textarea :value="art.editText" @input="art.setEditText(($event.target as HTMLTextAreaElement).value)" placeholder="正文"></textarea>
      <!-- M2-2.b 图片画廊挂点：只预览已落盘的图片附件（inline 图不在本卡范围） -->
      <ImageGallery :images="art.current.images ?? []" />
      <div class="src">来源：<a :href="art.current.source_url" target="_blank">{{ art.current.source_url }}</a></div>
      <div class="src">溯源哈希：{{ art.current.hash.slice(0, 16) }}</div>
      <button class="primary" @click="art.saveEdit">保存编辑</button>
    </div>
  </div>
</template>
