import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import { bridge } from "../bridge";
import { normalizeHostRect } from "../utils/browserLayout";
import { useBrowserStore } from "../capabilities/browser/public";
import { useLayoutStore } from "../stores/useLayoutStore";
import { isBrowserResourceAllowed } from "../capabilities/browser/resource/guard";
import { hasNonZeroSize, tabPositionKey } from "../utils/browserSync";
import { nativeSurfaceBlocked, trackNativeLayout } from "../utils/nativeSurfaceSync";

type Rect = {x:number;y:number;width:number;height:number};
type PositionIntent = {owner:symbol;id:string;rect:Rect;current:()=>boolean;settled:(ok:boolean)=>void};
// Module-wide ownership survives host remount. Coalesce to the newest
// unsent geometry, rather than starting a competing native positioning call.
let pending: PositionIntent | null = null;
let running: Promise<void> | null = null;
function flush() {
  if (running) return;
  const task = Promise.resolve().then(async () => {
    while (pending) {
      const intent = pending;
      pending = null;
      if (!intent.current()) continue;
      try {
        await bridge.tabPosition(intent.id,intent.rect);
        if (intent.current() && !pending) intent.settled(true);
      } catch {
        if (intent.current() && !pending) intent.settled(false);
      }
    }
  }).finally(() => {
    running=null;
    if (pending) flush();
  });
  running=task;
  trackNativeLayout(task);
}

export function useBrowserHost() {
  const browser = useBrowserStore();
  const layout = useLayoutStore();
  const browserHost = ref<HTMLElement | null>(null);
  const owner=Symbol("browser-host");
  let frame:number|null=null;
  let retryTimer:ReturnType<typeof setTimeout>|null=null;
  let ro:ResizeObserver|null=null;
  let unbind:(()=>void)|undefined;
  let epoch=0, disposed=false;
  let lastKey="";
  function eligible() {
    return !disposed && !nativeSurfaceBlocked.value && !browser.tabRecoveryRequired
      && !browser.tabRecoveryBusy && layout.mainView==="browser" && !!browser.activeTabId
      && !browser.rendererWindowUnavailable(browser.activeTabId) && isBrowserResourceAllowed();
  }
  function invalidate() {
    ++epoch;lastKey="";
    if(frame!==null)cancelAnimationFrame(frame);
    if(retryTimer!==null)clearTimeout(retryTimer);
    frame=null;retryTimer=null;
    if(pending?.owner===owner)pending=null;
  }
  function schedulePosition(retry=0,zeroRetries=0) {
    if(!eligible()||frame!==null)return;
    if(retryTimer!==null)clearTimeout(retryTimer);
    retryTimer=null;
    const admitted=epoch;
    frame=requestAnimationFrame(()=>{
      frame=null;
      if(admitted!==epoch||!eligible())return;
      const r=browserHost.value?.getBoundingClientRect();
      if(!hasNonZeroSize(r)||!r){
        if(zeroRetries<60)schedulePosition(retry,zeroRetries+1);
        else bridge.debugLog("schedulePosition: host bounds unavailable");
        return;
      }
      const rect=normalizeHostRect(r);
      const id=browser.activeTabId;
      const key=tabPositionKey(id,rect.x,rect.y,rect.width,rect.height);
      if(key===lastKey)return;
      lastKey=key;
      pending={
        owner,id,rect,
        current:()=>admitted===epoch&&eligible()&&browser.activeTabId===id,
        settled:(ok)=>{
          if(ok)return;
          lastKey="";
          if(retry<3)retryTimer=setTimeout(()=>{
            retryTimer=null;
            if(admitted===epoch)schedulePosition(retry+1);
          },180);
          else layout.showToast("当前页签定位未确认，请重新激活或同步显示。");
        },
      };
      flush();
    });
  }
  const positionBrowserNow=()=>schedulePosition();
  watch(()=>[layout.mainView,browser.activeTabId,nativeSurfaceBlocked.value,
    browser.tabRecoveryRequired,browser.tabRecoveryBusy,
    browser.rendererWindowUnavailable(browser.activeTabId)] as const,()=>{
      invalidate();if(eligible())schedulePosition();
    },{flush:"sync"});
  onMounted(()=>{
    unbind=browser.bindPositionScheduler(positionBrowserNow);
    void browser.syncViewVisibility();
    window.addEventListener("resize",positionBrowserNow);
    window.addEventListener("tauri://window-resized",positionBrowserNow);
    ro=new ResizeObserver(positionBrowserNow);
    if(browserHost.value)ro.observe(browserHost.value);
  });
  onBeforeUnmount(()=>{
    disposed=true;invalidate();unbind?.();ro?.disconnect();
    window.removeEventListener("resize",positionBrowserNow);
    window.removeEventListener("tauri://window-resized",positionBrowserNow);
  });
  return {browserHost,schedulePosition};
}
