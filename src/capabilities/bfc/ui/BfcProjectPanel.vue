<script setup lang="ts">
import { ref } from "vue"
import { useLayoutStore } from "../../../stores/useLayoutStore"
import { useBrowserStore } from "../../browser/public"

const layout = useLayoutStore()
const browser = useBrowserStore()
const latestBusy = ref(false)
const latestNotice = ref("")

const links = [
  { id: "website", icon: "🌐", label: "BFC Web 版", description: "面向用户的专题视频入口", url: "https://github.com/code5m/bcf-website" },
  { id: "workbench", icon: "🧭", label: "BFC 工作台", description: "本地整理、分类与差异监测", url: "http://127.0.0.1:4198/#topics" },
  { id: "updates", icon: "🆕", label: "新发现的收藏", description: "查看本地授权快照发现的待收录视频", url: "http://127.0.0.1:4198/#updates" },
]

async function openProject(url: string) {
  await browser.tabNew(url)
  layout.activateBrowser()
}
// The local BFC workbench currently protects the account after HTTP 412.
// Do not claim that refreshing its local snapshot fetched live Bilibili favorites.
async function acquireLatest() {
  if (latestBusy.value) return
  latestBusy.value = true
  try {
    await openProject("http://127.0.0.1:4198/manage#sync")
    latestNotice.value = "已打开 BFC 数据管理的安全补采页面。请在获准访问的 B站收藏页保存最新 HTML，再导入并点击新发现的收藏核对增量；当前不会直接抓取 B站。"
  } catch {
    latestNotice.value = "无法打开 BFC 工作台。请确认本地 127.0.0.1:4198 服务已经运行。"
  } finally {
    latestBusy.value = false
  }
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
    <div class="bfc-latest">
      <button type="button" class="bfc-refresh" :disabled="latestBusy" @click="acquireLatest">⟳ 获取最新收藏 · 安全补采</button>
      <p role="status" aria-live="polite">{{ latestNotice || "BFC 当前只有本地快照和获准导入流程；本地刷新不等于 B站实时同步。" }}</p>
    </div>
  </section>
</template>

<style scoped>
.bfc-panel { min-height: 100%; padding: 28px; background: var(--bfc-background, linear-gradient(135deg, #f4f7ff, #fff)); color: var(--chrome-text, #3c4043); }
.bfc-head { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; margin-bottom: 24px; }
.bfc-kicker { margin: 0 0 8px; color: var(--bfc-accent, #315ea8); font-size: 12px; letter-spacing: .08em; }
h1 { margin: 0 0 10px; font-size: 28px; }
.bfc-head p:not(.bfc-kicker) { max-width: 620px; margin: 0; color: var(--chrome-text, #3c4043); line-height: 1.7; }
.bfc-badge { border: 1px solid var(--chrome-border, #dadce0); border-radius: 999px; padding: 6px 10px; color: var(--bfc-accent, #315ea8); background: var(--chrome-address, #f1f3f4); white-space: nowrap; font-size: 12px; }
.bfc-links { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 14px; max-width: 760px; }
.bfc-card { display: flex; align-items: center; gap: 14px; padding: 18px; border: 1px solid var(--chrome-border, #dadce0); border-radius: 14px; background: var(--chrome-toolbar, #fff); color: var(--chrome-text, #3c4043); text-align: left; cursor: pointer; box-shadow: 0 4px 18px #315ea80d; }
.bfc-card:hover { border-color: var(--bfc-accent, #315ea8); transform: translateY(-1px); }
.bfc-card:focus-visible, .bfc-refresh:focus-visible { outline: 2px solid var(--bfc-accent, #315ea8); outline-offset: 2px; }
.bfc-icon { font-size: 28px; }
.bfc-copy { display: grid; gap: 5px; flex: 1; }
.bfc-copy small { color: var(--chrome-muted, #70757a); }
.bfc-latest { max-width: 760px; display: grid; gap: 8px; margin-top: 18px; }
.bfc-latest p { margin: 0; color: var(--chrome-text, #3c4043); line-height: 1.6; font-size: 13px; }
.bfc-refresh { justify-self: start; background: var(--chrome-toolbar, #fff); border: 1px solid var(--bfc-accent, #315ea8); border-radius: 8px; padding: 9px 14px; color: var(--bfc-accent, #315ea8); font-weight: 600; cursor: pointer; }
.bfc-refresh:disabled { opacity: .75; cursor: wait; }
@media (max-width: 720px) { .bfc-panel { padding: 18px; } .bfc-head { display: block; } .bfc-badge { display: inline-block; margin-top: 14px; } }
</style>
