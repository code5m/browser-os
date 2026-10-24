<script setup lang="ts">
import { useLayoutStore } from "../../../stores/useLayoutStore"
import { useBrowserStore } from "../../browser/public"

const layout = useLayoutStore()
const browser = useBrowserStore()

const links = [
  { id: "website", icon: "🌐", label: "BFC Web 版", description: "面向用户的专题视频入口", url: "https://github.com/code5m/bcf-website" },
  { id: "workbench", icon: "🧭", label: "BFC 工作台", description: "本地整理、分类与差异监测", url: "http://127.0.0.1:4198/#topics" },
]

async function openProject(url: string) {
  await browser.tabNew(url)
  layout.activateBrowser()
}
</script>

<template>
  <section class="bfc-panel" aria-labelledby="bfc-title">
    <header class="bfc-head">
      <div>
        <p class="bfc-kicker">BFC · Bilibili Favorites Curator</p>
        <h1 id="bfc-title">个人内容库</h1>
        <p>从 Home 进入 Web 版和本地工作台。模块可在能力管理器中停用，已打开的网页页签不会被强制关闭。</p>
      </div>
      <span class="bfc-badge">可装卸模块</span>
    </header>
    <div class="bfc-links">
      <button v-for="link in links" :key="link.id" type="button" class="bfc-card" @click="openProject(link.url)">
        <span class="bfc-icon" aria-hidden="true">{{ link.icon }}</span>
        <span class="bfc-copy"><strong>{{ link.label }}</strong><small>{{ link.description }}</small></span>
        <span aria-hidden="true">↗</span>
      </button>
    </div>
  </section>
</template>

<style scoped>
.bfc-panel { min-height: 100%; padding: 28px; background: linear-gradient(135deg, #f4f7ff, #fff); color: #1d2129; }
.bfc-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; margin-bottom: 24px; }
.bfc-kicker { margin: 0 0 8px; color: #315ea8; font-size: 12px; letter-spacing: .08em; }
h1 { margin: 0 0 10px; font-size: 28px; }
.bfc-head p:not(.bfc-kicker) { max-width: 620px; margin: 0; color: #667085; line-height: 1.7; }
.bfc-badge { border: 1px solid #b9cef2; border-radius: 999px; padding: 6px 10px; color: #315ea8; background: #edf4ff; white-space: nowrap; font-size: 12px; }
.bfc-links { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 14px; max-width: 760px; }
.bfc-card { display: flex; align-items: center; gap: 14px; padding: 18px; border: 1px solid #dfe6f2; border-radius: 14px; background: #fff; color: inherit; text-align: left; cursor: pointer; box-shadow: 0 8px 24px #315ea80d; }
.bfc-card:hover { border-color: #7da4e4; transform: translateY(-1px); }
.bfc-icon { font-size: 28px; }
.bfc-copy { display: grid; gap: 5px; flex: 1; }
.bfc-copy small { color: #667085; }
@media (max-width: 720px) { .bfc-panel { padding: 18px; } .bfc-head { display: block; } .bfc-badge { display: inline-block; margin-top: 14px; } }
</style>
