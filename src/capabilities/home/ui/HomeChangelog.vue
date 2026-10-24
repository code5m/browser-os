<script setup lang="ts">
import { ref } from "vue";

const open = ref(false);
const releases = [
  {
    version: "0.1.0",
    date: "2026-10-18",
    title: "开发版 · 工作台体验更新",
    items: [
      "Git Studio 增加历史文件编辑/预览区、只看变更、全文查看和冲突三栏处理布局。",
      "数据库 IDEA 项目导入增加目录选择器、导入数量和跳过原因提示。",
      "定时任务主页改为 Automation 工作台样式，区分 Shell 脚本与命令片段。",
    ],
  },
];
</script>

<template>
  <section class="changelog" aria-labelledby="home-changelog-title">
    <div class="changelog-head">
      <div><h2 id="home-changelog-title">版本更新</h2><p>每次版本更新都在这里留下记录。</p></div>
      <button type="button" @click="open=!open">{{ open ? "收起历史" : `查看历史（${releases.length}）` }}</button>
    </div>
    <article class="release latest">
      <div class="release-meta"><strong>v{{ releases[0].version }}</strong><time>{{ releases[0].date }}</time><span>当前版本</span></div>
      <h3>{{ releases[0].title }}</h3>
      <ul><li v-for="item in releases[0].items" :key="item">{{ item }}</li></ul>
    </article>
    <div v-if="open" class="release-history">
      <article v-for="release in releases.slice(1)" :key="release.version" class="release">
        <div class="release-meta"><strong>v{{ release.version }}</strong><time>{{ release.date }}</time></div>
        <h3>{{ release.title }}</h3><ul><li v-for="item in release.items" :key="item">{{ item }}</li></ul>
      </article>
      <p v-if="releases.length===1" class="empty-history">这是当前项目的第一条版本记录。</p>
    </div>
  </section>
</template>

<style scoped>
.changelog{margin:20px 0 8px;padding:16px 18px;border:1px solid #dce6f0;border-radius:12px;background:linear-gradient(120deg,#f8fbff,#fff)}.changelog-head{display:flex;justify-content:space-between;align-items:center;gap:12px}.changelog h2{margin:0;color:#243b53;font-size:15px}.changelog-head p{margin:4px 0 0;color:#77899d;font-size:12px}.changelog button{border:1px solid #b8cde3;background:#fff;color:#2b6cb0;border-radius:6px;padding:6px 10px;cursor:pointer}.release{margin-top:12px;padding:12px;border:1px solid #e2eaf2;border-radius:9px;background:#fff}.release.latest{border-left:3px solid #2b6cb0}.release-meta{display:flex;align-items:center;gap:10px;color:#6d8093;font-size:11px}.release-meta strong{font-size:13px;color:#2b6cb0}.release-meta span{padding:2px 6px;border-radius:10px;background:#eaf3ff;color:#2b6cb0}.release h3{margin:7px 0 4px;font-size:13px;color:#334e68}.release ul{margin:0;padding-left:18px;color:#62778c;font-size:12px;line-height:1.7}.empty-history{color:#8798a9;font-size:12px}
</style>
