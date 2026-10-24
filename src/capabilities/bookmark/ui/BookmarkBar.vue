<script setup lang="ts">
import { computed, nextTick, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { Folder, Globe, ChevronsRight, X, PanelLeft } from '@lucide/vue';
import { bookmarkTree, type BookmarkNode } from '../model/bookmarkTree';
import { useBookmarkStore } from "../state/useBookmarkStore";
import { useBrowserStore } from "../../browser/public";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { createNativeMenu, menuAnchor, type NativeMenuEntry } from '../../../composables/nativeMenu';

const bookmarks = useBookmarkStore();
const browser = useBrowserStore();
const layout = useLayoutStore();
// Imported metadata has no mixed folder/link ordering; expose folders before loose links.
const roots = computed(() => bookmarkTree(bookmarks.items).sort((a,b)=>Number(!!a.url)-Number(!!b.url)));
const strip = ref<HTMLElement|null>(null), visibleCount = ref(0);
const nativeMenu=createNativeMenu(()=>layout.showToast('无法打开收藏菜单，请在桌面客户端重试'));
let observer: ResizeObserver | undefined;
function measure() {
  if (!strip.value) return;
  let width=0, count=0;
  for (const button of strip.value.querySelectorAll<HTMLElement>('.bookmark-item')) {
    width += button.getBoundingClientRect().width + 3;
    if (width > strip.value.clientWidth) break;
    count++;
  }
  visibleCount.value=count;
}
async function choose(item:BookmarkNode, event:MouseEvent) {
  if (!item.url) { await showFolder(item.children,event); return; }
  await openBookmark(item.url,event.ctrlKey || event.metaKey || event.button===1);
}
async function openBookmark(url:string,newTab=false) {
  if(newTab)await browser.tabNew(url);
  else { browser.url=url; await browser.navigateCurrent(); }
}
function menuEntries(nodes:BookmarkNode[],newTab=false):NativeMenuEntry[] {
  return nodes.map(node=>node.url
    ? {text:node.title,action:()=>openBookmark(node.url!,newTab)}
    : {text:node.title,children:menuEntries(node.children,newTab)});
}
async function showFolder(nodes:BookmarkNode[],event:MouseEvent) {
  layout.closeNavSection();
  await nativeMenu.open(nodes.length ? [...menuEntries(nodes),{text:'在新标签页打开…',children:menuEntries(nodes,true)}] : [{text:'暂无收藏',enabled:false}],menuAnchor(event.currentTarget as Element));
}
async function showContext(item:BookmarkNode,event:MouseEvent) {
  if(!item.url)return showFolder(item.children,event);
  await nativeMenu.open([
    {text:'在当前标签页打开',action:()=>openBookmark(item.url!)},
    {text:'在新标签页打开',action:()=>openBookmark(item.url!,true)},
  ],menuAnchor(event.currentTarget as Element));
}
function shortcut(event:KeyboardEvent) {
  if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.code==='KeyB') {
    event.preventDefault(); bookmarks.toggleBar();
  }
}
watch([roots, () => bookmarks.barVisible, () => layout.mainView], async () => { await nextTick(); measure(); });
watch(strip, element => { observer?.disconnect(); if (element) { observer=new ResizeObserver(measure); observer.observe(element); measure(); } });
onMounted(() => { if (!bookmarks.loaded) void bookmarks.load(); window.addEventListener('keydown', shortcut); });
onBeforeUnmount(() => { nativeMenu.dispose(); observer?.disconnect(); window.removeEventListener('keydown', shortcut); });
</script>

<template>
  <section v-if="bookmarks.barVisible" class="bookmark-surface">
    <nav class="browser-bookmark-bar" aria-label="收藏栏">
      <div ref="strip" class="bookmark-strip">
        <button v-for="(item,index) in roots" :key="item.id" class="bookmark-item"
          :class="{clipped:index>=visibleCount}" :tabindex="index>=visibleCount ? -1 : 0" :aria-hidden="index>=visibleCount"
          :title="item.url || item.title" :aria-haspopup="!item.url ? 'menu' : undefined" @click="choose(item,$event)" @auxclick.middle.prevent="choose(item,$event)" @contextmenu.prevent="showContext(item,$event)">
          <component :is="item.url ? Globe : Folder" :size="15" :class="{folder:!item.url}"/><span>{{item.title}}</span>
        </button>
        <span v-if="!roots.length" class="bookmark-empty">在地址栏点击星标，即可将网页添加到收藏栏</span>
      </div>
      <button class="bookmark-overflow" :class="{invisible:visibleCount>=roots.length}" :disabled="visibleCount>=roots.length" title="更多收藏" aria-label="更多收藏" aria-haspopup="menu" @click="showFolder(roots.slice(visibleCount),$event)"><ChevronsRight :size="17"/></button>
      <span class="bookmark-divider"/>
      <button class="bookmark-all" title="所有收藏" aria-haspopup="menu" @click="showFolder(roots,$event)"><PanelLeft :size="15"/><span>所有收藏</span></button>
      <button title="隐藏收藏栏" aria-label="隐藏收藏栏" @click="bookmarks.toggleBar()"><X :size="14"/></button>
    </nav>
  </section>
</template>

<style scoped>
.bookmark-surface{flex:none;background:var(--chrome-toolbar,#fff);color:var(--chrome-text,#3c4043);border-bottom:1px solid var(--chrome-border,#dadce0)}
.browser-bookmark-bar{display:flex;align-items:center;gap:3px;height:34px;padding:0 10px;box-sizing:border-box;overflow:hidden}
.bookmark-strip{display:flex;align-items:center;gap:3px;flex:1;min-width:0;overflow:hidden;height:100%}
.browser-bookmark-bar button{display:inline-flex;align-items:center;gap:7px;flex:none;max-width:178px;height:28px;padding:0 9px;border:0;border-radius:5px;background:transparent;color:inherit;font:12px/1.4 system-ui,sans-serif;white-space:nowrap;cursor:pointer}
.browser-bookmark-bar button:hover{background:var(--chrome-hover,#eef0f2)}
.bookmark-item span{overflow:hidden;text-overflow:ellipsis;min-width:0}.bookmark-item svg{flex:none}.folder{color:#b88b24}
.bookmark-item.clipped{visibility:hidden;pointer-events:none}.bookmark-overflow.invisible{visibility:hidden}.bookmark-divider{height:18px;width:1px;background:var(--chrome-border,#dadce0);margin:0 5px}
.bookmark-empty{font:12px system-ui,sans-serif;color:#70757a;padding:0 6px;white-space:nowrap}
</style>
