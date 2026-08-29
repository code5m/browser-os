<script setup lang="ts">
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";

const ws = useWorkspaceStore();
</script>

<template>
  <div class="tree">
    <div v-for="node in ws.tree.nodes" :key="node.host" class="domain">
      <div class="domain-name">📁 {{ node.host }} <span class="count">{{ node.items.length }}</span></div>
      <div
        v-for="item in node.items"
        :key="item.id"
        class="item"
        :class="{ active: ws.current && ws.current.id === item.id, sel: ws.selected.has(item.id) }"
      >
        <input type="checkbox" :checked="ws.selected.has(item.id)" @click.stop="ws.toggle(item.id)" />
        <div class="item-body" @click="ws.openArtifact(item)" @contextmenu="ws.onArtifactContext($event, item as any)">
          <div class="title">{{ item.title }}</div>
          <div class="meta">
            {{ item.created_at.slice(0, 10) }}
            <span v-for="t in item.tags" :key="t" class="tag">#{{ t }}</span>
          </div>
        </div>
        <button class="del" @click.stop="ws.removeArtifact(item)">✕</button>
      </div>
    </div>
    <div v-if="!ws.flatArtifacts.length" class="empty">暂无成果。打开浏览器 → 右键 → 保存选区/整页</div>

    <!-- 成果右键菜单 -->
    <div
      v-if="ws.ctxMenu.show"
      class="ctx-menu"
      :style="{ left: ws.ctxMenu.x + 'px', top: ws.ctxMenu.y + 'px' }"
      @click.stop
    >
      <div class="ctx-item" @click="ws.ctxReveal">📂 打开所在目录</div>
      <div class="ctx-item" @click="ws.ctxOpenSource">🔗 在浏览器打开来源</div>
      <div class="ctx-item danger" @click="ws.ctxRemove">🗑 删除</div>
    </div>

    <!-- 编辑区 -->
    <div v-if="ws.current" class="editor">
      <h3>编辑成果</h3>
      <input v-model="ws.editTitle" placeholder="标题" />
      <input v-model="ws.editTags" placeholder="标签（逗号分隔）" />
      <textarea v-model="ws.editText" placeholder="正文"></textarea>
      <div class="src">来源：<a :href="ws.current.source_url" target="_blank">{{ ws.current.source_url }}</a></div>
      <div class="src">溯源哈希：{{ ws.current.hash.slice(0, 16) }}</div>
      <button class="primary" @click="ws.saveEdit">保存编辑</button>
    </div>
  </div>
</template>
